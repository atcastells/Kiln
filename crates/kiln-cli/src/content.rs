//! `kiln content-manifest`: Godot lists the nodes of the scenes of a content
//! set and Kiln writes them to the `manifest` of the set, where `kiln check`
//! compares them with the definitions.

use crate::production::config;
use kiln_core::content::{index, schema::ContentSet};
use kiln_core::Severity;
use std::path::Path;

const SCRIPT: &str = include_str!("../godot/content_manifest.gd");

/// The `res://` scenes of the elements, sorted and without repeats: what
/// Godot is asked to read.
pub fn scenes(set: &ContentSet, elements: &[index::Element]) -> Vec<String> {
    let mut out: Vec<String> = elements
        .iter()
        .filter_map(|e| {
            let field = set.kinds.get(&e.kind)?.scene.as_deref()?;
            Some(e.value.field(field)?.as_str()?.to_string())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Writes the manifest of every set that has one, or only of `only`.
/// Returns the names of the sets written.
pub fn export(
    project: &Path,
    config_file: Option<&Path>,
    only: Option<&str>,
    mut say: impl FnMut(String),
) -> Result<Vec<String>, String> {
    let cfg = config(project, config_file)?;
    let (sets, _) = cfg.content_sets();
    if let Some(name) = only.filter(|n| !sets.contains_key(*n)) {
        return Err(format!("no hay conjunto de contenido «{name}» en kiln.toml"));
    }
    let wanted: Vec<_> = sets
        .iter()
        .filter(|(n, s)| s.manifest.is_some() && only.is_none_or(|o| o == n.as_str()))
        .collect();
    if wanted.is_empty() {
        return Err("ningún conjunto de contenido declara `manifest` en kiln.toml".into());
    }
    let gp = cfg
        .tools
        .godot_project
        .as_ref()
        .ok_or("falta [tools] godot_project en kiln.toml")?;
    let mut done = Vec::new();
    for (name, set) in wanted {
        let rel = set.manifest.as_deref().unwrap_or_default();
        let rel_path = Path::new(rel);
        if rel_path.is_absolute() || rel_path.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(format!("{name}: manifest «{rel}» debe quedar dentro del proyecto"));
        }
        let mut findings = Vec::new();
        let elements = index::load(project, name, set, &mut findings);
        if let Some(f) = findings.iter().find(|f| f.severity == Severity::Error) {
            return Err(format!("{}: {}", f.subject, f.message));
        }
        let scenes = scenes(set, &elements);
        if scenes.is_empty() {
            return Err(format!("{name}: ningún elemento declara escena"));
        }
        let dir = project.join(&cfg.tools.cache).join("content").join(name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dir = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
        let (input, output, script) = (dir.join("job.json"), dir.join("out.json"), dir.join("content_manifest.gd"));
        let job = serde_json::json!({ "set": name, "id_meta": set.id_meta, "scenes": scenes });
        std::fs::write(&input, job.to_string()).map_err(|e| e.to_string())?;
        std::fs::write(&script, SCRIPT).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&output);
        say(format!("{name}: Godot lee {} escenas ({})", scenes.len(), cfg.tools.godot));
        let run = crate::run::godot(
            &cfg,
            vec![
                "--path".into(),
                project.join(gp).into(),
                "--script".into(),
                (&script).into(),
                "--".into(),
                (&input).into(),
                (&output).into(),
            ],
            &dir,
            "content-manifest",
        )?;
        let text = std::fs::read_to_string(&output).map_err(|_| {
            format!("Godot no escribió el manifiesto: {}", run.stderr.lines().last().unwrap_or(""))
        })?;
        let out: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let failed = out["scenes"].as_array().into_iter().flatten().find(|s| s.get("error").is_some());
        if let Some(s) = failed {
            return Err(format!("{}: {}", s["scene"].as_str().unwrap_or("?"), s["error"].as_str().unwrap_or("")));
        }
        if !run.success() {
            return Err(format!("Godot falló al leer las escenas de {name}"));
        }
        let target = project.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())? + "\n";
        std::fs::write(&target, json).map_err(|e| e.to_string())?;
        say(format!("{name}: manifiesto en {}", target.display()));
        done.push(name.clone());
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::content::{index::Element, parse::parse};
    use kiln_core::Config;

    #[test]
    fn scenes_are_sorted_and_unique() {
        let cfg = Config::parse("[project]\nname = \"t\"\n[content.s]\ndefinitions = [\"a\"]\n[content.s.kinds.Room]\nid = \"id\"\nscene = \"scene\"\n[content.s.kinds.Door]\nid = \"id\"\n").unwrap();
        let (sets, problems) = cfg.content_sets();
        assert!(problems.is_empty());
        let el = |kind: &str, text: &str| Element { kind: kind.into(), id: "x".into(), file: "a.ron".into(), line: 1, value: parse(text).unwrap() };
        let elements = vec![
            el("Room", "(id: \"b\", scene: \"res://z.tscn\")"),
            el("Room", "(id: \"a\", scene: \"res://a.tscn\")"),
            el("Room", "(id: \"c\", scene: \"res://z.tscn\")"),
            el("Door", "(id: \"d\", scene: \"res://not_a_scene_field.tscn\")"),
        ];
        assert_eq!(scenes(&sets["s"], &elements), vec!["res://a.tscn", "res://z.tscn"]);
    }

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

    /// Needs Godot 4 on the PATH: `cargo test -p kiln-cli -- --ignored content`.
    #[test]
    #[ignore]
    fn godot_export_matches_fixture_and_catches_a_moved_door() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/content");
        let root = std::env::temp_dir().join(format!("kiln-content-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_dir(&fixture, &root);
        let rel = "kiln/content_manifest.json";

        assert_eq!(export(&root, None, None, |_| {}).unwrap(), vec!["world"]);
        assert_eq!(
            std::fs::read_to_string(root.join(rel)).unwrap(),
            std::fs::read_to_string(fixture.join(rel)).unwrap()
        );

        let scene = root.join("godot/astra/warehouse.tscn");
        let text = std::fs::read_to_string(&scene).unwrap();
        std::fs::write(&scene, text.replace("\"sector:yard\"]", "\"sector:roof\"]")).unwrap();
        export(&root, None, None, |_| {}).unwrap();
        let cat = kiln_core::open(&root).unwrap();
        assert!(
            cat.findings
                .iter()
                .any(|f| f.code == "content_manifest_mismatch" && f.subject == "world:data/content/world.ron#door:warehouse_yard"),
            "{:?}",
            cat.findings
        );
        assert!(export(&root, None, Some("nope"), |_| {}).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
