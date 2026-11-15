//! Godot scene manifest (version 2): JSON written by the Godot exporter, one file per content set.
//! It is compared against the definitions, never a content source.

use super::{index::{subject, Element}, schema::ContentSet, value::Value};
use crate::catalog::{Finding, Severity};
use serde::Deserialize;
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::path::Path;

type Meta = BTreeMap<String, Json>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub set: String,
    pub scenes: Vec<SceneEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SceneEntry {
    pub scene: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Child {
    pub name: String,
    #[serde(default)]
    pub meta: Meta,
}

fn finding(severity: Severity, subject: String, code: &str, message: String) -> Finding {
    Finding { severity, subject, code: code.into(), message }
}

pub fn load(root: &Path, set_name: &str, set: &ContentSet, findings: &mut Vec<Finding>) -> Option<Manifest> {
    let rel = set.manifest.as_deref()?;
    let at = subject(set_name, rel, None);
    let syntax = |message: String| finding(Severity::Error, subject(set_name, rel, None), "content_manifest_syntax", message);
    let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
        let message = "manifiesto de Godot ausente: ejecuta `kiln content-manifest`".to_string();
        findings.push(finding(Severity::Warn, at, "content_manifest_missing", message));
        return None;
    };
    let manifest: Manifest = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(e) => {
            findings.push(syntax(format!("línea {}, columna {}: {}", e.line(), e.column(), e)));
            return None;
        }
    };
    let problem = if manifest.version != 2 {
        format!("versión {} no soportada, se esperaba 2", manifest.version)
    } else if manifest.set != set_name {
        format!("el manifiesto es del conjunto «{}», no de «{set_name}»", manifest.set)
    } else {
        return Some(manifest);
    };
    findings.push(syntax(problem));
    None
}

/// Ordered comparison of a definition value with a JSON value.
pub(crate) fn same(value: &Value, json: &Json) -> bool {
    matches_with(value, json, true)
}

fn matches_with(value: &Value, json: &Json, ordered: bool) -> bool {
    match (value, json) {
        (Value::Str(a), Json::String(b)) => a == b,
        (Value::Ident(a), Json::String(b)) => a.eq_ignore_ascii_case(b),
        (Value::Int(a), Json::Number(b)) => b.as_f64() == Some(*a as f64),
        (Value::Float(a), Json::Number(b)) => b.as_f64() == Some(*a),
        (Value::Bool(a), Json::Bool(b)) => a == b,
        (Value::List(a) | Value::Tuple(a), Json::Array(b)) if a.len() == b.len() => {
            if ordered {
                a.iter().zip(b).all(|(x, y)| matches_with(x, y, true))
            } else {
                let mut used = vec![false; b.len()];
                a.iter().all(|x| {
                    let hit = (0..b.len()).find(|&i| !used[i] && matches_with(x, &b[i], false));
                    hit.map(|i| used[i] = true).is_some()
                })
            }
        }
        _ => false,
    }
}

