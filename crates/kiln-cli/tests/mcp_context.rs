//! The context tools: the agent's voice and the person's voice.
mod common;

use common::{Mcp, temp_project};
use serde_json::json;

fn context_toml(p: &common::TempDir) -> String {
    p.read("context.toml")
}

#[test]
fn context_get_returns_layers_and_claims() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("context_get", json!({}));
    assert!(!r.is_error, "{}", r.text);
    let j = r.json();
    assert!(j["layers"].as_array().is_some_and(|l| !l.is_empty()), "{j}");
    assert!(j["context"]["claims"].is_array(), "{j}");
}

#[test]
fn propose_never_overwrites_what_is_pinned() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let layer = m.call("context_get", json!({})).json()["layers"][0]["id"].as_str().unwrap().to_string();
    let added = m.call("context_edit", json!({ "kind": "claim", "action": "add", "text": "fijado por la persona", "layer": layer }));
    assert!(!added.is_error, "{}", added.text);
    let id = {
        let j = m.call("context_get", json!({})).json();
        j["context"]["claims"].as_array().unwrap().iter().find(|c| c["text"] == "fijado por la persona").unwrap().clone()
    };
    assert_eq!(id["pinned"], true);
    let id = id["id"].as_str().unwrap().to_string();

    let r = m.call("context_propose", json!({ "rewrites": [{ "id": id, "text": "versión del agente" }] }));
    assert!(!r.is_error, "{}", r.text);
    assert_eq!(r.json()["proposed"], json!([id]));
    let file = context_toml(&p);
    assert!(file.contains("fijado por la persona"), "{file}");
    assert!(file.contains("proposed = \"versión del agente\""), "{file}");
}

#[test]
fn intent_proposal_on_a_pinned_intent_is_only_proposed() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let w = m.call("context_propose", json!({ "intent": "Un juego de niebla." }));
    assert_eq!(w.json()["intent"], "written", "{}", w.text);
    m.call("context_edit", json!({ "kind": "intent", "action": "pin" }));
    let r = m.call("context_propose", json!({ "intent": "Otro juego." }));
    assert_eq!(r.json()["intent"], "proposed", "{}", r.text);
    let file = context_toml(&p);
    assert!(file.contains("Un juego de niebla."), "{file}");
    assert!(file.contains("proposed = \"Otro juego.\""), "{file}");
}

#[test]
fn context_edit_adds_a_pinned_claim_and_rejects_unknown_kinds() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let layer = m.call("context_get", json!({})).json()["layers"][0]["id"].as_str().unwrap().to_string();
    let r = m.call("context_edit", json!({ "kind": "claim", "action": "add", "text": "frase dictada", "layer": layer }));
    assert!(!r.is_error, "{}", r.text);
    let j = m.call("context_get", json!({})).json();
    let c = j["context"]["claims"].as_array().unwrap().iter().find(|c| c["text"] == "frase dictada").unwrap().clone();
    assert_eq!(c["pinned"], true);

    let bad = m.call("context_edit", json!({ "kind": "nope" }));
    assert!(bad.is_error);
    for k in ["text", "intent", "answer", "claim"] {
        assert!(bad.text.contains(k), "{}", bad.text);
    }
}

#[test]
fn settle_on_examples_fails_with_what_is_missing() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("settle", json!({}));
    assert!(r.is_error);
    assert!(r.text.contains("falta el párrafo de qué juego es"), "{}", r.text);
}

#[test]
fn annotations_separate_the_two_voices() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools = m.tools();
    let a = |n: &str| tools.iter().find(|t| t["name"] == n).unwrap_or_else(|| panic!("falta {n}"))["annotations"].clone();
    assert_eq!(a("context_get")["readOnlyHint"], true);
    assert_eq!((a("context_propose")["readOnlyHint"].clone(), a("context_propose")["destructiveHint"].clone()), (json!(false), json!(false)));
    assert_eq!((a("context_edit")["readOnlyHint"].clone(), a("context_edit")["destructiveHint"].clone()), (json!(false), json!(true)));
    assert_eq!((a("settle")["readOnlyHint"].clone(), a("settle")["destructiveHint"].clone()), (json!(false), json!(false)));
}
