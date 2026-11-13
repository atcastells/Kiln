//! A local server on 127.0.0.1 only: the graph page and its JSON. The
//! desktop shell (Tauri) will wrap the same page in a later delivery.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

const PAGE: &str = include_str!("../ui/index.html");

pub fn run(project: PathBuf, config: Option<PathBuf>, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("no se puede abrir el puerto {port}: {e}"))?;
    eprintln!(
        "Kiln en http://127.0.0.1:{port}  (proyecto {})",
        project.display()
    );
    for stream in listener.incoming().flatten() {
        if let Err(e) = handle(stream, &project, config.as_deref()) {
            eprintln!("petición fallida: {e}");
        }
    }
    Ok(())
}

fn handle(mut stream: TcpStream, project: &Path, config: Option<&Path>) -> std::io::Result<()> {
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or("/");
    let (status, kind, body) = match path {
        "/" | "/index.html" => (
            "200 OK",
            "text/html; charset=utf-8",
            PAGE.as_bytes().to_vec(),
        ),
        "/api/catalog" => match crate::load(project, config) {
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
        _ => ("404 Not Found", "text/plain", b"no existe".to_vec()),
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}
