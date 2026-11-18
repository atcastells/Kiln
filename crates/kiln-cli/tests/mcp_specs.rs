//! `spec_write` and `spec_layout` over MCP.
mod common;

use common::{Mcp, temp_project};
use serde_json::json;

#[test]
fn write_accepts_the_example_spec_and_rejects_bad_ones_untouched() {
    let p = temp_project("");
    let before = p.read("specs/hoist.toml");
    let mut m = Mcp::start(p.path());

    let w = m.call("spec_write", json!({ "id": "hoist", "toml": before }));
    assert!(!w.is_error, "{}", w.text);
    assert_eq!(w.json()["file"], "specs/hoist.toml");

    let bad = m.call("spec_write", json!({ "id": "hoist", "toml": "not = [toml" }));
    assert!(bad.is_error, "{}", bad.text);
    let mismatch = m.call("spec_write", json!({ "id": "other", "toml": before }));
    assert!(mismatch.is_error, "{}", mismatch.text);
    assert_eq!(p.read("specs/hoist.toml"), before);
    assert!(!p.exists("specs/other.toml"));
}

#[test]
fn write_with_error_findings_returns_them_and_writes_nothing() {
    let p = temp_project("");
    let before = p.read("specs/hoist.toml");
    let broken = before.replacen("kind = \"reuse\"", "kind = \"magic\"", 1);
    let mut m = Mcp::start(p.path());
    let r = m.call("spec_write", json!({ "id": "hoist", "toml": broken }));
    assert!(r.is_error, "{}", r.text);
    let j = r.json();
    assert!(j["error"].is_string(), "{j}");
    assert!(!j["findings"].as_array().unwrap().is_empty(), "{j}");
    assert_eq!(p.read("specs/hoist.toml"), before);
}

#[test]
fn layout_moves_a_part_and_rejects_an_unknown_one() {
    let p = temp_project("");
    let before = p.read("specs/hoist.toml");
    let mut m = Mcp::start(p.path());
    let ok = m.call(
        "spec_layout",
        json!({ "id": "hoist", "parts": [{ "part": "post", "at": [[1.0, 2.0, 3.0]] }] }),
    );
    assert!(!ok.is_error, "{}", ok.text);
    let after: toml::Table = p.read("specs/hoist.toml").parse().unwrap();
    let at = after["parts"][0]["at"].as_array().unwrap();
    assert_eq!(at[0].as_array().unwrap()[2].as_float(), Some(3.0));
    assert_eq!(after["asset"]["status"].as_str(), Some("layout"));

    let saved = p.read("specs/hoist.toml");
    assert_ne!(saved, before);
    let bad = m.call(
        "spec_layout",
        json!({ "id": "hoist", "parts": [{ "part": "nope", "at": [[0, 0, 0]] }] }),
    );
    assert!(bad.is_error, "{}", bad.text);
    assert!(bad.json()["error"].as_str().unwrap().contains("nope"));
    assert_eq!(p.read("specs/hoist.toml"), saved);
}

#[test]
fn both_tools_are_annotated_as_idempotent_overwrites() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools = m.tools();
    for name in ["spec_write", "spec_layout"] {
        let t = tools.iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("{name} missing"));
        assert_eq!(t["annotations"]["readOnlyHint"], false, "{name}");
        assert_eq!(t["annotations"]["destructiveHint"], true, "{name}");
        assert_eq!(t["annotations"]["idempotentHint"], true, "{name}");
    }
}

const ROCK: &str = r#"[asset]
id = "rock"
name = "Rock"
budget_tris = 6000

[generator]
seed = 7

[[generator.ops]]
op = "sphere"
radius_m = [2, 5]
segments = 32

[[generator.ops]]
op = "noise"
amplitude = 0.2
size = 0.5
"#;

#[test]
fn write_accepts_a_generated_spec_with_a_recipe() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let w = m.call("spec_write", json!({ "id": "rock", "toml": ROCK }));
    assert!(!w.is_error, "{}", w.text);
    assert!(p.exists("specs/rock.toml"));
    let back: toml::Table = p.read("specs/rock.toml").parse().unwrap();
    assert!(back["generator"].is_table());
}

#[test]
fn write_rejects_an_unknown_op_and_creates_no_file() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let bad = ROCK.replacen("op = \"noise\"", "op = \"crater\"", 1);
    let r = m.call("spec_write", json!({ "id": "rock", "toml": bad }));
    assert!(r.is_error, "{}", r.text);
    let codes: Vec<_> = r.json()["findings"].as_array().unwrap().iter().map(|f| f["code"].clone()).collect();
    assert!(codes.contains(&json!("generator_op")), "{codes:?}");
    assert!(!p.exists("specs/rock.toml"));
}

#[test]
fn write_over_a_generated_spec_with_an_invalid_one_keeps_the_old_bytes() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let ok = m.call("spec_write", json!({ "id": "rock", "toml": ROCK }));
    assert!(!ok.is_error, "{}", ok.text);
    let before = std::fs::read(p.path().join("specs/rock.toml")).unwrap();
    let bad = ROCK.replacen("op = \"noise\"", "op = \"crater\"", 1);
    let r = m.call("spec_write", json!({ "id": "rock", "toml": bad }));
    assert!(r.is_error, "{}", r.text);
    assert_eq!(std::fs::read(p.path().join("specs/rock.toml")).unwrap(), before);
}
