//! `new_game_options` and `new_game` over MCP.
mod common;

use common::{Mcp, empty_dir, temp_project};
use serde_json::json;

fn minimal() -> serde_json::Value {
    json!({
        "name": "Faro",
        "basics": {"camera": "primera persona", "licenses": "CC0", "tris": "hasta 5000"}
    })
}

#[test]
fn an_empty_folder_starts_at_step_one_and_new_game_unlocks_the_tools() {
    let d = empty_dir();
    let mut m = Mcp::start(d.path());

    let o = m.call("new_game_options", json!({}));
    assert!(!o.is_error, "{}", o.text);
    assert_eq!(o.json()["step"], 1);
    assert!(o.json()["basics"].as_array().is_some_and(|b| !b.is_empty()));

    let s = m.call("status", json!({}));
    assert!(s.is_error);
    assert!(s.text.contains("new_game"), "{}", s.text);

    let n = m.call("new_game", minimal());
    assert!(!n.is_error, "{}", n.text);
    assert!(d.exists("kiln.toml"));

    let s = m.call("status", json!({}));
    assert!(!s.is_error, "{}", s.text);

    let again = m.call("new_game", minimal());
    assert!(again.is_error);
    assert!(again.text.contains("kiln.toml"), "{}", again.text);

    let o = m.call("new_game_options", json!({}));
    assert!(o.json()["step"].is_number(), "{}", o.text);
}

#[test]
fn a_blank_name_is_refused() {
    let d = empty_dir();
    let mut m = Mcp::start(d.path());
    let n = m.call("new_game", json!({"name": "x".repeat(201)}));
    assert!(n.is_error);
    assert!(!d.exists("kiln.toml"));
}

#[test]
fn options_on_a_project_return_a_step() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let o = m.call("new_game_options", json!({}));
    assert!(!o.is_error, "{}", o.text);
    assert!(o.json()["step"].is_number(), "{}", o.text);
}
