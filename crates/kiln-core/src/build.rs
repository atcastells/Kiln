//! The assembly plan of an asset: every copy of every part of its spec, with
//! the model it comes from, the size it must take, where it stands and the
//! palette material it wears. Blender reads the plan and builds the asset
//! (`kiln build`); nothing here needs Blender, so the plan can be tested.

use crate::catalog::{Catalog, Material, rel};
use crate::config::Config;
use crate::spec::Spec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Gap between parts laid out by Kiln when the spec gives no positions.
const GAP_M: f64 = 0.3;
/// The side of the box that stands in for a part with no model and no size.
const BOX_M: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub asset: String,
    pub name: String,
    pub states: Vec<String>,
    /// Absolute paths of what Blender writes.
    pub glb: String,
    pub blend: String,
    pub report: String,
    /// The spec gave no positions for some parts and Kiln put them in a row.
    pub auto_layout: Vec<String>,
    /// Where Blender writes which palette key each `part/zone` material wears.
    pub materials: String,
    pub items: Vec<Item>,
}

/// One copy of one part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// `<part>` or `<part>_<n>` when the part has more than one copy.
    pub name: String,
    pub part: String,
    pub kind: String,
    /// The glTF it is made from, absolute; none for a stand-in box.
    pub model: Option<String>,
    /// Stretch the model to `size_m` (variant, retouch) or keep its own size (reuse).
    pub fit: bool,
    /// Width, height and depth in metres (glTF axes: Y up).
    pub size_m: [f64; 3],
    /// Centre of its base, metres, Y up.
    pub at: [f64; 3],
    /// Degrees about X, Y and Z.
    pub turn: [f64; 3],
    /// The states it shows in (all when empty).
    pub only_in: Vec<String>,
    pub moves: Option<String>,
    /// What is still to do by hand: a stand-in box, a retouch.
    pub todo: Option<String>,
    /// Blender paints each material slot of the model with, in this order:
    /// `zones[slot]`, else `material`, else `slots[slot]`, else the model's own.
    /// The part's palette material, on every slot.
    pub material: Option<Paint>,
    /// Zone (slot) -> palette material, the spec's per-zone choices.
    pub zones: BTreeMap<String, Paint>,
    /// How to split the piece into zones (`zones::mode`), if it is split.
    pub split: Option<String>,
    /// Slot -> palette material, from the piece's bindings.
    pub slots: BTreeMap<String, Paint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Paint {
    pub key: String,
    pub color: Option<[f64; 3]>,
    /// The colour map, absolute.
    pub albedo: Option<String>,
}

/// What Blender wrote after building: read back by the catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    /// The GLB, relative to the project root (set by Kiln when read).
    #[serde(default)]
    pub glb: String,
    #[serde(default)]
    pub size_m: Option<[f64; 3]>,
    #[serde(default)]
    pub tris: u64,
    #[serde(default)]
    pub items: Vec<ReportItem>,
    #[serde(default)]
    pub built_at: String,
    /// `part/zone` -> the palette key it wears (none: the model's own); set
    /// by Kiln when read, from materials.json.
    #[serde(default)]
    pub materials: BTreeMap<String, Option<String>>,
    /// Colour variants already painted: id -> GLB relative to the project root.
    #[serde(default)]
    pub variants: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportItem {
    pub name: String,
    #[serde(default)]
    pub tris: u64,
    #[serde(default)]
    pub todo: Option<String>,
}

pub fn out_dir(root: &Path, config: &Config, asset: &str) -> std::path::PathBuf {
    root.join(&config.tools.build).join(asset)
}

fn find_paint(root: &Path, cat: &Catalog, key: &str) -> Option<Paint> {
    cat.materials
        .iter()
        .find(|m| m.key == key)
        .map(|m| paint(root, m))
}

