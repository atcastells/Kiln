//! Asset sites Kiln can search for the pieces a spec marks as `search`:
//! Poly Haven, Poly Pizza and Sketchfab. This module only reads what the
//! sites answer and decides what may be shown and brought in; the CLI does
//! the network calls. Every piece brought in keeps where it came from, its
//! author and its licence in the `sources.json` of its kit.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SITES: [&str; 3] = ["polyhaven", "polypizza", "sketchfab"];

/// One model a site offers for a search.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub site: String,
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(default)]
    pub author_url: String,
    /// Normalized label (CC0, CC-BY, CC-BY-SA...), empty when unreadable.
    pub license: String,
    /// The site's own wording of it.
    #[serde(default)]
    pub license_label: String,
    pub page_url: String,
    #[serde(default)]
    pub thumb: String,
    #[serde(default)]
    pub tris: Option<u64>,
    /// Size in metres (w, h, d) when the site says it.
    #[serde(default)]
    pub size_m: Option<[f64; 3]>,
    /// A direct file to download, when the site gives one without a key.
    #[serde(default)]
    pub download: Option<String>,
}

impl Candidate {
    /// The attribution line for the game's credits and the kit's LICENSE.md.
    pub fn credit(&self) -> String {
        let label = if self.license_label.is_empty() || self.license_label == self.license {
            self.license.clone()
        } else {
            format!("{} ({})", self.license_label, self.license)
        };
        format!(
            "\"{}\" by {}, {label}, {}",
            self.name, self.author, self.page_url
        )
    }

    /// The folder name of the piece in its site's kit.
    pub fn slug(&self) -> String {
        slug(&self.name, &self.id)
    }
}

/// `name` in lower case with `_` between words, plus a short id when the
/// name says nothing (`model`, empty).
pub fn slug(name: &str, id: &str) -> String {
    let mut s = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('_') && !s.is_empty() {
            s.push('_');
        }
    }
    let s = s.trim_end_matches('_').to_string();
    let short: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    match s.as_str() {
        "" | "model" | "untitled" => format!("model_{}", short.to_lowercase()),
        _ => s
            .chars()
            .take(48)
            .collect::<String>()
            .trim_end_matches('_')
            .to_string(),
    }
}

/// A site's licence wording as a Kiln label; empty when it is none we know
/// (paid, "standard", editorial...).
pub fn normalize_license(label: &str) -> String {
    let l = label.to_lowercase();
    if l.contains("cc0") || l.contains("public domain") {
        return "CC0".into();
    }
    let long = l.contains("attribution") || l.contains("cc-by") || l.contains("cc by");
    if !long {
        return String::new();
    }
    let nc = l.contains("noncommercial") || l.contains("non-commercial") || l.contains("-nc");
    let nd = l.contains("noderiv") || l.contains("-nd");
    let sa = l.contains("sharealike") || l.contains("share-alike") || l.contains("-sa");
    let mut out = String::from("CC-BY");
    for (on, tag) in [(nc, "-NC"), (nd, "-ND"), (sa, "-SA")] {
        if on {
            out.push_str(tag);
        }
    }
    out
}

/// Keeps the candidates whose licence the project admits; returns how many it dropped.
pub fn admitted(cands: Vec<Candidate>, licenses: &[String]) -> (Vec<Candidate>, usize) {
    let n = cands.len();
    let keep: Vec<Candidate> = cands
        .into_iter()
        .filter(|c| !c.license.is_empty() && licenses.contains(&c.license))
        .collect();
    let dropped = n - keep.len();
    (keep, dropped)
}

