//! A local server on 127.0.0.1 only: the graph page, its JSON and the jobs
//! of the agent. The desktop shell (Tauri) will wrap the same page later.

use crate::agent::{self, Ctx, Jobs, Task};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

const PAGE: &str = include_str!("../ui/index.html");
const THREE: &str = include_str!("../ui/vendor/three.js");

/// The modules of the new UI, served under `/ui/<path>`. A new file is one
/// more line here; `--ui-dir` reads the same paths from disk instead.
const UI_FILES: &[(&str, &str)] = &[
    ("app/index.html", include_str!("../ui/app/index.html")),
    ("app/main.js", include_str!("../ui/app/main.js")),
    ("app/store.js", include_str!("../ui/app/store.js")),
    ("app/html.js", include_str!("../ui/app/html.js")),
    ("app/api.js", include_str!("../ui/app/api.js")),
    ("app/router.js", include_str!("../ui/app/router.js")),
    ("app/derive.js", include_str!("../ui/app/derive.js")),
    ("app/styles.css", include_str!("../ui/app/styles.css")),
    ("app/shell.js", include_str!("../ui/app/shell.js")),
    ("app/nav.js", include_str!("../ui/app/nav.js")),
    ("app/ui/list.js", include_str!("../ui/app/ui/list.js")),
    ("app/ui/feedback.js", include_str!("../ui/app/ui/feedback.js")),
    ("app/ui/dialog.js", include_str!("../ui/app/ui/dialog.js")),
    ("app/views/registry.js", include_str!("../ui/app/views/registry.js")),
    ("app/views/home.js", include_str!("../ui/app/views/home.js")),
    ("app/search.js", include_str!("../ui/app/search.js")),
    ("app/ui/palette.js", include_str!("../ui/app/ui/palette.js")),
    ("app/views/common.js", include_str!("../ui/app/views/common.js")),
    ("app/views/catalog.js", include_str!("../ui/app/views/catalog.js")),
    ("app/views/materials.js", include_str!("../ui/app/views/materials.js")),
    ("app/views/content.js", include_str!("../ui/app/views/content.js")),
    ("app/views/tree.js", include_str!("../ui/app/views/tree.js")),
    ("app/format.js", include_str!("../ui/app/format.js")),
    ("app/views/context.js", include_str!("../ui/app/views/context.js")),
    ("app/views/wizard.js", include_str!("../ui/app/views/wizard.js")),
    ("app/views/talk.js", include_str!("../ui/app/views/talk.js")),
    ("app/views/spec.js", include_str!("../ui/app/views/spec.js")),
    ("app/views/build.js", include_str!("../ui/app/views/build.js")),
    ("app/views/jobs.js", include_str!("../ui/app/views/jobs.js")),
    ("app/views/view3d.js", include_str!("../ui/app/views/view3d.js")),
];

/// `kiln serve --ui-dir`: read the UI modules from here, not from the binary.
static UI_DIR: OnceLock<PathBuf> = OnceLock::new();

type Reply = (&'static str, &'static str, Vec<u8>);

fn ui_mime(path: &str) -> Option<&'static str> {
    match path.rsplit_once('.')?.1 {
        "js" | "mjs" => Some("text/javascript; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "html" => Some("text/html; charset=utf-8"),
        _ => None,
    }
}

/// A UI file by its path under `ui/`. Only plain names (letters, digits,
/// `.`, `_`, `-`) are accepted, so nothing can climb out of the folder.
fn ui_file(rel: &str, dir: Option<&Path>) -> Option<(&'static str, Vec<u8>)> {
    let plain = |c: &str| {
        !c.is_empty()
            && c != "."
            && c != ".."
            && c.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    };
    if !rel.split('/').all(plain) {
        return None;
    }
    let kind = ui_mime(rel)?;
    let body = match dir {
        Some(d) => std::fs::read(d.join(rel)).ok()?,
        None => UI_FILES.iter().find(|(n, _)| *n == rel)?.1.as_bytes().to_vec(),
    };
    Some((kind, body))
}

/// `/ui/<path>` and the entry of the new UI (`/`; `/app` keeps answering with
/// the same page so old links work); `None` for any other path, so the caller
/// keeps routing.
fn route_ui(path: &str, dir: Option<&Path>) -> Option<Reply> {
    let path = path.split(['?', '#']).next().unwrap_or("");
    let rel = match path {
        "/" | "/index.html" | "/app" | "/app/" => "app/index.html",
        p => p.strip_prefix("/ui/")?,
    };
    Some(match ui_file(rel, dir) {
        Some((kind, body)) => ("200 OK", kind, body),
        None => ("404 Not Found", "text/plain", b"no existe".to_vec()),
    })
}

