//! What Godot measures for Kiln. Pieces saved as Godot resources (`.res`,
//! `.tres`, `.tscn`) only open in Godot: `kiln godot` runs it without a
//! window over them, and it writes their size, triangles and material slots,
//! with a GLB of each, to `<cache>/godot/pieces.json`. The catalog reads that
//! file back, so measuring is done once and not on every read.

use crate::catalog::{Catalog, Finding, Severity, rel, round3};
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const GODOT_EXT: [&str; 4] = ["res", "tres", "tscn", "scn"];

/// One piece for Godot to open: `res://` path in, GLB and numbers out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Request {
    pub key: String,
    /// The piece as Godot names it (`res://kits/props/barrel.res`).
    pub res: String,
    /// Where Godot writes its GLB, absolute.
    pub glb: String,
}

/// What Godot answered for one piece.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Measure {
    #[serde(default)]
    pub size_m: Option<[f64; 3]>,
    #[serde(default)]
    pub min: Option<[f64; 3]>,
    #[serde(default)]
    pub tris: Option<u64>,
    #[serde(default)]
    pub material_slots: Vec<String>,
    /// The exported GLB, absolute (the catalog makes it relative).
    #[serde(default)]
    pub glb: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

pub fn cache_file(root: &Path, config: &Config) -> PathBuf {
    root.join(&config.tools.cache).join("godot/pieces.json")
}

/// The pieces of the catalog that only Godot can read, under the Godot project.
pub fn requests(root: &Path, config: &Config, cat: &Catalog) -> Vec<Request> {
    let Some(gp) = &config.tools.godot_project else {
        return Vec::new();
    };
    let gp = root.join(gp);
    let out_dir = root.join(&config.tools.cache).join("godot");
    cat.pieces
        .iter()
        .filter(|p| GODOT_EXT.contains(&p.format.as_str()))
        .filter_map(|p| {
            let file = root.join(p.file.as_ref()?);
            let inside = file.strip_prefix(&gp).ok()?;
            Some(Request {
                key: p.key.clone(),
                res: format!("res://{}", inside.to_string_lossy().replace('\\', "/")),
                glb: out_dir
                    .join(format!("{}.glb", p.key))
                    .to_string_lossy()
                    .into(),
            })
        })
        .collect()
}

/// Fills the pieces Godot measured, and says which measures are old.
pub fn apply(root: &Path, config: &Config, cat: &mut Catalog) {
    let path = cache_file(root, config);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(measures) = serde_json::from_str::<BTreeMap<String, Measure>>(&text) else {
        cat.findings.push(Finding {
            severity: Severity::Warn,
            subject: rel(root, &path),
            code: "godot_cache_unreadable".into(),
            message: "las medidas de Godot no se leen; vuelve a medir".into(),
        });
        return;
    };
    let when = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    for p in cat.pieces.iter_mut() {
        let Some(m) = measures.get(&p.key) else {
            continue;
        };
        if let Some(e) = &m.error {
            cat.findings.push(Finding {
                severity: Severity::Warn,
                subject: p.key.clone(),
                code: "godot_failed".into(),
                message: format!("Godot no pudo medirla: {e}"),
            });
            continue;
        }
        p.size_m = m.size_m.map(round3);
        p.min = m.min.map(round3);
        p.tris = m.tris;
        p.material_slots = m.material_slots.clone();
        p.measured_by = Some("godot".into());
        p.preview = m
            .glb
            .as_ref()
            .map(PathBuf::from)
            .filter(|g| g.exists())
            .map(|g| rel(root, &g));
        let newer = p.file.as_ref().and_then(|f| {
            std::fs::metadata(root.join(f))
                .and_then(|m| m.modified())
                .ok()
        });
        if let (Some(src), Some(when)) = (newer, when)
            && src > when
        {
            cat.findings.push(Finding {
                severity: Severity::Info,
                subject: p.key.clone(),
                code: "godot_stale".into(),
                message: "el fichero cambió después de medirlo; vuelve a medir".into(),
            });
        }
    }
}

/// The Godot material path of a palette key, from the `[materials] godot` template.
pub fn material_path(template: &str, key: &str) -> String {
    template.replace("{key}", key)
}

