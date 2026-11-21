//! Runs Claude Code without a window (`claude -p`) to break a request into
//! an asset spec, and keeps what it does as a job the graph page can follow.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::SystemTime;

const SKILL: &str = include_str!("../../../skills/asset/SKILL.md");
const CONTEXT_SKILL: &str = include_str!("../../../skills/context/SKILL.md");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub asked: String,
    /// running | done | failed
    pub status: String,
    /// One line per thing the agent does, for the page.
    pub log: Vec<String>,
    /// The spec it wrote, relative to the project root.
    pub spec: Option<String>,
    /// What the agent said to the person, without the `kiln` block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
    /// What the agent offers for the wizard, from its `kiln` block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<kiln_core::chat::Offer>,
    /// ask | layout | sort | refine | chat, or the first word of a tool job
    /// (montar, revisar, ...).
    pub kind: String,
    /// Unix seconds when the job was created.
    pub started: u64,
    /// The project folder the job works on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// It writes the project's builds and caches (Blender, Godot, imports):
    /// only one such job runs at a time in a project.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub writes: bool,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub use crate::jobs::Jobs;

pub struct Ctx {
    pub project: PathBuf,
    pub config: Option<PathBuf>,
    pub specs_dir: String,
    /// Folders the page may read files from (models, textures): the project,
    /// its build and its cache.
    pub roots: Vec<PathBuf>,
    /// The context of the project (`[context] file`), absolute.
    pub context_file: PathBuf,
    pub layers: Vec<kiln_core::context::Layer>,
}

/// What the agent is asked: a new asset, or to place the parts of a spec.
#[derive(Debug, Clone)]
pub enum Task {
    Ask(String),
    Layout(String),
    /// Sort the person's explanation into claims, layer by layer.
    Sort,
    /// Ask a round of questions with suggested answers, on one layer or all.
    Refine(Option<String>),
}

pub fn start(jobs: &Jobs, ctx: Arc<Ctx>, task: Task) -> u64 {
    let asked = match &task {
        Task::Ask(t) => t.clone(),
        Task::Layout(f) => format!("colocar las piezas de {f}"),
        Task::Sort => "ordenar el contexto del proyecto".into(),
        Task::Refine(None) => "preguntas para afinar el contexto".into(),
        Task::Refine(Some(l)) => format!("preguntas sobre la capa {l}"),
    };
    let kind = match &task {
        Task::Ask(_) => "ask",
        Task::Layout(_) => "layout",
        Task::Sort => "sort",
        Task::Refine(_) => "refine",
    };
    let id = {
        let mut j = jobs.lock().unwrap();
        let id = j.len() as u64 + 1;
        j.push(Job {
            id,
            asked: asked.clone(),
            status: "running".into(),
            log: vec!["Leyendo el catálogo del proyecto".into()],
            spec: None,
            reply: None,
            offer: None,
            kind: kind.into(),
            started: now_secs(),
            project: Some(ctx.project.display().to_string()),
            writes: false,
        });
        id
    };
    let jobs = jobs.clone();
    std::thread::spawn(move || {
        let result = run(&jobs, id, &ctx, &task);
        let mut j = jobs.lock().unwrap();
        let job = &mut j[(id - 1) as usize];
        match result {
            Ok(spec) => {
                job.log.push(match &spec {
                    Some(s) => format!("Escrito: {s}"),
                    None => "El agente terminó sin escribir nada".into(),
                });
                job.status = if spec.is_some() { "done" } else { "failed" }.into();
                job.spec = spec;
            }
            Err(e) => {
                job.log.push(format!("Error: {e}"));
                job.status = "failed".into();
            }
        }
    });
    id
}

/// A job that is not the agent (Blender, Godot): `work` says its lines and
/// returns the file it made, relative to the project root. Tests use it; Kiln
/// itself always launches jobs in a project (`job_in`).
#[cfg(test)]
pub fn job<F>(jobs: &Jobs, asked: String, work: F) -> u64
where
    F: FnOnce(&dyn Fn(String)) -> Result<Option<String>, String> + Send + 'static,
{
    spawn_job(jobs, None, asked, work).expect("a job outside any project always starts")
}