/// The previous UI, served at `/classic` for one more release, with a banner
/// that says so and links to the new one.
const CLASSIC_BANNER: &str = r#"<div role="note" style="background:#f7ecd2;color:#4a3000;border:1px solid #d9b866;border-radius:6px;padding:6px 12px;font:13px system-ui,sans-serif">Interfaz anterior: se retirará en la próxima entrega. <a href="/" style="color:#7a3a0a">Ir a la interfaz nueva</a></div>"#;

fn classic_page() -> Vec<u8> {
    PAGE.replacen("<body>", &format!("<body>{CLASSIC_BANNER}"), 1).into_bytes()
}

fn route_ui_served(path: &str) -> Option<Reply> {
    route_ui(path, UI_DIR.get().map(PathBuf::as_path))
}

/// The project the server works on; `None` while the folder has no
/// `kiln.toml` and the start wizard is open.
type Slot = Arc<RwLock<Option<Arc<Ctx>>>>;

use crate::ops::{self, ctx as make_ctx};

pub fn run(
    project: PathBuf,
    config: Option<PathBuf>,
    port: u16,
    ui_dir: Option<PathBuf>,
) -> Result<(), String> {
    if let Some(d) = ui_dir {
        eprintln!("interfaz leida de {}", d.display());
        let _ = UI_DIR.set(d);
    }
    let new = config.is_none() && !project.join(kiln_core::config::FILE).exists();
    let slot: Slot = Arc::new(RwLock::new(if new {
        None
    } else {
        Some(Arc::new(make_ctx(&project, config)?))
    }));
    let jobs: Jobs = Default::default();
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("no se puede abrir el puerto {port}: {e}"))?;
    eprintln!(
        "Kiln en http://127.0.0.1:{port}  ({} {})",
        if new { "juego nuevo en" } else { "proyecto" },
        project.display()
    );
    let project = Arc::new(project);
    for stream in listener.incoming().flatten() {
        let (slot, jobs, project) = (slot.clone(), jobs.clone(), project.clone());
        std::thread::spawn(move || {
            let current = slot.read().unwrap().clone();
            let r = match current {
                Some(ctx) => handle(stream, &ctx, &jobs),
                None => handle_new(stream, &project, &slot, &jobs),
            };
            if let Err(e) = r {
                eprintln!("petición fallida: {e}");
            }
        });
    }
    Ok(())
}

fn from_page(req: &Request) -> bool {
    let header = |k: &str| {
        req.headers
            .iter()
            .find(|(h, _)| h == k)
            .map(|(_, v)| v.as_str())
    };
    header("x-kiln") == Some("1")
        && header("host")
            .is_some_and(|h| h.starts_with("127.0.0.1:") || h.starts_with("localhost:"))
}

fn respond(stream: TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    respond_with(stream, status, kind, body, None)
}

