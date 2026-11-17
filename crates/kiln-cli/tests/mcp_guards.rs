//! Guarantees of the MCP surface: the tool list and its annotations are
//! pinned, bad ids and huge texts are refused without touching the disk, and
//! API keys never show up in any output.
mod common;

use common::{Mcp, empty_dir, temp_project};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

/// (name, readOnly, destructive, idempotent, openWorld); absent hint = false.
const SURFACE: [(&str, bool, bool, bool, bool); 28] = [
    ("bind", false, true, true, false),
    ("build", false, true, false, false),
    ("catalog", true, false, false, false),
    ("check", true, false, false, false),
    ("content", true, false, false, false),
    ("context_edit", false, true, false, false),
    ("context_get", true, false, false, false),
    ("context_propose", false, false, false, false),
    ("godot_measure", false, false, false, false),
    ("import", false, true, false, true),
    ("job", true, false, false, false),
    ("jobs", true, false, false, false),
    ("material_set", false, true, true, false),
    ("material_validate", false, true, true, false),
    ("new_game", false, false, false, false),
    ("new_game_options", true, false, false, false),
    ("paint", false, true, false, false),
    ("review", false, true, false, false),
    ("review_sheets", true, false, false, false),
    ("search", true, false, false, true),
    ("settle", false, false, false, false),
    ("spec_get", true, false, false, false),
    ("spec_layout", false, true, true, false),
    ("spec_write", false, true, true, false),
    ("status", true, false, false, false),
    ("tools", false, false, false, false),
    ("tree", true, false, false, false),
    ("zones_split", false, true, true, false),
];

#[test]
fn tool_list_and_annotations_are_pinned() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let mut tools = m.tools();
    tools.sort_by_key(|t| t["name"].as_str().unwrap().to_string());
    let flag = |t: &serde_json::Value, k: &str| t["annotations"][k].as_bool().unwrap_or(false);
    let got: Vec<(String, bool, bool, bool, bool)> = tools
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().to_string(),
                flag(t, "readOnlyHint"),
                flag(t, "destructiveHint"),
                flag(t, "idempotentHint"),
                flag(t, "openWorldHint"),
            )
        })
        .collect();
    let want: Vec<(String, bool, bool, bool, bool)> =
        SURFACE.iter().map(|&(n, a, b, c, d)| (n.to_string(), a, b, c, d)).collect();
    assert_eq!(got, want);

    let r = m.request("prompts/list", json!({}));
    let mut names: Vec<&str> = r["prompts"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["kiln-asset", "kiln-context", "kiln-review"]);
}

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let rel = e.path().strip_prefix(base).unwrap().to_string_lossy().to_string();
            if e.file_type().unwrap().is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(base, &e.path(), out);
            } else {
                out.insert(rel, std::fs::read(e.path()).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn bad_ids_and_paths_are_refused_without_touching_the_disk() {
    let p = temp_project("");
    let outside = std::env::temp_dir().join("x.toml");
    let outside_before = outside.exists();
    let before = snapshot(p.path());
    let mut m = Mcp::start(p.path());
    let spec = "[asset]\nid = \"x\"\n";
    for bad in ["../x", "a/b", "..", "/etc/passwd", "x\\y"] {
        let calls = [
            ("spec_get", json!({"id": bad})),
            ("spec_write", json!({"id": bad, "toml": spec})),
            ("spec_layout", json!({"id": bad, "parts": []})),
            ("material_set", json!({"asset": bad, "part": "p", "zone": "z", "material": "m"})),
            ("review_sheets", json!({"asset": bad})),
            ("build", json!({"asset": bad})),
            ("paint", json!({"asset": bad})),
        ];
        for (tool, args) in calls {
            let r = m.call(tool, args);
            assert!(r.is_error, "{tool} with {bad:?} was accepted: {}", r.text);
        }
    }
    let r = m.call("tree", json!({"node": "../../etc"}));
    assert!(r.is_error, "tree accepted ../../etc: {}", r.text);
    drop(m);
    assert_eq!(before, snapshot(p.path()), "the project changed");
    assert_eq!(outside.exists(), outside_before, "a file appeared in the temp dir");
}

/// Sends a `prompts/get` over raw JSON-RPC and returns the response.
fn raw_prompt(dir: &Path, args: serde_json::Value) -> serde_json::Value {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["mcp", "--project"])
        .arg(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params":
        {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}});
    writeln!(stdin, "{init}").unwrap();
    out.next().unwrap().unwrap();
    writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).unwrap();
    writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "id": 2, "method": "prompts/get",
        "params": {"name": "kiln-asset", "arguments": args}}))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out.next().unwrap().unwrap()).unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    v
}