fn s(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Poly Haven lists every model at once (`/assets?t=models`): the match is
/// local, on name, tags and categories. All of it is CC0.
pub fn parse_polyhaven(list: &Value, query: &str, max: usize) -> Vec<Candidate> {
    let words: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(str::to_string)
        .collect();
    let assets: Vec<(&String, &Value, String, String)> = list
        .as_object()
        .into_iter()
        .flatten()
        .map(|(id, a)| {
            let name = format!("{} {}", id.replace('_', " "), s(a, "name")).to_lowercase();
            let mut hay = name.clone();
            for k in ["tags", "categories"] {
                for t in a.get(k).and_then(Value::as_array).into_iter().flatten() {
                    hay.push(' ');
                    hay.push_str(&t.as_str().unwrap_or_default().to_lowercase());
                }
            }
            (id, a, hay, name)
        })
        .collect();
    // a word or its singular (pulleys -> pulley)
    let has = |hay: &str, w: &str| hay.contains(w) || hay.contains(w.trim_end_matches('s'));
    // A word most models carry ("wooden") says little: each word weighs by
    // how rare it is, and a model must match one that is not common.
    let n = assets.len().max(1) as f64;
    let weight: Vec<f64> = words
        .iter()
        .map(|w| {
            let df = assets
                .iter()
                .filter(|(_, _, h, _)| has(h, w))
                .count()
                .max(1) as f64;
            (n / df).ln()
        })
        .collect();
    // common: in more than one model of eight
    let rare = n.min(8.0).ln() - 1e-9;
    let mut scored: Vec<(f64, Candidate)> = Vec::new();
    for (id, a, hay, name) in &assets {
        let matched: Vec<(&String, f64)> = words
            .iter()
            .zip(weight.iter().copied())
            .filter(|(w, _)| has(hay, w))
            .collect();
        // every telling word of the query (all but one when it has three or more)
        let telling = weight.iter().filter(|wt| **wt >= rare).count();
        let hits = matched.iter().filter(|(_, wt)| *wt >= rare).count();
        if hits == 0 || hits + usize::from(telling >= 3) < telling {
            continue;
        }
        // a word of the name counts double
        let score = matched
            .iter()
            .map(|(w, wt)| if has(name, w) { 2.0 * wt } else { *wt })
            .sum();
        scored.push((score, polyhaven_candidate(id, a)));
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    scored.into_iter().take(max).map(|(_, c)| c).collect()
}

/// One Poly Haven model (an entry of `/assets`, or `/info/<id>`).
pub fn polyhaven_candidate(id: &str, a: &Value) -> Candidate {
    let author = a
        .get("authors")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let size_m = a.get("dimensions").and_then(Value::as_array).and_then(|d| {
        let mm: Vec<f64> = d.iter().filter_map(Value::as_f64).collect();
        // Poly Haven gives millimetres, x (width), y (depth), z (height)
        (mm.len() == 3).then(|| [mm[0], mm[2], mm[1]].map(|v| v.round() / 1000.0))
    });
    Candidate {
        site: "polyhaven".into(),
        id: id.into(),
        name: s(a, "name"),
        author,
        author_url: String::new(),
        license: "CC0".into(),
        license_label: "CC0".into(),
        page_url: format!("https://polyhaven.com/a/{id}"),
        thumb: format!("https://cdn.polyhaven.com/asset_img/thumbs/{id}.png?width=256"),
        tris: a.get("polycount").and_then(Value::as_u64),
        size_m,
        download: None,
    }
}

/// The glTF of a Poly Haven model at `res` (`1k`, `2k`...): the main file
/// and the textures it pulls in, as (relative path, url).
pub fn polyhaven_files(files: &Value, res: &str) -> Option<Vec<(String, String)>> {
    let g = files.get("gltf")?.get(res)?.get("gltf")?;
    let url = g.get("url")?.as_str()?;
    let name = url.rsplit('/').next()?.to_string();
    let mut out = vec![(name, url.to_string())];
    for (rel, f) in g
        .get("include")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some(u) = f.get("url").and_then(Value::as_str) {
            out.push((rel.clone(), u.to_string()));
        }
    }
    Some(out)
}

/// Poly Pizza search (`/v1/search/<q>`): low poly GLBs, CC0 or CC-BY.
pub fn parse_polypizza(v: &Value) -> Vec<Candidate> {
    v.get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|r| {
            let id = s(r, "ID");
            let creator = r.get("Creator").cloned().unwrap_or_default();
            let label = s(r, "Licence");
            Candidate {
                site: "polypizza".into(),
                name: s(r, "Title"),
                author: s(&creator, "Username"),
                author_url: s(&creator, "PURL"),
                license: normalize_license(&label),
                license_label: label,
                page_url: format!("https://poly.pizza/m/{id}"),
                thumb: s(r, "Thumbnail"),
                tris: r.get("TriangleCount").and_then(Value::as_u64),
                size_m: None,
                download: Some(s(r, "Download")).filter(|d| !d.is_empty()),
                id,
            }
        })
        .collect()
}