fn respond_with(
    mut stream: TcpStream,
    status: &str,
    kind: &str,
    body: &[u8],
    etag: Option<&str>,
) -> std::io::Result<()> {
    let etag = etag.map(|e| format!("ETag: {e}\r\n")).unwrap_or_default();
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n{etag}Connection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

/// The server of a folder without `kiln.toml`: the page and the first step
/// of the wizard, which writes the project and opens it.
fn handle_new(stream: TcpStream, project: &Path, slot: &Slot, jobs: &Jobs) -> std::io::Result<()> {
    let req = read_request(&stream)?;
    let json = |v: serde_json::Value| serde_json::to_vec(&v).unwrap_or_default();
    let (status, kind, body) = match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/classic") => ("200 OK", "text/html; charset=utf-8", classic_page()),
        ("GET", p) if route_ui_served(p).is_some() => route_ui_served(p).unwrap(),
        ("GET", "/api/jobs") => ("200 OK", "application/json", json(jobs_list(jobs))),
        ("GET", "/api/new") => ("200 OK", "application/json", json(ops::new_game::options(project, None).unwrap_or_default())),
        ("GET", p) if p.starts_with("/api/jobs/") => match job_of(jobs, p) {
            Some(job) => ("200 OK", "application/json", json(serde_json::to_value(job).unwrap_or_default())),
            None => ("404 Not Found", "application/json", br#"{"error":"no existe ese trabajo"}"#.to_vec()),
        },
        ("POST", _) if !from_page(&req) => ("403 Forbidden", "application/json", br#"{"error":"solo desde la pagina de Kiln"}"#.to_vec()),
        ("POST", "/api/chat") => match turn_of(&req.body) {
            Ok(t) => ("200 OK", "application/json", json(serde_json::json!({ "job": agent::chat(jobs, project.to_path_buf(), None, t) }))),
            Err(e) => ("400 Bad Request", "application/json", json(serde_json::json!({ "error": e }))),
        },
        ("POST", "/api/new") => {
            let made = serde_json::from_slice::<kiln_core::new_game::NewGame>(&req.body)
                .map_err(|e| e.to_string())
                .and_then(|g| ops::new_game::create(project, g));
            match made {
                Ok((files, ctx)) => {
                    *slot.write().unwrap() = Some(Arc::new(ctx));
                    ("200 OK", "application/json", json(serde_json::json!({ "ok": true, "files": files })))
                }
                Err(e) => ("400 Bad Request", "application/json", json(serde_json::json!({ "error": e }))),
            }
        }
        _ => ("409 Conflict", "application/json", br#"{"error":"todavia no hay proyecto: termina el primer paso"}"#.to_vec()),
    };
    respond(stream, status, kind, &body)
}

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> std::io::Result<Request> {
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (
        parts.next().unwrap_or("").to_string(),
        parts.next().unwrap_or("/").to_string(),
    );
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    let len = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0)
        .min(64 * 1024);
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

/// A read that explicitly asks for a fresh load from disk. Only Kiln's own UI
/// sends `X-Kiln-Fresh: 1` ("Releer"); browsers send `Cache-Control: no-cache`
/// on hard reloads, which must not drop the server cache.
fn wants_fresh(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(h, v)| h == "x-kiln-fresh" && v.trim() == "1")
}

fn catalog_request(ctx: &Ctx, headers: &[(String, String)]) -> (&'static str, Option<String>, Vec<u8>) {
    let inm = headers.iter().find(|(h, _)| h == "if-none-match").map(|(_, v)| v.as_str());
    if wants_fresh(headers) {
        drop_catalog(&ctx.project);
    }
    catalog_reply(ctx, inm)
}

fn handle(stream: TcpStream, ctx: &Arc<Ctx>, jobs: &Jobs) -> std::io::Result<()> {
    let req = read_request(&stream)?;
    if req.method == "GET" && req.path == "/api/catalog" {
        let (status, etag, body) = catalog_request(ctx, &req.headers);
        return respond_with(stream, status, "application/json", &body, etag.as_deref());
    }
    let from_page = from_page(&req);
    let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap_or_default();
    let field = |k: &str| {
        body.get(k)
            .and_then(|t| t.as_str())
            .map(|t| t.trim().to_string())
            .unwrap_or_default()
    };
    let bad = |e: &str| {
        (
            "400 Bad Request",
            "application/json",
            serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default(),
        )
    };
    let json = |v: serde_json::Value| {
        (
            "200 OK",
            "application/json",
            serde_json::to_vec(&v).unwrap_or_default(),
        )
    };
    let answer = |r: Result<serde_json::Value, String>| match r {
        Ok(v) => json(v),
        Err(e) => bad(&e),
    };
    let done = |r: Result<(), String>| match r {
        Ok(()) => json(serde_json::json!({ "ok": true })),
        Err(e) => bad(&e),
    };
    let started = |r: Result<u64, String>| match r {
        Ok(job) => json(serde_json::json!({ "job": job })),
        Err(e) => bad(&e),
    };
    let (status, kind, body) = match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/classic") => ("200 OK", "text/html; charset=utf-8", classic_page()),
        ("GET", p) if route_ui_served(p).is_some() => route_ui_served(p).unwrap(),
        ("GET", "/api/jobs") => json(jobs_list(jobs)),
        // Only the page itself may post here: a custom header forces a CORS
        // preflight this server never answers, so another site cannot.
        ("POST", _) if !from_page => (
            "403 Forbidden",
            "application/json",
            br#"{"error":"solo desde la pagina de Kiln"}"#.to_vec(),
        ),
        ("POST", "/api/ask") => {
            let text = field("text");
            let layout = field("layout");
            if !layout.is_empty() {
                json(
                    serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Layout(layout)) }),
                )
            } else if text.is_empty() || text.len() > 500 {
                bad("la peticion va vacia o pasa de 500 caracteres")
            } else {
                json(serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Ask(text)) }))
            }
        }
        ("POST", "/api/build") => started(ops::production::build(jobs, ctx.clone(), &field("asset"))),
        ("POST", "/api/review") => started(ops::production::review(jobs, ctx.clone(), &field("asset"))),
        ("POST", "/api/godot") => started(Ok(ops::production::godot_measure(jobs, ctx.clone()))),
        ("POST", "/api/search") => answer(ops::sources::search(ctx, &field("asset"), &field("part"))),
        ("POST", "/api/import") => started(ops::production::import(
            jobs,
            ctx.clone(),
            &field("asset"),
            &field("part"),
            &field("site"),
            &field("id"),
        )),
        ("POST", "/api/bind") => {
            done(ops::materials::bind(ctx, &field("piece"), &field("slot"), &field("material")))
        }
        ("POST", "/api/material") => {
            let variant = field("variant");
            done(ops::materials::material_set(
                ctx,
                &field("asset"),
                &field("part"),
                &field("zone"),
                &field("material"),
                Some(variant.as_str()).filter(|v| !v.is_empty()),
            ))
        }
        ("POST", "/api/validate") => {
            match ops::materials::material_validate(ctx, &field("asset"), &field("part")) {
                Ok(piece) => json(serde_json::json!({ "ok": true, "piece": piece })),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/split") => done(ops::materials::zones_split(ctx, &field("piece"), &field("mode"))),
        ("POST", "/api/paint") => started(ops::production::paint(
            jobs,
            ctx.clone(),
            &field("asset"),
            Some(field("variant")),
        )),
        ("POST", "/api/new") => bad("esta carpeta ya tiene kiln.toml"),
        ("POST", "/api/settle") => done(ops::context::settle(ctx)),
        ("POST", "/api/chat") => match turn_of(&req.body) {
            Ok(mut t) => {
                t.history.clear();
                json(serde_json::json!({ "job": agent::chat(jobs, ctx.project.clone(), Some(ctx.clone()), t) }))
            }
            Err(e) => bad(&e),
        },
        ("GET", "/api/chat") => match kiln_core::chat::load(&ctx.chat_file()) {
            Ok(c) => json(serde_json::json!({ "messages": c.messages })),
            Err(e) => bad(&e),
        },
        ("POST", "/api/context/sort") => {
            json(serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Sort) }))
        }
        ("POST", "/api/context/refine") => json(
            serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Refine(Some(field("layer")).filter(|l| !l.is_empty()))) }),
        ),
        ("POST", p) if p.starts_with("/api/context/") => {
            use ops::context::Edit;
            let edit = match p {
                "/api/context/text" => Some(Edit::Text(
                    body.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string(),
                )),
                "/api/context/intent" => Some(Edit::Intent { action: field("action"), text: field("text") }),
                "/api/context/answer" => Some(Edit::Answer {
                    id: field("id"),
                    answer: field("answer"),
                    status: field("status"),
                }),
                "/api/context/claim" => Some(Edit::Claim {
                    id: field("id"),
                    action: field("action"),
                    text: field("text"),
                    layer: field("layer"),
                }),
                _ => None,
            };
            match edit {
                Some(e) => done(ops::context::context_edit(ctx, e)),
                None => bad("no existe"),
            }
        }
        ("GET", "/api/new") => answer(ops::new_game::options(&ctx.project, Some(ctx))),
        ("GET", "/api/context") => answer(ops::read::context_get(ctx)),
        ("GET", "/vendor/three.js") => (
            "200 OK",
            "text/javascript; charset=utf-8",
            THREE.as_bytes().to_vec(),
        ),
        ("GET", p) if p.starts_with("/files/") => match file(ctx, &p[7..]) {
            Some((kind, body)) => ("200 OK", kind, body),
            None => ("404 Not Found", "text/plain", b"no existe".to_vec()),
        },
        ("GET", p) if p.starts_with("/api/jobs/") => match job_of(jobs, p) {
            Some(job) => json(serde_json::to_value(job).unwrap_or_default()),
            None => (
                "404 Not Found",
                "application/json",
                br#"{"error":"no existe ese trabajo"}"#.to_vec(),
            ),
        },
        _ => ("404 Not Found", "text/plain", b"no existe".to_vec()),
    };
    // Whatever a write changed on disk, the next catalog is read again.
    if req.method == "POST" {
        drop_catalog(&ctx.project);
    }
    respond(stream, status, kind, &body)
}

