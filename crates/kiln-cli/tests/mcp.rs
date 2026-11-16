//! `kiln mcp` over stdio: the handshake, the tools and what goes to stdout.
mod common;

use common::{Mcp, empty_dir, temp_project};

#[test]
fn initialize_announces_tools_and_prompts() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools = m.tools();
    assert!(tools.iter().any(|t| t["name"] == "status"), "{tools:?}");
    let prompts = m.request("prompts/list", serde_json::json!({}));
    assert!(prompts["prompts"].is_array(), "{prompts}");
}

#[test]
fn status_reads_the_project() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let s = m.call("status", serde_json::json!({}));
    assert!(!s.is_error, "{}", s.text);
    assert_eq!(s.json()["project"], "Puerto Niebla");
}

#[test]
fn without_kiln_toml_tools_say_what_to_do() {
    let d = empty_dir();
    let mut m = Mcp::start(d.path());
    let s = m.call("status", serde_json::json!({}));
    assert!(s.is_error);
    assert!(s.text.contains("new_game"), "{}", s.text);
}

#[test]
fn stdout_carries_only_json_rpc_and_logs_go_to_stderr() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    m.call("status", serde_json::json!({}));
    let err = m.stop();
    assert!(err.contains("Kiln MCP por stdio"), "{err}");
}