/// The palette material each `part/zone` name wears, without assembling
/// again: variant > the part's zone > the part's material > the piece's
/// binding > none (the model keeps its own). A key outside the palette counts
/// as not chosen.
pub fn paints(
    root: &Path,
    cat: &Catalog,
    spec: &Spec,
    names: &[String],
    variant: Option<&str>,
) -> BTreeMap<String, Option<Paint>> {
    let var = variant.and_then(|v| spec.variants.iter().find(|x| x.id == v));
    names
        .iter()
        .map(|name| {
            let found = name.split_once('/').and_then(|(part, zone)| {
                let p = spec.parts.iter().find(|p| p.id == part)?;
                let piece = p
                    .source
                    .from
                    .as_ref()
                    .and_then(|f| cat.pieces.iter().find(|c| &c.key == f));
                let keys = [
                    var.and_then(|v| v.materials.get(name.as_str())),
                    p.materials.get(zone),
                    p.material.as_ref(),
                    piece.and_then(|c| c.bindings.get(zone)),
                ];
                keys.into_iter()
                    .flatten()
                    .find_map(|k| find_paint(root, cat, k))
            });
            (name.clone(), found)
        })
        .collect()
}

/// The Blender file of a built asset. It sits in a hidden folder because a
/// `.blend` in sight of Godot stops its import when the build is in the project.
pub fn blend_file(dir: &Path, asset: &str) -> std::path::PathBuf {
    dir.join(".kiln").join(format!("{asset}.blend"))
}

fn paint(root: &Path, m: &Material) -> Paint {
    Paint {
        key: m.key.clone(),
        color: m.color,
        albedo: m
            .albedo
            .as_ref()
            .map(|a| root.join(a).to_string_lossy().into()),
    }
}

pub fn plan(root: &Path, config: &Config, cat: &Catalog, spec: &Spec) -> Plan {
    let dir = out_dir(root, config, &spec.asset.id);
    let file = |ext: &str| {
        dir.join(format!("{}.{ext}", spec.asset.id))
            .to_string_lossy()
            .into_owned()
    };
    let find_paint = |key: &str| find_paint(root, cat, key);
    let zone_modes = crate::zones::load(root).unwrap_or_default();
    let mut items = Vec::new();
    let mut auto = Vec::new();
    for p in &spec.parts {
        let piece = p
            .source
            .from
            .as_ref()
            .and_then(|f| cat.pieces.iter().find(|c| &c.key == f))
            .filter(|_| p.source.kind != "search");
        let model = piece
            .and_then(|c| c.preview.as_ref())
            .map(|f| root.join(f).to_string_lossy().into_owned());
        let size = p
            .size_m
            .or(piece.and_then(|c| c.size_m))
            .unwrap_or([BOX_M; 3]);
        let todo = match (&p.source.kind[..], &model) {
            ("search", _) => Some(format!(
                "caja en lugar de un modelo por buscar: {}",
                p.source.query.as_deref().unwrap_or(&p.what)
            )),
            (_, None) => Some(match piece {
                Some(c) => format!("{} no tiene glTF; mídela con Godot", c.key),
                None => "caja: la pieza de origen no está en el catálogo".into(),
            }),
            ("retouch", _) => Some(format!(
                "retocar: {}",
                p.source.why.as_deref().unwrap_or(&p.what)
            )),
            _ => None,
        };
        let slots: BTreeMap<String, Paint> = piece
            .map(|c| {
                c.bindings
                    .iter()
                    .filter_map(|(s, m)| Some((s.clone(), find_paint(m)?)))
                    .collect()
            })
            .unwrap_or_default();
        let zones: BTreeMap<String, Paint> = p
            .materials
            .iter()
            .filter_map(|(z, m)| Some((z.clone(), find_paint(m)?)))
            .collect();
        let split = p
            .source
            .from
            .as_deref()
            .and_then(|f| crate::zones::mode(&zone_modes, f))
            .map(String::from);
        let n = p.count.unwrap_or(1).max(1) as usize;
        if p.at.len() < n {
            auto.push(p.id.clone());
        }
        for i in 0..n {
            items.push(Item {
                name: if n == 1 {
                    p.id.clone()
                } else {
                    format!("{}_{}", p.id, i + 1)
                },
                part: p.id.clone(),
                kind: p.source.kind.clone(),
                model: model.clone(),
                // A reused piece keeps its size unless it has no model to keep.
                fit: p.source.kind != "reuse" || model.is_none(),
                size_m: if p.source.kind == "reuse" {
                    piece.and_then(|c| c.size_m).unwrap_or(size)
                } else {
                    size
                },
                at: p.at.get(i).copied().unwrap_or([f64::NAN; 3]),
                turn: p.turn.get(i).copied().unwrap_or([0.0; 3]),
                only_in: p.only_in.clone(),
                moves: p.moves.clone(),
                todo: todo.clone(),
                material: p.material.as_deref().and_then(find_paint),
                zones: zones.clone(),
                split: split.clone(),
                slots: slots.clone(),
            });
        }
    }
    lay_out(&mut items);
    Plan {
        asset: spec.asset.id.clone(),
        name: spec.asset.name.clone(),
        states: spec.asset.states.clone(),
        glb: file("glb"),
        blend: blend_file(&dir, &spec.asset.id).to_string_lossy().into(),
        report: dir.join("report.json").to_string_lossy().into(),
        auto_layout: auto,
        materials: dir.join("materials.json").to_string_lossy().into(),
        items,
    }
}

