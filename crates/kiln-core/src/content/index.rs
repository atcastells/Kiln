//! Loads the elements of a content set from its definition files.

use super::{parse, schema::ContentSet, value::Value};
use crate::catalog::{Finding, Severity};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Element {
    pub kind: String,
    /// `id_prefix` plus the value of the id field.
    pub id: String,
    /// Path relative to the project root, with `/` separators.
    pub file: String,
    pub line: usize,
    /// Always a `Value::Struct`.
    pub value: Value,
}

pub fn subject(set: &str, file: &str, id: Option<&str>) -> String {
    match id {
        Some(id) => format!("{set}:{file}#{id}"),
        None => format!("{set}:{file}"),
    }
}

pub fn load(root: &Path, set_name: &str, set: &ContentSet, findings: &mut Vec<Finding>) -> Vec<Element> {
    let mut paths: Vec<_> = set
        .definitions
        .iter()
        .filter_map(|g| glob::glob(&root.join(g).to_string_lossy()).ok())
        .flatten()
        .flatten()
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    paths.dedup();
    let mut error = |code: &str, subject: String, message: String| {
        findings.push(Finding { severity: Severity::Error, subject, code: code.into(), message });
    };
    if paths.is_empty() {
        findings.push(Finding {
            severity: Severity::Warn,
            subject: set_name.into(),
            code: "content_no_definitions".into(),
            message: format!("ningún fichero coincide con {:?}", set.definitions),
        });
        return Vec::new();
    }
    let mut out: Vec<Element> = Vec::new();
    for path in &paths {
        let file = crate::catalog::rel(root, path);
        let at = subject(set_name, &file, None);
        let value = match std::fs::read_to_string(path) {
            Err(e) => {
                error("content_syntax", at, format!("no se puede leer: {e}"));
                continue;
            }
            Ok(text) => match parse::parse(&text) {
                Ok(v) => v,
                Err(e) => {
                    error("content_syntax", at, format!("línea {}, columna {}: {}", e.line, e.column, e.message));
                    continue;
                }
            },
        };
        let items = match (&set.root, &value) {
            (None, Value::List(items)) => items,
            (Some(field), v @ Value::Struct { .. }) => match v.field(field) {
                Some(Value::List(items)) => items,
                _ => {
                    error("content_syntax", at, format!("se esperaba un registro con la lista `{field}`"));
                    continue;
                }
            },
            _ => {
                let shape = if set.root.is_some() { "un registro" } else { "una lista" };
                error("content_syntax", at, format!("el nivel superior debería ser {shape}"));
                continue;
            }
        };
        for item in items {
            let Value::Struct { name, line, .. } = item else {
                error("content_syntax", at.clone(), "cada elemento debe ser una estructura".into());
                continue;
            };
            let kind_name = name.as_ref().or(set.default_kind.as_ref());
            let Some((kind_name, kind)) = kind_name.and_then(|n| set.kinds.get_key_value(n)) else {
                let shown = name.as_deref().unwrap_or("sin nombre");
                error("content_unknown_kind", at.clone(), format!("línea {line}: tipo «{shown}» no declarado"));
                continue;
            };
            let Some(id) = item.field(&kind.id).and_then(Value::as_str) else {
                error("content_missing_id", at.clone(), format!("línea {line}: {kind_name} sin campo `{}` de texto", kind.id));
                continue;
            };
            let id = format!("{}{id}", kind.id_prefix.as_deref().unwrap_or(""));
            if let Some(first) = out.iter().find(|e| e.id == id) {
                let msg = format!("el id ya está definido en {}", first.file);
                error("content_duplicate_id", subject(set_name, &file, Some(&id)), msg);
                continue;
            }
            out.push(Element { kind: kind_name.clone(), id, file: file.clone(), line: *line, value: item.clone() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    fn dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-index-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (f, text) in files {
            let p = d.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        d
    }

    fn set(extra: &str, kinds: &str) -> ContentSet {
        let toml = format!(
            "[project]\nname = \"demo\"\n[content.s]\ndefinitions = [\"defs/*.ron\"]\n{extra}\n{kinds}"
        );
        let (mut sets, problems) = Config::parse(&toml).unwrap().content_sets();
        assert!(problems.is_empty(), "{problems:?}");
        sets.remove("s").unwrap()
    }

    const LIST_KINDS: &str = "[content.s.kinds.Sector]\nid = \"id\"\n[content.s.kinds.Door]\nid = \"id\"\n[content.s.kinds.Object]\nid = \"id\"\n[content.s.kinds.Installation]\nid = \"id\"\n";
    const MODULE_KIND: &str = "[content.s.kinds.Module]\nid = \"key\"\nid_prefix = \"sector:\"\n";

    fn run(root: &Path, set: &ContentSet) -> (Vec<Element>, Vec<Finding>) {
        let mut findings = Vec::new();
        let els = load(root, "s", set, &mut findings);
        (els, findings)
    }

    fn only(findings: &[Finding], code: &str, severity: Severity, subject: &str) -> Finding {
        assert_eq!(findings.len(), 1, "{findings:?}");
        let f = &findings[0];
        assert_eq!((f.code.as_str(), f.severity, f.subject.as_str()), (code, severity, subject));
        f.clone()
    }

    #[test]
    fn subject_forms() {
        assert_eq!(subject("s", "a/b.ron", Some("x")), "s:a/b.ron#x");
        assert_eq!(subject("s", "a/b.ron", None), "s:a/b.ron");
    }

    #[test]
    fn list_set_loads_named_variants() {
        let text = "[\n Sector(id: \"s1\", scene: \"a\"),\n Door(id: \"d1\", connects: (\"s1\", \"s2\")),\n Object(id: \"o1\", asset: \"x\"),\n\n Installation(id: \"i1\", sector: \"s1\", objects: []),\n Sector(id: \"s2\", scene: \"b\"),\n]";
        let root = dir("list", &[("defs/a.ron", text)]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert!(findings.is_empty(), "{findings:?}");
        let got: Vec<_> = els.iter().map(|e| (e.kind.as_str(), e.id.as_str(), e.file.as_str(), e.line)).collect();
        assert_eq!(
            got,
            vec![
                ("Sector", "s1", "defs/a.ron", 2),
                ("Door", "d1", "defs/a.ron", 3),
                ("Object", "o1", "defs/a.ron", 4),
                ("Installation", "i1", "defs/a.ron", 6),
                ("Sector", "s2", "defs/a.ron", 7),
            ]
        );
        assert!(matches!(els[0].value, Value::Struct { .. }));
    }

    #[test]
    fn root_set_prefixes_ids() {
        let text = "(modules: [\n (key: \"access\", size: (3, 2)),\n (key: \"lab\", size: (1, 1)),\n])";
        let root = dir("root", &[("defs/m.ron", text)]);
        let s = set("root = \"modules\"\ndefault_kind = \"Module\"", MODULE_KIND);
        let (els, findings) = run(&root, &s);
        assert!(findings.is_empty(), "{findings:?}");
        let got: Vec<_> = els.iter().map(|e| (e.kind.as_str(), e.id.as_str(), e.line)).collect();
        assert_eq!(got, vec![("Module", "sector:access", 2), ("Module", "sector:lab", 3)]);
    }

    #[test]
    fn no_definitions_warns() {
        let root = dir("none", &[]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert!(els.is_empty());
        only(&findings, "content_no_definitions", Severity::Warn, "s");
    }

    #[test]
    fn syntax_error_has_line_and_column() {
        let root = dir("syntax", &[("defs/a.ron", "[\n Sector(id: ,)\n]")]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert!(els.is_empty());
        let f = only(&findings, "content_syntax", Severity::Error, "s:defs/a.ron");
        assert!(f.message.contains("línea 2, columna"), "{}", f.message);
    }

    #[test]
    fn wrong_top_level_shape() {
        let root = dir("shape", &[("defs/a.ron", "(modules: [])")]);
        let (_, findings) = run(&root, &set("", LIST_KINDS));
        only(&findings, "content_syntax", Severity::Error, "s:defs/a.ron");

        let root = dir("shape2", &[("defs/a.ron", "[Module(key: \"a\")]")]);
        let s = set("root = \"modules\"\ndefault_kind = \"Module\"", MODULE_KIND);
        let (_, findings) = run(&root, &s);
        only(&findings, "content_syntax", Severity::Error, "s:defs/a.ron");

        let root = dir("shape3", &[("defs/a.ron", "(other: [])")]);
        let (_, findings) = run(&root, &s);
        only(&findings, "content_syntax", Severity::Error, "s:defs/a.ron");

        let root = dir("shape4", &[("defs/a.ron", "[1]")]);
        let (_, findings) = run(&root, &set("", LIST_KINDS));
        only(&findings, "content_syntax", Severity::Error, "s:defs/a.ron");
    }

    #[test]
    fn unknown_kind_is_skipped() {
        let root = dir("kind", &[("defs/a.ron", "[\n Ghost(id: \"g\"),\n Sector(id: \"s1\"),\n]")]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert_eq!(els.len(), 1);
        let f = only(&findings, "content_unknown_kind", Severity::Error, "s:defs/a.ron");
        assert!(f.message.contains("Ghost") && f.message.contains('2'), "{}", f.message);
    }

    #[test]
    fn unnamed_without_default_kind_is_unknown_kind() {
        let root = dir("unnamed", &[("defs/a.ron", "[(id: \"g\")]")]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert!(els.is_empty());
        only(&findings, "content_unknown_kind", Severity::Error, "s:defs/a.ron");
    }

    #[test]
    fn missing_or_non_string_id() {
        let root = dir("id", &[("defs/a.ron", "[\n Sector(scene: \"a\"),\n Sector(id: 3),\n]")]);
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert!(els.is_empty());
        assert_eq!(findings.len(), 2, "{findings:?}");
        for f in &findings {
            assert_eq!((f.code.as_str(), f.severity), ("content_missing_id", Severity::Error));
            assert_eq!(f.subject, "s:defs/a.ron");
        }
        assert!(findings[0].message.contains('2') && findings[1].message.contains('3'));
    }

    #[test]
    fn duplicate_across_files_keeps_first() {
        let root = dir(
            "dup",
            &[("defs/a.ron", "[Sector(id: \"s1\")]"), ("defs/b.ron", "[Sector(id: \"s1\")]")],
        );
        let (els, findings) = run(&root, &set("", LIST_KINDS));
        assert_eq!(els.len(), 1);
        assert_eq!(els[0].file, "defs/a.ron");
        let f = only(&findings, "content_duplicate_id", Severity::Error, "s:defs/b.ron#s1");
        assert!(f.message.contains("defs/a.ron"), "{}", f.message);
    }
}
