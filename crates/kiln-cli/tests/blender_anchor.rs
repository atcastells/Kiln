//! The anchors of a spec come out of the assembly as empty nodes, in place.
//! Needs Blender on the PATH: `cargo test -p kiln-cli -- --ignored anchor`.
mod common;

use common::temp_project;
use std::process::Command;

const SPEC: &str = r#"
[asset]
id = "anchor_probe"
name = "Prueba de anclaje"
asked = "prueba"
status = "layout"

[[parts]]
id = "body"
what = "caja del kit"
size_m = [0.6, 0.6, 0.6]
source = { kind = "reuse", from = "harbor/crate" }
at = [[0, 0, 0]]

[[anchors]]
id = "mount"
class = "socket"
at = [0.5, 1.25, -2.0]
turn = [0, 90, 0]

[[anchors]]
id = "floor"
class = "socket"
"#;

#[test]
#[ignore]
fn anchors_reach_the_glb_as_empty_nodes_in_place() {
    let p = temp_project("");
    std::fs::write(p.path().join("specs/anchor_probe.toml"), SPEC).unwrap();
    let toml = format!("{}\n[anchors.socket]\n", p.read("kiln.toml"));
    std::fs::write(p.path().join("kiln.toml"), toml).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["build", "--asset", "anchor_probe", "--project"])
        .arg(p.path())
        .output()
        .expect("kiln build");
    assert!(
        out.status.success(),
        "kiln build falló (¿Blender en el PATH?): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let glb = p.path().join("kiln/build/anchor_probe/anchor_probe.glb");
    let s = kiln_core::glb::structure(&glb).unwrap();
    let node = |name: &str| {
        s.nodes
            .iter()
            .find(|n| n.name == name)
            .unwrap_or_else(|| panic!("falta el nodo {name}: {:?}", s.nodes))
    };
    let mount = node("anchor_mount");
    assert!(mount.parent.is_none() && mount.mesh.is_none(), "{mount:?}");
    for (got, want) in mount.translation.iter().zip([0.5, 1.25, -2.0]) {
        assert!((got - want).abs() < 1e-3, "{mount:?}");
    }
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let [x, y, z, w] = mount.rotation;
    let sign = if w < 0.0 { -1.0 } else { 1.0 };
    assert!(
        x.abs() < 1e-3
            && z.abs() < 1e-3
            && (y * sign - half).abs() < 1e-3
            && (w * sign - half).abs() < 1e-3,
        "{mount:?}"
    );
    let floor = node("anchor_floor");
    assert!(
        floor.translation.iter().all(|v| v.abs() < 1e-3),
        "{floor:?}"
    );
    // the anchors do not count as part of the asset's size
    let report: serde_json::Value =
        serde_json::from_str(&p.read("kiln/build/anchor_probe/report.json")).unwrap();
    assert!(report["size_m"][1].as_f64().unwrap() < 1.0, "{report}");
}
