//! Field rules driven by the schema: presence, type, numeric range and closed lists.

use super::{index::{subject, Element}, schema::ContentSet, value::Value};
use crate::catalog::{Finding, Severity};

pub fn check(set_name: &str, set: &ContentSet, elements: &[Element], findings: &mut Vec<Finding>) {
    for e in elements {
        let Some(kind) = set.kinds.get(&e.kind) else { continue };
        let mut error = |code: &str, message: String| {
            findings.push(Finding {
                severity: Severity::Error,
                subject: subject(set_name, &e.file, Some(&e.id)),
                code: code.into(),
                message,
            })
        };
        for rule in &kind.fields {
            let name = &rule.name;
            let Some(value) = e.value.field(name) else {
                if rule.required {
                    error("content_field_missing", format!("falta el campo obligatorio `{name}`"));
                }
                continue;
            };
            if let Some(ty) = rule.ty {
                let want = format!("{ty:?}").to_lowercase();
                let got = value.type_name();
                if got != want && !(want == "float" && got == "int") {
                    error("content_field_type", format!("`{name}` debe ser {want} y es {got}"));
                    continue;
                }
            }
            let number = match value {
                Value::Int(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            };
            if let Some(n) = number
                && (rule.min.is_some_and(|m| n < m) || rule.max.is_some_and(|m| n > m))
            {
                let bounds = match (rule.min, rule.max) {
                    (Some(a), Some(b)) => format!("entre {a} y {b}"),
                    (Some(a), None) => format!("como mínimo {a}"),
                    (None, b) => format!("como máximo {}", b.unwrap_or_default()),
                };
                error("content_field_range", format!("`{name}` vale {n} y debe estar {bounds}"));
            }
            let word = match value {
                Value::Str(s) | Value::Ident(s) => Some(s),
                _ => None,
            };
            if let (Some(w), Some(list)) = (word, &rule.one_of)
                && !list.contains(w)
            {
                error("content_field_value", format!("`{name}` vale «{w}» y debe ser uno de {}", list.join(", ")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const KINDS: &str = r#"
[content.s.kinds.Room]
id = "name"
fields = [
  { name = "size", required = true, type = "tuple" },
  { name = "kind", type = "ident", one_of = ["Lab", "Dock"] },
  { name = "tag", type = "string", one_of = ["a", "b"] },
  { name = "capacity", type = "int", min = 0, max = 500 },
  { name = "ratio", type = "float", min = 0.5, max = 1.5 },
]
"#;

    fn run(text: &str) -> Vec<Finding> {
        let toml = format!("[project]\nname = \"demo\"\n[content.s]\ndefinitions = [\"defs/*.ron\"]\n{KINDS}");
        let (mut sets, problems) = Config::parse(&toml).unwrap().content_sets();
        assert!(problems.is_empty(), "{problems:?}");
        let set = sets.remove("s").unwrap();
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("kiln-fields-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("defs")).unwrap();
        std::fs::write(root.join("defs/a.ron"), format!("[{text}]")).unwrap();
        let mut f = Vec::new();
        let els = crate::content::index::load(&root, "s", &set, &mut f);
        assert!(f.is_empty(), "{f:?}");
        check("s", &set, &els, &mut f);
        f
    }

    fn one(f: &[Finding], code: &str, needle: &str) {
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!((f[0].code.as_str(), f[0].subject.as_str(), f[0].severity), (code, "s:defs/a.ron#r", Severity::Error));
        assert!(f[0].message.contains(needle), "{}", f[0].message);
    }

    #[test]
    fn valid_element_has_no_findings() {
        let f = run("Room(name: \"r\", size: (3, 2), kind: Lab, tag: \"a\", capacity: 10, ratio: 1.0)");
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn required_field_missing() {
        one(&run("Room(name: \"r\")"), "content_field_missing", "size");
    }

    #[test]
    fn wrong_type() {
        one(&run("Room(name: \"r\", size: [3, 2])"), "content_field_type", "size");
        one(&run("Room(name: \"r\", size: (1, 1), capacity: \"ten\")"), "content_field_type", "capacity");
    }

    #[test]
    fn int_satisfies_float() {
        assert!(run("Room(name: \"r\", size: (1, 1), ratio: 1)").is_empty());
        one(&run("Room(name: \"r\", size: (1, 1), capacity: 1.5)"), "content_field_type", "capacity");
    }

    #[test]
    fn range_is_inclusive() {
        for ok in ["0", "500", "250"] {
            let f = run(&format!("Room(name: \"r\", size: (1, 1), capacity: {ok})"));
            assert!(f.is_empty(), "{ok}: {f:?}");
        }
        one(&run("Room(name: \"r\", size: (1, 1), capacity: -1)"), "content_field_range", "-1");
        one(&run("Room(name: \"r\", size: (1, 1), capacity: 501)"), "content_field_range", "501");
        assert!(run("Room(name: \"r\", size: (1, 1), ratio: 0.5)").is_empty());
        assert!(run("Room(name: \"r\", size: (1, 1), ratio: 1.5)").is_empty());
        one(&run("Room(name: \"r\", size: (1, 1), ratio: 1.6)"), "content_field_range", "ratio");
    }

    #[test]
    fn value_outside_closed_list() {
        one(&run("Room(name: \"r\", size: (1, 1), kind: Gym)"), "content_field_value", "Gym");
        one(&run("Room(name: \"r\", size: (1, 1), tag: \"z\")"), "content_field_value", "tag");
    }

    #[test]
    fn absent_optional_field_is_fine() {
        assert!(run("Room(name: \"r\", size: (1, 1))").is_empty());
    }
}
