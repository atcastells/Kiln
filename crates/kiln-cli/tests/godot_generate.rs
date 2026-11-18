//! `kiln generate` builds a rock from a recipe in real Godot: the same seed
//! gives the same structure, another seed another one, and the triangles match
//! the static count.
//! Needs Godot on the PATH: `cargo test -p kiln-cli --test godot_generate -- --ignored`.
mod common;

use common::{empty_dir, TempDir};
use serde_json::Value;
use std::process::{Command, Output};

const SMOOTH: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\nbudget_tris = 6000\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = [2, 5]\nsegments = 32\n\n[[generator.ops]]\nop = \"stretch\"\nx = [1.0, 1.6]\n\n[[generator.ops]]\nop = \"noise\"\namplitude = [0.1, 0.3]\nsize = 0.5\n";

fn project(recipe: &str) -> TempDir {
    let p = empty_dir();
    std::fs::create_dir_all(p.path().join("specs")).unwrap();
    std::fs::write(
        p.path().join("kiln.toml"),
        "[project]\nname = \"t\"\n\n[assets]\nspecs = \"specs\"\n\n[tools]\n",
    )
    .unwrap();
    std::fs::write(p.path().join("specs/rock.toml"), recipe).unwrap();
    p
}

fn faceted() -> String {
    format!("{SMOOTH}\n[[generator.ops]]\nop = \"facet\"\n")
}

fn generate(p: &TempDir, seed: u32) -> Value {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["generate", "--asset", "rock", "--seed", &seed.to_string()])
        .arg("--project")
        .arg(p.path())
        .output()
        .expect("kiln");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_str(&p.read("kiln/cache/generated/rock/generated.json")).expect("generated.json")
}

#[test]
#[ignore]
fn same_seed_gives_the_same_structure() {
    let p = project(&faceted());
    let (a, b) = (generate(&p, 7), generate(&p, 7));
    assert_eq!(a["structure_sha256"], b["structure_sha256"]);
    assert_eq!(a["glb_sha256"], b["glb_sha256"]);
}

#[test]
#[ignore]
fn another_seed_gives_another_mesh() {
    let p = project(&faceted());
    let (a, b) = (generate(&p, 7), generate(&p, 8));
    // the structural hash covers nodes, triangle counts and materials, not
    // vertex positions: two seeds of one recipe share it and differ in the mesh
    assert_eq!(a["structure_sha256"], b["structure_sha256"]);
    assert_ne!(a["glb_sha256"], b["glb_sha256"]);
    assert_ne!(a["size_m"], b["size_m"]);
}

#[test]
#[ignore]
fn triangles_match_the_static_count() {
    let p = project(&faceted());
    let r = generate(&p, 7);
    assert_eq!(r["tris"], 2 * 32 * (16 + 1));
    assert_eq!(r["tris_expected"], r["tris"]);
    let size = r["size_m"].as_array().unwrap();
    assert!(size.len() == 3 && size.iter().all(|v| v.as_f64().is_some_and(|n| n > 0.0)), "{r}");
}

#[test]
#[ignore]
fn a_smooth_recipe_also_generates() {
    let p = project(SMOOTH);
    let r = generate(&p, 7);
    assert!(p.exists("kiln/cache/generated/rock/7.glb"));
    assert_eq!(r["tris"], 2 * 32 * (16 + 1));
}
