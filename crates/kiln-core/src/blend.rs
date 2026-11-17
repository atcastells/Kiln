//! `.blend` files inside a kit are pieces of the catalog. Kiln does not read
//! them: a job outside this crate runs Blender and exports a GLB of each to
//! `<cache>/blender/`, and records it in `<cache>/blender/pieces.json` with the
//! hash of the `.blend` it came from. The catalog reads the GLB back.

use crate::catalog::{Catalog, Finding, Severity, rel, round3};
use crate::config::Config;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One `.blend` for Blender to export.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Request {
    pub key: String,
    /// The `.blend`, absolute.
    pub blend: String,
    /// Where Blender writes the GLB, absolute.
    pub glb: String,
    pub sha256: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Entry {
    sha256: String,
    glb: String,
}

pub fn cache_file(root: &Path, config: &Config) -> PathBuf {
    root.join(&config.tools.cache).join("blender/pieces.json")
}

fn glb_name(key: &str) -> String {
    format!("{}.glb", key.replace('/', "__"))
}

fn is_blend(p: &crate::catalog::Piece) -> bool {
    p.format == "blend" && p.file.is_some()
}

/// The `.blend` pieces whose export is missing or older than the file.
pub fn requests(root: &Path, config: &Config, cat: &Catalog) -> Vec<Request> {
    let dir = root.join(&config.tools.cache).join("blender");
    let entries = read_entries(root, config);
    cat.pieces
        .iter()
        .filter(|p| is_blend(p))
        .filter_map(|p| {
            let blend = root.join(p.file.as_ref()?);
            let sha256 = crate::asset::sha256_file(&blend).ok()?;
            if fresh(&dir, entries.get(&p.key), &sha256).is_some() {
                return None;
            }
            Some(Request {
                key: p.key.clone(),
                blend: blend.to_string_lossy().into(),
                glb: dir.join(glb_name(&p.key)).to_string_lossy().into(),
                sha256,
            })
        })
        .collect()
}

/// Notes that the request was exported: its GLB now stands for that `.blend`.
pub fn record(root: &Path, config: &Config, done: &Request) -> Result<(), String> {
    let mut entries = read_entries(root, config);
    let glb = glb_name(&done.key);
    entries.insert(
        done.key.clone(),
        Entry {
            sha256: done.sha256.clone(),
            glb,
        },
    );
    let file = cache_file(root, config);
    let text = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))
}

