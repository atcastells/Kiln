//! The core: one Kiln process that the page and every `kiln mcp` share. A job
//! launched from an MCP session keeps running after the client closes, the
//! page shows it, a later session finds it, and jobs survive a restart.
#![cfg(unix)]
mod common;

use common::{Mcp, TempDir, empty_dir};
use serde_json::json;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const ROCK: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = 2\nsegments = 16\n";

/// A project whose Godot is a script that takes `secs`, notes it ran and fails.
fn slow_project(secs: u32) -> TempDir {
    use std::os::unix::fs::PermissionsExt;
    let d = empty_dir();
    let p = d.path();
    std::fs::create_dir_all(p.join("specs")).unwrap();
    let godot = p.join("fake-godot");
    std::fs::write(&godot, format!("#!/bin/sh\nsleep {secs}\ntouch \"$(dirname \"$0\")/ran\"\nexit 1\n")).unwrap();
    std::fs::set_permissions(&godot, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        p.join("kiln.toml"),
        format!("[project]\nname = \"Lento\"\n\n[assets]\nspecs = \"specs\"\n\n[tools]\ngodot = \"{}\"\n", godot.display()),
    )
    .unwrap();
    std::fs::write(p.join("specs/rock.toml"), ROCK).unwrap();
    d
}

fn env(home: &TempDir) -> Vec<(&'static str, &str)> {
    vec![("KILN_HOME", home.path().to_str().unwrap())]
}

fn kiln(home: &TempDir, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .env("KILN_HOME", home.path())
        .output()
        .unwrap()
}

fn port(home: &TempDir) -> u16 {
    let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(home.path().join("core.json")).unwrap()).unwrap();
    info["port"].as_u64().expect("the core serves the page") as u16
}

fn get(port: u16, path: &str) -> serde_json::Value {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    serde_json::from_slice(&raw[split..]).unwrap()
}

/// A write as the page sends it, carrying the `rev` it loaded.
fn post_from(port: u16, path: &str, rev: u64) -> u16 {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nx-kiln: 1\r\nx-kiln-rev: {rev}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    )
    .unwrap();
    let mut raw = String::new();
    let _ = s.read_to_string(&mut raw);
    raw.split_whitespace().nth(1).unwrap().parse().unwrap()
}

/// Like `post_from`, with a body and the answer's body.
fn post_body(port: u16, path: &str, rev: u64, body: &str) -> (u16, String) {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nx-kiln: 1\r\nx-kiln-rev: {rev}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = String::new();
    let _ = s.read_to_string(&mut raw);
    let status = raw.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, raw.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default())
}

fn wait_for(what: impl Fn() -> bool, secs: u64) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if what() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn job_of(page: &serde_json::Value, id: u64) -> Option<serde_json::Value> {
    page["jobs"].as_array()?.iter().find(|j| j["id"] == id).cloned()
}

#[test]
fn a_job_outlives_the_client_and_the_page_and_a_new_session_see_it() {
    let home = empty_dir();
    let p = slow_project(2);
    let mut m = Mcp::start_with(p.path(), &env(&home));
    let r = m.call("generate", json!({"asset": "rock"}));
    assert!(!r.is_error, "{}", r.text);
    let id = r.json()["job"].as_u64().unwrap();
    let busy = m.call("generate", json!({"asset": "rock"}));
    assert!(busy.is_error && busy.text.contains("ya hay un trabajo en marcha"), "{}", busy.text);
    drop(m); // the client closes while Godot runs

    let port = port(&home);
    let page = get(port, "/api/jobs");
    assert_eq!(job_of(&page, id).map(|j| j["state"].clone()), Some(json!("running")), "{page}");
    let session = get(port, "/api/session");
    assert_eq!(session["project"], std::fs::canonicalize(p.path()).unwrap().display().to_string());

    assert!(wait_for(|| p.path().join("ran").exists(), 15), "the job stopped with its client");
    assert!(wait_for(|| job_of(&get(port, "/api/jobs"), id).is_some_and(|j| j["state"] == "failed"), 10));

    let mut again = Mcp::start_with(p.path(), &env(&home));
    let j = again.call("job", json!({"id": id, "wait_s": 1}));
    assert!(!j.is_error, "{}", j.text);
    assert_eq!(j.json()["status"], "failed", "{}", j.text);
    let list = again.call("jobs", json!({})).json();
    assert!(list.as_array().unwrap().iter().any(|j| j["id"] == id), "{list}");
}

#[test]
fn jobs_survive_a_restart_and_one_left_running_says_so() {
    let home = empty_dir();
    let p = slow_project(30);
    let mut m = Mcp::start_with(p.path(), &env(&home));
    let id = m.call("generate", json!({"asset": "rock"})).json()["job"].as_u64().unwrap();
    let refused = kiln(&home, &["stop"]);
    assert!(!refused.status.success(), "stop left a running job behind");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--force"));
    drop(m);
    let stopped = kiln(&home, &["stop", "--force"]);
    assert!(stopped.status.success(), "{}", String::from_utf8_lossy(&stopped.stderr));
    assert!(wait_for(|| !home.path().join("core.sock").exists(), 5));

    let mut back = Mcp::start_with(p.path(), &env(&home));
    let j = back.call("job", json!({"id": id, "wait_s": 0}));
    assert_eq!(j.json()["status"], "failed", "{}", j.text);
    assert!(j.text.contains("Kiln se cerró"), "{}", j.text);
    let next = back.call("generate", json!({"asset": "rock"}));
    assert_eq!(next.json()["job"], id + 1, "{}", next.text);
    let _ = kiln(&home, &["stop", "--force"]);
}

