//! The catalog: every piece and material the project already has, read from
//! its kits, its manifests and its material tables.

use crate::{config::Config, glb, license};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct Catalog {
    pub project: String,
    pub root: String,
    pub kits: Vec<Kit>,
    pub pieces: Vec<Piece>,
    pub materials: Vec<Material>,
    /// The asset specs of the project (`[assets] specs`).
    pub specs: Vec<crate::spec::Spec>,
    /// The project by levels, from the game down to its assets.
    pub tree: Option<crate::tree::Tree>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Kit {
    /// `<kits folder>/<kit>` relative to the project root.
    pub path: String,
    pub name: String,
    /// Licences named in the kit's LICENSE.md (empty when there is none).
    pub licenses: Vec<String>,
    pub has_license_file: bool,
    /// The LICENSE.md carries an attribution line (credit at kit level).
    pub has_credit: bool,
    pub pieces: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Piece {
    /// `kit/piece`, the key other parts of Kiln use.
    pub key: String,
    pub kit: String,
    pub name: String,
    /// The model file relative to the project root, if there is one.
    pub file: Option<String>,
    /// glb, gltf, res, ... or `none` when only a manifest names it.
    pub format: String,
    pub size_m: Option<[f64; 3]>,
    pub min: Option<[f64; 3]>,
    pub tris: Option<u64>,
    pub material_slots: Vec<String>,
    pub images: Option<usize>,
    /// Licences of the piece: its own credit's, else the kit's.
    pub licenses: Vec<String>,
    pub credit: Option<String>,
    /// What the kit's JSON manifests say of it, as written.
    pub manifest: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Material {
    pub key: String,
    pub name: String,
    /// `table` (a row of a materials TOML) or `texture` (a texture set).
    pub source: String,
    pub file: String,
    pub detail: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    /// The piece or material key, or the kit path.
    pub subject: String,
    pub code: String,
    pub message: String,
}

const MODEL_EXT: [&str; 6] = ["glb", "gltf", "res", "tres", "tscn", "obj"];

pub fn index(root: &Path, config: &Config) -> Catalog {
    let mut cat = Catalog {
        project: config.project.name.clone(),
        root: root.display().to_string(),
        kits: Vec::new(),
        pieces: Vec::new(),
        materials: Vec::new(),
        specs: Vec::new(),
        tree: None,
        findings: Vec::new(),
    };
    for kits in &config.assets.kits {
        for dir in sorted_dirs(&root.join(kits)) {
            index_kit(root, &dir, &mut cat);
        }
    }
    for t in &config.materials.tables {
        index_table(root, t, &mut cat);
    }
    for t in &config.materials.textures {
        index_textures(root, &root.join(t), &config.materials, &mut cat);
    }
    cat
}

fn index_kit(root: &Path, dir: &Path, cat: &mut Catalog) {
    let name = file_name(dir);
    let lic_path = dir.join("LICENSE.md");
    let lic_text = std::fs::read_to_string(&lic_path).ok();
    let kit_licenses = lic_text.as_deref().map(license::find).unwrap_or_default();
    let manifests = read_manifests(dir);

    // Models: files with a model extension, and subfolders holding one (a glTF folder).
    let mut models: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in read_dir(dir) {
        if entry.is_dir() {
            if let Some(inner) = read_dir(&entry)
                .into_iter()
                .find(|p| ext(p).is_some_and(|e| e == "glb" || e == "gltf"))
            {
                models.insert(file_name(&entry), inner);
            }
        } else if ext(&entry).is_some_and(|e| MODEL_EXT.contains(&e.as_str())) {
            models.entry(stem(&entry)).or_insert(entry);
        }
    }
    let mut names: Vec<String> = models.keys().cloned().collect();
    for k in manifests.keys() {
        if !names.contains(k) {
            names.push(k.clone());
        }
    }
    names.sort();

    let start = cat.pieces.len();
    for piece in names {
        let key = format!("{name}/{piece}");
        let manifest = manifests.get(&piece).cloned().unwrap_or_default();
        let credit = manifest
            .get("credit")
            .and_then(Value::as_str)
            .map(str::to_string);
        let own = credit.as_deref().map(license::find).unwrap_or_default();
        let mut p = Piece {
            key: key.clone(),
            kit: name.clone(),
            name: piece.clone(),
            file: models.get(&piece).map(|f| rel(root, f)),
            format: models
                .get(&piece)
                .and_then(|f| ext(f))
                .unwrap_or_else(|| "none".into()),
            size_m: None,
            min: None,
            tris: None,
            material_slots: Vec::new(),
            images: None,
            licenses: if own.is_empty() {
                kit_licenses.clone()
            } else {
                own
            },
            credit,
            manifest,
        };
        if let Some(file) = models
            .get(&piece)
            .filter(|f| matches!(ext(f).as_deref(), Some("glb" | "gltf")))
        {
            match glb::read(file) {
                Ok(info) => {
                    p.size_m = Some(round3(info.size()));
                    p.min = Some(round3(info.min));
                    p.tris = Some(info.tris);
                    p.material_slots = info.materials;
                    p.images = Some(info.images);
                }
                Err(e) => cat.findings.push(Finding {
                    severity: Severity::Error,
                    subject: key,
                    code: "glb_unreadable".into(),
                    message: format!("no se puede leer el GLB: {e}"),
                }),
            }
        }
        cat.pieces.push(p);
    }
    cat.kits.push(Kit {
        path: rel(root, dir),
        name,
        licenses: kit_licenses,
        has_license_file: lic_text.is_some(),
        has_credit: lic_text.as_deref().is_some_and(|t| {
            let l = t.to_lowercase();
            l.contains("licensed under")
                || l.contains("crédito")
                || l.contains("credit")
                || l.contains("attribution")
        }),
        pieces: cat.pieces.len() - start,
    });
}

/// Each JSON file of the kit that is an object of objects: one entry per piece.
fn read_manifests(dir: &Path) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for f in read_dir(dir)
        .into_iter()
        .filter(|p| ext(p).as_deref() == Some("json"))
    {
        let Ok(Value::Object(top)) = std::fs::read_to_string(&f)
            .map_err(|_| ())
            .and_then(|t| serde_json::from_str(&t).map_err(|_| ()))
        else {
            continue;
        };
        for (k, v) in top {
            if k.starts_with('_') {
                continue;
            }
            if let Value::Object(fields) = v {
                let entry: &mut BTreeMap<String, Value> = out.entry(k).or_default();
                for (fk, fv) in fields {
                    entry.entry(fk).or_insert(fv);
                }
                entry
                    .entry("_manifest".into())
                    .or_insert(Value::String(file_name(&f)));
            }
        }
    }
    out
}

fn index_table(root: &Path, file: &str, cat: &mut Catalog) {
    let path = root.join(file);
    let Ok(text) = std::fs::read_to_string(&path) else {
        cat.findings.push(Finding {
            severity: Severity::Error,
            subject: file.into(),
            code: "table_missing".into(),
            message: "la tabla de materiales de kiln.toml no existe".into(),
        });
        return;
    };
    let Ok(doc) = text.parse::<toml::Table>() else {
        cat.findings.push(Finding {
            severity: Severity::Error,
            subject: file.into(),
            code: "table_unreadable".into(),
            message: "TOML roto".into(),
        });
        return;
    };
    for (_, v) in doc {
        let Some(rows) = v.as_array() else { continue };
        for row in rows.iter().filter_map(|r| r.as_table()) {
            let Some(id) = row.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let detail = row
                .iter()
                .filter(|(k, _)| *k != "id" && *k != "name")
                .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or(Value::Null)))
                .collect();
            cat.materials.push(Material {
                key: id.into(),
                name: row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .into(),
                source: "table".into(),
                file: rel(root, &path),
                detail,
            });
        }
    }
}

