//! `kiln astra-manifest`: Godot lists the ASTRA ids of the sector scenes and
//! Kiln writes them to `[integrations.astra] godot_manifest`, where
//! `kiln check` compares them with the RON definitions.

use crate::production::config;
use kiln_core::astra::{
    self,
    manifest::{Manifest, SceneEntry},
};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

const SCRIPT: &str = include_str!("../godot/astra_manifest.gd");

#[derive(Deserialize)]
struct GodotOut {
    scenes: Vec<GodotScene>,
}

#[derive(Deserialize)]
struct GodotScene {
    #[serde(flatten)]
    entry: SceneEntry,
    #[serde(default)]
    error: Option<String>,
}

/// Runs Godot over the scenes of the sectors and writes the manifest.
/// Returns how many scenes it read.
pub fn export(
    project: &Path,
    config_file: Option<&Path>,
    mut say: impl FnMut(String),
) -> Result<usize, String> {
    let cfg = config(project, config_file)?;
    let a = cfg
        .astra()
        .ok_or("falta [integrations.astra] con enabled = true en kiln.toml")?;
    let rel = a
        .godot_manifest
        .as_ref()
        .ok_or("falta godot_manifest en [integrations.astra]")?;
    let gp = cfg
        .tools
        .godot_project
        .as_ref()
        .ok_or("falta [tools] godot_project en kiln.toml")?;
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(format!(
            "godot_manifest «{rel}» debe quedar dentro del proyecto"
        ));
    }
    let (files, findings) = astra::load_files(project, a);
    if let Some(f) = findings.iter().find(|f| f.code == "astra_ron_syntax") {
        return Err(format!("{}: {}", f.subject, f.message));
    }
    if files.is_empty() {
        return Err(format!(
            "ningún fichero RON coincide con {:?}",
            a.definitions
        ));
    }
    let scenes = astra::scenes(&files);
    let dir = project.join(&cfg.tools.cache).join("astra");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
    let (input, output, script) = (
        dir.join("scenes.json"),
        dir.join("out.json"),
        dir.join("astra_manifest.gd"),
    );
    let json = serde_json::to_string(&scenes).map_err(|e| e.to_string())?;
    std::fs::write(&input, json).map_err(|e| e.to_string())?;
    std::fs::write(&script, SCRIPT).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&output);
    say(format!(
        "Godot lee {} escenas ({})",
        scenes.len(),
        cfg.tools.godot
    ));
    let run = Command::new(&cfg.tools.godot)
        .args(["--headless", "--path"])
        .arg(project.join(gp))
        .arg("--script")
        .arg(&script)
        .arg("--")
        .args([&input, &output])
        .output()
        .map_err(|e| format!("no se puede lanzar {}: {e}", cfg.tools.godot))?;
    let text = std::fs::read_to_string(&output).map_err(|_| {
        let err = String::from_utf8_lossy(&run.stderr);
        format!(
            "Godot no escribió el manifiesto: {}",
            err.lines().last().unwrap_or("")
        )
    })?;
    let out: GodotOut = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(s) = out.scenes.iter().find(|s| s.error.is_some()) {
        return Err(format!(
            "{}: {}",
            s.entry.scene,
            s.error.as_deref().unwrap_or("")
        ));
    }
    let m = Manifest {
        version: 1,
        scenes: out.scenes.into_iter().map(|s| s.entry).collect(),
    };
    let target = project.join(rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&target, json).map_err(|e| e.to_string())?;
    say(format!("Manifiesto en {}", target.display()));
    Ok(m.scenes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::astra::manifest;

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let (src, dst) = (e.path(), to.join(e.file_name()));
            if src.is_dir() {
                if e.file_name() != ".godot" && e.file_name() != "cache" {
                    copy_dir(&src, &dst);
                }
            } else {
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }

    /// Needs Godot 4 on the PATH: `cargo test -p kiln-cli -- --ignored astra`.
    #[test]
    #[ignore]
    fn godot_export_matches_fixture_and_catches_a_moved_door() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/astra");
        let root = std::env::temp_dir().join(format!("kiln-astra-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_dir(&fixture, &root);
        let rel = "kiln/astra_scene_manifest.json";

        assert_eq!(export(&root, None, |_| {}).unwrap(), 2);
        let committed = manifest::load(&fixture, rel).unwrap();
        assert_eq!(manifest::load(&root, rel).unwrap(), committed);

        let scene = root.join("godot/astra/warehouse.tscn");
        let text = std::fs::read_to_string(&scene).unwrap();
        std::fs::write(&scene, text.replace("\"sector:yard\"]", "\"sector:roof\"]")).unwrap();
        export(&root, None, |_| {}).unwrap();
        let cat = kiln_core::open(&root).unwrap();
        assert!(
            cat.findings
                .iter()
                .any(|f| f.code == "astra_manifest_mismatch"
                    && f.subject == "data/astra/world.ron#door:warehouse_yard"),
            "{:?}",
            cat.findings
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
