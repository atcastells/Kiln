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
    /// The piece as Godot names it (`res://kit/medievalkit/Barrel.res`).
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
