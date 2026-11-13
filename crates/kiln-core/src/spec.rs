//! The asset spec (`<specs>/<id>.toml`): what an asset is in the game, the
//! parts it breaks into, where each part's model comes from and which
//! material of the palette each one takes. The agent writes it; Kiln reads
//! it, checks it against the catalog and draws it as a graph.

use crate::catalog::{Catalog, Finding, Severity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub asset: Asset,
    #[serde(default)]
    pub parts: Vec<Part>,
    /// Looks of the asset: only the zones that change from the base.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    /// Relative path of the file it was read from (set by Kiln).
    #[serde(default, skip_deserializing)]
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    /// The request as the person wrote it.
    #[serde(default)]
    pub asked: String,
    /// Systems of the game that use it (required when `rules.require_use`).
    #[serde(default)]
    pub used_by: Vec<String>,
    #[serde(default, rename = "where")]
    pub where_: String,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub budget_tris: Option<u64>,
    /// brief | inventory | approved | built | checked | done
    #[serde(default = "brief")]
    pub status: String,
    /// What the agent says to the person: its reading of the request and its doubts.
    #[serde(default)]
    pub notes: String,
    /// The level of the tree it sits at (`asset` when left out).
    #[serde(default)]
    pub level: Option<String>,
    /// For a spec above the asset level: its children, as ids or `level:id`.
    #[serde(default)]
    pub children: Vec<String>,
}

fn brief() -> String {
    "brief".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// `part/zone` -> palette key.
    #[serde(default)]
    pub materials: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub what: String,
    #[serde(default)]
    pub size_m: Option<[f64; 3]>,
    #[serde(default)]
    pub count: Option<u32>,
    /// A key of the palette (catalog material).
    #[serde(default)]
    pub material: Option<String>,
    /// Zone (material slot of the piece) -> palette key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub materials: BTreeMap<String, String>,
    #[serde(default)]
    pub moves: Option<String>,
    #[serde(default)]
    pub only_in: Vec<String>,
    pub source: Source,
    /// Where each copy stands, in metres from the asset's origin (the centre
    /// of its footprint on the ground, Y up): the centre of the copy's base.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at: Vec<[f64; 3]>,
    /// How each copy is turned, degrees about X, Y and Z.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub turn: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// reuse | variant | search | retouch
    pub kind: String,
    /// The catalog key of the piece it reuses or varies (`kit/piece`).
    #[serde(default)]
    pub from: Option<String>,
    /// The search words, for `search`.
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
}

pub const KINDS: [&str; 4] = ["reuse", "variant", "search", "retouch"];

pub fn parse(text: &str) -> Result<Spec, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Every spec in the folder, with a finding for each file that does not read.
pub fn load_all(
    root: &Path,
    dir: &str,
    rules: &crate::config::Rules,
    cat: &mut Catalog,
) -> Vec<Spec> {
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(root.join(dir))
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    for f in files
        .into_iter()
        .filter(|f| f.extension().is_some_and(|e| e == "toml"))
    {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        match std::fs::read_to_string(&f)
            .map_err(|e| e.to_string())
            .and_then(|t| parse(&t))
        {
            Ok(mut s) => {
                s.file = rel;
                check(&s, rules, cat);
                out.push(s);
            }
            Err(e) => cat.findings.push(Finding {
                severity: Severity::Error,
                subject: rel,
                code: "spec_unreadable".into(),
                message: format!("ficha ilegible: {e}"),
            }),
        }
    }
    out
}