pub(crate) fn show(value: &Value) -> String {
    match value {
        Value::Str(s) | Value::Ident(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::List(v) | Value::Tuple(v) => format!("[{}]", v.iter().map(show).collect::<Vec<_>>().join(", ")),
        Value::Struct { .. } => "{..}".into(),
    }
}

pub fn compare(set_name: &str, set: &ContentSet, elements: &[Element], m: &Manifest, findings: &mut Vec<Finding>) {
    let rel = set.manifest.as_deref().unwrap_or("");
    let mut mismatch = |subject: String, message: String| {
        findings.push(finding(Severity::Error, subject, "content_manifest_mismatch", message));
    };
    let nodes = || m.scenes.iter().flat_map(|s| s.nodes.iter().map(move |n| (s, n)));
    for e in elements {
        let Some(kind) = set.kinds.get(&e.kind) else { continue };
        let at = subject(set_name, &e.file, Some(&e.id));
        let scene = kind.scene.as_ref().and_then(|f| e.value.field(f)).and_then(Value::as_str);
        let node = match scene {
            Some(path) => {
                let found = m.scenes.iter().filter(|s| s.scene == path).flat_map(|s| &s.nodes).find(|n| n.id == e.id);
                if found.is_none() {
                    let other = nodes().find(|(_, n)| n.id == e.id).map(|(s, _)| s);
                    mismatch(at, match other {
                        Some(s) => format!("el nodo está en la escena {} del manifiesto, pero la definición dice {path}", s.scene),
                        None => format!("el nodo no aparece en la escena {path} del manifiesto"),
                    });
                    continue;
                }
                found
            }
            None if !kind.node_meta.is_empty() => {
                let found = nodes().map(|(_, n)| n).find(|n| n.id == e.id);
                if found.is_none() {
                    mismatch(at, "el nodo no aparece en el manifiesto".into());
                    continue;
                }
                found
            }
            None => None,
        };
        let Some(node) = node else { continue };
        for (field, key) in &kind.node_meta {
            let Some(value) = e.value.field(field) else { continue };
            match node.meta.get(key) {
                Some(j) if matches_with(value, j, false) => {}
                found => mismatch(
                    subject(set_name, &e.file, Some(&e.id)),
                    format!(
                        "`{field}` distinto: definición {}, manifiesto {}",
                        show(value),
                        found.map_or("ninguno".to_string(), Json::to_string)
                    ),
                ),
            }
        }
    }
    for (_, n) in nodes() {
        if !elements.iter().any(|e| e.id == n.id) {
            mismatch(subject(set_name, rel, Some(&n.id)), "el nodo del manifiesto no tiene definición".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const KINDS: &str = "[content.s.kinds.Room]\nid = \"key\"\nid_prefix = \"room:\"\nscene = \"scene\"\n\
        [content.s.kinds.Link]\nid = \"key\"\nnode_meta = { ends = \"ends_meta\", side = \"side\" }\n";

    fn set() -> ContentSet {
        let toml = format!("[project]\nname = \"d\"\n[content.s]\ndefinitions = [\"d/*.ron\"]\nmanifest = \"m.json\"\n{KINDS}");
        let (mut sets, p) = Config::parse(&toml).unwrap().content_sets();
        assert!(p.is_empty(), "{p:?}");
        sets.remove("s").unwrap()
    }

    fn st(fields: Vec<(&str, Value)>) -> Value {
        Value::Struct { name: None, fields: fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect(), line: 1 }
    }
    fn s(x: &str) -> Value {
        Value::Str(x.into())
    }

    fn room(key: &str, scene: &str) -> Element {
        Element {
            kind: "Room".into(),
            id: format!("room:{key}"),
            file: "d/a.ron".into(),
            line: 1,
            value: st(vec![("key", s(key)), ("scene", s(scene))]),
        }
    }
    fn link(key: &str, ends: [&str; 2], side: &str) -> Element {
        Element {
            kind: "Link".into(),
            id: key.into(),
            file: "d/a.ron".into(),
            line: 2,
            value: st(vec![
                ("key", s(key)),
                ("ends", Value::Tuple(ends.iter().map(|e| s(e)).collect())),
                ("side", Value::Ident(side.into())),
            ]),
        }
    }
    fn elements() -> Vec<Element> {
        vec![room("a", "res://a.tscn"), link("l1", ["room:a", "room:b"], "East")]
    }
    fn good() -> Manifest {
        serde_json::from_str(
            r#"{"version":2,"set":"s","scenes":[
              {"scene":"res://a.tscn","nodes":[{"id":"room:a"}]},
              {"scene":"res://b.tscn","nodes":[{"id":"l1","meta":{"ends_meta":["room:b","room:a"],"side":"east"}}]}]}"#,
        )
        .unwrap()
    }
    fn run(els: &[Element], m: &Manifest) -> Vec<Finding> {
        let mut f = Vec::new();
        compare("s", &set(), els, m, &mut f);
        f
    }
    fn only(f: Vec<Finding>, needle: &str) {
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].code == "content_manifest_mismatch" && f[0].severity == Severity::Error);
        assert!(f[0].message.contains(needle), "{}", f[0].message);
    }

    fn load_text(text: Option<&str>) -> (Option<Manifest>, Vec<Finding>) {
        let d = std::env::temp_dir().join(format!("kiln-cm-{}-{}", std::process::id(), text.map_or(0, str::len)));
        std::fs::create_dir_all(&d).unwrap();
        if let Some(t) = text {
            std::fs::write(d.join("m.json"), t).unwrap();
        }
        let mut f = Vec::new();
        let m = load(&d, "s", &set(), &mut f);
        (m, f)
    }

    #[test]
    fn missing_warns() {
        let d = std::env::temp_dir().join(format!("kiln-cm-none-{}", std::process::id()));
        let mut f = Vec::new();
        assert!(load(&d, "s", &set(), &mut f).is_none());
        assert_eq!((f[0].code.as_str(), f[0].severity), ("content_manifest_missing", Severity::Warn));
        assert!(f[0].message.contains("kiln content-manifest"));
    }

    #[test]
    fn broken_version_and_set_are_syntax_errors() {
        for text in ["{", r#"{"version":1,"set":"s","scenes":[]}"#, r#"{"version":2,"set":"x","scenes":[]}"#] {
            let (m, f) = load_text(Some(text));
            assert!(m.is_none());
            assert_eq!((f[0].code.as_str(), f[0].severity), ("content_manifest_syntax", Severity::Error), "{text}");
        }
    }

    #[test]
    fn valid_loads_with_optional_parts_absent() {
        let (m, f) = load_text(Some(r#"{"version":2,"set":"s","scenes":[{"scene":"x","nodes":[{"id":"n"}]}]}"#));
        assert!(f.is_empty() && m.unwrap().scenes[0].nodes[0].children.is_empty());
    }

    #[test]
    fn no_manifest_configured_is_silent() {
        let mut set = set();
        set.manifest = None;
        let mut f = Vec::new();
        assert!(load(Path::new("."), "s", &set, &mut f).is_none() && f.is_empty());
    }

    #[test]
    fn matching_set_is_clean() {
        assert!(run(&elements(), &good()).is_empty());
    }

    #[test]
    fn scene_node_missing_or_in_other_scene() {
        only(run(&[room("a", "res://a.tscn"), room("zz", "res://a.tscn"), link("l1", ["room:a", "room:b"], "East")], &good()), "no aparece");
        only(run(&[room("a", "res://b.tscn"), link("l1", ["room:a", "room:b"], "East")], &good()), "res://a.tscn");
    }

    #[test]
    fn node_meta_kind_missing_everywhere() {
        let els = vec![room("a", "res://a.tscn"), link("l9", ["room:a", "room:b"], "East")];
        let f = run(&els, &good());
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f.iter().any(|x| x.subject == "s:d/a.ron#l9"));
        assert!(f.iter().any(|x| x.subject == "s:m.json#l1"));
    }

    #[test]
    fn meta_differs_or_absent() {
        only(run(&[room("a", "res://a.tscn"), link("l1", ["room:a", "room:c"], "East")], &good()), "ends");
        only(run(&[room("a", "res://a.tscn"), link("l1", ["room:a", "room:b"], "West")], &good()), "side");
        let m: Manifest = serde_json::from_str(r#"{"version":2,"set":"s","scenes":[{"scene":"res://a.tscn","nodes":[{"id":"room:a"},{"id":"l1","meta":{"side":"east"}}]}]}"#).unwrap();
        only(run(&elements(), &m), "ninguno");
    }

    #[test]
    fn manifest_node_without_element() {
        only(run(&[link("l1", ["room:a", "room:b"], "East")], &good()), "no tiene definición");
    }

    #[test]
    fn same_rules() {
        let j = |t: &str| serde_json::from_str::<Json>(t).unwrap();
        assert!(same(&Value::Ident("East".into()), &j("\"east\"")));
        assert!(!same(&Value::Str("East".into()), &j("\"east\"")));
        assert!(same(&Value::Tuple(vec![Value::Int(0), Value::Int(1)]), &j("[0, 1]")));
        assert!(!same(&Value::Tuple(vec![Value::Int(0), Value::Int(1)]), &j("[1, 0]")));
        assert!(!same(&Value::List(vec![Value::Int(0)]), &j("[0, 1]")));
        assert!(same(&Value::Float(1.5), &j("1.5")) && same(&Value::Bool(true), &j("true")));
        assert!(!same(&Value::Int(1), &j("\"1\"")));
    }
}
