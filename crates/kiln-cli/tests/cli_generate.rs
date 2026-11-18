mod common;

use std::process::{Command, Output};

const ROCK: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = [1, 5]\nsegments = 16\n";
const PARTS: &str = "[asset]\nid = \"crate\"\nname = \"Crate\"\n\n[[parts]]\nid = \"body\"\nwhat = \"a box\"\nsource = { kind = \"search\", query = \"box\" }\n";

fn project() -> common::TempDir {
    let dir = common::empty_dir();
    std::fs::create_dir_all(dir.path().join("specs")).unwrap();
    std::fs::write(
        dir.path().join("kiln.toml"),
        "[project]\nname = \"t\"\n\n[assets]\nspecs = \"specs\"\n\n[tools]\ngodot = \"/nonexistent/godot\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("specs/rock.toml"), ROCK).unwrap();
    std::fs::write(dir.path().join("specs/crate.toml"), PARTS).unwrap();
    dir
}

fn kiln(args: &[&str], dir: &common::TempDir) -> (Output, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .arg("--project")
        .arg(dir.path())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out, text)
}

#[test]
fn usage_lists_generate() {
    let out = Command::new(env!("CARGO_BIN_EXE_kiln")).output().unwrap();
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && text.contains("generate"), "{text}");
}

#[test]
fn generate_names_the_tool_it_could_not_launch() {
    let dir = project();
    let (out, text) = kiln(&["generate", "--asset", "rock"], &dir);
    assert!(!out.status.success());
    assert!(text.contains("/nonexistent/godot"), "{text}");
}

#[test]
fn generate_rejects_a_seed_that_is_not_a_number() {
    let dir = project();
    let (out, text) = kiln(&["generate", "--asset", "rock", "--seed", "nope"], &dir);
    assert!(!out.status.success());
    assert!(text.contains("--seed debe ser un número"), "{text}");
}

#[test]
fn generate_refuses_an_asset_made_of_parts() {
    let dir = project();
    let (out, text) = kiln(&["generate", "--asset", "crate"], &dir);
    assert!(!out.status.success());
    assert!(text.contains("no es un asset generado"), "{text}");
}
