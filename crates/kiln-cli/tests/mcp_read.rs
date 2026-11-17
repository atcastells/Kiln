//! Read tools over `kiln mcp`.
mod common;

use common::{Mcp, temp_project};
use serde_json::json;

#[test]
fn tools_list_has_the_read_tools_as_read_only() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools = m.tools();
    for name in ["status", "catalog", "check", "tree", "spec_get", "content"] {
        let t = tools.iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("{name} missing"));
        assert_eq!(t["annotations"]["readOnlyHint"], true, "{name}: {t}");
    }
}

#[test]
fn status_lists_the_tools_and_marks_a_missing_one_absent() {
    let p = temp_project("");
    let file = p.path().join("kiln.toml");
    let toml = std::fs::read_to_string(&file).unwrap().replace(
        "[tools]\n",
        "[tools]\nblender = \"/nonexistent/kiln-no-blender\"\ngodot = \"/nonexistent/kiln-no-godot\"\n\
         gltf_validator = \"/nonexistent/kiln-no-validator\"\ngltf_transform = \"/nonexistent/kiln-no-transform\"\n",
    );
    std::fs::write(&file, toml).unwrap();
    let mut m = Mcp::start(p.path());
    let r = m.call("status", json!({}));
    assert!(!r.is_error, "{}", r.text);
    let j = r.json();
    let tools = j["tools"].as_array().unwrap_or_else(|| panic!("no tools: {j}"));
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["blender", "godot", "gltf_validator", "gltf-transform", "claude", "curl"]);
    assert_eq!((tools[0]["found"].clone(), tools[0]["ok"].clone()), (json!(false), json!(false)), "{}", tools[0]);
}

#[test]
fn catalog_pages_and_reports_total() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("catalog", json!({ "what": "pieces", "limit": 1 }));
    assert!(!r.is_error, "{}", r.text);
    let j = r.json();
    assert!(j["total"].as_u64().unwrap() > 1, "{j}");
    assert_eq!(j["items"].as_array().unwrap().len(), 1);
}

#[test]
fn catalog_unknown_what_lists_the_valid_values() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("catalog", json!({ "what": "bogus" }));
    assert!(r.is_error);
    assert!(r.text.contains("kits") && r.text.contains("specs"), "{}", r.text);
}

#[test]
fn check_counts_match_the_status() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let c = m.call("check", json!({})).json();
    let s = m.call("status", json!({})).json();
    assert_eq!(c["counts"], s["findings"]);
    let only = m.call("check", json!({ "severity": "error" })).json();
    assert_eq!(only["total"], c["counts"]["error"]);
}

#[test]
fn tree_gives_the_root_then_a_child() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let root = m.call("tree", json!({}));
    assert!(!root.is_error, "{}", root.text);
    let kids = root.json()["children"].clone();
    let id = kids[0]["key"].as_str().expect("child key").to_string();
    let child = m.call("tree", json!({ "node": id }));
    assert!(!child.is_error, "{}", child.text);
}

#[test]
fn spec_get_reads_hoist_and_rejects_slashes() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("spec_get", json!({ "id": "hoist" }));
    assert!(!r.is_error, "{}", r.text);
    assert!(r.json()["toml"].as_str().unwrap().contains("hoist"));
    let bad = m.call("spec_get", json!({ "id": "../hoist" }));
    assert!(bad.is_error);
}

#[test]
fn content_lists_the_world_set() {
    let p = temp_project("content");
    let mut m = Mcp::start(p.path());
    let all = m.call("content", json!({}));
    assert!(!all.is_error, "{}", all.text);
    assert!(all.json().as_array().unwrap().iter().any(|s| s["name"] == "world"));
    let one = m.call("content", json!({ "set": "nope" }));
    assert!(one.is_error);
}

#[test]
fn spec_get_carries_the_contract_and_the_last_verification_when_there_are() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let before = m.call("spec_get", json!({ "id": "hoist" })).json();
    assert!(before.get("contract").is_none() && before.get("validation").is_none(), "{before}");
    let built = p.path().join("kiln/build/hoist");
    std::fs::create_dir_all(&built).unwrap();
    std::fs::write(built.join("asset.json"), r#"{"id":"hoist","outputs":[]}"#).unwrap();
    std::fs::write(built.join("validation.json"), r#"{"valid":true}"#).unwrap();
    let after = m.call("spec_get", json!({ "id": "hoist" })).json();
    assert_eq!(after["contract"]["id"], "hoist", "{after}");
    assert_eq!(after["validation"]["valid"], true, "{after}");
}
