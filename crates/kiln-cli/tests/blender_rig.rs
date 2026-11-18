//! A piece with a skeleton keeps its bones, weights and animations through
//! the export of the `.blend` and through the build.
//! Needs Blender on the PATH: `cargo test -p kiln-cli -- --ignored rig`.
mod common;

use common::{TempDir, temp_project};
use std::path::{Path, PathBuf};
use std::process::Command;

const HUMANOID: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rig_humanoid.py");

/// The examples project with the test humanoid as the piece `crew/humanoid`.
fn project() -> TempDir {
    let p = temp_project("");
    let kit = p.path().join("godot/kits/crew");
    std::fs::create_dir_all(&kit).unwrap();
    std::fs::copy(
        p.path().join("godot/kits/harbor/LICENSE.md"),
        kit.join("LICENSE.md"),
    )
    .unwrap();
    let blend = kit.join("humanoid.blend");
    let out = Command::new("blender")
        .args(["--background", "--factory-startup", "--python", HUMANOID, "--"])
        .arg(&blend)
        .output()
        .expect("blender");
    assert!(blend.exists(), "{}", String::from_utf8_lossy(&out.stderr));
    p
}

fn kiln(project: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .expect("kiln")
}

/// The GLB Kiln exported from the `.blend` of the piece.
fn piece_glb(project: &Path) -> PathBuf {
    let out = kiln(project, &["godot"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = kiln(project, &["index"]);
    let cat: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let piece = cat["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["key"] == "crew/humanoid")
        .cloned()
        .expect("la pieza crew/humanoid");
    project.join(piece["preview"].as_str().unwrap())
}

fn assert_rig(glb: &Path) {
    let s = kiln_core::glb::structure(glb).unwrap();
    assert_eq!(s.skins.len(), 1, "{:?}", s.skins);
    assert_eq!(s.skins[0].joints.len(), 6, "{:?}", s.skins);
    let mut names: Vec<&str> = s.animations.iter().map(|a| a.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["idle", "walk"]);
    assert!(s.animations.iter().all(|a| a.channels > 0 && a.duration > 0.5));
}

#[test]
#[ignore]
fn piece_keeps_rig() {
    let p = project();
    assert_rig(&piece_glb(p.path()));
}

const DEFORM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rig_deform.gd");

fn spec(project: &Path, count: u32) {
    let text = format!(
        "[asset]\nid = \"crew_member\"\nname = \"Tripulante\"\nasked = \"rig test\"\n\
         budget_tris = 300\nstatus = \"layout\"\nanimations = [\"idle\", \"walk\"]\n\n\
         [[parts]]\nid = \"body\"\nwhat = \"humanoid\"\ncount = {count}\nsize_m = [0.62, 1.67, 0.22]\n\
         source = {{ kind = \"reuse\", from = \"crew/humanoid\" }}\nat = [{}]\n",
        vec!["[0, 0, 0]"; count as usize].join(", ")
    );
    std::fs::write(project.join("specs/crew_member.toml"), text).unwrap();
}

/// How far apart Godot deforms two GLBs of the same model, vertex by vertex.
fn deform(a: &Path, b: &Path) -> serde_json::Value {
    let dir = common::empty_dir();
    let d = dir.path();
    std::fs::write(d.join("project.godot"), "config_version=5\n").unwrap();
    std::fs::copy(a, d.join("a.glb")).unwrap();
    std::fs::copy(b, d.join("b.glb")).unwrap();
    let godot = |args: &[&str]| {
        Command::new("godot")
            .args(["--headless", "--path"])
            .arg(d)
            .args(args)
            .output()
            .expect("godot")
    };
    godot(&["--import"]);
    let out = godot(&["--script", DEFORM, "--", "res://a.glb", "res://b.glb", "out.json"]);
    let text = std::fs::read_to_string(d.join("out.json"))
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)));
    serde_json::from_str(&text).unwrap()
}

#[test]
#[ignore]
fn build_keeps_rig() {
    let p = project();
    let piece = piece_glb(p.path());
    spec(p.path(), 1);
    let out = kiln(p.path(), &["build", "--asset", "crew_member"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let built = p.path().join("kiln/build/crew_member/crew_member.glb");
    assert_rig(&built);
    let nodes = kiln_core::glb::structure(&built).unwrap().nodes;
    let root = nodes.iter().find(|n| n.name == "body").expect("the root is named after the part");
    assert!(root.mesh.is_none() && root.extras["kiln_part"] == "body", "{root:?}");
    assert!(nodes.iter().any(|n| n.name == "body_mesh" && n.mesh.is_some()));

    let d = deform(&piece, &built);
    assert_eq!(d["verts"][0], d["verts"][1], "{d}");
    assert!(d["samples"].as_u64().unwrap() > 400 && d["missing"] == 0, "{d}");
    assert!(d["motion_m"].as_f64().unwrap() > 0.1, "the test moves: {d}");
    assert!(d["worst_m"].as_f64().unwrap() < 1e-4, "{d}");

    let report: serde_json::Value =
        serde_json::from_str(&p.read("kiln/build/crew_member/report.json")).unwrap();
    assert_eq!(report["tris"], 72, "{report}");
    let size: Vec<f64> = report["size_m"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert!(
        (size[0] - 0.62).abs() < 0.01 && (size[1] - 1.67).abs() < 0.01 && (size[2] - 0.22).abs() < 0.01,
        "{size:?}"
    );
}

#[test]
#[ignore]
fn a_rigged_piece_is_not_copied() {
    let p = project();
    piece_glb(p.path());
    spec(p.path(), 2);
    let out = kiln(p.path(), &["build", "--asset", "crew_member"]);
    assert!(!out.status.success());
    assert!(!p.exists("kiln/build/crew_member/crew_member.glb"));
}
