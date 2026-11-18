//! `kiln verify` and `kiln publish` of an asset generated from a recipe, with a
//! real Godot: a batch of seeds is generated and measured, and the publication
//! is the recipe, the script and the resolved seeds, never a GLB.
//! Needs Godot on the PATH: `cargo test -p kiln-cli --test godot_recipe -- --ignored`.
mod common;

use common::{empty_dir, TempDir};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const HEAD: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\nbudget_tris = 6000\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = [2, 5]\nsegments = 32\n\n[[generator.ops]]\nop = \"stretch\"\nx = [1.0, 1.6]\n\n";
const NOISE: &str = "[[generator.ops]]\nop = \"noise\"\namplitude = [0.1, 0.25]\nsize = 0.5\n";
const FACET: &str = "\n[[generator.ops]]\nop = \"facet\"\n";
const TRIS: u64 = 2 * 32 * (16 + 1);

fn rock() -> String {
    format!("{HEAD}{NOISE}{FACET}")
}

/// A project with the recipe body in `specs/rock.toml` and `extra` appended to `kiln.toml`.
fn project(recipe: &str, extra: &str) -> TempDir {
    let p = empty_dir();
    std::fs::create_dir_all(p.path().join("specs")).unwrap();
    std::fs::write(
        p.path().join("kiln.toml"),
        format!(
            "[project]\nname = \"t\"\n\n[assets]\nspecs = \"specs\"\n\n[tools]\ngltf_validator = \"kiln-no-such-validator\"\n\n[publish]\ndir = \"godot/assets\"\n\n{extra}"
        ),
    )
    .unwrap();
    std::fs::write(p.path().join("specs/rock.toml"), recipe).unwrap();
    p
}

/// Runs `kiln <cmd> --asset rock --project <p>`: exit code and stdout + stderr.
fn kiln(p: &TempDir, cmd: &str, extra: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .arg(cmd)
        .args(extra)
        .arg("--project")
        .arg(p.path())
        .output()
        .expect("kiln");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

fn run(p: &TempDir, cmd: &str) -> (i32, String) {
    kiln(p, cmd, &["--asset", "rock"])
}

fn json(p: &TempDir, rel: &str) -> Value {
    serde_json::from_str(&p.read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn report(p: &TempDir) -> Value {
    json(p, "kiln/build/rock/validation.json")
}

fn verified(p: &TempDir) -> Value {
    let (code, text) = run(p, "verify");
    assert_eq!(code, 0, "{text}");
    report(p)
}

fn glbs_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                found.extend(glbs_under(&path));
            } else if path.extension().is_some_and(|x| x == "glb") {
                found.push(path.display().to_string());
            }
        }
    }
    found
}