fn index_textures(
    root: &Path,
    dir: &Path,
    mats: &crate::config::MaterialsSection,
    cat: &mut Catalog,
) {
    for entry in read_dir(dir) {
        if entry.is_dir() {
            index_textures(root, &entry, mats, cat);
            continue;
        }
        let n = file_name(&entry);
        let Some(key) = n
            .rsplit_once('.')
            .and_then(|(s, _)| s.strip_suffix(mats.albedo_suffix.as_str()))
        else {
            continue;
        };
        if !matches!(
            ext(&entry).as_deref(),
            Some("jpg" | "jpeg" | "png" | "webp")
        ) {
            continue;
        }
        let mut detail = BTreeMap::new();
        let lic = std::fs::read_to_string(entry.with_file_name("LICENSE.md"))
            .map(|t| license::find(&t))
            .unwrap_or_default();
        detail.insert("licenses".into(), serde_json::json!(lic));
        for map in &mats.map_suffixes {
            let has = read_dir(dir)
                .iter()
                .any(|p| file_name(p).starts_with(&format!("{key}{map}.")));
            detail.insert(map.trim_start_matches(['_', '-']).into(), Value::Bool(has));
        }
        cat.materials.push(Material {
            key: key.into(),
            name: key.into(),
            source: "texture".into(),
            file: rel(root, &entry),
            detail,
        });
    }
}

fn read_dir(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}
fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    read_dir(dir).into_iter().filter(|p| p.is_dir()).collect()
}
fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
fn ext(p: &Path) -> Option<String> {
    p.extension().map(|s| s.to_string_lossy().to_lowercase())
}
fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}
fn round3(v: [f64; 3]) -> [f64; 3] {
    v.map(|x| (x * 1000.0).round() / 1000.0)
}