/// Sketchfab search (`/v3/search?type=models`); downloading needs the
/// person's token and goes through `/v3/models/<uid>/download`.
pub fn parse_sketchfab(v: &Value) -> Vec<Candidate> {
    v.get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|r| r.get("isDownloadable").and_then(Value::as_bool) != Some(false))
        .map(|r| {
            let user = r.get("user").cloned().unwrap_or_default();
            let label = r.get("license").map(|l| s(l, "label")).unwrap_or_default();
            let thumb = r
                .pointer("/thumbnails/images")
                .and_then(Value::as_array)
                .and_then(|imgs| {
                    imgs.iter()
                        .filter(|i| i.get("width").and_then(Value::as_u64).unwrap_or(0) >= 200)
                        .min_by_key(|i| i.get("width").and_then(Value::as_u64).unwrap_or(0))
                        .or(imgs.first())
                })
                .map(|i| s(i, "url"))
                .unwrap_or_default();
            let author = match s(&user, "displayName") {
                d if d.is_empty() => s(&user, "username"),
                d => d,
            };
            Candidate {
                site: "sketchfab".into(),
                id: s(r, "uid"),
                name: s(r, "name"),
                author,
                author_url: s(&user, "profileUrl"),
                license: normalize_license(&label),
                license_label: label,
                page_url: s(r, "viewerUrl"),
                thumb,
                tris: r.get("faceCount").and_then(Value::as_u64),
                size_m: None,
                download: None,
            }
        })
        .collect()
}

/// The Sketchfab search slugs for the licences the project admits.
pub fn sketchfab_licenses(licenses: &[String]) -> Vec<&'static str> {
    [
        ("CC0", "cc0"),
        ("CC-BY", "by"),
        ("CC-BY-SA", "by-sa"),
        ("CC-BY-ND", "by-nd"),
        ("CC-BY-NC", "by-nc"),
        ("CC-BY-NC-SA", "by-nc-sa"),
        ("CC-BY-NC-ND", "by-nc-nd"),
    ]
    .into_iter()
    .filter(|(l, _)| licenses.iter().any(|x| x == l))
    .map(|(_, s)| s)
    .collect()
}

/// What a kit records of each piece brought in (`<kit>/sources.json`,
/// one entry per piece; the catalog reads its `credit`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Origin {
    pub credit: String,
    pub site: String,
    pub id: String,
    pub page_url: String,
    pub author: String,
    pub license: String,
    pub fetched: String,
    pub file: String,
}

impl Origin {
    pub fn of(c: &Candidate, file: &str, fetched: &str) -> Origin {
        Origin {
            credit: c.credit(),
            site: c.site.clone(),
            id: c.id.clone(),
            page_url: c.page_url.clone(),
            author: c.author.clone(),
            license: c.license.clone(),
            fetched: fetched.into(),
            file: file.into(),
        }
    }
}

/// The LICENSE.md of a site's kit, from the origins of its pieces.
pub fn license_md(site: &str, origins: &[(&String, &Origin)]) -> String {
    let (title, note) = match site {
        "polyhaven" => (
            "Poly Haven",
            "Modelos de Poly Haven (https://polyhaven.com), CC0. Powered by Poly Haven.",
        ),
        "polypizza" => (
            "Poly Pizza",
            "Modelos de Poly Pizza (https://poly.pizza); cada uno con su licencia.",
        ),
        _ => (
            "Sketchfab",
            "Modelos de Sketchfab (https://sketchfab.com); cada uno con su licencia.",
        ),
    };
    let mut out = format!("# {title}\n\n{note}\n\nTraídos por Kiln. Crédito de cada pieza:\n\n");
    for (piece, o) in origins {
        out.push_str(&format!("- `{piece}`: {} — {}\n", o.credit, o.license));
    }
    out
}

