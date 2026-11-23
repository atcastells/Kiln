//! Prompts over MCP: the four skills, their arguments and the request limit.
//! A request over 500 characters is a protocol error (invalid params).
mod common;

use common::{Mcp, temp_project};

fn args_of(prompts: &[serde_json::Value], name: &str) -> Vec<String> {
    let p = prompts.iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("falta {name}: {prompts:?}"));
    p["arguments"]
        .as_array()
        .map(|a| a.iter().map(|x| x["name"].as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn list_has_the_four_with_their_arguments() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.request("prompts/list", serde_json::json!({}));
    let ps = r["prompts"].as_array().unwrap();
    assert_eq!(ps.len(), 4, "{r}");
    // Claude Code splits a slash command on spaces and hands the words out in
    // order: `request` is the one and optional argument (approved by the developer).
    let asset = args_of(ps, "kiln-asset");
    assert!(asset == ["request"], "{asset:?}");
    let a = ps.iter().find(|p| p["name"] == "kiln-asset").unwrap();
    assert!(a["arguments"][0]["required"] != true, "{a}");
    let ctx = args_of(ps, "kiln-context");
    assert!(ctx.contains(&"mode".into()) && ctx.contains(&"layer".into()), "{ctx:?}");
    assert!(args_of(ps, "kiln-review").is_empty());
    // A slash command with no arguments: `/mcp__kiln__kiln-start` runs as it is.
    assert!(args_of(ps, "kiln-start").is_empty());
}

#[test]
fn start_prompt_walks_from_tools_to_publishing() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.request("prompts/get", serde_json::json!({"name": "kiln-start"}));
    let msgs = r["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1, "{r}");
    let t = msgs[0]["content"]["text"].as_str().unwrap();
    assert!(!t.starts_with("---"), "no front matter: {t}");
    let order = ["`setup`", "`example_create`", "`spec_write`", "`build`", "`preview`", "`verify`", "`publish`"];
    let at: Vec<usize> = order.iter().map(|w| t.find(w).unwrap_or_else(|| panic!("{w}: {t}"))).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "the steps are in order: {at:?}");
    assert!(t.contains("No uses nunca `reviewer = \"kiln\"`"), "D1: {t}");
    // The same text for clients that do not pass prompts on.
    let g = m.call("guide", serde_json::json!({"topic": "start"}));
    assert!(!g.is_error && g.text.contains("example_create"), "{}", g.text);
}

#[test]
fn asset_prompt_is_one_message_with_header_skill_and_request() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.request(
        "prompts/get",
        serde_json::json!({"name": "kiln-asset", "arguments": {"request": "un ascensor oxidado"}}),
    );
    let msgs = r["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1, "{r}");
    assert_eq!(msgs[0]["role"], "user");
    let t = msgs[0]["content"]["text"].as_str().unwrap();
    assert!(t.contains("spec_write"), "{t}");
    assert!(t.contains("Desgrana la petición de un asset"), "{t}");
    assert!(t.ends_with("un ascensor oxidado"), "{t}");
}

#[test]
fn asset_prompt_carries_the_generation_vocabulary() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.request("prompts/get", serde_json::json!({"name": "kiln-asset", "arguments": {"request": "una roca"}}));
    let t = r["messages"][0]["content"]["text"].as_str().unwrap();
    assert!(t.contains("Vocabulario de generación"), "{t}");
    assert!(t.contains("Vocabulario de modelado"), "{t}");
    for op in ["sphere", "stretch", "noise", "facet"] {
        assert!(t.contains(&format!("- {op} (")), "{op}: {t}");
    }
}

/// A client that does not hand prompts to the model reads the same text with `guide`.
#[test]
fn guide_tool_gives_the_prompt_texts() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let prompt = m.request("prompts/get", serde_json::json!({"name": "kiln-asset", "arguments": {"request": "x"}}));
    let prompt = prompt["messages"][0]["content"]["text"].as_str().unwrap().to_string();
    let asset = m.call("guide", serde_json::json!({"topic": "asset"}));
    let asset: String = serde_json::from_str(&asset.text).unwrap();
    assert!(asset.contains("Vocabulario de modelado") && prompt.starts_with(&asset), "{asset}");
    let review = m.call("guide", serde_json::json!({"topic": "review"}));
    let review: String = serde_json::from_str(&review.text).unwrap();
    assert!(review.contains("review_submit"), "{review}");
    assert!(m.call("guide", serde_json::json!({"topic": "otra"})).is_error);
}

#[test]
fn asset_prompt_without_request_asks_the_person_first() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    for args in [serde_json::json!({}), serde_json::json!({"request": "  "})] {
        let r = m.request("prompts/get", serde_json::json!({"name": "kiln-asset", "arguments": args}));
        let t = r["messages"][0]["content"]["text"].as_str().unwrap().to_string();
        assert!(t.contains("spec_write"), "{t}");
        assert!(t.ends_with("Pregunta a la persona qué asset quiere y espera su respuesta antes de empezar."), "{t}");
    }
}

#[test]
fn context_and_review_prompts_carry_their_header() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let c = m.request("prompts/get", serde_json::json!({"name": "kiln-context", "arguments": {"mode": "sort"}}));
    let t = c["messages"][0]["content"]["text"].as_str().unwrap();
    assert!(t.contains("context_propose") && t.contains("context_edit"), "{t}");
    let r = m.request("prompts/get", serde_json::json!({"name": "kiln-review"}));
    let t = r["messages"][0]["content"]["text"].as_str().unwrap();
    assert!(t.contains("review_sheets"), "{t}");
}

/// `Mcp::request` panics on protocol errors, so this talks raw JSON-RPC to its own process.
fn raw_get(dir: &std::path::Path, args: serde_json::Value) -> serde_json::Value {
    use std::io::{BufRead, Write};
    let home = common::empty_dir();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["mcp", "--project"])
        .arg(dir)
        .env("KILN_HOME", home.path())
        .env("KILN_CORE", "off")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let init = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params":
        {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}});
    let get = serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "prompts/get",
        "params": {"name": "kiln-asset", "arguments": args}});
    writeln!(stdin, "{init}").unwrap();
    out.next().unwrap().unwrap();
    writeln!(stdin, "{}", serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).unwrap();
    writeln!(stdin, "{get}").unwrap();
    let v: serde_json::Value = serde_json::from_str(&out.next().unwrap().unwrap()).unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    v
}

#[test]
fn request_over_500_chars_is_a_protocol_error() {
    let p = temp_project("");
    let v = raw_get(p.path(), serde_json::json!({"request": "a".repeat(501)}));
    assert!(v.get("error").is_some(), "{v}");
    let ok = raw_get(p.path(), serde_json::json!({"request": "a".repeat(500)}));
    assert!(ok.get("error").is_none(), "{ok}");
}
