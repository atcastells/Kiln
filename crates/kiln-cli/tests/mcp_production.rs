//! Production tools of `kiln mcp`: validation, jobs and the review sheets.
//! Blender, Godot, claude and curl never run here.
mod common;

use common::{Mcp, temp_project};
use serde_json::json;

fn tool(m: &mut Mcp, name: &str) -> serde_json::Value {
    m.tools().into_iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("falta {name}"))
}

#[test]
fn build_validates_the_asset() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    for args in [json!({"asset": ""}), json!({"asset": "a/b"})] {
        let r = m.call("build", args);
        assert!(r.is_error, "{}", r.text);
    }
}

#[test]
fn import_rejects_unknown_sites_and_bad_ids() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("import", json!({"asset": "a", "part": "b", "site": "evil", "id": "x"}));
    assert!(r.is_error && r.text.contains("polyhaven"), "{}", r.text);
    let r = m.call("import", json!({"asset": "a", "part": "../b", "site": "polyhaven", "id": "x"}));
    assert!(r.is_error, "{}", r.text);
    let r = m.call("import", json!({"asset": "a", "part": "b", "site": "polyhaven", "id": "x".repeat(201)}));
    assert!(r.is_error, "{}", r.text);
}

#[test]
fn job_unknown_is_an_error_and_jobs_starts_empty() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let r = m.call("job", json!({"id": 999, "wait_s": 0}));
    assert!(r.is_error, "{}", r.text);
    let r = m.call("jobs", json!({}));
    assert!(!r.is_error, "{}", r.text);
    assert_eq!(r.json(), json!([]));
}

#[test]
fn review_sheets_returns_one_image_per_sheet() {
    let p = temp_project("");
    let round = p.path().join("kiln/build/old_hoist/review/round-1");
    std::fs::create_dir_all(&round).unwrap();
    let png: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    for f in ["1-turn.png", "2-elevations.png", "3-parts.png", "4-silhouette.png"] {
        std::fs::write(round.join(f), png).unwrap();
    }
    std::fs::write(round.join("measures.json"), r#"{"h":1}"#).unwrap();
    let mut m = Mcp::start(p.path());
    let r = m.call("review_sheets", json!({"asset": "old_hoist"}));
    assert!(!r.is_error, "{}", r.text);
    let content = r.raw["content"].as_array().unwrap();
    let images: Vec<_> = content.iter().filter(|c| c["type"] == "image").collect();
    assert_eq!(images.len(), 4, "{}", r.text);
    assert!(images.iter().all(|c| c["mimeType"] == "image/png" && c["data"] == "iVBORw0KGgo="));
    let info = r.json();
    assert_eq!(info["round"], 1);
    assert_eq!(info["measures"]["h"], 1);
    assert_eq!(info["sheets"][0]["path"], "kiln/build/old_hoist/review/round-1/1-turn.png");
    let r = m.call("review_sheets", json!({"asset": "nope"}));
    assert!(r.is_error, "{}", r.text);
}

#[test]
fn annotations_tell_what_each_tool_does() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let b = tool(&mut m, "build");
    assert_eq!(b["annotations"]["readOnlyHint"], false);
    assert_eq!(b["annotations"]["destructiveHint"], true);
    let i = tool(&mut m, "import");
    assert_eq!(i["annotations"]["destructiveHint"], true);
    assert_eq!(i["annotations"]["openWorldHint"], true);
    for n in ["job", "jobs", "review_sheets"] {
        assert_eq!(tool(&mut m, n)["annotations"]["readOnlyHint"], true, "{n}");
    }
}