/// The checks of a spec against the catalog.
pub fn check(s: &Spec, rules: &crate::config::Rules, cat: &mut Catalog) {
    let id = format!("asset:{}", s.asset.id);
    let mut out = Vec::new();
    let mut push = |sev, subject: String, code: &str, msg: String| {
        out.push(Finding {
            severity: sev,
            subject,
            code: code.into(),
            message: msg,
        })
    };
    if rules.require_use && s.asset.used_by.is_empty() {
        push(
            Severity::Warn,
            id.clone(),
            "no_use",
            "el proyecto pide que cada asset tenga un sistema que lo use y este no tiene ninguno"
                .into(),
        );
    }
    if s.parts.is_empty() {
        push(
            Severity::Error,
            id.clone(),
            "no_parts",
            "la ficha no tiene piezas".into(),
        );
    }
    let mut parts_tris = 0u64;
    for p in &s.parts {
        let subject = format!("part:{}/{}", s.asset.id, p.id);
        if !KINDS.contains(&p.source.kind.as_str()) {
            push(
                Severity::Error,
                subject.clone(),
                "source_kind",
                format!(
                    "fuente «{}» desconocida ({})",
                    p.source.kind,
                    KINDS.join(", ")
                ),
            );
        }
        match (&p.source.kind[..], &p.source.from) {
            ("reuse" | "variant" | "retouch", Some(from)) => {
                match cat.pieces.iter().find(|c| &c.key == from) {
                    Some(c) => parts_tris += c.tris.unwrap_or(0) * p.count.unwrap_or(1) as u64,
                    None => push(
                        Severity::Error,
                        subject.clone(),
                        "from_missing",
                        format!("{from} no está en el catálogo"),
                    ),
                }
            }
            ("reuse" | "variant", None) => push(
                Severity::Error,
                subject.clone(),
                "from_missing",
                "reutilizar o variar pide `from` con una pieza del catálogo".into(),
            ),
            ("search", _) if p.source.query.is_none() => push(
                Severity::Warn,
                subject.clone(),
                "query_missing",
                "buscar sin palabras de búsqueda".into(),
            ),
            _ => {}
        }
        match &p.material {
            Some(m) if !cat.materials.iter().any(|x| &x.key == m) => push(
                Severity::Warn,
                subject.clone(),
                "material_missing",
                format!("el material «{m}» no está en la paleta"),
            ),
            None if p.source.kind != "reuse" && p.materials.is_empty() => push(
                Severity::Warn,
                subject.clone(),
                "material_unset",
                "pieza sin material de la paleta".into(),
            ),
            _ => {}
        }
        for m in p.materials.values() {
            if !cat.materials.iter().any(|x| &x.key == m) {
                push(
                    Severity::Warn,
                    subject.clone(),
                    "material_missing",
                    format!("el material «{m}» no está en la paleta"),
                );
            }
        }
        let slots = p
            .source
            .from
            .as_ref()
            .and_then(|f| cat.pieces.iter().find(|c| &c.key == f))
            // a split piece gets its zones in Blender; the catalog cannot name them
            .filter(|c| c.split.is_none())
            .map(|c| &c.material_slots)
            .filter(|sl| !sl.is_empty());
        if let Some(slots) = slots {
            for zone in p.materials.keys().filter(|z| !slots.contains(z)) {
                push(
                    Severity::Warn,
                    subject.clone(),
                    "zone_missing",
                    format!("la pieza no tiene la zona «{zone}»"),
                );
            }
        }
        let n = p.count.unwrap_or(1) as usize;
        if !p.at.is_empty() && p.at.len() != n {
            push(
                Severity::Warn,
                subject.clone(),
                "layout_count",
                format!("{} posiciones para {n} copias", p.at.len()),
            );
        }
        for st in &p.only_in {
            if !s.asset.states.contains(st) {
                push(
                    Severity::Error,
                    subject.clone(),
                    "state_unknown",
                    format!("estado «{st}» que el asset no declara"),
                );
            }
        }
        if p.source.kind == "search" {
            push(
                Severity::Info,
                subject.clone(),
                "needs_approval",
                "espera tu permiso para buscar y bajar".into(),
            );
        }
    }
    for (i, v) in s.variants.iter().enumerate() {
        if !plain_id(&v.id) {
            push(
                Severity::Error,
                id.clone(),
                "variant_id",
                format!(
                    "«{}» no vale como nombre de variante: solo letras, números, guiones y guiones bajos",
                    v.id
                ),
            );
        }
        if s.variants[..i].iter().any(|o| o.id == v.id) {
            push(
                Severity::Error,
                id.clone(),
                "variant_duplicate",
                format!("la variante «{}» está repetida", v.id),
            );
        }
        for (key, m) in &v.materials {
            let part = key.split('/').next().unwrap_or("");
            if !s.parts.iter().any(|p| p.id == part) {
                push(
                    Severity::Warn,
                    id.clone(),
                    "variant_part",
                    format!(
                        "la variante «{}» nombra la pieza «{part}», que la ficha no tiene",
                        v.id
                    ),
                );
            }
            if !cat.materials.iter().any(|x| &x.key == m) {
                push(
                    Severity::Warn,
                    id.clone(),
                    "material_missing",
                    format!("el material «{m}» no está en la paleta"),
                );
            }
        }
    }
    if let Some(b) = s.asset.budget_tris
        && parts_tris > b
    {
        push(
            Severity::Warn,
            id.clone(),
            "budget",
            format!("las piezas reutilizadas ya suman {parts_tris} triángulos, por encima de {b}"),
        );
    }
    cat.findings.extend(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_hoist_example() {
        let s = parse(include_str!("../../../examples/specs/hoist.toml")).unwrap();
        assert_eq!(s.asset.id, "hoist");
        assert!(s.parts.iter().any(|p| p.source.kind == "reuse"));
    }

    const ZONES: &str = r#"
[asset]
id = "a"
name = "A"
[[parts]]
id = "beam"
what = "viga"
materials = { wood = "oak", iron = "iron" }
[parts.source]
kind = "reuse"
from = "kit/beam"
[[variants]]
id = "old"
materials = { "beam/wood" = "pine", "ghost/wood" = "oak" }
"#;

    #[test]
    fn reads_zone_materials_and_variants() {
        let s = parse(ZONES).unwrap();
        assert_eq!(s.parts[0].materials["wood"], "oak");
        assert_eq!(s.variants.len(), 1);
        assert_eq!(s.variants[0].materials["beam/wood"], "pine");
    }

    #[test]
    fn checks_zones_and_variants() {
        let piece = crate::catalog::Piece {
            key: "kit/beam".into(),
            kit: "kit".into(),
            name: "beam".into(),
            file: None,
            format: "glb".into(),
            size_m: None,
            min: None,
            tris: None,
            material_slots: vec!["wood".into()],
            split: None,
            images: None,
            licenses: vec![],
            credit: None,
            manifest: Default::default(),
            measured_by: None,
            preview: None,
            bindings: Default::default(),
        };
        let mat = |k: &str| crate::catalog::Material {
            key: k.into(),
            name: k.into(),
            source: "table".into(),
            file: String::new(),
            detail: Default::default(),
            color: None,
            albedo: None,
        };
        let mut cat = Catalog {
            project: String::new(),
            root: String::new(),
            kits: vec![],
            pieces: vec![piece],
            materials: vec![mat("oak"), mat("iron")],
            specs: vec![],
            tree: None,
            builds: Default::default(),
            findings: vec![],
        };
        check(&parse(ZONES).unwrap(), &Default::default(), &mut cat);
        let has = |c: &str| cat.findings.iter().any(|f| f.code == c);
        assert!(has("zone_missing"));
        assert!(has("material_missing"));
        assert!(has("variant_part"));
        assert!(!has("material_unset"));
        assert!(!has("variant_duplicate"));
    }
}

/// A name that is safe in a file name: a variant's id ends up in its GLB's.
pub fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The spec's TOML with one part's source changed (a piece brought in from a
/// site turns a `search` into a `reuse` or a `variant`). The rest of the file
/// keeps its values; comments are lost.
pub fn set_source(
    text: &str,
    part: &str,
    kind: &str,
    from: &str,
    why: &str,
) -> Result<String, String> {
    let mut doc: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let p = part_table(&mut doc, part)?;
    let src = p
        .entry("source")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or("source no es una tabla")?;
    src.insert("kind".into(), kind.into());
    src.insert("from".into(), from.into());
    src.insert("why".into(), why.into());
    toml::to_string(&doc).map_err(|e| e.to_string())
}

fn part_table<'a>(doc: &'a mut toml::Table, part: &str) -> Result<&'a mut toml::Table, String> {
    doc.get_mut("parts")
        .and_then(|v| v.as_array_mut())
        .and_then(|a| {
            a.iter_mut()
                .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(part))
        })
        .and_then(|p| p.as_table_mut())
        .ok_or(format!("la ficha no tiene la pieza «{part}»"))
}

