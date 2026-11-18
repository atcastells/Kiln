//! The UI target: which criteria are met, from the pinned claims `Cn ·` and the target file.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::catalog::{Finding, Severity};
use crate::config::{Level, UiSection};
use crate::context::Claim;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TargetFile {
    #[serde(default)]
    pub criteria: Vec<Row>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub id: String,
    pub marker: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OpenCriterion {
    pub id: String,
    pub status: String,
    pub marker: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progress {
    pub met: usize,
    pub total: usize,
    pub percent: u32,
    pub open: Vec<OpenCriterion>,
    pub problems: Vec<String>,
}

/// Where each theme variation is assigned: `file:line` of every line of a declared script or scene
/// that sets `theme_type_variation` to its name as a literal. A textual search that follows the
/// same containment rules as the UI rules: names that leave the project or cannot be read are skipped
/// (the rules report them as `ui-files`).
pub fn variation_uses(root: &std::path::Path, ui: &UiSection, names: &[&str]) -> BTreeMap<String, Vec<String>> {
    let mut uses: BTreeMap<String, Vec<String>> = names.iter().map(|n| (n.to_string(), Vec::new())).collect();
    let base = glob::Pattern::escape(&root.to_string_lossy());
    let mut files = BTreeSet::new();
    for g in ui.scripts.iter().chain(&ui.scenes).filter(|g| !super::names_outside(g)) {
        let Ok(paths) = glob::glob(&format!("{base}/{g}")) else { continue };
        for p in paths.flatten().filter(|p| p.is_file()) {
            if let Ok(rel) = p.strip_prefix(root) {
                files.insert(rel.display().to_string());
            }
        }
    }
    for rel in files {
        let read = super::inside(root, &rel).and_then(|real| std::fs::read_to_string(real).map_err(|e| e.to_string()));
        let Ok(text) = read else { continue };
        for (i, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with('#') || !code.contains("theme_type_variation") {
                continue;
            }
            for name in names {
                if code.contains(&format!("\"{name}\"")) || code.contains(&format!("'{name}'")) {
                    uses.get_mut(*name).unwrap().push(format!("{rel}:{}", i + 1));
                }
            }
        }
    }
    uses
}

pub fn load(path: &std::path::Path) -> Result<TargetFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The criterion id a claim starts with: an uppercase letter, digits, then " ·".
fn criterion_id(text: &str) -> Option<&str> {
    let (id, _) = text.split_once(" ·")?;
    let digits = id.strip_prefix(|c: char| c.is_ascii_uppercase())?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

fn number(id: &str) -> u64 {
    id[1..].parse().unwrap_or(u64::MAX)
}

enum Kiln {
    Met,
    Unmet,
    Off,
    Unknown,
}

/// Whether the rule behind a `kiln:*` marker has no Warn or Error finding. If the theme or the files
/// could not be read, the rules could not measure and nothing is met.
fn kiln_met(rule: &str, findings: &[Finding], ui: &UiSection) -> Kiln {
    let (level, codes): (Level, &[&str]) = match rule {
        "contrast" => (ui.contrast.level, &["ui-contrast", "ui-contrast-unbacked"]),
        "glyphs" => (ui.glyphs.level, &["ui-glyph"]),
        "theme_coherent" => (ui.theme_coherent.level, &["ui-variation-missing", "ui-variation-unused"]),
        "theme_only" => (ui.theme_only.level, &["ui-style-outside-theme"]),
        _ => return Kiln::Unknown,
    };
    if level == Level::Off {
        return Kiln::Off;
    }
    let blind = findings.iter().any(|f| matches!(f.code.as_str(), "ui-theme" | "ui-files") && f.severity != Severity::Info);
    let hit = findings.iter().any(|f| codes.contains(&f.code.as_str()) && f.severity != Severity::Info);
    if blind || hit { Kiln::Unmet } else { Kiln::Met }
}

pub fn evaluate(claims: &[Claim], target: &TargetFile, findings: &[Finding], ui: &UiSection) -> Progress {
    let ids: BTreeSet<&str> = claims.iter().filter(|c| c.pinned).filter_map(|c| criterion_id(&c.text)).collect();
    let rows: BTreeMap<&str, &Row> = target.criteria.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut problems = Vec::new();
    let mut open = Vec::new();
    for id in &ids {
        let pending = "pending".to_string();
        let (status, marker) = match rows.get(id) {
            None => {
                problems.push(format!("{id} sin fila en ui_target"));
                (pending, String::new())
            }
            Some(r) => {
                let status = if let Some(rule) = r.marker.strip_prefix("kiln:") {
                    match kiln_met(rule, findings, ui) {
                        Kiln::Met => "met".to_string(),
                        Kiln::Unmet => "unmet".to_string(),
                        Kiln::Off => {
                            problems.push(format!("{id}: la regla {rule} está apagada"));
                            pending
                        }
                        Kiln::Unknown => {
                            problems.push(format!("{id}: la regla kiln:{rule} no existe"));
                            pending
                        }
                    }
                } else if matches!(r.marker.as_str(), "po" | "test") {
                    if matches!(r.status.as_str(), "met" | "unmet" | "pending") {
                        r.status.clone()
                    } else {
                        problems.push(format!("{id}: status `{}` no es met, unmet ni pending", r.status));
                        pending
                    }
                } else {
                    problems.push(format!("{id}: marker `{}` desconocido", r.marker));
                    pending
                };
                (status, r.marker.clone())
            }
        };
        if status != "met" {
            open.push(OpenCriterion { id: id.to_string(), status, marker });
        }
    }
    for r in target.criteria.iter().filter(|r| !ids.contains(r.id.as_str())) {
        problems.push(format!("{} tiene fila en ui_target y ninguna afirmación fijada", r.id));
    }
    open.sort_by_key(|o| number(&o.id));
    let (total, met) = (ids.len(), ids.len() - open.len());
    let percent = if total == 0 { 0 } else { (met as f64 * 100.0 / total as f64).round() as u32 };
    Progress { met, total, percent, open, problems }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(text: &str, pinned: bool) -> Claim {
        let json = serde_json::json!({ "id": "x", "layer": "ui", "text": text, "pinned": pinned });
        serde_json::from_value(json).unwrap()
    }

    fn finding(code: &str, severity: Severity) -> Finding {
        Finding { severity, subject: "s".into(), code: code.into(), message: String::new() }
    }

    fn file(rows: &str) -> TargetFile {
        toml::from_str(rows).unwrap()
    }

    fn ui(extra: &str) -> UiSection {
        let toml = format!("[project]\nname = \"x\"\n[ui]\ntheme = \"t.tres\"\n{extra}");
        crate::config::Config::parse(&toml).unwrap().ui.unwrap()
    }

    const ROWS: &str = r#"
[[criteria]]
id = "C1"
marker = "kiln:contrast"
[[criteria]]
id = "C2"
marker = "po"
status = "met"
[[criteria]]
id = "C10"
marker = "test"
status = "pending"
"#;

    fn claims() -> Vec<Claim> {
        vec![claim("C10 · tests", true), claim("C1 · contrast", true), claim("C2 · po", true), claim("C3 · loose", false)]
    }

    #[test]
    fn three_criteria_give_one_third_and_the_open_ones_in_numeric_order() {
        let f = [finding("ui-contrast", Severity::Warn)];
        let p = evaluate(&claims(), &file(ROWS), &f, &ui(""));
        assert_eq!((p.met, p.total, p.percent), (1, 3, 33));
        let ids: Vec<&str> = p.open.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["C1", "C10"]);
        assert_eq!(p.open[0].status, "unmet");
        assert_eq!(p.open[1].status, "pending");
        assert!(p.problems.is_empty());
    }

    #[test]
    fn a_kiln_criterion_is_met_without_findings_and_info_does_not_count() {
        let f = [finding("ui-contrast-unbacked", Severity::Info)];
        let p = evaluate(&claims(), &file(ROWS), &f, &ui(""));
        assert_eq!((p.met, p.total), (2, 3));
    }

    #[test]
    fn a_missing_row_is_pending_and_an_orphan_row_is_a_problem() {
        let rows = format!("{ROWS}[[criteria]]\nid = \"C9\"\nmarker = \"po\"\nstatus = \"met\"\n");
        let mut c = claims();
        c.push(claim("C4 · no row", true));
        let p = evaluate(&c, &file(&rows), &[], &ui(""));
        assert_eq!(p.total, 4);
        assert!(p.open.iter().any(|o| o.id == "C4" && o.status == "pending"));
        assert!(p.problems.contains(&"C4 sin fila en ui_target".to_string()), "{:?}", p.problems);
        assert!(p.problems.iter().any(|m| m.starts_with("C9 ")), "{:?}", p.problems);
    }

    #[test]
    fn a_broken_theme_makes_every_kiln_criterion_unmet() {
        let f = [finding("ui-theme", Severity::Error)];
        let p = evaluate(&claims(), &file(ROWS), &f, &ui(""));
        assert_eq!(p.open[0].id, "C1");
        assert_eq!(p.open[0].status, "unmet");
    }

    #[test]
    fn unreadable_files_make_every_kiln_criterion_unmet() {
        let f = [finding("ui-files", Severity::Warn)];
        let p = evaluate(&claims(), &file(ROWS), &f, &ui(""));
        assert_eq!(p.open[0].id, "C1");
        assert_eq!(p.open[0].status, "unmet");
        let info = [finding("ui-files", Severity::Info)];
        assert_eq!(evaluate(&claims(), &file(ROWS), &info, &ui("")).met, 2);
    }

    #[test]
    fn a_rule_that_is_off_leaves_its_criterion_pending_with_a_problem() {
        let p = evaluate(&claims(), &file(ROWS), &[], &ui("[ui.contrast]\nlevel = \"off\"\n"));
        assert_eq!(p.open[0].id, "C1");
        assert_eq!(p.open[0].status, "pending");
        assert!(p.problems.contains(&"C1: la regla contrast está apagada".to_string()), "{:?}", p.problems);
    }

    #[test]
    fn unknown_rule_marker_or_status_is_a_problem_and_pending() {
        let rows = "[[criteria]]\nid = \"C1\"\nmarker = \"kiln:nope\"\n\
                    [[criteria]]\nid = \"C2\"\nmarker = \"poo\"\nstatus = \"met\"\n\
                    [[criteria]]\nid = \"C10\"\nmarker = \"po\"\nstatus = \"done\"\n";
        let p = evaluate(&claims(), &file(rows), &[], &ui(""));
        assert_eq!(p.met, 0);
        assert!(p.open.iter().all(|o| o.status == "pending"), "{:?}", p.open);
        for id in ["C1:", "C2:", "C10:"] {
            assert!(p.problems.iter().any(|m| m.starts_with(id)), "{id} {:?}", p.problems);
        }
    }

    #[test]
    fn only_ids_of_a_letter_and_digits_count() {
        assert_eq!(criterion_id("C12 · x"), Some("C12"));
        assert_eq!(criterion_id("Cx · x"), None);
        assert_eq!(criterion_id("C · x"), None);
        assert_eq!(criterion_id("C1 x"), None);
    }
}
