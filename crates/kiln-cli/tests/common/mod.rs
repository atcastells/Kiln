//! Helpers shared by the integration tests: temporary projects, a `kiln
//! serve` process and a raw HTTP client. Each test file uses a part of them.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A folder under the system temp dir, removed when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    fn new() -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let dir = std::env::temp_dir().join(format!("kiln-test-{}-{n}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.0.join(rel)).unwrap_or_default()
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.0.join(rel).exists()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A temporary copy of `examples/<example>` (`""` is the examples root).
pub fn temp_project(example: &str) -> TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(example);
    let dir = TempDir::new();
    copy_dir(&src, dir.path());
    dir
}

/// An empty temporary folder: no `kiln.toml`.
pub fn empty_dir() -> TempDir {
    TempDir::new()
}

/// A running `kiln serve`, killed when dropped.
pub struct Server {
    child: Child,
    pub port: u16,
}

impl Server {
    pub fn start(dir: &TempDir) -> Server {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
            .args(["serve", "--project"])
            .arg(dir.path())
            .args(["--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            if let Ok(Some(status)) = child.try_wait() {
                panic!("kiln serve exited early: {status}");
            }
            if start.elapsed() > Duration::from_secs(10) {
                let _ = child.kill();
                panic!("kiln serve did not open port {port} in 10 s");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Server { child, port }
    }

    fn request(&self, head: &str, body: &[u8]) -> (u16, Vec<u8>) {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
        s.write_all(head.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("no header end") + 4;
        let status = std::str::from_utf8(&raw[..split]).unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, raw[split..].to_vec())
    }

    pub fn get(&self, path: &str) -> (u16, Vec<u8>) {
        let head = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n", self.port);
        self.request(&head, b"")
    }

    /// `page` adds `x-kiln: 1` and the local `Host`, as the page does.
    pub fn post(&self, path: &str, json: &str, page: bool) -> (u16, Vec<u8>) {
        let extra = if page { format!("x-kiln: 1\r\nHost: 127.0.0.1:{}\r\n", self.port) } else { String::new() };
        let head = format!(
            "POST {path} HTTP/1.1\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            json.len()
        );
        self.request(&head, json.as_bytes())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The `error` string of a JSON body, if any.
pub fn error_of(body: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    v.get("error")?.as_str().map(String::from)
}

pub fn json_of(body: &[u8]) -> serde_json::Value {
    serde_json::from_slice(body).unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(body)))
}

/// A `kiln mcp` process spoken to over stdio, already initialized.
pub struct Mcp {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
    out: std::sync::mpsc::Receiver<serde_json::Value>,
    next: u64,
    /// The default `KILN_HOME`, so tests never touch the user's registry.
    _home: TempDir,
}

/// What a tool call returned: whether it is an error and its first text.
pub struct Called {
    pub is_error: bool,
    pub text: String,
    pub raw: serde_json::Value,
}

impl Called {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.text).unwrap_or_else(|_| panic!("no es JSON: {}", self.text))
    }
}

impl Mcp {
    pub fn start(dir: &Path) -> Mcp {
        Self::start_with(dir, &[])
    }

    pub fn start_with(dir: &Path, env: &[(&str, &str)]) -> Mcp {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
        cmd.args(["mcp", "--project"]).arg(dir);
        Self::spawn(cmd, env, None)
    }

    /// `kiln mcp` without `--project`, run from `cwd`. `env` should set
    /// `KILN_HOME` when the test reads the registry.
    pub fn start_free(cwd: &Path, env: &[(&str, &str)]) -> Mcp {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
        cmd.arg("mcp").current_dir(cwd);
        Self::spawn(cmd, env, None)
    }

    fn spawn(mut cmd: Command, env: &[(&str, &str)], roots: Option<&[&str]>) -> Mcp {
        let home = TempDir::new();
        let mut child = cmd
            .env("KILN_HOME", home.path())
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("kiln mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, out) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
                let v: serde_json::Value =
                    serde_json::from_str(&line).unwrap_or_else(|_| panic!("stdout no es JSON-RPC: {line}"));
                if tx.send(v).is_err() {
                    break;
                }
            }
        });
        let mut m = Mcp { child, stdin: Some(stdin), out, next: 1, _home: home };
        let caps = if roots.is_some() { serde_json::json!({"roots": {}}) } else { serde_json::json!({}) };
        m.request(
            "initialize",
            serde_json::json!({"protocolVersion": "2025-06-18", "capabilities": caps, "clientInfo": {"name": "test", "version": "0"}}),
        );
        m.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        if let Some(uris) = roots {
            let asked = loop {
                let v = m.out.recv_timeout(Duration::from_secs(10)).expect("kiln mcp no pidió las roots");
                if v["method"] == "roots/list" {
                    break v;
                }
            };
            let list: Vec<_> = uris.iter().map(|u| serde_json::json!({"uri": u})).collect();
            m.send(serde_json::json!({"jsonrpc": "2.0", "id": asked["id"], "result": {"roots": list}}));
        }
        m
    }

    /// Like `start_free`, from a client that answers `roots/list` with `uris`.
    pub fn start_free_with_roots(cwd: &Path, env: &[(&str, &str)], uris: &[&str]) -> Mcp {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
        cmd.arg("mcp").current_dir(cwd);
        Self::spawn(cmd, env, Some(uris))
    }

    fn send(&mut self, v: serde_json::Value) {
        let stdin = self.stdin.as_mut().expect("kiln mcp ya está cerrado");
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    }

    /// Sends a request and returns its `result` (panics on a protocol error).
    pub fn request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next;
        self.next += 1;
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let v = self.out.recv_timeout(Duration::from_secs(40)).expect("sin respuesta de kiln mcp");
            if v["id"] == id && v.get("method").is_none() {
                assert!(v.get("error").is_none(), "error de protocolo: {v}");
                return v["result"].clone();
            }
        }
    }

    pub fn tools(&mut self) -> Vec<serde_json::Value> {
        self.request("tools/list", serde_json::json!({}))["tools"].as_array().unwrap().clone()
    }

    pub fn call(&mut self, name: &str, args: serde_json::Value) -> Called {
        let raw = self.request("tools/call", serde_json::json!({"name": name, "arguments": args}));
        Called {
            is_error: raw["isError"].as_bool().unwrap_or(false),
            text: raw["content"][0]["text"].as_str().unwrap_or_default().to_string(),
            raw,
        }
    }

    /// Closes stdin, waits for the process and returns its stderr.
    pub fn stop(mut self) -> String {
        drop(self.stdin.take());
        let mut err = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        let _ = self.child.wait();
        err
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
