//! Writing specs from the calling agent, checked before they reach the disk.

use super::{check_id, check_len, confine, rel};
use crate::agent::Ctx;
use kiln_core::{Finding, Severity, spec};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct Written {
    pub file: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Serialize)]
pub struct Rejected {
    pub error: String,
    pub findings: Vec<Finding>,
}

pub struct PartLayout {
    pub part: String,
    pub at: Vec<[f64; 3]>,
    pub turn: Option<Vec<[f64; 3]>>,
}

fn reject(error: impl Into<String>) -> Rejected {
    Rejected { error: error.into(), findings: vec![] }
}

/// The spec file of `id` in the catalog, if there is one.
fn existing(ctx: &Ctx, id: &str) -> Option<PathBuf> {
    let cat = crate::load(&ctx.project, ctx.config.as_deref()).ok()?;
    cat.specs.iter().find(|s| s.asset.id == id).map(|s| ctx.project.join(&s.file))
}

/// The spec of `id` with only its recipe replaced by the one `toml_text`
/// carries: anything else the file holds (anchors, variants, keys Kiln does
/// not know) stays. Without a spec on disk the text goes as it is.
pub fn with_recipe(ctx: &Ctx, id: &str, toml_text: &str) -> Result<String, String> {
    let Some(path) = existing(ctx, id) else {
        return Ok(toml_text.to_string());
    };
    let parse = |t: &str| t.parse::<toml::Table>().map_err(|e| format!("la ficha no se lee: {e}"));
    let mut doc = parse(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)?;
    let sent = parse(toml_text)?;
    let recipe = sent.get("generator").ok_or("la ficha enviada no trae receta")?;
    doc.insert("generator".into(), recipe.clone());
    toml::to_string(&doc).map_err(|e| e.to_string())
}

/// Writes `text` as the spec of `id`, reloads the catalog and puts the disk
/// back as it was if the spec has errors.
fn write_checked(ctx: &Ctx, id: &str, text: &str, target: PathBuf) -> Result<Written, Rejected> {
    let before = std::fs::read(&target).ok();
    std::fs::write(&target, text).map_err(|e| reject(format!("no se pudo escribir la ficha: {e}")))?;
    let file = rel(&ctx.project, &target);
    let cat = crate::load(&ctx.project, ctx.config.as_deref());
    let mine = |f: &Finding| {
        f.subject == format!("asset:{id}") || f.subject.starts_with(&format!("part:{id}/")) || f.subject == file
    };
    let (errors, findings): (Vec<_>, Vec<_>) = match cat {
        Ok(c) => c.findings.into_iter().filter(mine).partition(|f| f.severity == Severity::Error),
        Err(e) => {
            restore(&target, before);
            return Err(reject(e));
        }
    };
    if errors.is_empty() {
        return Ok(Written { file, findings });
    }
    restore(&target, before);
    Err(Rejected { error: "la ficha tiene errores; no se ha escrito".into(), findings: errors })
}

fn restore(target: &PathBuf, before: Option<Vec<u8>>) {
    let _ = match before {
        Some(b) => std::fs::write(target, b),
        None => std::fs::remove_file(target),
    };
}

pub fn spec_write(ctx: &Ctx, id: &str, toml_text: &str) -> Result<Written, Rejected> {
    check_id("el asset", id).map_err(reject)?;
    check_len("la ficha", toml_text, 200_000).map_err(reject)?;
    let s = spec::parse(toml_text).map_err(|e| reject(format!("la ficha no se lee: {e}")))?;
    if s.asset.id != id {
        return Err(reject(format!("el id de la ficha («{}») no es «{id}»", s.asset.id)));
    }
    let path = existing(ctx, id).map(|p| rel(&ctx.project, &p)).unwrap_or(format!("{}/{id}.toml", ctx.specs_dir));
    let target = confine(ctx, &path).map_err(reject)?;
    write_checked(ctx, id, toml_text, target)
}

