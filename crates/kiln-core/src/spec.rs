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
pub fn load_all(root: &Path, dir: &str, cat: &mut Catalog) -> Vec<Spec> {
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
                check(&s, cat);
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
pub fn check(s: &Spec, cat: &mut Catalog) {
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
    if s.asset.used_by.is_empty() {
        push(
            Severity::Warn,
            id.clone(),
            "no_use",
            "ningún sistema del juego lo usa todavía".into(),
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