fn read_entries(root: &Path, config: &Config) -> BTreeMap<String, Entry> {
    std::fs::read_to_string(cache_file(root, config))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The exported GLB when it matches the `.blend` hash and is on disk.
fn fresh(dir: &Path, entry: Option<&Entry>, sha256: &str) -> Option<PathBuf> {
    let e = entry?;
    let name = Path::new(&e.glb).file_name()?;
    let glb = dir.join(name);
    (e.sha256 == sha256 && glb.is_file()).then_some(glb)
}

/// Fills the `.blend` pieces from their exported GLB and flags the ones to export.
pub fn apply(root: &Path, config: &Config, cat: &mut Catalog) {
    if !cat.pieces.iter().any(is_blend) {
        return;
    }
    let dir = root.join(&config.tools.cache).join("blender");
    let entries = read_entries(root, config);
    for p in cat.pieces.iter_mut().filter(|p| is_blend(p)) {
        let sha = p
            .file
            .as_ref()
            .and_then(|f| crate::asset::sha256_file(&root.join(f)).ok());
        let glb = sha
            .as_deref()
            .and_then(|s| fresh(&dir, entries.get(&p.key), s));
        let Some(glb) = glb else {
            cat.findings.push(Finding {
                severity: Severity::Warn,
                subject: p.key.clone(),
                code: "blend_stale".into(),
                message: "el .blend no se ha exportado o ha cambiado desde entonces; \
                          ejecuta el trabajo de medida (`kiln godot`) o construye un asset que lo use"
                    .into(),
            });
            continue;
        };
        match crate::glb::read(&glb) {
            Ok(info) => {
                p.size_m = Some(round3(info.size()));
                p.min = Some(round3(info.min));
                p.tris = Some(info.tris);
                p.material_slots = info.materials;
                p.images = Some(info.images);
                p.measured_by = Some("blender".into());
                p.preview = Some(rel(root, &glb));
            }
            Err(e) => cat.findings.push(Finding {
                severity: Severity::Error,
                subject: p.key.clone(),
                code: "glb_unreadable".into(),
                message: format!("no se puede leer el GLB: {e}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn glb_bytes() -> Vec<u8> {
        let doc = json!({
            "asset": {"version": "2.0"},
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
            "accessors": [{"count": 3, "min": [0.0, 0.0, 0.0], "max": [1.0, 2.0, 0.5]}],
            "materials": [{"name": "wood"}]
        });
        let mut j = serde_json::to_vec(&doc).unwrap();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend(((20 + j.len()) as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(j);
        out
    }

    fn project(name: &str) -> (PathBuf, Config) {
        let root = std::env::temp_dir().join(format!("kiln-blend-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("kits/props")).unwrap();
        let cfg = Config::parse("[project]\nname = \"t\"\n[assets]\nkits = [\"kits\"]\n").unwrap();
        (root, cfg)
    }

    fn export(root: &Path, cfg: &Config, key: &str, blend: &Path) {
        let dir = root.join(&cfg.tools.cache).join("blender");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(glb_name(key)), glb_bytes()).unwrap();
        let sha = crate::asset::sha256_file(blend).unwrap();
        let json = json!({ key: { "sha256": sha, "glb": glb_name(key) } });
        std::fs::write(cache_file(root, cfg), json.to_string()).unwrap();
    }

    fn opened(root: &Path, cfg: &Config) -> Catalog {
        let mut cat = crate::catalog::index(root, cfg);
        apply(root, cfg, &mut cat);
        cat
    }

    #[test]
    fn never_exported_is_requested_and_flagged() {
        let (root, cfg) = project("never");
        std::fs::write(root.join("kits/props/door.blend"), b"fake").unwrap();
        let cat = opened(&root, &cfg);
        let p = &cat.pieces[0];
        assert_eq!((p.key.as_str(), p.format.as_str()), ("props/door", "blend"));
        assert!(p.size_m.is_none());
        assert!(
            cat.findings
                .iter()
                .any(|f| f.code == "blend_stale" && f.subject == "props/door")
        );
        let reqs = requests(&root, &cfg, &cat);
        assert_eq!(reqs.len(), 1);
        assert!(reqs[0].glb.ends_with("blender/props__door.glb"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn exported_and_matching_is_measured() {
        let (root, cfg) = project("ok");
        let blend = root.join("kits/props/door.blend");
        std::fs::write(&blend, b"fake").unwrap();
        export(&root, &cfg, "props/door", &blend);
        let cat = opened(&root, &cfg);
        let p = &cat.pieces[0];
        assert_eq!(p.size_m, Some([1.0, 2.0, 0.5]));
        assert_eq!(p.measured_by.as_deref(), Some("blender"));
        assert_eq!(p.tris, Some(1));
        assert!(
            p.preview
                .as_deref()
                .is_some_and(|v| v.ends_with("props__door.glb"))
        );
        assert!(cat.findings.iter().all(|f| f.code != "blend_stale"));
        assert!(requests(&root, &cfg, &cat).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn changed_blend_is_stale_again() {
        let (root, cfg) = project("changed");
        let blend = root.join("kits/props/door.blend");
        std::fs::write(&blend, b"fake").unwrap();
        export(&root, &cfg, "props/door", &blend);
        std::fs::write(&blend, b"fake, edited").unwrap();
        let cat = opened(&root, &cfg);
        assert!(cat.pieces[0].size_m.is_none());
        assert!(cat.findings.iter().any(|f| f.code == "blend_stale"));
        assert_eq!(requests(&root, &cfg, &cat).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn glb_beside_blend_wins() {
        let (root, cfg) = project("both");
        std::fs::write(root.join("kits/props/x.blend"), b"fake").unwrap();
        std::fs::write(root.join("kits/props/x.glb"), glb_bytes()).unwrap();
        std::fs::write(root.join("kits/props/x.blend1"), b"backup").unwrap();
        let cat = opened(&root, &cfg);
        assert_eq!(cat.pieces.len(), 1);
        assert_eq!(cat.pieces[0].format, "glb");
        assert!(cat.findings.iter().all(|f| f.code != "blend_stale"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_blend_adds_nothing() {
        let (root, cfg) = project("none");
        std::fs::write(root.join("kits/props/x.glb"), glb_bytes()).unwrap();
        let mut cat = crate::catalog::index(&root, &cfg);
        let before = serde_json::to_string(&cat).unwrap();
        apply(&root, &cfg, &mut cat);
        assert_eq!(serde_json::to_string(&cat).unwrap(), before);
        assert!(requests(&root, &cfg, &cat).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
