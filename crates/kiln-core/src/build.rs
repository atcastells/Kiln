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
    /// Mount points, exported as empty nodes named `anchor_<id>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<PlanAnchor>,
    /// Box-UV scale of modelled parts; set only when some item has shapes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv_per_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAnchor {
    pub name: String,
    /// Metres from the asset's origin, Y up.
    pub at: [f64; 3],
    /// Degrees about X, Y and Z.
    pub turn: [f64; 3],
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
    /// The forms Blender builds the part from (metres, glTF axes, part frame).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<crate::model::Shape>,
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
    /// The assets it is made of that changed since it was assembled, or that
    /// are to be assembled again themselves: it is to be assembled again.
    #[serde(default, skip_deserializing)]
    pub stale: Vec<String>,
    /// The pieces (`kit/piece`) whose source changed since it was assembled
    /// (a `.blend` saved again): it is to be assembled again.
    #[serde(default, skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
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
                let open = p.source.kind != "asset";
                let keys = [
                    var.and_then(|v| v.materials.get(name.as_str())),
                    p.materials.get(zone).filter(|_| open),
                    p.material.as_ref().filter(|_| open),
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

/// The GLB a built asset left, or the one of its colour variant; `None`
/// while it has not been assembled.
pub fn child_glb(
    root: &Path,
    config: &Config,
    id: &str,
    variant: Option<&str>,
) -> Option<std::path::PathBuf> {
    let name = match variant {
        Some(v) => format!("{id}--{v}.glb"),
        None => format!("{id}.glb"),
    };
    Some(out_dir(root, config, id).join(name)).filter(|f| f.exists())
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

/// Refuses a spec whose shapes `kiln check` would reject, so Blender never
/// gets shapes outside the limits (R6), even when build runs without check.
pub fn refuse(spec: &Spec) -> Result<(), String> {
    for p in &spec.parts {
        let problems = match (p.source.kind == "model", p.shapes.is_empty()) {
            (true, _) => {
                let copies = crate::model::copies_problem(&p.shapes, p.count.unwrap_or(1));
                crate::model::problems(&p.shapes).into_iter().chain(copies).collect()
            }
            (false, false) => vec!["`shapes` solo vale en una pieza con fuente `model`".into()],
            (false, true) => continue,
        };
        if !problems.is_empty() {
            return Err(format!(
                "no se monta: la pieza «{}» tiene formas que `kiln check` rechaza: {}",
                p.id,
                problems.join("; ")
            ));
        }
    }
    Ok(())
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
            .filter(|_| !matches!(&p.source.kind[..], "search" | "asset"));
        let child = (p.source.kind == "asset")
            .then_some(p.source.from.as_deref())
            .flatten();
        let model = match child {
            Some(c) => child_glb(root, config, c, p.source.variant.as_deref())
                .map(|f| f.to_string_lossy().into_owned()),
            None => piece
                .and_then(|c| c.preview.as_ref())
                .map(|f| root.join(f).to_string_lossy().into_owned()),
        };
        let child_size = child.and_then(|c| cat.builds.get(c)).and_then(|r| r.size_m);
        let is_model = p.source.kind == "model";
        let size = p
            .size_m
            .or(is_model
                .then(|| {
                    crate::model::bounds(&p.shapes).map(|(lo, hi)| [0, 1, 2].map(|i| hi[i] - lo[i]))
                })
                .flatten())
            .or(piece.and_then(|c| c.size_m))
            .or(child_size)
            .unwrap_or([BOX_M; 3]);
        let todo = match (&p.source.kind[..], &model) {
            ("model", _) => None,
            ("asset", None) => Some(format!("caja: monta antes «{}»", child.unwrap_or("?"))),
            ("asset", Some(_)) => None,
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
                // A reused piece or a built asset keeps its size unless it has
                // no model to keep.
                fit: !is_model
                    && (!matches!(&p.source.kind[..], "reuse" | "asset") || model.is_none()),
                size_m: if p.source.kind == "reuse" {
                    piece.and_then(|c| c.size_m).unwrap_or(size)
                } else if p.source.kind == "asset" {
                    child_size.unwrap_or(size)
                } else {
                    size
                },
                at: p.at.get(i).copied().unwrap_or([f64::NAN; 3]),
                turn: p.turn.get(i).copied().unwrap_or([0.0; 3]),
                only_in: p.only_in.clone(),
                moves: p.moves.clone(),
                todo: todo.clone(),
                // an asset is a closed box: it comes with its own materials
                material: p
                    .material
                    .as_deref()
                    .filter(|_| child.is_none())
                    .and_then(find_paint),
                zones: if child.is_none() {
                    zones.clone()
                } else {
                    BTreeMap::new()
                },
                split: split.clone(),
                slots: slots.clone(),
                shapes: p.shapes.clone(),
            });
        }
    }
    lay_out(&mut items);
    let uv_per_m = items
        .iter()
        .any(|i| !i.shapes.is_empty())
        .then(|| config.uv_per_m());
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
        anchors: spec
            .anchors
            .iter()
            .filter(|a| crate::spec::plain_id(&a.id))
            .map(|a| PlanAnchor {
                name: format!("anchor_{}", a.id),
                at: a.at,
                turn: a.turn,
            })
            .collect(),
        uv_per_m,
    }
}

