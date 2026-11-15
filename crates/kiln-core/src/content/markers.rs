//! Sublists of an element compared with the child nodes of its scene node.

use super::{index::{subject, Element}, manifest::{same, show, Manifest}, schema::ContentSet, value::Value};
use crate::catalog::{Finding, Severity};

pub fn check(set_name: &str, set: &ContentSet, elements: &[Element], m: &Manifest, findings: &mut Vec<Finding>) {
    for e in elements {
        let Some(kind) = set.kinds.get(&e.kind) else { continue };
        let Some(node) = m.scenes.iter().flat_map(|s| &s.nodes).find(|n| n.id == e.id) else { continue };
        let at = subject(set_name, &e.file, Some(&e.id));
        let mut error = |code: &str, message: String| {
            findings.push(Finding { severity: Severity::Error, subject: at.clone(), code: code.into(), message });
        };
        for marker in &kind.markers {
            let items = match e.value.field(&marker.field) {
                Some(Value::List(items)) => items.as_slice(),
                _ => &[],
            };
            let names: Vec<&str> = items.iter().filter_map(|i| i.field(&marker.name).and_then(Value::as_str)).collect();
            for item in items {
                let Some(name) = item.field(&marker.name).and_then(Value::as_str) else { continue };
                let wanted = marker.node.replace("{name}", name);
                let Some(child) = node.children.iter().find(|c| c.name == wanted) else {
                    error("content_marker_missing", format!("{}: falta el nodo hijo `{wanted}` en la escena", marker.field));
                    continue;
                };
                for field in &marker.meta {
                    let Some(expected) = item.field(field) else { continue };
                    match child.meta.get(field) {
                        Some(found) if same(expected, found) => {}
                        found => error(
                            "content_marker_mismatch",
                            format!(
                                "{wanted}: `{field}` esperado {}, encontrado {}",
                                show(expected),
                                found.map_or("ninguno".to_string(), |j| j.to_string())
                            ),
                        ),
                    }
                }
            }
            let (prefix, suffix) = marker.node.split_once("{name}").unwrap_or((&marker.node, ""));
            for c in &node.children {
                let stem = c.name.strip_prefix(prefix).and_then(|r| r.strip_suffix(suffix));
                if stem.is_some_and(|s| !names.contains(&s)) {
                    error("content_marker_extra", format!("{}: el nodo hijo `{}` no corresponde a ningún elemento", marker.field, c.name));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn set() -> ContentSet {
        let toml = "[project]\nname = \"d\"\n[content.s]\ndefinitions = [\"d/*.ron\"]\nmanifest = \"m.json\"\n\
            [content.s.kinds.Room]\nid = \"key\"\nid_prefix = \"room:\"\n\
            [[content.s.kinds.Room.markers]]\nfield = \"doors\"\nname = \"name\"\nnode = \"door_{name}\"\nmeta = [\"side\", \"at\"]\n";
        let (mut sets, p) = Config::parse(toml).unwrap().content_sets();
        assert!(p.is_empty(), "{p:?}");
        sets.remove("s").unwrap()
    }

    fn door(name: &str, side: &str, at: i64) -> Value {
        let f = |k: &str, v| (k.to_string(), v);
        Value::Struct {
            name: None,
            line: 1,
            fields: vec![f("name", Value::Str(name.into())), f("side", Value::Ident(side.into())), f("at", Value::Int(at))],
        }
    }

    fn room(key: &str, doors: Option<Vec<Value>>) -> Element {
        let mut fields = vec![("key".to_string(), Value::Str(key.into()))];
        fields.extend(doors.map(|d| ("doors".to_string(), Value::List(d))));
        Element {
            kind: "Room".into(),
            id: format!("room:{key}"),
            file: "d/a.ron".into(),
            line: 1,
            value: Value::Struct { name: None, fields, line: 1 },
        }
    }

    fn run(el: Element, children: &str) -> Vec<Finding> {
        let m: Manifest = serde_json::from_str(&format!(
            r#"{{"version":2,"set":"s","scenes":[{{"scene":"x","nodes":[{{"id":"room:a","children":[{children}]}}]}}]}}"#
        ))
        .unwrap();
        let mut f = Vec::new();
        check("s", &set(), &[el], &m, &mut f);
        f
    }

    const EAST: &str = r#"{"name":"door_e","meta":{"side":"east","at":0}}"#;

    fn one(f: Vec<Finding>, code: &str, needle: &str) {
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].code == code && f[0].severity == Severity::Error);
        assert_eq!(f[0].subject, "s:d/a.ron#room:a");
        assert!(f[0].message.contains(needle), "{}", f[0].message);
    }

    #[test]
    fn matching_is_clean() {
        assert!(run(room("a", Some(vec![door("e", "East", 0)])), EAST).is_empty());
    }

    #[test]
    fn missing_child() {
        one(run(room("a", Some(vec![door("e", "East", 0)])), ""), "content_marker_missing", "door_e");
    }

    #[test]
    fn changed_meta() {
        one(run(room("a", Some(vec![door("e", "East", 3)])), EAST), "content_marker_mismatch", "at");
    }

    #[test]
    fn extra_child() {
        let kids = format!(r#"{EAST},{{"name":"door_w"}},{{"name":"other"}}"#);
        one(run(room("a", Some(vec![door("e", "East", 0)])), &kids), "content_marker_extra", "door_w");
    }

    #[test]
    fn absent_list_means_empty() {
        one(run(room("a", None), EAST), "content_marker_extra", "door_e");
        assert!(run(room("a", None), "").is_empty());
    }

    #[test]
    fn element_not_in_manifest_is_silent() {
        let mut f = Vec::new();
        let m: Manifest = serde_json::from_str(r#"{"version":2,"set":"s","scenes":[]}"#).unwrap();
        check("s", &set(), &[room("a", Some(vec![door("e", "East", 0)]))], &m, &mut f);
        assert!(f.is_empty());
    }
}
