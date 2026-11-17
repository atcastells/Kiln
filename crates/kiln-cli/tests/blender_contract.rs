//! A build writes the contract of the asset, and building the same thing
//! twice gives an equivalent result: the same structural hash.
//! Needs Blender on the PATH: `cargo test -p kiln-cli -- --ignored contract`.
mod common;

use common::temp_project;
use serde_json::Value;
use std::process::Command;

fn build(project: &std::path::Path) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["build", "--asset", "hoist", "--project"])
        .arg(project)
        .output()
        .expect("kiln build");
    assert!(
        out.status.success(),
        "kiln build falló (¿Blender en el PATH?): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text =
        std::fs::read_to_string(project.join("kiln/build/hoist/asset.json")).expect("asset.json");
    serde_json::from_str(&text).unwrap()
}

#[test]
#[ignore]
fn two_builds_of_the_same_spec_are_equivalent() {
    let p = temp_project("");
    let first = build(p.path());
    assert_eq!(first["id"], "hoist");
    assert_eq!(first["source"]["spec"]["path"], "specs/hoist.toml");
    assert_eq!(first["function"]["used_by"][0], "dock_work");
    let pieces = first["source"]["pieces"].as_array().unwrap();
    assert!(
        pieces
            .iter()
            .any(|x| x["key"] == "harbor/beam"
                && x["sha256"].as_str().is_some_and(|h| h.len() == 64)),
        "{pieces:?}"
    );
    let out = &first["outputs"][0];
    assert!(
        out["variant"].is_null() && out["file"] == "hoist.glb",
        "{out}"
    );
    assert!(
        first["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "blender"),
        "{}",
        first["tools"]
    );
    assert!(
        first["size_m"][1].as_f64().unwrap() > 4.0,
        "{}",
        first["size_m"]
    );

    let second = build(p.path());
    assert_eq!(
        first["outputs"][0]["structure_sha256"], second["outputs"][0]["structure_sha256"],
        "dos montajes de la misma ficha no son equivalentes"
    );
    eprintln!(
        "same bytes: {}",
        first["outputs"][0]["sha256"] == second["outputs"][0]["sha256"]
    );
}