/// The spec's TOML with one zone's material set (empty `key` removes it): on
/// the part, or on the named variant as `part/zone`. Comments are lost.
pub fn set_material(
    text: &str,
    part: &str,
    zone: &str,
    key: &str,
    variant: Option<&str>,
) -> Result<String, String> {
    if zone.is_empty() {
        return Err("falta la zona".into());
    }
    let mut doc: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let p = part_table(&mut doc, part)?;
    if let Some(v) = variant.filter(|v| !plain_id(v)) {
        return Err(format!(
            "«{v}» no vale como nombre de variante: solo letras, números, guiones y guiones bajos"
        ));
    }
    let Some(vid) = variant else {
        edit_materials(p, zone, key)?;
        if p.get("materials").and_then(|m| m.as_table()).is_some_and(|m| m.is_empty()) {
            p.remove("materials");
        }
        return toml::to_string(&doc).map_err(|e| e.to_string());
    };
    let list = doc
        .entry("variants")
        .or_insert_with(|| toml::Value::Array(vec![]))
        .as_array_mut()
        .ok_or("variants no es una lista")?;
    let at = match list.iter().position(|v| v.get("id").and_then(|i| i.as_str()) == Some(vid)) {
        Some(i) => i,
        None => {
            let mut t = toml::Table::new();
            t.insert("id".into(), vid.into());
            list.push(toml::Value::Table(t));
            list.len() - 1
        }
    };
    let v = list[at].as_table_mut().ok_or("variante no es una tabla")?;
    edit_materials(v, &format!("{part}/{zone}"), key)?;
    toml::to_string(&doc).map_err(|e| e.to_string())
}

