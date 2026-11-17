//! A `.blend` in a kit is a piece: Kiln has Blender export it, measures it
//! from the export and knows when the `.blend` changed.
//! Needs Blender on the PATH: `cargo test -p kiln-cli -- --ignored piece`.
mod common;

use common::temp_project;
use std::path::Path;
use std::process::Command;

/// A slab with a subdivision modifier and a camera beside it, saved by Blender.
fn make_blend(file: &Path, width: f64) {
    let code = format!(
        "import bpy\n\
         bpy.ops.wm.read_factory_settings(use_empty=True)\n\
         bpy.ops.mesh.primitive_cube_add(size=1.0)\n\
         o = bpy.context.view_layer.objects.active\n\
         o.name = 'slab'\n\
         o.scale = ({width}, 1.0, 0.5)\n\
         o.modifiers.new('sub', 'SUBSURF').subdivision_type = 'SIMPLE'\n\
         bpy.ops.object.camera_add(location=(9, 9, 9))\n\
         bpy.ops.wm.save_as_mainfile(filepath=r'{}')\n",
        file.display()
    );
    let out = Command::new("blender")
        .args(["--background", "--factory-startup", "--python-expr", &code])
        .output()
        .expect("blender");
    assert!(file.exists(), "{}", String::from_utf8_lossy(&out.stderr));
}

fn kiln(project: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .expect("kiln")
}

fn piece(project: &Path) -> (serde_json::Value, Vec<String>) {
    let out = kiln(project, &["index"]);
    let cat: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let piece = cat["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["key"] == "harbor/slab")
        .cloned()
        .expect("la pieza harbor/slab");
    let codes = cat["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["subject"] == "harbor/slab")
        .map(|f| f["code"].as_str().unwrap().to_string())
        .collect();
    (piece, codes)
}

#[test]
#[ignore]
fn a_blend_piece_is_exported_measured_and_tracked() {
    let p = temp_project("");
    let blend = p.path().join("godot/kits/harbor/slab.blend");
    make_blend(&blend, 2.0);

    let (before, codes) = piece(p.path());
    assert_eq!(before["format"], "blend");
    assert!(
        before["size_m"].is_null() && codes.contains(&"blend_stale".to_string()),
        "{codes:?}"
    );

    let out = kiln(p.path(), &["godot"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (after, codes) = piece(p.path());
    assert!(!codes.contains(&"blend_stale".to_string()), "{codes:?}");
    assert_eq!(after["measured_by"], "blender");
    let size: Vec<f64> = after["size_m"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    // Blender (x 2, y 1, z 0.5) is glTF width 2, height 0.5, depth 1
    assert!(
        (size[0] - 2.0).abs() < 0.01
            && (size[1] - 0.5).abs() < 0.01
            && (size[2] - 1.0).abs() < 0.01,
        "{size:?}"
    );
    assert!(
        after["tris"].as_u64().unwrap() > 12,
        "the modifier is applied: {after}"
    );
    let glb = p.path().join(after["preview"].as_str().unwrap());
    let nodes = kiln_core::glb::structure(&glb).unwrap().nodes;
    assert_eq!(
        nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
        ["slab"],
        "only meshes are exported"
    );

    make_blend(&blend, 3.0);
    let (_, codes) = piece(p.path());
    assert!(
        codes.contains(&"blend_stale".to_string()),
        "a changed .blend is stale: {codes:?}"
    );

    // building an asset that uses the piece exports it again first
    let spec = "[asset]\nid = \"slab_probe\"\nname = \"Losa\"\nasked = \"prueba\"\nstatus = \"layout\"\n\n\
                [[parts]]\nid = \"slab\"\nwhat = \"losa\"\nsource = { kind = \"reuse\", from = \"harbor/slab\" }\nat = [[0, 0, 0]]\n";
    std::fs::write(p.path().join("specs/slab_probe.toml"), spec).unwrap();
    let out = kiln(p.path(), &["build", "--asset", "slab_probe"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(&p.read("kiln/build/slab_probe/report.json")).unwrap();
    assert!(
        (report["size_m"][0].as_f64().unwrap() - 3.0).abs() < 0.01,
        "{report}"
    );

    // a .blend retouched after the build makes the asset stale, and it cannot
    // be published until it is built and verified again
    let stale = |project: &Path| -> serde_json::Value {
        let out = kiln(project, &["index"]);
        let cat: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        cat["builds"]["slab_probe"]["stale"].clone()
    };
    assert_eq!(stale(p.path()), serde_json::json!([]));
    make_blend(&blend, 4.0);
    assert_eq!(stale(p.path()), serde_json::json!(["harbor/slab"]));
}