/// Active jobs and the latest finished ones, for a page that reloaded and
/// needs to find its job again. Logs stay out: `/api/jobs/<id>` has them.
fn jobs_list(jobs: &Jobs) -> serde_json::Value {
    const FINISHED_KEPT: usize = 5;
    let all = jobs.lock().unwrap();
    let recent: Vec<u64> = all
        .iter()
        .rev()
        .filter(|j| j.status != "running")
        .take(FINISHED_KEPT)
        .map(|j| j.id)
        .collect();
    let list: Vec<_> = all
        .iter()
        .filter(|j| j.status == "running" || recent.contains(&j.id))
        .map(|j| serde_json::json!({ "id": j.id, "label": j.asked, "state": j.status, "kind": j.kind, "started": j.started }))
        .collect();
    serde_json::json!({ "jobs": list })
}

fn job_of(jobs: &Jobs, path: &str) -> Option<agent::Job> {
    let id = path.strip_prefix("/api/jobs/")?.parse::<u64>().ok()?;
    ops::production::job_get(jobs, id)
}

use kiln_core::chat::MAX_TEXT_CHARS as MAX_TEXT;

#[derive(serde::Deserialize)]
struct TurnRequest {
    step: String,
    message: String,
    #[serde(default)]
    history: Vec<kiln_core::chat::Message>,
    #[serde(default)]
    draft: Option<serde_json::Value>,
}

