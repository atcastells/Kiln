//! The assembly turns every copy, whether it comes from a model or is a box.
//! Needs Blender on the PATH: `cargo test -p kiln-cli -- --ignored turn`.
mod common;

use common::temp_project;
use std::process::Command;

const SPEC: &str = r#"
[asset]
id = "turn_probe"
name = "Prueba de giro"
asked = "prueba"
status = "layout"

[[parts]]
id = "model"
what = "caja del kit girada"
count = 2
size_m = [0.6, 0.6, 0.6]
source = { kind = "reuse", from = "harbor/crate" }
at = [[0, 0, 0], [4, 0, 0]]
turn = [[0, 90, 0], [30, 45, 0]]

[[parts]]
id = "boxed"
what = "caja generada girada"
count = 2
size_m = [0.6, 0.6, 0.6]
source = { kind = "search", query = "nada" }
at = [[2, 0, 0], [6, 0, 0]]
turn = [[0, 90, 0], [30, 45, 0]]
"#;

/// The `rotation` of every named node of a GLB (identity when absent).
fn rotations(glb: &[u8]) -> std::collections::BTreeMap<String, [f64; 4]> {
    let n = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let j: serde_json::Value = serde_json::from_slice(&glb[20..20 + n]).unwrap();
    j["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|nd| {
            let name = nd["name"].as_str()?.to_string();
            let r = nd["rotation"].as_array().map_or([0.0, 0.0, 0.0, 1.0], |r| {
                [0, 1, 2, 3].map(|i| r[i].as_f64().unwrap())
            });
            Some((name, r))
        })
        .collect()
}

fn close(a: [f64; 4], b: [f64; 4]) -> bool {
    // q and -q are the same turn
    let d = |s: f64| (0..4).all(|i| (a[i] - s * b[i]).abs() < 1e-3);
    d(1.0) || d(-1.0)
}

#[test]
#[ignore]
fn turn_reaches_copies_of_models_like_boxes() {
    let p = temp_project("");
    std::fs::write(p.path().join("specs/turn_probe.toml"), SPEC).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["build", "--asset", "turn_probe", "--project"])
        .arg(p.path())
        .output()
        .expect("kiln build");
    assert!(
        out.status.success(),
        "kiln build falló (¿Blender en el PATH?): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let glb = std::fs::read(p.path().join("kiln/build/turn_probe/turn_probe.glb")).unwrap();
    let r = rotations(&glb);
    let get = |k: &str| *r.get(k).unwrap_or_else(|| panic!("falta el nodo {k}: {r:?}"));
    let s = std::f64::consts::FRAC_1_SQRT_2;
    assert!(close(get("boxed_1"), [0.0, s, 0.0, s]), "{r:?}");
    assert!(close(get("model_1"), [0.0, s, 0.0, s]), "{r:?}");
    assert!(close(get("model_2"), get("boxed_2")), "{r:?}");
}
