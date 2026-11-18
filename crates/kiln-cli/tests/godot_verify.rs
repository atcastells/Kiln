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

const HUMANOID: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rig_humanoid.py");

/// Builds and verifies the test humanoid with a spec that declares `animations`.
fn verify_rig(animations: &str, placement: &str) -> (bool, Value) {
    let p = temp_project("");
    let kit = p.path().join("godot/kits/crew");
    std::fs::create_dir_all(&kit).unwrap();
    std::fs::copy(
        p.path().join("godot/kits/harbor/LICENSE.md"),
        kit.join("LICENSE.md"),
    )
    .unwrap();
    let blend = Command::new("blender")
        .args(["--background", "--factory-startup", "--python", HUMANOID, "--"])
        .arg(kit.join("humanoid.blend"))
        .output()
        .expect("blender");
    assert!(blend.status.success(), "{}", err(&blend));
    let spec = format!(
        "[asset]\nid = \"crew_member\"\nname = \"Tripulante\"\nasked = \"rig test\"\n\
         budget_tris = 300\nstatus = \"layout\"\nanimations = {animations}\n\n\
         [[parts]]\nid = \"body\"\nwhat = \"humanoid\"\nsize_m = [0.62, 1.67, 0.22]\n\
         source = {{ kind = \"reuse\", from = \"crew/humanoid\" }}\n{placement}\n"
    );
    std::fs::write(p.path().join("specs/crew_member.toml"), spec).unwrap();
    let out = kiln(p.path(), &["build", "--asset", "crew_member"]);
    assert!(out.status.success(), "{}", err(&out));
    let out = kiln(p.path(), &["verify", "--asset", "crew_member"]);
    let report = serde_json::from_str(&p.read("kiln/build/crew_member/validation.json")).unwrap();
    (out.status.success(), report)
}

#[test]
#[ignore]
fn rig_passes_with_its_declared_animations() {
    let (ok, report) = verify_rig("[\"idle\", \"walk\"]", "at = [[0, 0, 0]]");
    assert!(ok && report["valid"] == true, "{report}");
    assert_eq!(check(&report, "godot_import")["status"], "passed", "{report}");
}

#[test]
#[ignore]
fn rig_fails_without_a_declared_animation() {
    let (ok, report) = verify_rig("[\"idle\", \"walk\", \"run\"]", "at = [[0, 0, 0]]");
    assert!(!ok && report["valid"] == false, "{report}");
    assert_eq!(check(&report, "godot_import")["status"], "failed", "{report}");
    let codes: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"godot_animation_missing") && codes.contains(&"rig_animation_missing"),
        "{codes:?}"
    );
}

#[test]
#[ignore]
fn a_moved_and_turned_rig_still_verifies() {
    let (ok, report) = verify_rig(
        "[\"idle\", \"walk\"]",
        "at = [[2, 0, 1]]\nturn = [[0, 90, 0]]",
    );
    assert!(ok && report["valid"] == true, "{report}");
}
