//! One `kiln mcp` for any project: the registry, `projects`, `project_open`,
//! `project_create` and how a server started without `--project` chooses.
mod common;

use common::{Mcp, TempDir, empty_dir, temp_project};
use serde_json::json;
use std::path::Path;

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn canon(p: &Path) -> String {
    std::fs::canonicalize(p).unwrap().display().to_string()
}

fn game() -> serde_json::Value {
    json!({"camera": "primera persona", "licenses": "CC0", "tris": "hasta 5000"})
}

fn home_env(h: &TempDir) -> [(&'static str, &str); 1] {
    [("KILN_HOME", s(h.path()))]
}

#[test]
fn two_games_in_one_session() {
    let home = empty_dir();
    let cwd = empty_dir();
    let a = temp_project("");
    let parent = empty_dir();
    let b = parent.path().join("Faro");
    let mut m = Mcp::start_free(cwd.path(), &home_env(&home));

    let st = m.call("status", json!({}));
    assert!(st.is_error && st.text.contains("project_open"), "{}", st.text);

    let o = m.call("project_open", json!({"path": s(a.path())}));
    assert!(!o.is_error, "{}", o.text);
    let st = m.call("status", json!({}));
    assert!(!st.is_error, "{}", st.text);
    assert_eq!(st.json()["project"], "Puerto Niebla");
    assert_eq!(st.json()["folder"], canon(a.path()));

    let c = m.call("project_create", json!({"path": s(&b), "name": "Faro", "basics": game()}));
    assert!(!c.is_error, "{}", c.text);
    assert!(b.join("kiln.toml").is_file());
    let st = m.call("status", json!({}));
    assert!(!st.is_error, "{}", st.text);
    assert_eq!(st.json()["folder"], canon(&b));

    let list = m.call("projects", json!({})).json();
    let rows = list["projects"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "{list}");
    assert_eq!(rows[0]["name"], "Faro");
    assert_eq!(rows[0]["open"], true);
    assert_eq!(rows[1]["name"], "Puerto Niebla");
    assert_eq!(rows[1]["open"], false);

    let o = m.call("project_open", json!({"path": s(a.path())}));
    assert!(!o.is_error, "{}", o.text);
    assert_eq!(m.call("status", json!({})).json()["folder"], canon(a.path()));
}

#[test]
fn project_open_refuses_what_is_not_a_project() {
    let home = empty_dir();
    let plain = empty_dir();
    let mut m = Mcp::start_free(plain.path(), &home_env(&home));
    for (path, says) in [
        ("relative/dir".to_string(), "absoluta"),
        (format!("{}/missing", s(plain.path())), "no existe"),
        (s(plain.path()).to_string(), "project_create"),
        ("/x".repeat(600), "1000"),
    ] {
        let r = m.call("project_open", json!({"path": path}));
        assert!(r.is_error && r.text.contains(says), "{path}: {}", r.text);
    }
    assert!(m.call("projects", json!({})).json()["projects"].as_array().unwrap().is_empty());
}

#[test]
fn a_godot_folder_opens_and_new_game_makes_it_a_kiln_project() {
    let home = empty_dir();
    let g = empty_dir();
    std::fs::write(g.path().join("project.godot"), "").unwrap();
    let cwd = empty_dir();
    let mut m = Mcp::start_free(cwd.path(), &home_env(&home));

    let o = m.call("project_open", json!({"path": s(g.path())}));
    assert!(!o.is_error, "{}", o.text);
    assert_eq!(o.json()["kiln"], false);
    assert!(o.json()["next"].as_str().unwrap().contains("new_game"));
    let st = m.call("status", json!({}));
    assert!(st.is_error && st.text.contains("Godot"), "{}", st.text);

    let n = m.call("new_game", json!({"name": "Isla", "basics": game()}));
    assert!(!n.is_error, "{}", n.text);
    let st = m.call("status", json!({}));
    assert!(!st.is_error, "{}", st.text);
    assert_eq!(st.json()["project"], "Isla");
}

#[test]
fn project_create_refuses_an_existing_project_or_a_missing_parent() {
    let home = empty_dir();
    let a = temp_project("");
    let cwd = empty_dir();
    let mut m = Mcp::start_free(cwd.path(), &home_env(&home));
    let r = m.call("project_create", json!({"path": s(a.path()), "name": "X", "basics": game()}));
    assert!(r.is_error && r.text.contains("kiln.toml"), "{}", r.text);
    let far = cwd.path().join("no/such/place");
    let r = m.call("project_create", json!({"path": s(&far), "name": "X", "basics": game()}));
    assert!(r.is_error && r.text.contains("madre"), "{}", r.text);
    assert!(!far.exists());
}

#[test]
fn without_project_the_working_folder_then_the_last_project_is_opened() {
    let home = empty_dir();
    let a = temp_project("");

    let mut m = Mcp::start_free(a.path(), &home_env(&home));
    assert_eq!(m.call("status", json!({})).json()["folder"], canon(a.path()));
    drop(m);

    let elsewhere = empty_dir();
    let mut m = Mcp::start_free(elsewhere.path(), &home_env(&home));
    let st = m.call("status", json!({}));
    assert!(!st.is_error, "{}", st.text);
    assert_eq!(st.json()["folder"], canon(a.path()));
}

#[test]
fn with_project_the_folder_is_noted_in_the_registry() {
    let home = empty_dir();
    let a = temp_project("");
    let mut m = Mcp::start_with(a.path(), &home_env(&home));
    let list = m.call("projects", json!({})).json();
    assert_eq!(list["projects"][0]["path"], canon(a.path()), "{list}");
    assert_eq!(list["projects"][0]["open"], true);
}

#[test]
fn the_clients_root_with_a_project_is_opened() {
    let home = empty_dir();
    let a = temp_project("");
    let cwd = empty_dir();
    let uri = format!("file://{}", canon(a.path()));
    let mut m = Mcp::start_free_with_roots(cwd.path(), &home_env(&home), &[&uri]);
    let mut folder = serde_json::Value::Null;
    for _ in 0..40 {
        let st = m.call("status", json!({}));
        if !st.is_error {
            folder = st.json()["folder"].clone();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(folder, canon(a.path()));
}

fn kiln_trust(home: &TempDir, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_kiln"))
        .arg("trust")
        .args(args)
        .env("KILN_HOME", home.path())
        .output()
        .unwrap()
}

#[test]
fn an_opened_project_runs_no_programs_until_the_person_trusts_it() {
    let home = empty_dir();
    let a = temp_project("");
    let cwd = empty_dir();
    let mut m = Mcp::start_free(cwd.path(), &home_env(&home));

    let o = m.call("project_open", json!({"path": s(a.path())}));
    assert_eq!(o.json()["trusted"], false, "{}", o.text);
    assert!(o.json()["next"].as_str().unwrap().contains("kiln trust"));
    assert_eq!(m.call("status", json!({})).json()["trusted"], false);
    for (tool, args) in [
        ("tools", json!({})),
        ("build", json!({"asset": "x"})),
        ("godot_measure", json!({})),
        ("publish", json!({"asset": "x"})),
        ("content_manifest", json!({})),
    ] {
        let r = m.call(tool, args);
        assert!(r.is_error && r.text.contains("kiln trust"), "{tool}: {}", r.text);
    }
    assert!(m.call("jobs", json!({})).text.contains("[]"), "no job may start");
    assert!(!m.call("catalog", json!({"what": "kits"})).is_error, "reading still works");

    let out = kiln_trust(&home, &["--project", s(a.path())]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(m.call("status", json!({})).json()["trusted"], true);
    let r = m.call("tools", json!({}));
    assert!(!r.is_error, "{}", r.text);
    let list = m.call("projects", json!({})).json();
    assert_eq!(list["projects"][0]["trusted"], true, "{list}");
}

#[test]
fn the_working_folder_is_opened_but_not_trusted() {
    let home = empty_dir();
    let a = temp_project("");
    let mut m = Mcp::start_free(a.path(), &home_env(&home));
    let st = m.call("status", json!({}));
    assert_eq!(st.json()["trusted"], false, "{}", st.text);
    assert!(m.call("tools", json!({})).is_error);
}

#[test]
fn a_project_kiln_creates_is_trusted() {
    let home = empty_dir();
    let cwd = empty_dir();
    let b = cwd.path().join("Faro");
    let mut m = Mcp::start_free(cwd.path(), &home_env(&home));
    let c = m.call("project_create", json!({"path": s(&b), "name": "Faro", "basics": game()}));
    assert!(!c.is_error, "{}", c.text);
    assert_eq!(m.call("status", json!({})).json()["trusted"], true);
    assert!(home.read("trusted.json").contains("Faro"), "{}", home.read("trusted.json"));
}

#[test]
fn kiln_trust_needs_a_project_folder() {
    let home = empty_dir();
    let plain = empty_dir();
    assert!(!kiln_trust(&home, &[]).status.success());
    assert!(!kiln_trust(&home, &["--project", s(plain.path())]).status.success());
    assert!(!home.exists("trusted.json"));
}

#[test]
fn jobs_say_which_project_they_belong_to() {
    let home = empty_dir();
    let a = temp_project("");
    let mut m = Mcp::start_with(a.path(), &home_env(&home));
    let r = m.call("content_manifest", json!({"set": "missing"}));
    assert!(r.is_error && r.text.contains("missing"), "{}", r.text);
    let r = m.call("content_manifest", json!({}));
    assert!(!r.is_error, "{}", r.text);
    let jobs = m.call("jobs", json!({})).json();
    let row = &jobs.as_array().or_else(|| jobs["jobs"].as_array()).unwrap()[0];
    assert_eq!(row["project"], canon(a.path()), "{jobs}");
}