/// A chat turn from the page, checked: a known step, a message of up to
/// `MAX_TEXT` characters, and the history cleaned by `chat::clean`.
fn turn_of(body: &[u8]) -> Result<agent::Turn, String> {
    let r: TurnRequest = serde_json::from_slice(body).map_err(|e| format!("petición mal formada: {e}"))?;
    if !["1", "2", "3", "4", "5", "contexto"].contains(&r.step.as_str()) {
        return Err(format!("paso desconocido: {}", r.step));
    }
    let message = r.message.trim();
    if message.is_empty() || message.chars().count() > MAX_TEXT {
        return Err(format!("el mensaje debe tener entre 1 y {MAX_TEXT} caracteres"));
    }
    let history = kiln_core::chat::clean(r.history);
    Ok(agent::Turn { step: r.step, message: message.into(), history, draft: r.draft })
}

/// A catalog built and serialized, kept while the project files do not change.
struct CatalogEntry {
    fingerprint: u64,
    reply: Option<(Arc<Vec<u8>>, String)>,
    /// How many times the loader ran for this project; tests read it.
    loads: usize,
}

fn catalogs() -> &'static Mutex<HashMap<PathBuf, CatalogEntry>> {
    static C: OnceLock<Mutex<HashMap<PathBuf, CatalogEntry>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Paths, sizes, modification times and (on unix) change times of everything
/// under the project (the loader reads specs, content, bindings, build reports
/// and the Godot cache from there), of the config file when it lives elsewhere,
/// and of every folder or file the config points at outside the project (kits,
/// materials tables and textures, build and cache folders...). Only stats, no
/// reads of the data; `target`, `.git` and `node_modules` are not catalog input.
///
/// The change time cannot be set from user space, so a write that keeps the
/// size and puts the modification time back is still seen on unix. Elsewhere
/// such an edit is only picked up by "Releer", which drops the cache.
fn fingerprint(ctx: &Ctx) -> u64 {
    fn stat(h: &mut impl Hasher, path: &Path, meta: &std::fs::Metadata) {
        path.hash(h);
        meta.len().hash(h);
        meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .hash(h);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            (meta.ctime(), meta.ctime_nsec()).hash(h);
        }
    }
    fn walk(h: &mut impl Hasher, dir: &Path) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name();
            if matches!(name.to_str(), Some("target" | ".git" | "node_modules")) {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            stat(h, &e.path(), &meta);
            if meta.is_dir() {
                walk(h, &e.path());
            }
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if let Some(c) = &ctx.config
        && let Ok(meta) = std::fs::metadata(c)
    {
        stat(&mut h, c, &meta);
    }
    walk(&mut h, &ctx.project);
    for p in outside_inputs(ctx) {
        match std::fs::metadata(&p) {
            Ok(meta) => {
                stat(&mut h, &p, &meta);
                if meta.is_dir() {
                    walk(&mut h, &p);
                }
            }
            Err(_) => p.hash(&mut h), // a missing input: creating it changes the fingerprint
        }
    }
    h.finish()
}

/// Inputs of the loader that the config places outside the project folder.
fn outside_inputs(ctx: &Ctx) -> Vec<PathBuf> {
    let cfg = match &ctx.config {
        Some(c) => kiln_core::Config::load_file(c),
        None => kiln_core::Config::load(&ctx.project),
    };
    let Ok(cfg) = cfg else { return Vec::new() };
    let mut rel: Vec<String> = cfg.kit_roots();
    rel.extend(cfg.materials.tables.iter().cloned());
    rel.extend(cfg.materials.textures.iter().cloned());
    rel.extend([
        cfg.materials.bindings.clone(),
        cfg.tools.build.clone(),
        cfg.tools.cache.clone(),
        cfg.context.file.clone(),
        cfg.assets.specs.clone().unwrap_or_else(|| "kiln/specs".into()),
        kiln_core::zones::FILE.into(),
    ]);
    for l in &cfg.levels {
        rel.extend(l.files.iter().map(|f| {
            // a glob: its fixed folder prefix
            let cut = f.find(['*', '?', '[']).unwrap_or(f.len());
            let fixed = &f[..cut];
            if cut == f.len() { fixed.to_string() } else { fixed.rsplit_once('/').map_or(String::new(), |(d, _)| d.to_string()) }
        }));
    }
    let root = std::fs::canonicalize(&ctx.project).unwrap_or_else(|_| ctx.project.clone());
    let mut out: Vec<PathBuf> = rel
        .iter()
        .filter(|r| !r.is_empty())
        .map(|r| ctx.project.join(r))
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .filter(|p| !p.starts_with(&root))
        .collect();
    out.sort();
    out.dedup();
    out
}

fn etag_matches(header: &str, etag: &str) -> bool {
    header
        .split(',')
        .map(str::trim)
        .any(|t| t == "*" || t.strip_prefix("W/").unwrap_or(t) == etag)
}

