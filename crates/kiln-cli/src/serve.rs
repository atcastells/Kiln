//! A local server on 127.0.0.1 only: the graph page, its JSON and the jobs
//! of the agent. The desktop shell (Tauri) will wrap the same page later.

use crate::agent::{self, Ctx, Jobs};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

const PAGE: &str = include_str!("../ui/index.html");

pub fn run(project: PathBuf, config: Option<PathBuf>, port: u16) -> Result<(), String> {
    let cfg = match &config {
        Some(c) => kiln_core::Config::load_file(c)?,
        None => kiln_core::Config::load(&project)?,
    };
    let ctx = Arc::new(Ctx {
        project: project.clone(),
        config,
        specs_dir: cfg
            .assets
            .specs
            .clone()
            .unwrap_or_else(|| "kiln/specs".into()),
    });
    let jobs: Jobs = Default::default();
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("no se puede abrir el puerto {port}: {e}"))?;
    eprintln!(
        "Kiln en http://127.0.0.1:{port}  (proyecto {})",
        project.display()
    );
    for stream in listener.incoming().flatten() {
        let (ctx, jobs) = (ctx.clone(), jobs.clone());
        std::thread::spawn(move || {
            if let Err(e) = handle(stream, &ctx, &jobs) {
                eprintln!("petición fallida: {e}");
            }
        });
    }
    Ok(())
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

fn handle(mut stream: TcpStream, ctx: &Arc<Ctx>, jobs: &Jobs) -> std::io::Result<()> {
    let req = read_request(&stream)?;
    let header = |k: &str| {
        req.headers
            .iter()
            .find(|(h, _)| h == k)
            .map(|(_, v)| v.as_str())
    };
    let json = |v: serde_json::Value| {
        (
            "200 OK",
            "application/json",
            serde_json::to_vec(&v).unwrap_or_default(),
        )
    };
    let (status, kind, body) = match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/" | "/index.html") => (
            "200 OK",
            "text/html; charset=utf-8",
            PAGE.as_bytes().to_vec(),
        ),
        ("GET", "/api/catalog") => match crate::load(&ctx.project, ctx.config.as_deref()) {
            Ok(cat) => (
                "200 OK",
                "application/json",
                serde_json::to_vec(&cat).unwrap_or_default(),
            ),
            Err(e) => (
                "500 Internal Server Error",
                "application/json",
                serde_json::to_vec(&serde_json::json!({ "error": e })).unwrap_or_default(),
            ),
        },
        // Only the page itself may start the agent: a custom header forces a CORS
        // preflight this server never answers, so another site cannot post here.
        ("POST", "/api/ask")
            if header("x-kiln") == Some("1")
                && header("host").is_some_and(|h| {
                    h.starts_with("127.0.0.1:") || h.starts_with("localhost:")
                }) =>
        {
            let text = serde_json::from_slice::<serde_json::Value>(&req.body)
                .ok()
                .and_then(|v| {
                    v.get("text")
                        .and_then(|t| t.as_str())
                        .map(str::trim)
                        .map(String::from)
                })
                .unwrap_or_default();
            if text.is_empty() || text.len() > 500 {
                (
                    "400 Bad Request",
                    "application/json",
                    br#"{"error":"la peticion va vacia o pasa de 500 caracteres"}"#.to_vec(),
                )
            } else {
                json(serde_json::json!({ "job": agent::start(jobs, ctx.clone(), text) }))
            }
        }
        ("GET", p) if p.starts_with("/api/jobs/") => match p[10..]
            .parse::<u64>()
            .ok()
            .and_then(|id| jobs.lock().unwrap().get((id.max(1) - 1) as usize).cloned())
        {
            Some(job) => json(serde_json::to_value(job).unwrap_or_default()),
            None => (
                "404 Not Found",
                "application/json",
                br#"{"error":"no existe ese trabajo"}"#.to_vec(),
            ),
        },
        _ => ("404 Not Found", "text/plain", b"no existe".to_vec()),
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}