/// CREDITS.md: every piece brought in that asks for credit, for the game.
pub fn credits_md(all: &[(String, String, Origin)]) -> String {
    let mut out = String::from(
        "# Créditos de assets\n\nGenerado por Kiln con las piezas traídas de sitios externos.\n\n",
    );
    let need: Vec<_> = all.iter().filter(|(_, _, o)| o.license != "CC0").collect();
    if need.is_empty() {
        out.push_str("Ninguna pieza traída pide crédito.\n");
    }
    for (kit, piece, o) in need {
        out.push_str(&format!("- {}  (`{kit}/{piece}`)\n", o.credit));
    }
    if all.iter().any(|(_, _, o)| o.site == "polyhaven") {
        out.push_str("\nPowered by Poly Haven (CC0, sin crédito obligatorio).\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn reads_the_three_sites() {
        let ph = parse_polyhaven(&fixture("polyhaven-assets.json"), "wooden crates", 5);
        assert_eq!(ph[0].id, "wooden_crate_01");
        assert_eq!(ph[0].license, "CC0");
        assert_eq!(ph[0].tris, Some(6576));
        let size = ph[0].size_m.unwrap();
        assert!((size[0] - 0.825).abs() < 1e-9 && (size[1] - 0.35).abs() < 1e-9);

        let pp = parse_polypizza(&fixture("polypizza-search.json"));
        assert_eq!(
            pp.iter().map(|c| c.license.as_str()).collect::<Vec<_>>(),
            ["CC0", "CC-BY"]
        );
        assert!(pp[0].download.as_deref().unwrap().ends_with(".glb"));

        let sf = parse_sketchfab(&fixture("sketchfab-search.json"));
        assert_eq!(sf.len(), 3);
        assert_eq!(sf[0].license, "CC-BY");
        assert!(sf[0].thumb.starts_with("https://media.sketchfab.com/"));
        assert!(
            sf[0]
                .credit()
                .contains("by chiragbhoir, CC Attribution (CC-BY)")
        );
    }

    #[test]
    fn only_admitted_licences_are_shown() {
        for (label, want) in [
            ("CC Attribution", "CC-BY"),
            ("CC Attribution-NonCommercial-ShareAlike", "CC-BY-NC-SA"),
            ("CC Attribution-NoDerivs", "CC-BY-ND"),
            ("CC0 Public Domain", "CC0"),
            ("CC-BY 3.0", "CC-BY"),
            ("Standard", ""),
            ("Free Standard", ""),
        ] {
            assert_eq!(normalize_license(label), want, "{label}");
        }
        let mk = |l: &str| Candidate {
            license: l.into(),
            ..Default::default()
        };
        let (keep, dropped) = admitted(
            vec![mk("CC0"), mk("CC-BY-NC"), mk(""), mk("CC-BY")],
            &["CC0".into(), "CC-BY".into()],
        );
        assert_eq!((keep.len(), dropped), (2, 2));
        assert_eq!(
            sketchfab_licenses(&["CC0".into(), "CC-BY".into()]),
            ["cc0", "by"]
        );
    }

    #[test]
    fn slugs_name_the_folder() {
        assert_eq!(slug("Wooden Ship's Pulley", "x"), "wooden_ship_s_pulley");
        assert_eq!(
            slug("Wooden Pulley Wheel: EOA:2022.91.2", "x"),
            "wooden_pulley_wheel_eoa_2022_91_2"
        );
        assert_eq!(slug("Model", "1ed7131ab949"), "model_1ed7131a");
    }
}