fn seed_list(v: &Value) -> Vec<(u64, String)> {
    v["recipe"]["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["seed"].as_u64().unwrap(), s["glb_sha256"].as_str().unwrap().to_string()))
        .collect()
}

fn check_status(v: &Value, name: &str) -> String {
    let c = v["checks"].as_array().unwrap().iter().find(|c| c["name"] == name);
    c.unwrap_or_else(|| panic!("no check {name}: {v}"))["status"].as_str().unwrap().to_string()
}

#[test]
#[ignore]
fn verify_generates_the_batch_and_writes_the_report() {
    let p = project(&rock(), "");
    let v = verified(&p);
    assert_eq!(v["valid"], true, "{v}");
    let names: Vec<&str> = v["checks"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["kiln", "gltf_validator", "godot_import"], "{v}");
    assert_eq!(check_status(&v, "kiln"), "passed");
    assert_eq!(check_status(&v, "gltf_validator"), "absent");
    assert_eq!(check_status(&v, "godot_import"), "passed");
    let seeds = v["recipe"]["seeds"].as_array().unwrap();
    assert_eq!(seeds.len(), 16);
    assert_eq!(seeds[0]["seed"], 7);
    for s in seeds {
        assert_eq!(s["tris"], TRIS, "{s}");
        assert_eq!(s["flipped"], 0, "{s}");
        assert_eq!(s["degenerate"], 64, "{s}");
    }
    assert_eq!(v["recipe"]["worst"]["tris"], TRIS);
    assert_eq!(glbs_under(&p.path().join("kiln/cache/generated/rock/verify")).len(), 16);
}

#[test]
#[ignore]
fn verifying_twice_gives_the_same_seeds_and_meshes() {
    let p = project(&rock(), "");
    let a = verified(&p);
    let b = verified(&p);
    assert_eq!(seed_list(&a), seed_list(&b));
    assert_eq!(a["recipe"]["recipe_sha256"], b["recipe"]["recipe_sha256"]);
    assert_eq!(a["recipe"]["script_sha256"], b["recipe"]["script_sha256"]);
}

#[test]
#[ignore]
fn verify_seeds_is_read_from_the_config() {
    let p = project(&rock(), "[generate]\nverify_seeds = 3\n");
    let v = verified(&p);
    assert_eq!(v["recipe"]["seeds"].as_array().unwrap().len(), 3, "{v}");
}

#[test]
#[ignore]
fn verify_does_not_touch_the_preview() {
    let p = project(&rock(), "");
    let (code, text) = kiln(&p, "generate", &["--asset", "rock", "--seed", "7"]);
    assert_eq!(code, 0, "{text}");
    let glb = std::fs::read(p.path().join("kiln/cache/generated/rock/7.glb")).unwrap();
    let meta = std::fs::read(p.path().join("kiln/cache/generated/rock/generated.json")).unwrap();
    verified(&p);
    assert_eq!(std::fs::read(p.path().join("kiln/cache/generated/rock/7.glb")).unwrap(), glb);
    assert_eq!(std::fs::read(p.path().join("kiln/cache/generated/rock/generated.json")).unwrap(), meta);
}

#[test]
#[ignore]
fn publish_writes_the_recipe_the_script_and_the_contract() {
    let p = project(&rock(), "");
    verified(&p);
    let (code, text) = run(&p, "publish");
    assert_eq!(code, 0, "{text}");
    for f in ["recipe.json", "kiln_generator.gd", "asset.json"] {
        assert!(p.exists(&format!("godot/assets/rock/{f}")), "falta {f}");
    }
    assert!(glbs_under(&p.path().join("godot/assets")).is_empty());
    let asset = json(&p, "godot/assets/rock/asset.json");
    assert_eq!(asset["version"], 1, "{asset}");
    assert_eq!(asset["kind"], "generated", "{asset}");
    let recipe = json(&p, "godot/assets/rock/recipe.json");
    let seeds = recipe["seeds"].as_array().unwrap();
    assert_eq!(seeds.len(), 16);
    for s in seeds {
        assert_eq!(s["ops"][0]["op"], "sphere", "{s}");
        for op in s["ops"].as_array().unwrap() {
            for (k, v) in op["params"].as_object().unwrap() {
                assert!(v.is_number(), "{k}: {v}");
            }
        }
    }
    let (code, text) = run(&p, "publish");
    assert_eq!(code, 0, "{text}");
    assert_eq!(json(&p, "godot/assets/rock/asset.json")["version"], 1);
    let root = p.path().display().to_string();
    for f in ["recipe.json", "asset.json"] {
        assert!(!p.read(&format!("godot/assets/rock/{f}")).contains(&root), "{f} lleva la ruta del proyecto");
    }
}

#[test]
#[ignore]
fn a_changed_recipe_must_be_verified_again() {
    let p = project(&rock(), "");
    verified(&p);
    let (code, text) = run(&p, "publish");
    assert_eq!(code, 0, "{text}");
    assert_eq!(json(&p, "godot/assets/rock/asset.json")["version"], 1);
    let changed = rock().replace("amplitude = [0.1, 0.25]", "amplitude = [0.1, 0.2]");
    assert_ne!(changed, rock());
    std::fs::write(p.path().join("specs/rock.toml"), changed).unwrap();
    let (code, text) = run(&p, "publish");
    assert_ne!(code, 0, "{text}");
    assert!(text.contains("cambió después de verificarla"), "{text}");
    verified(&p);
    let (code, text) = run(&p, "publish");
    assert_eq!(code, 0, "{text}");
    assert_eq!(json(&p, "godot/assets/rock/asset.json")["version"], 2);
}

#[test]
#[ignore]
fn a_recipe_that_can_reach_zero_radius_is_refused_everywhere() {
    let recipe = "[asset]\nid = \"rock\"\nname = \"Rock\"\nbudget_tris = 6000\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = [2, 5]\nsegments = 32\n\n[[generator.ops]]\nop = \"stretch\"\ny = 0.2\n\n[[generator.ops]]\nop = \"noise\"\namplitude = 0.9\nsize = 0.5\n";
    let p = project(recipe, "");
    let (code, text) = kiln(&p, "check", &[]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("generator_radius"), "{text}");
    let (code, text) = run(&p, "verify");
    assert_ne!(code, 0, "{text}");
    let v = report(&p);
    assert_eq!(v["valid"], false, "{v}");
    assert!(v["recipe"]["seeds"].as_array().unwrap().is_empty(), "{v}");
    assert!(glbs_under(&p.path().join("kiln/cache/generated/rock")).is_empty());
    let (code, text) = run(&p, "publish");
    assert_ne!(code, 0, "{text}");
    assert!(!p.exists("godot/assets/rock"));
}

#[test]
#[ignore]
fn publish_without_verifying_is_refused() {
    let p = project(&rock(), "");
    let (code, text) = run(&p, "publish");
    assert_ne!(code, 0, "{text}");
    assert!(text.contains("verifica antes el asset"), "{text}");
}