/// The catalog answer: `200` with its `ETag`, or `304` when the caller's
/// `If-None-Match` is the current one. The catalog is loaded again only when
/// the fingerprint of the project files changed or a write dropped it.
fn catalog_reply(ctx: &Ctx, if_none_match: Option<&str>) -> (&'static str, Option<String>, Vec<u8>) {
    let fp = fingerprint(ctx);
    let mut map = catalogs().lock().unwrap();
    let entry = map.entry(ctx.project.clone()).or_insert(CatalogEntry {
        fingerprint: fp,
        reply: None,
        loads: 0,
    });
    if entry.fingerprint != fp {
        entry.fingerprint = fp;
        entry.reply = None;
    }
    if entry.reply.is_none() {
        entry.loads += 1;
        match crate::load(&ctx.project, ctx.config.as_deref()) {
            Ok(cat) => {
                let body = serde_json::to_vec(&cat).unwrap_or_default();
                let mut h = std::collections::hash_map::DefaultHasher::new();
                body.hash(&mut h);
                entry.reply = Some((Arc::new(body), format!("\"{:016x}\"", h.finish())));
            }
            Err(e) => {
                let body = serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default();
                return ("500 Internal Server Error", None, body);
            }
        }
    }
    let (body, etag) = entry.reply.clone().unwrap();
    drop(map);
    if if_none_match.is_some_and(|h| etag_matches(h, &etag)) {
        return ("304 Not Modified", Some(etag), Vec::new());
    }
    ("200 OK", Some(etag), body.as_ref().clone())
}

/// Forget the cached catalog of a project (after a write).
fn drop_catalog(project: &Path) {
    if let Some(e) = catalogs().lock().unwrap().get_mut(project) {
        e.reply = None;
    }
}

#[cfg(test)]
fn catalog_loads(project: &Path) -> usize {
    catalogs().lock().unwrap().get(project).map_or(0, |e| e.loads)
}

#[cfg(test)]
fn catalog_response(ctx: &Ctx) -> Reply {
    let (status, _, body) = catalog_reply(ctx, None);
    (status, "application/json", body)
}

/// A model or texture under one of the roots; the path is relative to the
/// project, or absolute (a build or cache folder outside it).
fn file(ctx: &Ctx, path: &str) -> Option<(&'static str, Vec<u8>)> {
    let path = decode(path.split('?').next()?)?;
    let kind = match path.rsplit_once('.')?.1.to_lowercase().as_str() {
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "bin" => "application/octet-stream",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        // what a visual review left (review.json, measures.json)
        "json" if path.contains("/review/") => "application/json",
        _ => return None,
    };
    let full = std::fs::canonicalize(ctx.project.join(&path)).ok()?;
    if !ctx.roots.iter().any(|r| full.starts_with(r)) {
        return None;
    }
    Some((kind, std::fs::read(full).ok()?))
}