#[test]
fn one_megabyte_inputs_are_refused_and_nothing_changes() {
    let p = temp_project("");
    let before = snapshot(p.path());
    let big = "a".repeat(1_000_000);
    let mut m = Mcp::start(p.path());

    let calls = [
        ("spec_write", json!({"id": "big_one", "toml": big})),
        ("context_propose", json!({"claims": [{"layer": "world", "text": big}]})),
        ("context_edit", json!({"kind": "claim", "action": "add", "layer": "world", "text": big})),
        ("catalog", json!({"what": "pieces", "filter": big})),
    ];
    for (tool, args) in calls {
        let r = m.call(tool, args);
        assert!(r.is_error, "{tool} accepted 1 MB: {}", &r.text.chars().take(200).collect::<String>());
    }
    drop(m);

    let v = raw_prompt(p.path(), json!({"request": big}));
    assert!(v.get("error").is_some(), "kiln-asset accepted a 1 MB request");

    assert_eq!(before, snapshot(p.path()), "the project changed");
}

#[test]
fn keys_never_leak() {
    const POLY: &str = "pp-secret-7f3a91";
    const SKETCH: &str = "sf-secret-c42e08";
    let p = temp_project("");
    let home = empty_dir();
    let home_str = home.path().to_string_lossy().to_string();
    let mut m = Mcp::start_with(
        p.path(),
        &[("KILN_POLYPIZZA_KEY", POLY), ("KILN_SKETCHFAB_TOKEN", SKETCH), ("HOME", &home_str)],
    );
    let mut seen = Vec::new();
    let mut call = |m: &mut Mcp, tool: &str, args: serde_json::Value| {
        let r = m.call(tool, args);
        seen.push(format!("{tool}: {}", r.raw));
        seen.push(r.text);
    };
    call(&mut m, "status", json!({}));
    for what in ["kits", "pieces", "materials", "specs"] {
        call(&mut m, "catalog", json!({"what": what}));
    }
    call(&mut m, "check", json!({}));
    call(&mut m, "tree", json!({}));
    call(&mut m, "spec_get", json!({"id": "hoist"}));
    call(&mut m, "context_get", json!({}));
    call(&mut m, "content", json!({}));
    call(&mut m, "jobs", json!({}));
    call(&mut m, "new_game_options", json!({}));
    call(&mut m, "search", json!({"asset": "hoist", "part": "no_such_part"}));
    call(&mut m, "import", json!({"asset": "hoist", "part": "no_such_part", "site": "nowhere", "id": "1"}));
    for (name, args) in [
        ("kiln-asset", json!({"request": "un ascensor oxidado"})),
        ("kiln-context", json!({"mode": "sort"})),
        ("kiln-review", json!({})),
    ] {
        let r = m.request("prompts/get", json!({"name": name, "arguments": args}));
        seen.push(r.to_string());
    }
    seen.push(m.request("tools/list", json!({})).to_string());
    seen.push(m.stop());

    assert!(seen.len() > 20);
    for s in &seen {
        assert!(!s.contains(POLY) && !s.contains(SKETCH), "a key leaked: {}", s.chars().take(300).collect::<String>());
    }
}