#[test]
fn the_page_follows_the_project_a_session_opens() {
    let home = empty_dir();
    let a = common::temp_project("");
    let b = slow_project(1);
    let mut m = Mcp::start_free(empty_dir().path(), &env(&home));
    assert!(!m.call("project_open", json!({"path": a.path().to_str().unwrap()})).is_error);
    let port = port(&home);
    let first = get(port, "/api/session");
    let canon = |d: &Path| std::fs::canonicalize(d).unwrap().display().to_string();
    assert_eq!(first["project"], canon(a.path()));
    assert!(!m.call("project_open", json!({"path": b.path().to_str().unwrap()})).is_error);
    let second = get(port, "/api/session");
    assert_eq!(second["project"], canon(b.path()));
    assert!(second["rev"].as_u64() > first["rev"].as_u64());
    assert_eq!(get(port, "/api/catalog")["project"], "Lento");
    // a page still on the first project cannot write into the second
    let old = first["rev"].as_u64().unwrap();
    assert_eq!(post_from(port, "/api/settle", old), 409);
    assert_ne!(post_from(port, "/api/settle", second["rev"].as_u64().unwrap()), 409);

    // `kiln serve --project` with a core running shows that project there
    let shown = kiln(&home, &["serve", "--project", a.path().to_str().unwrap()]);
    assert!(shown.status.success(), "{}", String::from_utf8_lossy(&shown.stderr));
    assert_eq!(get(port, "/api/session")["project"], canon(a.path()));
    assert_eq!(get(port, "/api/session")["trusted"], true, "the person named it on the command line");
    drop(m);
    let _ = kiln(&home, &["stop"]);
}

/// Every POST of the page that runs Blender, Godot or Claude.
const RUNS: &[&str] = &[
    "/api/godot",
    "/api/build",
    "/api/paint",
    "/api/generate",
    "/api/review",
    "/api/import",
    "/api/ask",
    "/api/chat",
    "/api/context/sort",
    "/api/context/refine",
];

#[test]
fn an_untrusted_project_is_read_only_on_the_page() {
    let home = empty_dir();
    let p = slow_project(0);
    let mut m = Mcp::start_free(empty_dir().path(), &env(&home));
    assert!(!m.call("project_open", json!({"path": p.path().to_str().unwrap()})).is_error);
    let port = port(&home);
    let s = get(port, "/api/session");
    assert_eq!(s["trusted"], false, "{s}");
    let rev = s["rev"].as_u64().unwrap();
    for route in RUNS {
        let (status, body) = post_body(port, route, rev, "{}");
        assert_eq!(status, 403, "{route}: {body}");
        assert!(body.contains("kiln trust --project"), "{route}: {body}");
    }
    assert!(get(port, "/api/jobs")["jobs"].as_array().unwrap().is_empty(), "nothing was launched");
    assert!(!p.path().join("ran").exists(), "Godot ran in an untrusted project");

    // a folder with only a Godot project: the new-game page cannot run Claude either
    let g = empty_dir();
    std::fs::write(g.path().join("project.godot"), "").unwrap();
    assert!(!m.call("project_open", json!({"path": g.path().to_str().unwrap()})).is_error);
    let s = get(port, "/api/session");
    assert_eq!(s["trusted"], false, "{s}");
    assert_eq!(post_body(port, "/api/chat", s["rev"].as_u64().unwrap(), "{}").0, 403);

    // `kiln serve` in a folder without naming it does not vouch for it
    let served = std::process::Command::new(env!("CARGO_BIN_EXE_kiln"))
        .arg("serve")
        .current_dir(p.path())
        .env("KILN_HOME", home.path())
        .output()
        .unwrap();
    assert!(served.status.success(), "{}", String::from_utf8_lossy(&served.stderr));
    let s = get(port, "/api/session");
    assert_eq!(s["project"], std::fs::canonicalize(p.path()).unwrap().display().to_string());
    assert_eq!(s["trusted"], false, "{s}");

    // `kiln trust` counts at once, without opening the project again
    assert!(!m.call("project_open", json!({"path": p.path().to_str().unwrap()})).is_error);
    let trusted = kiln(&home, &["trust", "--project", p.path().to_str().unwrap()]);
    assert!(trusted.status.success(), "{}", String::from_utf8_lossy(&trusted.stderr));
    let s = get(port, "/api/session");
    assert_eq!(s["trusted"], true, "{s}");
    let (status, body) = post_body(port, "/api/generate", s["rev"].as_u64().unwrap(), r#"{"asset":"rock"}"#);
    assert_eq!(status, 200, "{body}");
    assert!(wait_for(|| p.path().join("ran").exists(), 10), "Godot did not run once trusted");
    drop(m);
    let _ = kiln(&home, &["stop", "--force"]);
}

#[test]
fn two_sessions_share_one_core() {
    let home = empty_dir();
    let p = slow_project(1);
    let mut a = Mcp::start_with(p.path(), &env(&home));
    let mut b = Mcp::start_with(p.path(), &env(&home));
    let id = a.call("generate", json!({"asset": "rock"})).json()["job"].as_u64().unwrap();
    let seen = b.call("jobs", json!({})).json();
    assert!(seen.as_array().unwrap().iter().any(|j| j["id"] == id), "{seen}");
    let pid = |h: &TempDir| std::fs::read_to_string(h.path().join("core.json")).unwrap();
    let before = pid(&home);
    drop(a);
    drop(b);
    assert_eq!(pid(&home), before, "one core for both");
    let _ = kiln(&home, &["stop", "--force"]);
}
