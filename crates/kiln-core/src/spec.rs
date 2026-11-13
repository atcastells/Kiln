//! The asset spec (`<specs>/<id>.toml`): what an asset is in the game, the
//! parts it breaks into, where each part's model comes from and which
//! material of the palette each one takes. The agent writes it; Kiln reads
//! it, checks it against the catalog and draws it as a graph.

use crate::catalog::{Catalog, Finding, Severity};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub asset: Asset,
    #[serde(default)]
    pub parts: Vec<Part>,
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
            None if p.source.kind != "reuse" => push(
                Severity::Warn,
                subject.clone(),
                "material_unset",
                "pieza sin material de la paleta".into(),
            ),
            _ => {}
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
    let p = doc
        .get_mut("parts")
        .and_then(|v| v.as_array_mut())
        .and_then(|a| {
            a.iter_mut()
                .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(part))
        })
        .and_then(|p| p.as_table_mut())
        .ok_or(format!("la ficha no tiene la pieza «{part}»"))?;
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
}
