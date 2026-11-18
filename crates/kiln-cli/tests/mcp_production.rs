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

const ROCK: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 7\n\n\
[[generator.ops]]\nop = \"sphere\"\nradius_m = 2.0\nsegments = 16\n";

#[test]
fn mcp_generate_is_listed() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    tool(&mut m, "generate");
}

#[test]
fn mcp_generate_refuses_an_unknown_op_and_writes_nothing() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let bad = ROCK.replace("\"sphere\"", "\"crater\"");
    let r = m.call("generate", json!({"asset": "rock", "recipe": bad}));
    assert!(r.is_error && r.text.contains("generator_op"), "{}", r.text);
    assert!(!p.exists("specs/rock.toml"));
}

#[test]
fn mcp_generate_starts_a_job_that_fails_when_godot_is_missing() {
    let p = temp_project("");
    let cfg = p.read("kiln.toml").replace("[tools]\n", "[tools]\ngodot = \"/nonexistent/godot\"\n");
    std::fs::write(p.path().join("kiln.toml"), cfg).unwrap();
    let mut m = Mcp::start(p.path());
    let r = m.call("generate", json!({"asset": "rock", "seed": 3, "recipe": ROCK}));
    assert!(!r.is_error, "{}", r.text);
    let id = r.json()["job"].as_u64().expect("job id");
    let (mut since, mut last, mut seen) = (0, json!(null), String::new());
    for _ in 0..10 {
        last = m.call("job", json!({"id": id, "since": since, "wait_s": 5})).json();
        seen.push_str(&last.to_string());
        if last["status"] != "running" {
            break;
        }
        since = last["next"].as_u64().unwrap();
    }
    assert_eq!(last["status"], "failed", "{last}");
    assert!(seen.contains("godot"), "{seen}");
}

fn finish(m: &mut Mcp, id: u64) -> (serde_json::Value, String) {
    let (mut since, mut last, mut seen) = (0, json!(null), String::new());
    for _ in 0..10 {
        last = m.call("job", json!({"id": id, "since": since, "wait_s": 5})).json();
        seen.push_str(&last.to_string());
        if last["status"] != "running" {
            break;
        }
        since = last["next"].as_u64().unwrap();
    }
    (last, seen)
}

fn recipe_project(spec: &str, extra_cfg: &str) -> common::TempDir {
    let p = temp_project("");
    let cfg = p.read("kiln.toml").replace("[tools]\n", &format!("{extra_cfg}[tools]\n"));
    std::fs::write(p.path().join("kiln.toml"), cfg).unwrap();
    std::fs::write(p.path().join("specs/rock.toml"), spec).unwrap();
    p
}

#[test]
fn mcp_recipe_verify_starts_a_job_that_needs_godot() {
    let p = recipe_project(ROCK, "");
    let cfg = p.read("kiln.toml").replace("[tools]\n", "[tools]\ngodot = \"/nonexistent/godot\"\n");
    std::fs::write(p.path().join("kiln.toml"), cfg).unwrap();
    let mut m = Mcp::start(p.path());
    let r = m.call("verify", json!({"asset": "rock"}));
    assert!(!r.is_error && !r.text.contains("es un asset generado"), "{}", r.text);
    let id = r.json()["job"].as_u64().expect("job id");
    let (last, seen) = finish(&mut m, id);
    assert_eq!(last["status"], "failed", "{last}");
    assert!(seen.contains("/nonexistent/godot"), "{seen}");
}

#[test]
fn mcp_recipe_verify_reports_a_static_error_without_godot() {
    let thin = format!(
        "{ROCK}\n[[generator.ops]]\nop = \"stretch\"\ny = 0.2\n\n[[generator.ops]]\nop = \"noise\"\namplitude = 0.9\nsize = 0.5\n"
    );
    let p = recipe_project("", "");
    std::fs::remove_file(p.path().join("specs/rock.toml")).unwrap();
    let mut m = Mcp::start(p.path());
    let w = m.call("spec_write", json!({ "id": "rock", "toml": thin }));
    assert!(w.is_error && w.text.contains("generator_radius"), "{}", w.text);
    std::fs::write(p.path().join("specs/rock.toml"), &thin).unwrap();
    let r = m.call("verify", json!({"asset": "rock"}));
    assert!(!r.is_error, "{}", r.text);
    let id = r.json()["job"].as_u64().expect("job id");
    let (last, seen) = finish(&mut m, id);
    assert_eq!(last["status"], "failed", "{last}");
    assert!(seen.contains("radio"), "{seen}");
    let got = m.call("spec_get", json!({ "id": "rock" })).json();
    assert_eq!(got["validation"]["valid"], false, "{got}");
    let codes: Vec<_> =
        got["validation"]["findings"].as_array().unwrap().iter().map(|f| f["code"].clone()).collect();
    assert!(codes.contains(&json!("generator_radius")), "{got}");
    assert_eq!(got["validation"]["recipe"]["seeds"], json!([]), "{got}");
}

#[test]
fn mcp_recipe_publish_refuses_what_was_not_verified() {
    let p = recipe_project(ROCK, "[publish]\ndir = \"godot/assets\"\n\n");
    let mut m = Mcp::start(p.path());
    let r = m.call("publish", json!({"asset": "rock"}));
    let (status, seen) = if r.is_error {
        ("failed".into(), r.text.clone())
    } else {
        let id = r.json()["job"].as_u64().expect("job id");
        let (last, seen) = finish(&mut m, id);
        (last["status"].as_str().unwrap().to_string(), seen)
    };
    assert_eq!(status, "failed", "{seen}");
    assert!(seen.contains("verifica antes el asset"), "{seen}");
    assert!(!p.exists("godot/assets/rock"));
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