fn decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn served(example: &str) -> serde_json::Value {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(example);
        let ctx = make_ctx(&dir, None).unwrap();
        let (status, _, body) = catalog_response(&ctx);
        assert_eq!(status, "200 OK");
        serde_json::from_slice(&body).unwrap()
    }

    #[test]
    fn catalog_serves_the_content_sets() {
        let cat = served("content");
        let sets = cat["content"].as_array().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0]["name"], "world");
        assert_eq!(sets[0]["kinds"].as_array().unwrap().len(), 4);
        assert_eq!(sets[0]["elements"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn catalog_without_content_serves_an_empty_list() {
        assert_eq!(served("")["content"].as_array().unwrap().len(), 0);
    }

    // ---- UI modules (T4)

    #[test]
    fn ui_modules_are_served_with_their_mime_type() {
        let (status, kind, body) = route_ui("/ui/app/main.js", None).unwrap();
        assert_eq!(status, "200 OK");
        assert!(kind.starts_with("text/javascript"), "{kind}");
        assert!(!body.is_empty());
        let (_, kind, page) = route_ui("/ui/app/index.html", None).unwrap();
        assert!(kind.starts_with("text/html"));
        assert_eq!(route_ui("/app", None).unwrap().2, page);
        assert_eq!(route_ui("/app/", None).unwrap().2, page);
        assert_eq!(route_ui("/ui/app/main.js?v=2", None).unwrap().0, "200 OK");
        assert_eq!(ui_mime("a/b.css"), Some("text/css; charset=utf-8"));
    }

    #[test]
    fn ui_unknown_and_traversal_paths_are_404() {
        for p in [
            "/ui/app/nope.js",
            "/ui/../src/serve.rs",
            "/ui/app/../app/main.js",
            "/ui/%2e%2e/index.html",
            "/ui//app/main.js",
            "/ui/app\\main.js",
            "/ui/",
            "/ui/app/main.txt",
        ] {
            assert_eq!(route_ui(p, None).unwrap().0, "404 Not Found", "{p}");
        }
        assert!(route_ui("/api/catalog", None).is_none());
        assert!(route_ui("/classic", None).is_none());
    }

    #[test]
    fn root_and_app_serve_the_new_ui_and_classic_the_old_one_with_a_banner() {
        let new = route_ui("/ui/app/index.html", None).unwrap().2;
        for p in ["/", "/index.html", "/app", "/app/", "/?x=1"] {
            let (status, kind, body) = route_ui(p, None).unwrap();
            assert_eq!(status, "200 OK", "{p}");
            assert!(kind.starts_with("text/html"), "{p}");
            assert_eq!(body, new, "{p} must be the new UI");
        }
        let classic = String::from_utf8(classic_page()).unwrap();
        assert!(classic.contains("Interfaz anterior: se retirará en la próxima entrega"));
        assert!(classic.contains(r#"href="/""#));
        assert!(classic.contains("<h1>Kiln</h1>"), "the old page is still there");
        assert_ne!(classic.as_bytes(), new.as_slice());
        assert!(classic.find("Interfaz anterior").unwrap() < classic.find("<h1>Kiln</h1>").unwrap());
        assert!(!String::from_utf8(new).unwrap().contains("Interfaz anterior: se retirará"));
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-serve-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn ui_dir_reads_from_disk_and_stays_inside() {
        let root = scratch("uidir");
        let dir = root.join("ui");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::write(dir.join("app/main.js"), "// from disk").unwrap();
        std::fs::write(root.join("secret.js"), "// secret").unwrap();
        let (status, _, body) = route_ui("/ui/app/main.js", Some(&dir)).unwrap();
        assert_eq!((status, body.as_slice()), ("200 OK", &b"// from disk"[..]));
        assert_eq!(route_ui("/ui/../secret.js", Some(&dir)).unwrap().0, "404 Not Found");
        assert_eq!(route_ui("/ui/app/missing.js", Some(&dir)).unwrap().0, "404 Not Found");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- catalog cache (T9)

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let (src, dst) = (e.path(), to.join(e.file_name()));
            if src.is_dir() { copy_dir(&src, &dst) } else { std::fs::copy(&src, &dst).unwrap(); }
        }
    }

    fn project_copy(name: &str) -> (PathBuf, Ctx) {
        let root = scratch(name);
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/content"), &root);
        let ctx = make_ctx(&root, None).unwrap();
        (root, ctx)
    }

    #[test]
    fn catalog_is_loaded_once_for_two_requests() {
        let (root, ctx) = project_copy("cache-hit");
        let a = catalog_reply(&ctx, None);
        let b = catalog_reply(&ctx, None);
        assert_eq!((a.0, b.0), ("200 OK", "200 OK"));
        assert_eq!(a.2, b.2);
        assert_eq!(catalog_loads(&ctx.project), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn touching_a_project_file_reloads_the_catalog() {
        let (root, ctx) = project_copy("cache-touch");
        catalog_reply(&ctx, None);
        let f = std::fs::OpenOptions::new().write(true).open(root.join("kiln.toml")).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2);
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_and_git_folders_do_not_invalidate_but_a_new_file_does() {
        let (root, ctx) = project_copy("cache-skip");
        catalog_reply(&ctx, None);
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("target/junk"), "x").unwrap();
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 1);
        std::fs::write(root.join("new-file.toml"), "x").unwrap();
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn project_name_of(body: &[u8]) -> String {
        serde_json::from_slice::<serde_json::Value>(body).unwrap()["project"].as_str().unwrap().to_string()
    }

    // What is guaranteed: a rebuilt catalog that comes out identical keeps its
    // ETag (it is a hash of the content), so `touch` reloads but does not change it.
    #[test]
    fn touching_reloads_but_an_identical_catalog_keeps_its_etag() {
        let (root, ctx) = project_copy("cache-etag-touch");
        let first = catalog_reply(&ctx, None);
        let f = std::fs::OpenOptions::new().write(true).open(root.join("kiln.toml")).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(7)).unwrap();
        let second = catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2, "the loader ran again");
        assert_eq!(first.1, second.1, "same content, same ETag");
        let _ = std::fs::remove_dir_all(&root);
    }

    // An edit that changes the content but not the size, with the modification
    // time put back, must not be served stale (unix: the change time moves).
    #[cfg(unix)]
    #[test]
    fn same_size_edit_with_restored_mtime_is_not_served_stale() {
        let (root, ctx) = project_copy("cache-same-size");
        let toml = root.join("kiln.toml");
        let (_, etag1, body1) = catalog_reply(&ctx, None);
        assert_eq!(project_name_of(&body1), "Content fixture");
        let mtime = std::fs::metadata(&toml).unwrap().modified().unwrap();
        let text = std::fs::read_to_string(&toml).unwrap();
        let edited = text.replacen("Content fixture", "Content fixtuRe", 1);
        assert_eq!(text.len(), edited.len());
        std::fs::write(&toml, edited).unwrap();
        std::fs::OpenOptions::new().write(true).open(&toml).unwrap().set_modified(mtime).unwrap();
        assert_eq!(std::fs::metadata(&toml).unwrap().modified().unwrap(), mtime);
        let (_, etag2, body2) = catalog_reply(&ctx, None);
        assert_eq!(project_name_of(&body2), "Content fixtuRe");
        assert_ne!(etag1, etag2);
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edit_to_a_data_file_inside_the_project_reloads() {
        let (root, ctx) = project_copy("cache-data-edit");
        catalog_reply(&ctx, None);
        let ron = std::fs::read_dir(root.join("data/content")).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "ron")).expect("a content file");
        let mut text = std::fs::read_to_string(&ron).unwrap();
        text.push_str("\n// edited\n");
        std::fs::write(&ron, text).unwrap();
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The config can put the Godot cache (or kits, tables...) outside the
    // project folder; the loader reads them, so the fingerprint must see them.
    #[test]
    fn inputs_outside_the_project_are_in_the_fingerprint() {
        let base = scratch("cache-outside");
        let root = base.join("proj");
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/content"), &root);
        let ext = base.join("ext-cache");
        std::fs::create_dir_all(&ext).unwrap();
        let toml = root.join("kiln.toml");
        let text = std::fs::read_to_string(&toml).unwrap().replacen("[tools]", "[tools]\ncache = \"../ext-cache\"", 1);
        std::fs::write(&toml, text).unwrap();
        let ctx = make_ctx(&std::fs::canonicalize(&root).unwrap(), None).unwrap();
        let before = fingerprint(&ctx);
        assert!(outside_inputs(&ctx).iter().any(|p| p.ends_with("ext-cache")));
        std::fs::write(ext.join("pieces.json"), "{}").unwrap();
        assert_ne!(before, fingerprint(&ctx));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn releer_asks_for_a_fresh_read_with_the_explicit_marker() {
        let h = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
        assert!(wants_fresh(&h("x-kiln-fresh", "1")));
        assert!(!wants_fresh(&h("x-kiln-fresh", "0")));
        assert!(!wants_fresh(&h("if-none-match", "\"x\"")));
    }

    #[test]
    fn cache_control_no_cache_alone_does_not_reload() {
        let (root, ctx) = project_copy("cache-nocache");
        let h = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
        catalog_request(&ctx, &h("cache-control", "no-cache"));
        catalog_request(&ctx, &h("pragma", "no-cache"));
        catalog_request(&ctx, &h("cache-control", "no-cache"));
        assert_eq!(catalog_loads(&ctx.project), 1);
        catalog_request(&ctx, &h("x-kiln-fresh", "1"));
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dropping_the_cache_forces_a_reload() {
        let (root, ctx) = project_copy("cache-drop");
        catalog_reply(&ctx, None);
        drop_catalog(&ctx.project);
        catalog_reply(&ctx, None);
        assert_eq!(catalog_loads(&ctx.project), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn catalog_answers_304_when_the_etag_matches() {
        let (root, ctx) = project_copy("cache-304");
        let (status, etag, body) = catalog_reply(&ctx, None);
        let etag = etag.unwrap();
        assert_eq!(status, "200 OK");
        assert!(!body.is_empty() && etag.starts_with('"') && etag.ends_with('"'));
        let (status, tag2, body) = catalog_reply(&ctx, Some(&etag));
        assert_eq!((status, tag2.as_deref(), body.len()), ("304 Not Modified", Some(etag.as_str()), 0));
        assert_eq!(catalog_reply(&ctx, Some("W/\"other\", *")).0, "304 Not Modified");
        assert_eq!(catalog_reply(&ctx, Some("\"other\"")).0, "200 OK");
        assert_eq!(catalog_loads(&ctx.project), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- jobs list (T10)

    fn job(id: u64, status: &str) -> agent::Job {
        agent::Job { id, asked: format!("tarea {id}"), status: status.into(), log: vec!["linea".into()], spec: None, reply: None, offer: None, kind: "montar".into(), started: 1_700_000_000 + id }
    }

    #[test]
    fn jobs_list_has_active_and_the_latest_finished_without_logs() {
        let jobs: Jobs = Default::default();
        {
            let mut j = jobs.lock().unwrap();
            for i in 1..=9 { j.push(job(i, if i == 2 { "running" } else if i % 2 == 0 { "failed" } else { "done" })); }
        }
        let v = jobs_list(&jobs);
        let list = v["jobs"].as_array().unwrap();
        let ids: Vec<u64> = list.iter().map(|j| j["id"].as_u64().unwrap()).collect();
        assert_eq!(ids, vec![2, 5, 6, 7, 8, 9]);
        assert_eq!(list[0]["state"], "running");
        assert_eq!(list[0]["label"], "tarea 2");
        assert_eq!(list[0]["kind"], "montar");
        assert_eq!(list[0]["started"], 1_700_000_002u64);
        assert!(list[0].get("log").is_none());
        assert_eq!(jobs_list(&Default::default())["jobs"].as_array().unwrap().len(), 0);
    }
}