/// Link GLB materials to Godot resources in the text of a `.import` file; a
/// name with no path loses its link and wears what the GLB carries.
pub fn link_materials(import_text: &str, links: &BTreeMap<String, Option<String>>) -> Result<String, String> {
    const KEY: &str = "_subresources=";
    let shape = |why: &str| format!("el .import no tiene la forma esperada: {why}");
    let at = import_text
        .match_indices(KEY)
        .find(|(i, _)| *i == 0 || import_text.as_bytes()[i - 1] == b'\n')
        .map(|(i, _)| i + KEY.len())
        .ok_or_else(|| shape("falta _subresources"))?;
    let rest = &import_text[at..];
    let (mut depth, mut in_str, mut escaped, mut len) = (0usize, false, false, None);
    for (i, c) in rest.char_indices() {
        if in_str {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_str = false,
                _ => {}
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '{' {
            depth += 1;
        } else if c == '}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                len = Some(i + 1);
                break;
            }
        } else if depth == 0 && !c.is_whitespace() {
            break;
        }
    }
    let len = len.ok_or_else(|| shape("_subresources no es un diccionario"))?;
    let mut value: serde_json::Value = serde_json::from_str(&rest[..len])
        .map_err(|e| shape(&format!("_subresources no se puede leer ({e})")))?;
    let root = value.as_object_mut().ok_or_else(|| shape("_subresources no es un diccionario"))?;
    let materials = root
        .entry("materials")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| shape("materials no es un diccionario"))?;
    for (name, path) in links {
        match path {
            Some(path) => materials.insert(
                name.clone(),
                serde_json::json!({ "use_external/enabled": true, "use_external/path": path }),
            ),
            None => materials.remove(name),
        };
    }
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| shape(&e.to_string()))?;
    Ok(format!("{}{}{}", &import_text[..at], pretty, &rest[len..]))
}

#[cfg(test)]
mod link_tests {
    use super::*;

    const LINKED: &str = include_str!("../tests/fixtures/godot/linked.glb.import");

    fn one(name: &str, path: &str) -> BTreeMap<String, Option<String>> {
        BTreeMap::from([(name.to_string(), Some(path.to_string()))])
    }

    #[test]
    fn a_name_without_path_loses_its_link() {
        let out = link_materials(LINKED, &BTreeMap::from([("post/wood".to_string(), None)])).unwrap();
        assert!(!out.contains("post/wood"));
        assert!(out.contains("gltf/naming_version=2"));
    }

    #[test]
    fn relink_replaces_path_and_drops_fallback() {
        let out = link_materials(LINKED, &one("post/wood", "res://materials/oak.tres")).unwrap();
        assert!(out.contains("res://materials/oak.tres"));
        assert!(!out.contains("red.tres"));
        assert!(!out.contains("fallback_path"));
        assert!(out.contains("gltf/naming_version=2\n"));
        assert!(out.contains("[deps]\n\nsource_file=\"res://built/t.glb\""));
    }

    #[test]
    fn adding_a_name_keeps_the_others() {
        let out = link_materials(LINKED, &one("post/iron", "res://materials/iron.tres")).unwrap();
        assert!(out.contains("res://materials/iron.tres"));
        assert!(out.contains("\"post/wood\""));
        assert!(out.contains("fallback_path"));
    }

    #[test]
    fn empty_subresources_gets_the_entry() {
        let text = "[params]\n\n_subresources={}\ngltf/naming_version=2\n";
        let out = link_materials(text, &one("a/b", "res://m/x.tres")).unwrap();
        assert!(out.contains("\"use_external/path\": \"res://m/x.tres\""));
        assert!(out.contains("\"use_external/enabled\": true"));
        assert!(out.ends_with("}\ngltf/naming_version=2\n"));
    }

    #[test]
    fn missing_subresources_is_an_error() {
        let err = link_materials("[params]\n\nfoo=1\n", &one("a", "res://x.tres")).unwrap_err();
        assert!(err.starts_with("el .import no tiene la forma esperada"));
    }

    #[test]
    fn material_path_fills_the_key() {
        assert_eq!(material_path("res://materials/{key}.tres", "oak"), "res://materials/oak.tres");
    }
}