/// Copies with no position go in a row along X, behind what is placed.
fn lay_out(items: &mut [Item]) {
    let placed_depth = items
        .iter()
        .filter(|i| !i.at[0].is_nan())
        .map(|i| i.at[2] + i.size_m[2] / 2.0)
        .fold(0.0, f64::max);
    let loose: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].at[0].is_nan())
        .collect();
    let total: f64 = loose
        .iter()
        .map(|&i| items[i].size_m[0] + GAP_M)
        .sum::<f64>()
        - GAP_M;
    let mut x = -total / 2.0;
    let behind = loose.len() < items.len();
    for i in loose {
        let [w, _, d] = items[i].size_m;
        let z = if behind {
            placed_depth + 1.0 + d / 2.0
        } else {
            0.0
        };
        items[i].at = [x + w / 2.0, 0.0, z];
        x += w + GAP_M;
    }
}

/// Reads what Blender reported for each spec that has been built.
pub fn reports(root: &Path, config: &Config, specs: &[Spec]) -> BTreeMap<String, Report> {
    specs
        .iter()
        .filter_map(|s| {
            let dir = out_dir(root, config, &s.asset.id);
            let text = std::fs::read_to_string(dir.join("report.json")).ok()?;
            let mut r: Report = serde_json::from_str(&text).ok()?;
            r.glb = rel(root, &dir.join(format!("{}.glb", s.asset.id)));
            r.materials = std::fs::read_to_string(dir.join("materials.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            r.variants = s
                .variants
                .iter()
                .map(|v| (&v.id, dir.join(format!("{}--{}.glb", s.asset.id, v.id))))
                .filter(|(_, f)| f.exists())
                .map(|(id, f)| (id.clone(), rel(root, &f)))
                .collect();
            Some((s.asset.id.clone(), r))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, w: f64, at: Option<[f64; 3]>) -> Item {
        Item {
            name: name.into(),
            part: name.into(),
            kind: "search".into(),
            model: None,
            fit: true,
            size_m: [w, 1.0, 1.0],
            at: at.unwrap_or([f64::NAN; 3]),
            turn: [0.0; 3],
            only_in: Vec::new(),
            moves: None,
            todo: None,
            material: None,
            zones: BTreeMap::new(),
            split: None,
            slots: BTreeMap::new(),
        }
    }

    fn cat() -> Catalog {
        let mat = |k: &str| Material {
            key: k.into(),
            name: k.into(),
            source: "table".into(),
            file: String::new(),
            detail: Default::default(),
            color: None,
            albedo: None,
        };
        let piece = |key: &str, bindings: &[(&str, &str)]| crate::catalog::Piece {
            key: key.into(),
            kit: "kit".into(),
            name: key.into(),
            file: None,
            format: "glb".into(),
            size_m: None,
            min: None,
            tris: None,
            material_slots: vec![],
            split: None,
            images: None,
            licenses: vec![],
            credit: None,
            manifest: Default::default(),
            measured_by: None,
            preview: None,
            bindings: bindings
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        };
        Catalog {
            project: String::new(),
            root: String::new(),
            kits: vec![],
            pieces: vec![piece("kit/beam", &[]), piece("kit/post", &[("wood", "oak")])],
            materials: vec![mat("oak"), mat("iron"), mat("pine")],
            specs: vec![],
            tree: None,
            builds: Default::default(),
            findings: vec![],
        }
    }

    const SPEC: &str = r#"
[asset]
id = "a"
name = "A"
[[parts]]
id = "beam"
what = "viga"
material = "iron"
materials = { wood = "oak", bad = "ghost" }
[parts.source]
kind = "reuse"
from = "kit/beam"
[[parts]]
id = "flat"
what = "plano"
material = "iron"
[parts.source]
kind = "reuse"
from = "kit/beam"
[[parts]]
id = "post"
what = "poste"
[parts.source]
kind = "reuse"
from = "kit/post"
[[variants]]
id = "old"
materials = { "beam/wood" = "pine", "flat/x" = "ghost" }
"#;

    #[test]
    fn paints_follow_the_order_variant_zone_part_binding_model() {
        let s = crate::spec::parse(SPEC).unwrap();
        let root = Path::new("/p");
        let names: Vec<String> = [
            "beam/wood", "beam/bad", "flat/x", "post/wood", "post/metal", "ghost/x", "nope",
        ]
        .map(String::from)
        .to_vec();
        let key = |m: &BTreeMap<String, Option<Paint>>, n: &str| {
            m[n].as_ref().map(|p| p.key.clone())
        };
        let base = paints(root, &cat(), &s, &names, None);
        assert_eq!(key(&base, "beam/wood").as_deref(), Some("oak"));
        // a key outside the palette falls through to the part's material
        assert_eq!(key(&base, "beam/bad").as_deref(), Some("iron"));
        assert_eq!(key(&base, "flat/x").as_deref(), Some("iron"));
        assert_eq!(key(&base, "post/wood").as_deref(), Some("oak"));
        assert_eq!(key(&base, "post/metal"), None);
        assert_eq!(key(&base, "ghost/x"), None);
        assert_eq!(key(&base, "nope"), None);
        let old = paints(root, &cat(), &s, &names, Some("old"));
        assert_eq!(key(&old, "beam/wood").as_deref(), Some("pine"));
        assert_eq!(key(&old, "flat/x").as_deref(), Some("iron"));
        let none = paints(root, &cat(), &s, &names, Some("missing"));
        assert_eq!(key(&none, "beam/wood").as_deref(), Some("oak"));
    }

    #[test]
    fn loose_parts_go_in_a_row_behind_the_placed_ones() {
        let mut v = vec![
            item("a", 1.0, None),
            item("b", 2.0, None),
            item("c", 1.0, Some([0.0, 0.0, 0.0])),
        ];
        lay_out(&mut v);
        // row of 1 + 0.3 + 2 = 3.3 m centred on X
        assert!((v[0].at[0] + 1.15).abs() < 1e-9);
        assert!((v[1].at[0] - 0.65).abs() < 1e-9);
        // behind c (its back at z = 0.5) by a metre, plus half their depth
        assert!((v[0].at[2] - 2.0).abs() < 1e-9);
        assert_eq!(v[2].at, [0.0, 0.0, 0.0]);
    }
}
