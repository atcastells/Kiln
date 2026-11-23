//! The beginner's road over MCP, without Blender or Godot: find a program
//! installed as an app, make the example game, change its settings, add a
//! colour, see a model in the conversation and get the link to the page.
mod common;

use common::{Mcp, TempDir, empty_dir};
use serde_json::json;
use std::path::Path;

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[cfg(unix)]
fn app(home: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = home.join("Applications/Blender.app/Contents/MacOS/Blender");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "#!/bin/sh\necho 'Blender 5.0.1'\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn start(cwd: &TempDir, kiln_home: &TempDir, home: &TempDir) -> Mcp {
    Mcp::start_free(
        cwd.path(),
        &[("KILN_HOME", s(kiln_home.path())), ("HOME", s(home.path())), ("KILN_NO_BROWSER", "1")],
    )
}

#[cfg(unix)]
#[test]
fn from_nothing_to_a_preview() {
    let (cwd, kiln_home, home, parent) = (empty_dir(), empty_dir(), empty_dir(), empty_dir());
    let blender = app(home.path());
    let mut m = start(&cwd, &kiln_home, &home);

    // 1. The programs: Blender found as an app and saved for every project.
    let st = m.call("setup", json!({}));
    assert!(!st.is_error, "{}", st.text);
    let v = st.json();
    let b = &v["tools"][0];
    if b["source"] != "PATH" {
        assert_eq!(b["path"], s(&blender), "{v}");
        assert_eq!(b["saved"], true, "{v}");
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(kiln_home.path().join("tools.json")).unwrap()).unwrap();
        assert_eq!(saved["blender"], s(&blender));
    }
    // An agent cannot make Kiln run a program it did not find.
    let bad = m.call("setup", json!({"choose": {"blender": "/bin/sh"}}));
    assert!(bad.is_error && bad.text.contains("kiln setup --blender"), "{}", bad.text);

    // 2. The example game, in one call; open and trusted.
    let game = parent.path().join("Mi primer juego");
    let ex = m.call("example_create", json!({"path": s(&game)}));
    assert!(!ex.is_error, "{}", ex.text);
    assert!(game.join("kiln/specs/farol.toml").is_file());
    let st = m.call("status", json!({})).json();
    assert_eq!(st["project"], "Mi primer juego");
    assert_eq!(st["trusted"], true);
    assert_eq!(st["assets"][0]["asset"], "farol");
    let next = st["next"].as_str().unwrap();
    assert!(next.contains("farol") && next.contains("`build`"), "{st}");
    // ... and never over a folder with things in it.
    let again = m.call("example_create", json!({"path": s(&game)}));
    assert!(again.is_error, "{}", again.text);

    // 3. Settings and palette.
    let c = m.call("config_get", json!({})).json();
    let keys: Vec<&str> = c["settings"].as_array().unwrap().iter().map(|r| r["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"publish.dir") && !keys.contains(&"tools.blender"), "{c}");
    let set = m.call("config_set", json!({"key": "rules.max_tris", "value": 8000}));
    assert!(!set.is_error, "{}", set.text);
    assert!(std::fs::read_to_string(game.join("kiln.toml")).unwrap().contains("max_tris = 8000"));
    let no = m.call("config_set", json!({"key": "tools.blender", "value": "/bin/sh"}));
    assert!(no.is_error, "{}", no.text);
    let pal = m.call("palette_add", json!({"id": "laton", "name": "Latón", "color": [0.8, 0.62, 0.25]}));
    assert!(!pal.is_error, "{}", pal.text);

    // 4. A preview: before the build it says what to do; after it, an image.
    let early = m.call("preview", json!({"asset": "farol"}));
    assert!(early.is_error && early.text.contains("`build`"), "{}", early.text);
    let out = game.join("kiln/build/farol");
    std::fs::create_dir_all(&out).unwrap();
    let crate_glb = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/godot/kits/harbor/crate.glb");
    std::fs::copy(crate_glb, out.join("farol.glb")).unwrap();
    let pv = m.call("preview", json!({"asset": "farol"}));
    assert!(!pv.is_error, "{}", pv.text);
    let info = pv.json();
    assert!(info["triangles"].as_u64().unwrap() > 0, "{info}");
    assert_eq!(pv.raw["content"][1]["type"], "image");
    assert_eq!(pv.raw["content"][1]["mimeType"], "image/png");
    let png = std::fs::read(out.join("preview.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    assert!(png.len() < 400_000, "small enough for a conversation: {} bytes", png.len());

    // 5. The page, at the asset.
    let v = m.call("open_viewer", json!({"asset": "farol"}));
    assert!(!v.is_error, "{}", v.text);
    let url = v.json()["url"].as_str().unwrap().to_string();
    assert!(url.starts_with("http://127.0.0.1:") && url.ends_with("/#/build/farol"), "{url}");
    assert_eq!(v.json()["opened"], false, "KILN_NO_BROWSER");
}
