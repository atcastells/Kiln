//! A `model` part is built by Blender from its shapes: triangles, bounds,
//! zones as materials and no todo. Needs Blender on the PATH:
//! `cargo test -p kiln-cli --test model_build -- --ignored`.
mod common;

use common::empty_dir;
use std::process::Command;

const KILN: &str = "[project]\nname = \"model_probe\"\nengine = \"godot\"\n\n\
    [assets]\nspecs = \"specs\"\nuv_per_m = 0.5\n\n\
    [materials]\ntables = [\"data/materials.toml\"]\ncolor_keys = [\"color\"]\n";

const MATERIALS: &str = "[[materials]]\nid = \"pine\"\nname = \"Pino\"\ncolor = [0.8, 0.65, 0.4]\n\n\
    [[materials]]\nid = \"iron\"\nname = \"Hierro\"\ncolor = [0.25, 0.25, 0.28]\n";

const SPEC: &str = "[asset]\nid = \"probe\"\nname = \"Prueba\"\nasked = \"prueba\"\nstatus = \"layout\"\n\n\
    [[parts]]\nid = \"frame\"\nwhat = \"marco\"\nsource = { kind = \"model\" }\n\
    materials = { Top = \"pine\", Leg = \"iron\" }\nat = [[0, 0, 0]]\n\n\
    [[parts.shapes]]\nzone = \"Top\"\nbox = [[0, 0, 0], [1.0, 0.5, 0.25]]\n\n\
    [[parts.shapes]]\nzone = \"Top\"\npara = { origin = [2, 0, 0], u = [0.5, 0, 0.2], v = [0, 0.3, 0], w = [0, 0, 0.4] }\n\n\
    [[parts.shapes]]\nzone = \"Leg\"\ntube = { from = [0, 1, 0], to = [1.5, 1, 0], radius = 0.1, segments = 6 }\n\n\
    [[parts.shapes]]\nzone = \"Leg\"\ntube = { from = [1.1, -1.6, 0.4], to = [0.35, 1.1, 1.15], radius = 0.1, segments = 6 }\n";
// The last tube is a near-tie of its two smallest direction components: in
// float32 Blender picked another reference axis and the bounds moved 25 mm.

#[test]
#[ignore]
fn a_model_part_is_built_from_its_shapes() {
    let p = empty_dir();
    for (rel, text) in [
        ("kiln.toml", KILN),
        ("data/materials.toml", MATERIALS),
        ("specs/probe.toml", SPEC),
    ] {
        let f = p.path().join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, text).unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(["build", "--asset", "probe", "--project"])
        .arg(p.path())
        .output()
        .expect("kiln");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let spec = kiln_core::spec::parse(SPEC).expect("spec");
    let shapes = &spec.parts[0].shapes;
    let (lo, hi) = kiln_core::model::bounds(shapes).unwrap();
    let info = kiln_core::glb::read(&p.path().join("kiln/build/probe/probe.glb")).unwrap();

    assert_eq!(info.tris, kiln_core::model::tris(shapes), "{info:?}");
    for (axis, got) in info.size().iter().enumerate() {
        let want = hi[axis] - lo[axis];
        assert!((got - want).abs() < 1e-4, "axis {axis}: {got} vs {want}");
    }
    let mut names = info.materials.clone();
    names.sort();
    assert_eq!(names, ["frame/Leg", "frame/Top"]);

    let report: serde_json::Value =
        serde_json::from_str(&p.read("kiln/build/probe/report.json")).unwrap();
    let items = report["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{report}");
    assert!(items[0]["todo"].is_null(), "{report}");
}
