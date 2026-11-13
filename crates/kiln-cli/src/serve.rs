//! A local server on 127.0.0.1 only: the graph page, its JSON and the jobs
//! of the agent. The desktop shell (Tauri) will wrap the same page later.

use crate::agent::{self, Ctx, Jobs, Task};
use crate::{fetch, production, review};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

const PAGE: &str = include_str!("../ui/index.html");
const THREE: &str = include_str!("../ui/vendor/three.js");

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
        roots: [
            project.clone(),
            project.join(&cfg.tools.build),
            project.join(&cfg.tools.cache),
        ]
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok().or(Some(p.clone())))
        .collect(),
        context_file: project.join(&cfg.context.file),
        layers: cfg.context.layers(),
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
    let from_page = header("x-kiln") == Some("1")
        && header("host")
            .is_some_and(|h| h.starts_with("127.0.0.1:") || h.starts_with("localhost:"));
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
        ("POST", "/api/build") => {
            let id = field("asset");
            if id.is_empty() {
                bad("falta el asset")
            } else {
                let c = ctx.clone();
                json(
                    serde_json::json!({ "job": agent::job(jobs, format!("montar {id}"), move |say| {
                    let glb = production::build_asset(&c.project, c.config.as_deref(), &id, say)?;
                    Ok(Some(rel(&c.project, &glb)))
                }) }),
                )
            }
        }
        ("POST", "/api/review") => {
            let id = field("asset");
            if id.is_empty() {
                bad("falta el asset")
            } else {
                let c = ctx.clone();
                json(
                    serde_json::json!({ "job": agent::job(jobs, format!("revisar {id}"), move |say| {
                    let file = review::run(&c.project, c.config.as_deref(), &id, say)?;
                    Ok(Some(rel(&c.project, &file)))
                }) }),
                )
            }
        }
        ("POST", "/api/godot") => {
            let c = ctx.clone();
            json(
                serde_json::json!({ "job": agent::job(jobs, "medir con Godot".into(), move |say| {
                production::measure(&c.project, c.config.as_deref(), say).map(|_| None)
            }) }),
            )
        }
        ("POST", "/api/search") => {
            let (asset, part) = (field("asset"), field("part"));
            match fetch::search(&ctx.project, ctx.config.as_deref(), &asset, &part) {
                Ok(found) => json(serde_json::to_value(found).unwrap_or_default()),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/import") => {
            let (asset, part, site, id) =
                (field("asset"), field("part"), field("site"), field("id"));
            if asset.is_empty() || part.is_empty() || site.is_empty() || id.is_empty() {
                bad("falta el asset, la pieza, el sitio o el modelo")
            } else {
                let c = ctx.clone();
                json(
                    serde_json::json!({ "job": agent::job(jobs, format!("traer {site}/{id} para {part}"), move |say| {
                    fetch::import(&c.project, c.config.as_deref(), &asset, &part, &site, &id, say).map(Some)
                }) }),
                )
            }
        }
        ("POST", "/api/bind") => {
            let (piece, slot, material) = (field("piece"), field("slot"), field("material"));
            let r = production::config(&ctx.project, ctx.config.as_deref()).and_then(|cfg| {
                kiln_core::bindings::set(&ctx.project, &cfg, &piece, &slot, &material)
            });
            match r {
                _ if piece.is_empty() || slot.is_empty() => bad("falta la pieza o el hueco"),
                Ok(()) => json(serde_json::json!({ "ok": true })),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/material") => {
            let (asset, part, zone, material, variant) = (
                field("asset"),
                field("part"),
                field("zone"),
                field("material"),
                field("variant"),
            );
            let r = crate::load(&ctx.project, ctx.config.as_deref()).and_then(|cat| {
                if !material.is_empty() && !cat.materials.iter().any(|m| m.key == material) {
                    return Err(format!("«{material}» no está en la paleta"));
                }
                let file = spec_file(&ctx.project, &cat, &asset)?;
                let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
                let variant = Some(variant.as_str()).filter(|v| !v.is_empty());
                let new = kiln_core::spec::set_material(&text, &part, &zone, &material, variant)?;
                std::fs::write(&file, new).map_err(|e| e.to_string())
            });
            match r {
                Ok(()) => json(serde_json::json!({ "ok": true })),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/validate") => {
            let (asset, part) = (field("asset"), field("part"));
            let r = crate::load(&ctx.project, ctx.config.as_deref()).and_then(|cat| {
                let cfg = production::config(&ctx.project, ctx.config.as_deref())?;
                let file = spec_file(&ctx.project, &cat, &asset)?;
                let p = cat
                    .specs
                    .iter()
                    .find(|s| s.asset.id == asset)
                    .and_then(|s| s.parts.iter().find(|p| p.id == part))
                    .ok_or(format!("la ficha no tiene la pieza «{part}»"))?;
                let piece = p
                    .source
                    .from
                    .as_deref()
                    .filter(|_| matches!(p.source.kind.as_str(), "reuse" | "variant"))
                    .filter(|f| cat.pieces.iter().any(|c| &c.key == f))
                    .ok_or("solo se valida una pieza que viene del catálogo")?;
                kiln_core::bindings::set_many(&ctx.project, &cfg, piece, &p.materials)?;
                let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
                let new = kiln_core::spec::clear_materials(&text, &part)?;
                std::fs::write(&file, new).map_err(|e| e.to_string())?;
                Ok(piece.to_string())
            });
            match r {
                Ok(piece) => json(serde_json::json!({ "ok": true, "piece": piece })),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/split") => {
            let (piece, mode) = (field("piece"), field("mode"));
            let r = crate::load(&ctx.project, ctx.config.as_deref()).and_then(|cat| {
                if !cat.pieces.iter().any(|p| p.key == piece) {
                    return Err(format!("«{piece}» no está en el catálogo"));
                }
                kiln_core::zones::set(&ctx.project, &piece, &mode)
            });
            match r {
                Ok(()) => json(serde_json::json!({ "ok": true })),
                Err(e) => bad(&e),
            }
        }
        ("POST", "/api/paint") => {
            let (id, variant) = (field("asset"), field("variant"));
            if id.is_empty() {
                bad("falta el asset")
            } else {
                let c = ctx.clone();
                json(
                    serde_json::json!({ "job": agent::job(jobs, format!("aplicar materiales a {id}"), move |say| {
                    let v = Some(variant.as_str()).filter(|v| !v.is_empty());
                    let glb = production::paint_asset(&c.project, c.config.as_deref(), &id, v, say)?;
                    Ok(Some(rel(&c.project, &glb)))
                }) }),
                )
            }
        }
        ("POST", "/api/context/sort") => {
            json(serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Sort) }))
        }
        ("POST", "/api/context/refine") => json(
            serde_json::json!({ "job": agent::start(jobs, ctx.clone(), Task::Refine(Some(field("layer")).filter(|l| !l.is_empty()))) }),
        ),
        ("POST", p) if p.starts_with("/api/context/") => {
            let r = kiln_core::context::load(&ctx.context_file).and_then(|mut c| {
                match p {
                    "/api/context/text" => {
                        c.text = body
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default()
                            .to_string()
                    }
                    "/api/context/answer" => {
                        c.answer(&field("id"), &field("answer"), &field("status"))?
                    }
                    "/api/context/claim" => c
                        .edit_claim(
                            &field("id"),
                            &field("action"),
                            &field("text"),
                            &field("layer"),
                        )
                        .map(|_| ())?,
                    _ => return Err("no existe".into()),
                }
                kiln_core::context::save(&ctx.context_file, &c)
            });
            match r {
                Ok(()) => json(serde_json::json!({ "ok": true })),
                Err(e) => bad(&e),
            }
        }
        ("GET", "/api/context") => match kiln_core::context::load(&ctx.context_file) {
            Ok(c) => json(serde_json::json!({
                "file": rel(&ctx.project, &ctx.context_file),
                "layers": ctx.layers,
                "context": c,
            })),
            Err(e) => bad(&e),
        },
        ("GET", "/vendor/three.js") => (
            "200 OK",
            "text/javascript; charset=utf-8",
            THREE.as_bytes().to_vec(),
        ),
        ("GET", p) if p.starts_with("/files/") => match file(ctx, &p[7..]) {
            Some((kind, body)) => ("200 OK", kind, body),
            None => ("404 Not Found", "text/plain", b"no existe".to_vec()),
        },
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

/// The spec file of an asset, absolute.
fn spec_file(
    project: &std::path::Path,
    cat: &kiln_core::Catalog,
    asset: &str,
) -> Result<PathBuf, String> {
    cat.specs
        .iter()
        .find(|s| s.asset.id == asset)
        .map(|s| project.join(&s.file))
        .ok_or(format!("no hay ficha con id «{asset}»"))
}

fn rel(root: &std::path::Path, p: &std::path::Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
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