fn edit_materials(t: &mut toml::Table, zone: &str, key: &str) -> Result<(), String> {
    let m = t
        .entry("materials")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or("materials no es una tabla")?;
    if key.is_empty() {
        m.remove(zone);
    } else {
        m.insert(zone.into(), key.into());
    }
    Ok(())
}

/// The spec's TOML without the part's `materials` (its choices now live in
/// the catalog).
pub fn clear_materials(text: &str, part: &str) -> Result<String, String> {
    let mut doc: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    part_table(&mut doc, part)?.remove("materials");
    toml::to_string(&doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod source_tests {
    #[test]
    fn a_search_becomes_a_reuse() {
        let text = "[asset]\nid = \"a\"\nname = \"A\"\n\n[[parts]]\nid = \"block\"\nwhat = \"polea\"\nsize_m = [0.2, 0.3, 0.1]\n[parts.source]\nkind = \"search\"\nquery = \"wooden pulley\"\n";
        let out =
            super::set_source(text, "block", "variant", "sketchfab/pulley", "traída").unwrap();
        let s = super::parse(&out).unwrap();
        assert_eq!(s.parts[0].source.kind, "variant");
        assert_eq!(s.parts[0].source.from.as_deref(), Some("sketchfab/pulley"));
        assert_eq!(s.parts[0].source.query.as_deref(), Some("wooden pulley"));
        assert!(super::set_source(text, "nope", "reuse", "x", "").is_err());
    }

    const BASE: &str = "[asset]\nid = \"a\"\nname = \"A\"\n\n[[parts]]\nid = \"block\"\nwhat = \"polea\"\nsize_m = [0.2, 0.3, 0.1]\n[parts.source]\nkind = \"search\"\nquery = \"q\"\n";

    #[test]
    fn sets_and_removes_zone_materials() {
        let t = super::set_material(BASE, "block", "wood", "oak", None).unwrap();
        let t = super::set_material(&t, "block", "iron", "iron", None).unwrap();
        let s = super::parse(&t).unwrap();
        assert_eq!(s.parts[0].materials["wood"], "oak");
        assert_eq!(s.parts[0].materials["iron"], "iron");
        let t = super::set_material(&t, "block", "iron", "", None).unwrap();
        assert_eq!(super::parse(&t).unwrap().parts[0].materials.len(), 1);
        let t = super::set_material(&t, "block", "wood", "", None).unwrap();
        assert!(!t.contains("materials"));
        assert!(super::set_material(BASE, "nope", "wood", "oak", None).is_err());
        assert_eq!(
            super::set_material(BASE, "block", "", "oak", None).unwrap_err(),
            "falta la zona"
        );
    }

    #[test]
    fn a_variant_id_that_is_not_a_plain_name_is_refused() {
        for bad in ["a/../../x", "..", "old one", "", "ñu"] {
            assert!(
                super::set_material(BASE, "block", "wood", "pine", Some(bad)).is_err(),
                "{bad}"
            );
            assert!(!super::plain_id(bad), "{bad}");
        }
        assert!(super::plain_id("old-2_b"));
    }

    #[test]
    fn sets_and_removes_variant_materials() {
        let t = super::set_material(BASE, "block", "wood", "pine", Some("winter")).unwrap();
        let s = super::parse(&t).unwrap();
        assert_eq!(s.variants[0].id, "winter");
        assert_eq!(s.variants[0].materials["block/wood"], "pine");
        let t = super::set_material(&t, "block", "wood", "", Some("winter")).unwrap();
        let s = super::parse(&t).unwrap();
        assert_eq!(s.variants.len(), 1);
        assert!(s.variants[0].materials.is_empty());
        assert!(super::set_material(BASE, "nope", "wood", "oak", Some("w")).is_err());
    }

    #[test]
    fn clears_a_parts_materials() {
        let t = super::set_material(BASE, "block", "wood", "oak", None).unwrap();
        let t = super::clear_materials(&t, "block").unwrap();
        assert!(super::parse(&t).unwrap().parts[0].materials.is_empty());
        assert!(super::clear_materials(BASE, "nope").is_err());
    }
}
