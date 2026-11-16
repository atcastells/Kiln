//! Materials, zones and search over `kiln mcp`.
mod common;

use common::{Mcp, temp_project};
use serde_json::json;

const BINDINGS: &str = "kiln/bindings.toml";

#[test]
fn bind_writes_the_bindings_file() {
    let p = temp_project("");
    let before = p.read(BINDINGS);
    let mut m = Mcp::start(p.path());
    let r = m.call("bind", json!({"piece": "harbor/crate", "slot": "Wood", "material": "oak"}));
    assert!(!r.is_error, "{}", r.text);
    assert_ne!(p.read(BINDINGS), before);
}

#[test]
fn bind_with_empty_piece_fails_and_writes_nothing() {
    let p = temp_project("");
    let before = p.read(BINDINGS);
    let existed = p.exists(BINDINGS);
    let mut m = Mcp::start(p.path());
    let r = m.call("bind", json!({"piece": "", "slot": "Wood", "material": "oak"}));
    assert!(r.is_error);
    assert!(r.text.contains("falta la pieza"), "{}", r.text);
    assert_eq!(p.exists(BINDINGS), existed);
    assert_eq!(p.read(BINDINGS), before);
}

#[test]
fn material_set_outside_the_palette_says_so() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call(
        "material_set",
        json!({"asset": "hoist", "part": "load", "zone": "Wood", "material": "nope"}),
    );
    assert!(r.is_error);
    assert!(r.text.contains("no está en la paleta"), "{}", r.text);
}

#[test]
fn zones_split_unknown_piece_fails() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("zones_split", json!({"piece": "nope/x", "mode": "loose"}));
    assert!(r.is_error);
    assert!(r.text.contains("no está en el catálogo"), "{}", r.text);
}

#[test]
fn search_unknown_part_fails_before_the_network() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("search", json!({"asset": "cargo_quay", "part": "nope"}));
    assert!(r.is_error);
    assert!(r.text.contains("no tiene la pieza"), "{}", r.text);
    let r = m.call("search", json!({"asset": "nope", "part": "deck"}));
    assert!(r.is_error);
}

#[test]
fn annotations_tell_reads_from_writes() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools = m.tools();
    let get = |n: &str| tools.iter().find(|t| t["name"] == n).unwrap_or_else(|| panic!("{n}"))["annotations"].clone();
    let s = get("search");
    assert_eq!(s["openWorldHint"], true, "{s}");
    assert_eq!(s["readOnlyHint"], true);
    for n in ["bind", "material_set", "material_validate", "zones_split"] {
        let a = get(n);
        assert_eq!(a["readOnlyHint"], false, "{n}: {a}");
        assert_eq!(a["destructiveHint"], true, "{n}: {a}");
        assert_eq!(a["idempotentHint"], true, "{n}: {a}");
    }
}