/// A job that writes the builds and caches of `project`. It is refused while
/// another such job runs in the same project, so the page and every MCP
/// session never build, paint or publish over each other.
pub fn job_in<F>(jobs: &Jobs, project: &Path, asked: String, work: F) -> Result<u64, String>
where
    F: FnOnce(&dyn Fn(String)) -> Result<Option<String>, String> + Send + 'static,
{
    spawn_job(jobs, Some(project), asked, work)
}

/// The write job running in `project`, said as an error, if there is one.
pub fn busy(jobs: &Jobs, project: &Path) -> Result<(), String> {
    let p = project.display().to_string();
    match jobs.lock().unwrap().iter().find(|j| j.writes && j.status == "running" && j.project.as_deref() == Some(p.as_str())) {
        Some(b) => Err(format!(
            "ya hay un trabajo en marcha en este proyecto (trabajo {}, «{}»): espera a que termine (`job` con su id) antes de lanzar otro",
            b.id, b.asked
        )),
        None => Ok(()),
    }
}

fn spawn_job<F>(jobs: &Jobs, project: Option<&Path>, asked: String, work: F) -> Result<u64, String>
where
    F: FnOnce(&dyn Fn(String)) -> Result<Option<String>, String> + Send + 'static,
{
    let project = project.map(|p| p.display().to_string());
    let id = {
        let mut j = jobs.lock().unwrap();
        if let Some(p) = &project
            && let Some(b) = j.iter().find(|j| j.writes && j.status == "running" && j.project.as_ref() == Some(p))
        {
            return Err(format!(
                "ya hay un trabajo en marcha en este proyecto (trabajo {}, «{}»): espera a que termine (`job` con su id) antes de lanzar otro",
                b.id, b.asked
            ));
        }
        let id = j.len() as u64 + 1;
        let kind = asked.split_whitespace().next().unwrap_or("job").to_string();
        j.push(Job {
            id,
            asked,
            status: "running".into(),
            log: Vec::new(),
            spec: None,
            reply: None,
            offer: None,
            kind,
            started: now_secs(),
            writes: project.is_some(),
            project,
        });
        id
    };
    let jobs = jobs.clone();
    std::thread::spawn(move || {
        let say = |l: String| log(&jobs, id, l);
        let result = work(&say);
        let mut j = jobs.lock().unwrap();
        let job = &mut j[(id - 1) as usize];
        match result {
            Ok(file) => {
                job.status = "done".into();
                job.spec = file;
            }
            Err(e) => {
                job.log.push(format!("Error: {e}"));
                job.status = "failed".into();
            }
        }
    });
    Ok(id)
}

fn log(jobs: &Jobs, id: u64, line: String) {
    if let Some(job) = jobs.lock().unwrap().get_mut((id - 1) as usize) {
        job.log.push(line);
    }
}

impl Ctx {
    /// The thread of the chat with the agent, next to the context.
    pub fn chat_file(&self) -> PathBuf {
        self.context_file.with_file_name("chat.toml")
    }
}

/// One message of the person to the agent, at one step of the wizard or in the Context tab.
pub struct Turn {
    pub step: String,
    pub message: String,
    pub history: Vec<kiln_core::chat::Message>,
    pub draft: Option<Value>,
}

const CHAT_WINDOW: usize = 20;

/// What the page's draft already fixes before the project exists: the name
/// and each basic the person chose, with its label.
pub fn draft_settled(draft: Option<&Value>) -> String {
    let Some(d) = draft else { return String::new() };
    let mut out = String::new();
    if let Some(n) = d.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()) {
        out.push_str(&format!("- Nombre: {n}\n"));
    }
    for b in &kiln_core::new_game::BASICS {
        if let Some(a) = d.pointer(&format!("/basics/{}", b.id)).and_then(Value::as_str).map(str::trim).filter(|a| !a.is_empty()) {
            out.push_str(&format!("- {}: {a}\n", b.label));
        }
    }
    out
}

