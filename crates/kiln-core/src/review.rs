//! The visual review of an assembled asset: what the reviewer agent writes
//! after looking at the sheets Blender renders, and how its fixes go back
//! into the spec. A fix is in the spec's own terms (`at`, `turn`, `size_m`,
//! `count`, `material`), so Kiln applies it without asking the agent again.

use serde::{Deserialize, Serialize};

/// What the reviewer writes (`review.toml` of a round).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Review {
    /// What it sees, before judging.
    #[serde(default)]
    pub seen: String,
    /// pass | revise
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub scores: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// In compare mode: `A` or `B`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    /// A part id, or `asset`.
    #[serde(default)]
    pub part: String,
    #[serde(default)]
    pub criterion: String,
    /// blocker | major | minor
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub problem: String,
    #[serde(default)]
    pub evidence: String,
    /// The change to the part, in spec fields.
    #[serde(default)]
    pub fix: toml::Table,
}

pub fn parse(text: &str) -> Result<Review, String> {
    toml::from_str(text).map_err(|e| format!("revisión ilegible: {e}"))
}

/// The fields of a part a fix may change.
pub const FIX_FIELDS: [&str; 5] = ["at", "turn", "size_m", "count", "material"];

/// The spec with every fix it can apply: returns the new text, what it
/// applied and what it left for the person (another part, another field).
pub fn apply(
    spec: &str,
    findings: &[Finding],
) -> Result<(String, Vec<String>, Vec<String>), String> {
    let mut doc: toml::Table = toml::from_str(spec).map_err(|e| e.to_string())?;
    let (mut done, mut left) = (Vec::new(), Vec::new());
    for f in findings {
        if f.fix.is_empty() {
            left.push(format!(
                "{}: {} (sin corrección en la ficha)",
                f.part, f.problem
            ));
            continue;
        }
        let part = doc
            .get_mut("parts")
            .and_then(|v| v.as_array_mut())
            .and_then(|a| {
                a.iter_mut()
                    .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(f.part.as_str()))
            })
            .and_then(|p| p.as_table_mut());
        let Some(part) = part else {
            left.push(format!(
                "{}: no es una pieza de la ficha; {}",
                f.part, f.problem
            ));
            continue;
        };
        let mut changed = Vec::new();
        for (k, v) in &f.fix {
            if FIX_FIELDS.contains(&k.as_str()) && valid(k, v) {
                part.insert(k.clone(), v.clone());
                changed.push(k.as_str());
            } else {
                left.push(format!("{}: {k} = {v} no se aplica solo", f.part));
            }
        }
        if !changed.is_empty() {
            done.push(format!(
                "{}: {} ({})",
                f.part,
                changed.join(", "),
                f.problem
            ));
        }
    }
    let text = toml::to_string(&doc).map_err(|e| e.to_string())?;
    Ok((text, done, left))
}

/// A value of the right shape for the field: numbers where numbers go.
fn valid(key: &str, v: &toml::Value) -> bool {
    let num = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).is_some();
    let triple = |x: &toml::Value| {
        x.as_array()
            .is_some_and(|a| a.len() == 3 && a.iter().all(num))
    };
    match key {
        "size_m" => triple(v),
        "at" | "turn" => v
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.iter().all(triple)),
        "count" => v.as_integer().is_some_and(|n| n > 0),
        "material" => v.as_str().is_some_and(|s| !s.is_empty()),
        _ => false,
    }
}

/// One round of the loop, as the page shows it (`review.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub n: u32,
    /// The folder of the round, relative to the project.
    pub dir: String,
    pub review: Review,
    /// In compare mode, the round it was weighed against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<u32>,
    /// The round the comparison kept (itself or `against`).
    pub kept: u32,
    #[serde(default)]
    pub applied: Vec<String>,
    #[serde(default)]
    pub left: Vec<String>,
    #[serde(default)]
    pub tris: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub asset: String,
    pub rounds: Vec<Round>,
    /// The round left built and in the spec.
    pub best: u32,
    pub model: String,
    pub done_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "[asset]\nid = \"a\"\nname = \"A\"\n\n[[parts]]\nid = \"crossbeam\"\nwhat = \"viga\"\nsize_m = [2.0, 0.18, 0.29]\nat = [[0, 4.8, 0]]\n[parts.source]\nkind = \"reuse\"\nfrom = \"k/beam\"\n";

    #[test]
    fn fixes_in_spec_terms_are_applied() {
        let r = parse(
            r#"
seen = "Un pórtico de dos postes."
verdict = "revise"
scores = { reads = 3, proportion = 2 }

[[findings]]
part = "crossbeam"
severity = "blocker"
problem = "flota"
fix = { at = [[0, 4.5, 0]], size_m = [2.0, 0.2, 0.3] }

[[findings]]
part = "crossbeam"
problem = "falta un herraje"
fix = { add = "bracket" }

[[findings]]
part = "ghost"
problem = "no existe"
fix = { at = [[0, 0, 0]] }

[[findings]]
part = "crossbeam"
problem = "forma rara"
fix = { size_m = [1, 2] }
"#,
        )
        .unwrap();
        assert_eq!(r.scores["proportion"], 2.0);
        let (text, done, left) = apply(SPEC, &r.findings).unwrap();
        let s = crate::spec::parse(&text).unwrap();
        assert_eq!(s.parts[0].at, vec![[0.0, 4.5, 0.0]]);
        assert_eq!(s.parts[0].size_m, Some([2.0, 0.2, 0.3]));
        assert_eq!(done.len(), 1);
        assert_eq!(left.len(), 3, "{left:?}");
    }
}
