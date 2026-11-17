//! `kiln verify` passes a good build, fails a broken GLB with the reason, and
//! `kiln publish` only takes what was verified.
//! Needs Blender and Godot on the PATH: `cargo test -p kiln-cli -- --ignored verify`.
mod common;

use common::temp_project;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn kiln(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .expect("kiln")
}

fn err(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn check<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("sin la comprobación {name}: {report}"))
}

#[test]
#[ignore]
fn verify_passes_a_good_build_and_publish_takes_only_what_was_verified() {
    let p = temp_project("");
    let toml = p.read("kiln.toml").replace(
        "[tools]\n",
        "[tools]\ngltf_validator = \"/nonexistent/kiln-no-validator\"\n",
    ) + "\n[publish]\ndir = \"godot/assets\"\n";
    std::fs::write(p.path().join("kiln.toml"), toml).unwrap();
    std::fs::write(
        p.path().join("godot/project.godot"),
        "config_version=5\n\n[application]\n\nconfig/name=\"kiln-test\"\n",
    )
    .unwrap();

    let out = kiln(p.path(), &["publish", "--asset", "hoist"]);
    assert!(
        !out.status.success() && err(&out).contains("verifica antes"),
        "{}",
        err(&out)
    );

    let out = kiln(p.path(), &["build", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let out = kiln(p.path(), &["verify", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let report: Value = serde_json::from_str(&p.read("kiln/build/hoist/validation.json")).unwrap();
    assert_eq!(report["valid"], true, "{report}");
    assert_eq!(check(&report, "kiln")["status"], "passed");
    assert_eq!(check(&report, "godot_import")["status"], "passed");
    // a validator that is not installed is absent, not a failure
    assert_eq!(check(&report, "gltf_validator")["status"], "absent");

    let out = kiln(p.path(), &["publish", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let published: Value = serde_json::from_str(&p.read("godot/assets/hoist/asset.json")).unwrap();
    assert_eq!(published["version"], 1, "{published}");
    assert!(p.exists("godot/assets/hoist/hoist.glb"));
    assert!(
        p.exists("godot/assets/hoist/hoist.glb.import"),
        "Godot importó lo publicado"
    );
    assert!(
        !p.exists("godot/assets/hoist/.kiln"),
        "nada de trabajo en la carpeta del juego"
    );

    let out = kiln(p.path(), &["publish", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let again: Value = serde_json::from_str(&p.read("godot/assets/hoist/asset.json")).unwrap();
    assert_eq!(again["version"], 1, "publicar lo mismo no sube la versión");

    // a GLB cut short fails with the reason, and what is not verified is not published
    let glb = p.path().join("kiln/build/hoist/hoist.glb");
    let bytes = std::fs::read(&glb).unwrap();
    std::fs::write(&glb, &bytes[..bytes.len() / 2]).unwrap();
    let out = kiln(p.path(), &["publish", "--asset", "hoist"]);
    assert!(
        !out.status.success() && err(&out).contains("cambió después"),
        "{}",
        err(&out)
    );
    let out = kiln(p.path(), &["verify", "--asset", "hoist"]);
    assert!(!out.status.success(), "{}", err(&out));
    let report: Value = serde_json::from_str(&p.read("kiln/build/hoist/validation.json")).unwrap();
    assert_eq!(report["valid"], false, "{report}");
    let codes: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"glb_invalid") || codes.contains(&"glb_broken_ref"),
        "{codes:?}"
    );
}

#[test]
#[ignore]
fn verify_uses_the_khronos_validator_when_it_is_installed() {
    if Command::new("gltf_validator")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("gltf_validator no está en el PATH: prueba omitida");
        return;
    }
    let p = temp_project("");
    let out = kiln(p.path(), &["build", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let out = kiln(p.path(), &["verify", "--asset", "hoist"]);
    assert!(out.status.success(), "{}", err(&out));
    let report: Value = serde_json::from_str(&p.read("kiln/build/hoist/validation.json")).unwrap();
    let khronos = check(&report, "gltf_validator");
    assert_eq!(khronos["status"], "passed", "{report}");
    assert!(
        khronos["detail"]
            .as_str()
            .unwrap()
            .contains("glTF Validator"),
        "{khronos}"
    );
}