/// `settled` is what the person already fixed (`draft_settled` or
/// `Context::settled_part`): the prompt states it so the agent builds on it.
pub fn chat_prompt(
    turn: &Turn,
    history: &[kiln_core::chat::Message],
    ctx: Option<&Ctx>,
    settled: &str,
) -> String {
    let settled = if settled.is_empty() {
        String::new()
    } else {
        format!("\nYa fijado por la persona (cuenta con ello y no lo contradigas; si crees que algo debe cambiar, dilo y propónlo):\n{settled}")
    };
    let setting = match ctx {
        None => format!(
            "Aún no hay proyecto: no escribas ficheros.\nBorrador del asistente: {}\nOpciones básicas: {}",
            turn.draft.as_ref().map(Value::to_string).unwrap_or_else(|| "{}".into()),
            serde_json::to_string(&kiln_core::new_game::BASICS).unwrap_or_default()
        ),
        Some(c) => format!(
            "Contexto del proyecto: {}\nCapas: {}\nEdita solo el fichero de contexto.",
            c.context_file.display(),
            serde_json::to_string(&c.layers).unwrap_or_default()
        ),
    };
    let thread = history[history.len().saturating_sub(CHAT_WINDOW)..]
        .iter()
        .map(|m| {
            let who = if m.who == "agent" { "agente" } else { "persona" };
            format!("{who} (paso {}): {}", m.step, m.text)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Conversas con la persona. Sigue la skill kiln-context que tienes en tu prompt de sistema (modo conversar).\nPaso: {}\n{setting}\n{settled}\nHilo:\n{thread}\n\nMensaje nuevo de la persona: {}",
        turn.step, turn.message
    )
}

/// The reply as the thread keeps it: the text plus its offer, numbered as
/// the buttons show it, so a later turn can refer to «the second one».
pub fn with_offer(reply: Option<String>, offer: Option<&kiln_core::chat::Offer>) -> Option<String> {
    let summary = offer.map(kiln_core::chat::Offer::summary).filter(|s| !s.is_empty());
    match (reply, summary) {
        (Some(r), Some(s)) => Some(format!("{r}\n\n{s}")),
        (r, s) => r.or(s),
    }
}

pub fn chat_tools(ctx: Option<&Ctx>) -> Vec<String> {
    let mut tools = vec!["Read", "Glob", "Grep"];
    if ctx.is_some() {
        tools.extend(["Write", "Edit"]);
    }
    tools.into_iter().map(String::from).collect()
}

/// A chat turn: its own path, because it must also work before the project exists.
pub fn chat(jobs: &Jobs, project: PathBuf, ctx: Option<Arc<Ctx>>, turn: Turn) -> u64 {
    let id = {
        let mut j = jobs.lock().unwrap();
        let id = j.len() as u64 + 1;
        j.push(Job {
            id,
            asked: turn.message.chars().take(80).collect(),
            status: "running".into(),
            log: vec!["Leyendo el hilo".into()],
            spec: None,
            reply: None,
            offer: None,
            kind: "chat".into(),
            started: now_secs(),
            project: Some(project.display().to_string()),
            writes: false,
        });
        id
    };
    let jobs = jobs.clone();
    std::thread::spawn(move || {
        let result = run_chat(&jobs, id, &project, ctx.as_deref(), &turn);
        let mut j = jobs.lock().unwrap();
        let job = &mut j[(id - 1) as usize];
        match result {
            Ok((reply, offer)) => {
                job.reply = reply;
                job.offer = offer;
                job.status = "done".into();
            }
            Err(e) => {
                job.log.push(format!("Error: {e}"));
                job.status = "failed".into();
            }
        }
    });
    id
}

type ChatReply = (Option<String>, Option<kiln_core::chat::Offer>);

fn run_chat(
    jobs: &Jobs,
    id: u64,
    project: &Path,
    ctx: Option<&Ctx>,
    turn: &Turn,
) -> Result<ChatReply, String> {
    let chat_file = ctx.map(Ctx::chat_file);
    let history = match &chat_file {
        Some(f) => kiln_core::chat::load(f)?.messages,
        None => turn.history.clone(),
    };
    let before = match ctx {
        Some(c) => Some(kiln_core::context::load(&c.context_file)?),
        None => None,
    };
    let settled = match (ctx, &before) {
        (Some(c), Some(b)) => b.settled_part(&c.layers),
        _ => draft_settled(turn.draft.as_ref()),
    };
    let prompt = chat_prompt(turn, &history, ctx, &settled);
    let mut cmd = claude();
    cmd.args(["-p", &prompt, "--output-format", "stream-json", "--verbose", "--append-system-prompt", CONTEXT_SKILL])
        .arg("--allowedTools")
        .args(chat_tools(ctx))
        .args(["--disallowedTools", "Bash"]);
    if let Some(dir) = ctx.and_then(|c| c.context_file.parent()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        cmd.args(["--permission-mode", "acceptEdits", "--add-dir"]).arg(dir);
    }
    let mut child = cmd
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se puede lanzar Claude Code (`claude`): {e}. Instálalo o pon su ruta en KILN_CLAUDE"))?;
    let out = child.stdout.take().unwrap();
    let mut final_reply = None;
    for line in BufReader::new(out).lines().map_while(Result::ok) {
        if let Some(l) = describe(&line, project) {
            log(jobs, id, l);
        }
        if let Some(t) = final_text(&line) {
            final_reply = Some(t);
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        return Err(format!("Claude Code salió con {status}: {}", err.trim()));
    }
    let text = final_reply.ok_or("Claude Code terminó sin dar respuesta")?;
    let (reply, offer) = reply_of(&text);
    let reply = with_offer(reply, offer.as_ref());
    if let (Some(c), Some(before)) = (ctx, &before)
        && let Ok(mut after) = kiln_core::context::load(&c.context_file)
        && after != *before
    {
        after.guard_pinned(before);
        kiln_core::context::save(&c.context_file, &after)?;
    }
    if let Some(f) = &chat_file {
        let mut thread = kiln_core::chat::load(f)?;
        for (who, text) in [("person", turn.message.clone()), ("agent", reply.clone().unwrap_or_else(|| "(sin respuesta)".into()))] {
            thread.messages.push(kiln_core::chat::Message { who: who.into(), step: turn.step.clone(), text });
        }
        kiln_core::chat::save(f, &thread)?;
    }
    Ok((reply, offer))
}

fn ask_prompt(asked: &str, config: &str, context: &str, cat_path: &Path, specs: &Path) -> String {
    format!(
        "Petición de la persona: «{asked}»\n\n{config}\n{context}\nCatálogo del proyecto: {}\nEscribe la ficha en la carpeta {} (fichero <id>.toml), siguiendo las instrucciones de la skill kiln-asset que tienes en tu prompt de sistema. No toques ningún otro fichero salvo para añadir preguntas al contexto.\n\nVocabulario de generación:\n{}",
        cat_path.display(),
        specs.display(),
        generator_vocabulary()
    )
}

/// The recipe vocabulary as text for the agent, rendered from the one table.
pub fn generator_vocabulary() -> String {
    let mut out = String::new();
    for op in kiln_core::generate::vocab::OPS {
        let again = if op.repeatable { "se puede repetir" } else { "una sola vez" };
        out.push_str(&format!("- {} ({}), {again}", op.id, op.label));
        if op.params.is_empty() {
            out.push_str(", sin parámetros");
        }
        for p in op.params {
            let unit = if p.unit.is_empty() { String::new() } else { format!(" {}", p.unit) };
            let int = if p.integer { ", entero" } else { "" };
            out.push_str(&format!(
                "; {} ({}): mín {}{unit}, máx {}{unit}, por defecto {}{unit}{int}",
                p.id, p.label, p.min, p.max, p.default
            ));
        }
        out.push('\n');
    }
    out.push_str("Cada parámetro es un número fijo o un rango `[mín, máx]` que sortea la semilla. `sphere` debe ser la primera operación.");
    out
}

fn run(jobs: &Jobs, id: u64, ctx: &Ctx, task: &Task) -> Result<Option<String>, String> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref())?;
    let summary = serde_json::json!({
        "project": cat.project,
        "pieces": cat.pieces.iter().map(|p| serde_json::json!({
            "key": p.key, "format": p.format, "size_m": p.size_m, "min": p.min, "tris": p.tris,
            "material_slots": p.material_slots, "licenses": p.licenses,
        })).collect::<Vec<_>>(),
        "materials": cat.materials.iter().map(|m| serde_json::json!({ "key": m.key, "name": m.name, "source": m.source })).collect::<Vec<_>>(),
        "specs": cat.specs.iter().map(|s| &s.asset.id).collect::<Vec<_>>(),
    });
    let tmp = std::env::temp_dir().join(format!("kiln-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let cat_path = tmp.join("catalog.json");
    std::fs::write(&cat_path, serde_json::to_vec_pretty(&summary).unwrap())
        .map_err(|e| e.to_string())?;
    let specs = ctx.project.join(&ctx.specs_dir);
    std::fs::create_dir_all(&specs).map_err(|e| e.to_string())?;
    let before = newest(&specs);

    let config = match &ctx.config {
        Some(c) => format!(
            "Configuración de Kiln del proyecto: {} (se pasa aparte; no hace falta que exista kiln.toml en la raíz)",
            c.display()
        ),
        None => "Configuración de Kiln del proyecto: kiln.toml en la raíz".into(),
    };
    let context = format!(
        "Contexto del proyecto: {} (léelo antes que nada; si no existe, aún no hay contexto)",
        ctx.context_file.display()
    );
    let layers = serde_json::to_string(&ctx.layers).unwrap_or_default();
    let ctx_before = kiln_core::context::load(&ctx.context_file)?;
    let prompt = match task {
        Task::Sort => format!(
            "Ordena el contexto del proyecto: la explicación de la persona está en el campo `text` del fichero {}. Sigue la skill kiln-context que tienes en tu prompt de sistema (modo ordenar).\n\nCapas: {layers}\n{config}\nCatálogo del proyecto: {}\nEdita solo el fichero de contexto.",
            ctx.context_file.display(),
            cat_path.display()
        ),
        Task::Refine(layer) => format!(
            "Afina el contexto del proyecto del fichero {}: añade una ronda de preguntas con respuestas sugeridas{}. Sigue la skill kiln-context que tienes en tu prompt de sistema (modo afinar).\n\nCapas: {layers}\n{config}\nCatálogo del proyecto: {}\nEdita solo el fichero de contexto.",
            ctx.context_file.display(),
            layer
                .as_ref()
                .map(|l| format!(", todas de la capa «{l}»"))
                .unwrap_or_default(),
            cat_path.display()
        ),
        Task::Ask(asked) => ask_prompt(asked, &config, &context, &cat_path, &specs),
        Task::Layout(file) => format!(
            "Coloca las piezas de la ficha {} (solo el paso 4, montaje, de la skill kiln-asset que tienes en tu prompt de sistema).\n\n{config}\nCatálogo del proyecto: {}\nEdita solo esa ficha.\n{context}",
            ctx.project.join(file).display(),
            cat_path.display()
        ),
    };
    let skill = match task {
        Task::Sort | Task::Refine(_) => CONTEXT_SKILL,
        _ => SKILL,
    };
    if let Some(d) = ctx.context_file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    log(jobs, id, "Claude Code está leyendo el proyecto".into());
    let mut child = claude()
        .args(["-p", &prompt, "--output-format", "stream-json", "--verbose", "--append-system-prompt", skill])
        .args(["--allowedTools", "Read", "Glob", "Grep", "Write", "Edit", "--permission-mode", "acceptEdits"])
        .args(["--disallowedTools", "Bash"])
        .arg("--add-dir")
        .arg(&tmp)
        .args(ctx.config.iter().filter_map(|c| c.parent()).chain(ctx.context_file.parent()).flat_map(|d| [std::ffi::OsStr::new("--add-dir"), d.as_os_str()]))
        .current_dir(&ctx.project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se puede lanzar Claude Code (`claude`): {e}. Instálalo o pon su ruta en KILN_CLAUDE"))?;
    let out = child.stdout.take().unwrap();
    for line in BufReader::new(out).lines().map_while(Result::ok) {
        if let Some(l) = describe(&line, &ctx.project) {
            log(jobs, id, l);
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&tmp);
    if !status.success() {
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        return Err(format!("Claude Code salió con {status}: {}", err.trim()));
    }
    if let Ok(mut after) = kiln_core::context::load(&ctx.context_file)
        && after != ctx_before
    {
        after.guard_pinned(&ctx_before);
        kiln_core::context::save(&ctx.context_file, &after)?;
        let props = |c: &kiln_core::context::Context| {
            c.claims.iter().filter(|c| c.proposed.is_some()).count()
        };
        let proposed = props(&after).saturating_sub(props(&ctx_before));
        if proposed > 0 {
            log(
                jobs,
                id,
                format!("Propone cambiar {proposed} afirmaciones que fijaste: decide tú"),
            );
        }
        if after.intent.proposed.is_some() && after.intent.proposed != ctx_before.intent.proposed {
            log(jobs, id, "Propone otro párrafo de qué juego es: decide tú".into());
        }
        let open = |c: &kiln_core::context::Context| {
            c.questions.iter().filter(|q| q.status == "open").count()
        };
        if open(&after) > open(&ctx_before) {
            log(
                jobs,
                id,
                format!(
                    "Deja {} preguntas nuevas en el contexto",
                    open(&after) - open(&ctx_before)
                ),
            );
        }
    }
    if matches!(task, Task::Sort | Task::Refine(_)) {
        return Ok(Some(
            ctx.context_file
                .strip_prefix(&ctx.project)
                .unwrap_or(&ctx.context_file)
                .to_string_lossy()
                .replace('\\', "/"),
        ));
    }
    Ok(newest(&specs)
        .filter(|n| Some(n) != before.as_ref())
        .map(|(p, _)| {
            p.strip_prefix(&ctx.project)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/")
        }))
}

/// Claude Code as the person is logged in to it: Kiln runs on their
/// subscription, so an API key in the environment must not take over.
pub fn claude() -> Command {
    let mut c = if let Ok(path) = std::env::var("KILN_CLAUDE") {
        Command::new(path)
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "claude"]);
        c
    } else {
        Command::new("claude")
    };
    c.env_remove("ANTHROPIC_API_KEY").env_remove("ANTHROPIC_AUTH_TOKEN");
    c
}

/// The most recently written spec file and its time.
fn newest(dir: &Path) -> Option<(PathBuf, SystemTime)> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .max_by_key(|(_, t)| *t)
}

/// The final text of the agent, from the `result` event of the stream.
pub fn final_text(line: &str) -> Option<String> {
    let v: Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "result" {
        return None;
    }
    if v.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    v.get("result")?.as_str().map(str::to_string)
}

/// The reply for the person and the offer, from the final text.
pub fn reply_of(text: &str) -> (Option<String>, Option<kiln_core::chat::Offer>) {
    let (reply, offer) = kiln_core::chat::split_offer(text);
    let reply = reply.trim();
    ((!reply.is_empty()).then(|| reply.to_string()), offer)
}

/// A line for the page from one event of the stream, or none.
pub fn describe(line: &str, project: &Path) -> Option<String> {
    let v: Value = serde_json::from_str(line).ok()?;
    let short = |p: &str| {
        p.strip_prefix(&*project.to_string_lossy())
            .unwrap_or(p)
            .trim_start_matches(['/', '\\'])
            .to_string()
    };
    match v.get("type")?.as_str()? {
        "assistant" => {
            let mut out = Vec::new();
            for c in v.pointer("/message/content")?.as_array()? {
                match c.get("type").and_then(Value::as_str) {
                    Some("tool_use") => {
                        let input = c.get("input").cloned().unwrap_or_default();
                        let arg = ["file_path", "pattern", "path"]
                            .iter()
                            .find_map(|k| input.get(*k).and_then(Value::as_str))
                            .map(short)
                            .unwrap_or_default();
                        let verb = match c.get("name").and_then(Value::as_str).unwrap_or("") {
                            "Read" => "Lee",
                            "Glob" | "Grep" => "Busca",
                            "Write" | "Edit" => "Escribe",
                            other => other,
                        };
                        out.push(format!("{verb} {arg}"));
                    }
                    Some("text") => {
                        let t = c.get("text").and_then(Value::as_str).unwrap_or("").trim();
                        if !t.is_empty() {
                            out.push(t.chars().take(300).collect());
                        }
                    }
                    _ => {}
                }
            }
            (!out.is_empty()).then(|| out.join(" · "))
        }
        "result" => v
            .get("is_error")
            .and_then(Value::as_bool)
            .filter(|e| *e)
            .map(|_| {
                format!(
                    "Claude Code devolvió un error: {}",
                    v.get("result").and_then(Value::as_str).unwrap_or("")
                )
            }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_lists_every_op_and_param() {
        let v = generator_vocabulary();
        for op in kiln_core::generate::vocab::OPS {
            assert!(v.contains(op.id), "{v}");
            for p in op.params {
                assert!(v.contains(p.id), "{v}");
            }
        }
    }

    #[test]
    fn vocabulary_in_the_ask_prompt() {
        let p = ask_prompt("una roca", "cfg", "ctx", Path::new("cat.json"), Path::new("specs"));
        assert!(p.contains("Vocabulario de generación"), "{p}");
        assert!(p.contains("sphere"), "{p}");
    }

    #[test]
    fn final_text_of_a_success_result() {
        let l = r#"{"type":"result","subtype":"success","is_error":false,"result":"Hola"}"#;
        assert_eq!(final_text(l), Some("Hola".to_string()));
    }

    #[test]
    fn final_text_ignores_errors_other_events_and_garbage() {
        let err = r#"{"type":"result","subtype":"error","is_error":true,"result":"boom"}"#;
        let asst = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"x"}]}}"#;
        assert_eq!(final_text(err), None);
        assert_eq!(final_text(asst), None);
        assert_eq!(final_text("not json"), None);
    }

    #[test]
    fn reply_of_splits_the_block() {
        let t = "Te propongo estos nombres.\n\n```kiln\n{\"names\":[\"Alba\",\"Brisa\"]}\n```";
        let (reply, offer) = reply_of(t);
        assert_eq!(reply.as_deref(), Some("Te propongo estos nombres."));
        assert_eq!(offer.unwrap().names, vec!["Alba", "Brisa"]);
    }

    fn turn_with(n: usize) -> Turn {
        Turn {
            step: "2".into(),
            message: "mensaje nuevo".into(),
            history: (1..=n)
                .map(|i| kiln_core::chat::Message {
                    who: if i % 2 == 0 { "agent" } else { "person" }.into(),
                    step: "2".into(),
                    text: format!("texto-{i:02}-fin"),
                })
                .collect(),
            draft: Some(serde_json::json!({"names": ["Alba"]})),
        }
    }

    #[test]
    fn chat_prompt_without_project_keeps_the_last_20_and_forbids_writing() {
        let t = turn_with(25);
        let p = chat_prompt(&t, &t.history, None, "");
        assert!(!p.contains("texto-05-fin"));
        assert!(p.contains("texto-06-fin") && p.contains("texto-25-fin"));
        assert!(p.contains("paso 2") && p.contains("mensaje nuevo"));
        assert!(p.contains("no escribas"));
        assert!(p.contains("Alba"));
    }

    #[test]
    fn chat_prompt_with_project_names_the_context_file() {
        let ctx = Ctx {
            project: PathBuf::from("/p"),
            config: None,
            specs_dir: "specs".into(),
            roots: vec![],
            context_file: PathBuf::from("/p/kiln/context.toml"),
            layers: vec![],
        };
        let t = turn_with(1);
        let p = chat_prompt(&t, &t.history, Some(&ctx), "");
        assert!(p.contains("/p/kiln/context.toml") && p.contains("Edita solo el fichero de contexto"));
        assert!(!p.contains("no escribas"));
        assert!(chat_tools(Some(&ctx)).contains(&"Write".to_string()));
    }

    #[test]
    fn the_draft_settles_the_name_and_the_chosen_basics() {
        let d = serde_json::json!({"name": "Salitre", "basics": {"camera": "primera persona", "style": ""}});
        let s = draft_settled(Some(&d));
        assert!(s.contains("- Nombre: Salitre") && s.contains("- Cámara: primera persona"));
        assert_eq!(s.lines().count(), 2);
        assert_eq!(draft_settled(None), "");
    }

    #[test]
    fn chat_prompt_carries_what_is_settled() {
        let t = turn_with(1);
        let p = chat_prompt(&t, &t.history, None, "- Cámara: primera persona\n");
        assert!(p.contains("Ya fijado") && p.contains("- Cámara: primera persona"));
        assert!(p.contains("no lo contradigas"));
        assert!(!chat_prompt(&t, &t.history, None, "").contains("Ya fijado"));
    }

    #[test]
    fn claude_runs_on_the_subscription_not_on_an_api_key() {
        let cmd = claude();
        let removed: Vec<_> = cmd.get_envs().filter(|(_, v)| v.is_none()).map(|(k, _)| k.to_string_lossy().into_owned()).collect();
        for k in ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"] {
            assert!(removed.contains(&k.to_string()), "{k} should be removed");
        }
    }

    #[test]
    fn chat_tools_without_project_cannot_write() {
        let tools = chat_tools(None);
        assert_eq!(tools, ["Read", "Glob", "Grep"]);
    }

    #[test]
    fn reply_of_only_a_block_has_no_reply() {
        let said = with_offer(Some("Mira.".into()), Some(&kiln_core::chat::Offer { names: vec!["Alba".into()], ..Default::default() }));
        assert_eq!(said.as_deref(), Some("Mira.\n\nPropuse: nombres 1) Alba"));
        assert_eq!(with_offer(None, None), None);
        let (reply, offer) = reply_of("```kiln\n{\"names\":[\"Alba\"]}\n```");
        assert_eq!(reply, None);
        assert!(offer.is_some());
    }
}
