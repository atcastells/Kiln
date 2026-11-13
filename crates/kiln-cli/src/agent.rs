//! Runs Claude Code without a window (`claude -p`) to break a request into
//! an asset spec, and keeps what it does as a job the graph page can follow.

use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const SKILL: &str = include_str!("../../../skills/asset/SKILL.md");

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: u64,
    pub asked: String,
    /// running | done | failed
    pub status: String,
    /// One line per thing the agent does, for the page.
    pub log: Vec<String>,
    /// The spec it wrote, relative to the project root.
    pub spec: Option<String>,
}

pub type Jobs = Arc<Mutex<Vec<Job>>>;

pub struct Ctx {
    pub project: PathBuf,
    pub config: Option<PathBuf>,
    pub specs_dir: String,
}

pub fn start(jobs: &Jobs, ctx: Arc<Ctx>, asked: String) -> u64 {
    let id = {
        let mut j = jobs.lock().unwrap();
        let id = j.len() as u64 + 1;
        j.push(Job {
            id,
            asked: asked.clone(),
            status: "running".into(),
            log: vec!["Leyendo el catálogo del proyecto".into()],
            spec: None,
        });
        id
    };
    let jobs = jobs.clone();
    std::thread::spawn(move || {
        let result = run(&jobs, id, &ctx, &asked);
        let mut j = jobs.lock().unwrap();
        let job = &mut j[(id - 1) as usize];
        match result {
            Ok(spec) => {
                job.log.push(match &spec {
                    Some(s) => format!("Ficha escrita: {s}"),
                    None => "El agente terminó sin escribir ficha".into(),
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

fn log(jobs: &Jobs, id: u64, line: String) {
    if let Some(job) = jobs.lock().unwrap().get_mut((id - 1) as usize) {
        job.log.push(line);
    }
}

fn run(jobs: &Jobs, id: u64, ctx: &Ctx, asked: &str) -> Result<Option<String>, String> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref())?;
    let summary = serde_json::json!({
        "project": cat.project,
        "pieces": cat.pieces.iter().map(|p| serde_json::json!({
            "key": p.key, "format": p.format, "size_m": p.size_m, "tris": p.tris,
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
    let prompt = format!(
        "Petición de la persona: «{asked}»\n\n{config}\nCatálogo del proyecto: {}\nEscribe la ficha en la carpeta {} (fichero <id>.toml), siguiendo las instrucciones de la skill kiln-asset que tienes en tu prompt de sistema. No toques ningún otro fichero.",
        cat_path.display(),
        specs.display()
    );
    log(jobs, id, "Claude Code está leyendo el proyecto".into());
    let mut child = claude()
        .args(["-p", &prompt, "--output-format", "stream-json", "--verbose", "--append-system-prompt", SKILL])
        .args(["--allowedTools", "Read", "Glob", "Grep", "Write", "Edit", "--permission-mode", "acceptEdits"])
        .args(["--disallowedTools", "Bash"])
        .arg("--add-dir")
        .arg(&tmp)
        .args(ctx.config.iter().filter_map(|c| c.parent()).flat_map(|d| [std::ffi::OsStr::new("--add-dir"), d.as_os_str()]))
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
    Ok(newest(&specs)
        .filter(|n| Some(n) != before.as_ref())
        .map(|(p, _)| {
            p.strip_prefix(&ctx.project)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/")
        }))
}

fn claude() -> Command {
    if let Ok(path) = std::env::var("KILN_CLAUDE") {
        return Command::new(path);
    }
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "claude"]);
        return c;
    }
    Command::new("claude")
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

/// A line for the page from one event of the stream, or none.
fn describe(line: &str, project: &Path) -> Option<String> {
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