/// Marks each built asset whose children changed after it was assembled: a
/// child assembled later, not assembled, or to be assembled again itself.
fn mark_stale(
    root: &Path,
    config: &Config,
    specs: &[Spec],
    reports: &mut BTreeMap<String, Report>,
) {
    let built = |id: &str| {
        std::fs::metadata(out_dir(root, config, id).join(format!("{id}.glb")))
            .and_then(|m| m.modified())
            .ok()
    };
    // children first, so a stale child makes its parents stale in one pass
    let mut ids: Vec<String> = Vec::new();
    for s in specs {
        for id in crate::spec::order(specs, &s.asset.id).unwrap_or_default() {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    for id in ids {
        let (Some(s), Some(at)) = (specs.iter().find(|s| s.asset.id == id), built(&id)) else {
            continue;
        };
        let stale: Vec<String> = crate::spec::made_of(s)
            .into_iter()
            .filter(|c| {
                built(c).is_none_or(|t| t > at)
                    || reports
                        .get(*c)
                        .is_some_and(|r| !r.stale.is_empty() || !r.changed.is_empty())
            })
            .map(String::from)
            .collect();
        if let Some(r) = reports.get_mut(&id) {
            r.stale = stale;
        }
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
    let mut out = specs
        .iter()
        .filter_map(|s| {
            let dir = out_dir(root, config, &s.asset.id);
            let text = std::fs::read_to_string(dir.join("report.json")).ok()?;
            let mut r: Report = serde_json::from_str(&text).ok()?;
            r.glb = rel(root, &dir.join(format!("{}.glb", s.asset.id)));
            // a piece retouched after the build (a `.blend` saved again)
            r.changed = std::fs::read_to_string(dir.join("asset.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .map(|c| crate::asset::changed_pieces(root, &c))
                .unwrap_or_default();
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
        .collect::<BTreeMap<_, _>>();
    mark_stale(root, config, specs, &mut out);
    out
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
            shapes: Vec::new(),
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
            pieces: vec![
                piece("kit/beam", &[]),
                piece("kit/post", &[("wood", "oak")]),
            ],
            materials: vec![mat("oak"), mat("iron"), mat("pine")],
            specs: vec![],
            tree: None,
            builds: Default::default(),
            content: vec![],
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
            "beam/wood",
            "beam/bad",
            "flat/x",
            "post/wood",
            "post/metal",
            "ghost/x",
            "nope",
        ]
        .map(String::from)
        .to_vec();
        let key =
            |m: &BTreeMap<String, Option<Paint>>, n: &str| m[n].as_ref().map(|p| p.key.clone());
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
    fn a_model_part_carries_its_shapes_and_no_todo() {
        let config = Config::parse("[project]\nname = \"t\"\n[assets]\nuv_per_m = 0.5\n").unwrap();
        let spec = crate::spec::parse(
            "[asset]\nid = \"m\"\nname = \"M\"\n[[parts]]\nid = \"p\"\nwhat = \"x\"\nmaterials = { wood = \"oak\" }\n[parts.source]\nkind = \"model\"\n[[parts.shapes]]\nzone = \"wood\"\nbox = [[0.0, 0.0, 0.0], [1.0, 0.5, 0.25]]\n",
        )
        .unwrap();
        let p = plan(Path::new("/p"), &config, &cat(), &spec);
        let it = &p.items[0];
        assert_eq!(it.shapes.len(), 1);
        assert!(it.model.is_none() && !it.fit && it.todo.is_none());
        assert_eq!(it.size_m, [1.0, 0.5, 0.25]);
        assert_eq!(it.zones["wood"].key, "oak");
        assert_eq!(p.uv_per_m, Some(0.5));
    }

    /// R6 holds at build time too: Blender never gets shapes `kiln check` refuses.
    #[test]
    fn build_refuses_shapes_outside_the_limits() {
        let part = |kind: &str, shape: &str| {
            crate::spec::parse(&format!(
                "[asset]\nid = \"m\"\nname = \"M\"\n[[parts]]\nid = \"p\"\nwhat = \"x\"\n[parts.source]\nkind = \"{kind}\"\nfrom = \"k/p\"\n[[parts.shapes]]\nzone = \"wood\"\n{shape}\n"
            ))
            .unwrap()
        };
        let good = "box = [[0.0, 0.0, 0.0], [1.0, 0.5, 0.25]]";
        let two = "tube = { from = [0, 0, 0], to = [0, 1, 0], radius = 0.1, segments = 2 }";
        assert!(refuse(&part("model", good)).is_ok());
        for (kind, shape) in [("model", two), ("reuse", good)] {
            let e = refuse(&part(kind, shape)).unwrap_err();
            assert!(e.contains("«p»"), "{kind}: {e}");
        }
    }

    #[test]
    fn a_plan_without_model_parts_serializes_no_shapes_or_uv() {
        let config = Config::parse("[project]\nname = \"t\"\n[assets]\nuv_per_m = 0.5\n").unwrap();
        let spec = crate::spec::parse(SPEC).unwrap();
        let p = plan(Path::new("/p"), &config, &cat(), &spec);
        let text = serde_json::to_string(&p).unwrap();
        assert!(!text.contains("shapes") && !text.contains("uv_per_m"), "{text}");
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

    #[test]
    fn an_asset_part_uses_the_built_child_and_goes_stale_after_it() {
        let root = std::env::temp_dir().join(format!("kiln-nest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = Config::parse("[project]\nname = \"t\"\n").unwrap();
        let ship = crate::spec::parse(
            "[asset]\nid = \"ship\"\nname = \"S\"\n[[parts]]\nid = \"engine\"\nwhat = \"m\"\ncount = 2\nmaterial = \"oak\"\nat = [[-1, 0, 0], [1, 0, 0]]\n[parts.source]\nkind = \"asset\"\nfrom = \"engine\"\nvariant = \"old\"\n",
        )
        .unwrap();
        let engine = crate::spec::parse(
            "[asset]\nid = \"engine\"\nname = \"E\"\n[[parts]]\nid = \"b\"\nwhat = \"b\"\n[parts.source]\nkind = \"reuse\"\nfrom = \"kit/beam\"\n[[variants]]\nid = \"old\"\n",
        )
        .unwrap();
        let mut cat = cat();
        // not built yet: a box that says what to assemble first
        let p = plan(&root, &config, &cat, &ship);
        assert_eq!(
            p.items[0].todo.as_deref(),
            Some("caja: monta antes «engine»")
        );
        assert!(p.items[0].material.is_none());

        let write = |id: &str, name: &str, size: &str| {
            let d = out_dir(&root, &config, id);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(name), b"glb").unwrap();
            std::fs::write(
                d.join("report.json"),
                format!("{{\"size_m\": {size}, \"tris\": 120}}"),
            )
            .unwrap();
        };
        write("engine", "engine.glb", "[1.0, 2.0, 3.0]");
        write("engine", "engine--old.glb", "[1.0, 2.0, 3.0]");
        let specs = vec![ship.clone(), engine];
        cat.builds = reports(&root, &config, &specs);
        let p = plan(&root, &config, &cat, &ship);
        let it = &p.items[1];
        assert!(it.model.as_deref().unwrap().ends_with("engine--old.glb"));
        assert!(!it.fit && it.todo.is_none());
        assert_eq!(it.size_m, [1.0, 2.0, 3.0]);
        assert_eq!(it.at, [1.0, 0.0, 0.0]);

        std::thread::sleep(std::time::Duration::from_millis(20));
        write("ship", "ship.glb", "[3.0, 2.0, 3.0]");
        assert!(reports(&root, &config, &specs)["ship"].stale.is_empty());
        std::thread::sleep(std::time::Duration::from_millis(20));
        write("engine", "engine.glb", "[1.0, 2.0, 3.0]");
        assert_eq!(reports(&root, &config, &specs)["ship"].stale, ["engine"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