pub fn spec_layout(ctx: &Ctx, id: &str, parts: Vec<PartLayout>) -> Result<Written, Rejected> {
    check_id("el asset", id).map_err(reject)?;
    let path = existing(ctx, id).ok_or_else(|| reject(format!("no hay ficha con id «{id}»")))?;
    let target = confine(ctx, &rel(&ctx.project, &path)).map_err(reject)?;
    let text = std::fs::read_to_string(&target).map_err(|e| reject(e.to_string()))?;
    let mut doc: toml::Table = text.parse().map_err(|e: toml::de::Error| reject(e.to_string()))?;
    let list = doc.get_mut("parts").and_then(|p| p.as_array_mut()).ok_or_else(|| reject("la ficha no tiene piezas"))?;
    for l in &parts {
        let entry = list
            .iter_mut()
            .filter_map(|p| p.as_table_mut())
            .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(&l.part))
            .ok_or_else(|| reject(format!("la ficha no tiene la pieza «{}»", l.part)))?;
        let triples = |v: &[[f64; 3]]| {
            let one = |p: &[f64; 3]| toml::Value::Array(p.iter().map(|n| toml::Value::Float(*n)).collect());
            toml::Value::Array(v.iter().map(one).collect())
        };
        entry.insert("at".into(), triples(&l.at));
        match &l.turn {
            Some(t) => entry.insert("turn".into(), triples(t)),
            None => entry.remove("turn"),
        };
    }
    if let Some(a) = doc.get_mut("asset").and_then(|a| a.as_table_mut()) {
        a.insert("status".into(), "layout".into());
    }
    let out = toml::to_string(&doc).map_err(|e| reject(e.to_string()))?;
    write_checked(ctx, id, &out, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing::{ctx, examples};

    fn json(file: &std::path::Path) -> serde_json::Value {
        serde_json::to_value(spec::parse(&std::fs::read_to_string(file).unwrap()).unwrap()).unwrap()
    }

    fn hoist(t: &crate::ops::testing::Temp) -> (PathBuf, String) {
        let p = t.0.join("specs/hoist.toml");
        let s = std::fs::read_to_string(&p).unwrap();
        (p, s)
    }

    #[test]
    fn rewriting_hoist_with_its_own_text_keeps_it() {
        let t = examples("");
        let (p, s) = hoist(&t);
        let w = spec_write(&ctx(&t), "hoist", &s).unwrap();
        assert_eq!(w.file, "specs/hoist.toml");
        assert_eq!(json(&p), serde_json::to_value(spec::parse(&s).unwrap()).unwrap());
    }

    #[test]
    fn invalid_specs_leave_the_disk_as_it_was() {
        let t = examples("");
        let (p, s) = hoist(&t);
        let bad_kind = s.replacen("kind = \"reuse\"", "kind = \"magic\"", 1);
        let cases = [
            ("hoist", "not = [toml".to_string()),
            ("other", s.clone()),
            ("hoist", bad_kind),
        ];
        for (id, text) in cases {
            assert!(spec_write(&ctx(&t), id, &text).is_err(), "{id}");
            assert_eq!(std::fs::read_to_string(&p).unwrap(), s);
        }
        assert!(!t.0.join("specs/other.toml").exists());
        let e = spec_write(&ctx(&t), "a/b", &s.replace("\"hoist\"", "\"a/b\"")).unwrap_err();
        assert!(e.error.contains("no vale"), "{}", e.error);
    }

    #[test]
    fn rejected_new_spec_is_deleted_and_valid_one_lands_in_specs_dir() {
        let t = examples("");
        let (_, s) = hoist(&t);
        let new = s.replace("id = \"hoist\"", "id = \"hoist2\"");
        let bad = new.replacen("kind = \"reuse\"", "kind = \"magic\"", 1);
        let r = spec_write(&ctx(&t), "hoist2", &bad).unwrap_err();
        assert!(r.findings.iter().any(|f| f.code == "source_kind"));
        assert!(!t.0.join("specs/hoist2.toml").exists());
        let w = spec_write(&ctx(&t), "hoist2", &new).unwrap();
        assert_eq!(w.file, "specs/hoist2.toml");
        assert!(t.0.join("specs/hoist2.toml").exists());
    }

    #[test]
    fn layout_changes_only_at_turn_and_status() {
        let t = examples("");
        let (p, _) = hoist(&t);
        let before = json(&p);
        let parts = vec![PartLayout { part: "post".into(), at: vec![[1.0, 2.0, 3.0]], turn: Some(vec![[0.0, 90.0, 0.0]]) }];
        spec_layout(&ctx(&t), "hoist", parts).unwrap();
        let mut after = json(&p);
        let post = &mut after["parts"][0];
        assert_eq!(post["at"], serde_json::json!([[1.0, 2.0, 3.0]]));
        assert_eq!(post["turn"], serde_json::json!([[0.0, 90.0, 0.0]]));
        post["at"] = before["parts"][0]["at"].clone();
        post.as_object_mut().unwrap().remove("turn");
        assert_eq!(after, before);
    }

    #[test]
    fn layout_rejects_unknown_part_and_writes_nothing() {
        let t = examples("");
        let (p, s) = hoist(&t);
        let parts = vec![PartLayout { part: "nope".into(), at: vec![[0.0; 3]], turn: None }];
        let e = spec_layout(&ctx(&t), "hoist", parts).unwrap_err();
        assert!(e.error.contains("nope"));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), s);
    }
}
