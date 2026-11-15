//! Reference rules driven by the schema: internal references, distinct fields, assets and scenes.

use super::{index::{subject, Element}, schema::ContentSet, value::Value};
use crate::catalog::{Finding, Severity};
use std::collections::BTreeSet;
use std::path::Path;

fn shown(v: &Value) -> String {
    v.as_str().map_or_else(|| format!("{v:?}"), str::to_string)
}

fn error(set: &str, e: &Element, code: &str, message: String) -> Finding {
    Finding { severity: Severity::Error, subject: subject(set, &e.file, Some(&e.id)), code: code.into(), message }
}

pub fn check(set_name: &str, set: &ContentSet, elements: &[Element], findings: &mut Vec<Finding>) {
    for e in elements {
        let Some(kind) = set.kinds.get(&e.kind) else { continue };
        for (field, target) in &kind.refs {
            let Some(value) = e.value.field(field) else { continue };
            let prefix = set.kinds.get(target).and_then(|k| k.id_prefix.as_deref()).unwrap_or("");
            for item in value.scalars() {
                let Some(name) = item.as_str() else {
                    let msg = format!("`{field}` debe contener ids de texto, no {}", shown(item));
                    findings.push(error(set_name, e, "content_unknown_ref", msg));
                    continue;
                };
                let prefixed = format!("{prefix}{name}");
                let found = elements.iter().find(|o| o.id == name).or_else(|| elements.iter().find(|o| o.id == prefixed));
                match found {
                    None => {
                        let msg = format!("`{field}` hace referencia a «{name}», que no existe");
                        findings.push(error(set_name, e, "content_unknown_ref", msg));
                    }
                    Some(o) if &o.kind != target => {
                        let msg = format!("`{field}`: «{name}» debería ser {target} y es {}", o.kind);
                        findings.push(error(set_name, e, "content_wrong_kind", msg));
                    }
                    _ => {}
                }
            }
        }
        for field in &kind.distinct {
            let Some(value) = e.value.field(field) else { continue };
            let items = value.scalars();
            if let Some(i) = (1..items.len()).find(|&i| items[..i].contains(&items[i])) {
                let msg = format!("`{field}` repite el valor «{}»", shown(items[i]));
                findings.push(error(set_name, e, "content_not_distinct", msg));
            }
        }
    }
}

/// `godot_project` is the Godot project directory, already resolved against the project root.
pub fn check_external(
    set_name: &str,
    set: &ContentSet,
    elements: &[Element],
    piece_keys: &BTreeSet<String>,
    godot_project: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    for e in elements {
        let Some(kind) = set.kinds.get(&e.kind) else { continue };
        if let Some(asset) = kind.asset.as_deref().and_then(|f| e.value.field(f)) {
            let name = shown(asset);
            if asset.as_str().is_none_or(|a| !piece_keys.contains(a)) {
                let mut f = error(set_name, e, "content_asset_missing", format!("el asset «{name}» no está en el catálogo"));
                if set.greybox {
                    f.severity = Severity::Warn;
                    f.message.push_str(" (se tolera como greybox)");
                }
                findings.push(f);
            }
        }
        if let Some(scene) = kind.scene.as_deref().and_then(|f| e.value.field(f)) {
            let scene = shown(scene);
            let inside = |rest: &str| Path::new(rest).components().all(|c| matches!(c, std::path::Component::Normal(_)));
            let message = match scene.strip_prefix("res://").filter(|r| inside(r)) {
                None => Some(format!("la escena «{scene}» debe ser una ruta res:// relativa y sin «..»")),
                Some(rest) => match godot_project {
                    None => Some("no se puede comprobar la escena: falta [tools] godot_project".to_string()),
                    Some(gp) if !gp.join(rest).exists() => Some(format!("la escena «{scene}» no existe en el proyecto Godot")),
                    Some(_) => None,
                },
            };
            if let Some(m) = message {
                findings.push(error(set_name, e, "content_scene_missing", m));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    const KINDS: &str = r#"
[content.s.kinds.Room]
id = "name"
scene = "scene"
[content.s.kinds.Hatch]
id = "name"
refs = { links = "Room", owner = "Room" }
distinct = ["links"]
[content.s.kinds.Prop]
id = "name"
asset = "model"
refs = { rooms = "Room", kit = "Kit" }
[content.s.kinds.Kit]
id = "name"
id_prefix = "kit:"
"#;

    fn dir(name: &str, text: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-refs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("defs")).unwrap();
        std::fs::write(d.join("defs/a.ron"), text).unwrap();
        d
    }

    fn load(name: &str, extra: &str, text: &str) -> (PathBuf, ContentSet, Vec<Element>) {
        let toml = format!("[project]\nname = \"demo\"\n[content.s]\ndefinitions = [\"defs/*.ron\"]\n{extra}\n{KINDS}");
        let (mut sets, problems) = Config::parse(&toml).unwrap().content_sets();
        assert!(problems.is_empty(), "{problems:?}");
        let set = sets.remove("s").unwrap();
        let root = dir(name, text);
        let mut findings = Vec::new();
        let els = crate::content::index::load(&root, "s", &set, &mut findings);
        assert!(findings.is_empty(), "{findings:?}");
        (root, set, els)
    }

    fn internal(name: &str, text: &str) -> Vec<Finding> {
        let (_, set, els) = load(name, "", text);
        let mut f = Vec::new();
        check("s", &set, &els, &mut f);
        f
    }

    fn one(f: &[Finding], code: &str, subject: &str, needle: &str) {
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!((f[0].code.as_str(), f[0].subject.as_str(), f[0].severity), (code, subject, Severity::Error));
        assert!(f[0].message.contains(needle), "{}", f[0].message);
    }

    #[test]
    fn valid_set_has_no_findings() {
        let f = internal("ok", r#"[
 Room(name: "r1", scene: "res://a.tscn"), Room(name: "r2", scene: "res://b.tscn"),
 Kit(name: "k1"),
 Hatch(name: "h1", links: ("r1", "r2"), owner: "r1"),
 Prop(name: "p1", model: "m", rooms: ["r1", "r2"], kit: "k1"),
]"#);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn unknown_ref_in_tuple_list_and_single() {
        let rooms = "Room(name: \"r1\", scene: \"x\"), Kit(name: \"k1\"),";
        let f = internal("u1", &format!("[{rooms} Hatch(name: \"h\", links: (\"r1\", \"ghost\"))]"));
        one(&f, "content_unknown_ref", "s:defs/a.ron#h", "ghost");
        assert!(f[0].message.contains("links"));
        let f = internal("u2", &format!("[{rooms} Prop(name: \"p\", model: \"m\", rooms: [\"r1\", \"nope\"])]"));
        one(&f, "content_unknown_ref", "s:defs/a.ron#p", "nope");
        let f = internal("u3", &format!("[{rooms} Hatch(name: \"h\", owner: \"zzz\")]"));
        one(&f, "content_unknown_ref", "s:defs/a.ron#h", "zzz");
    }

    #[test]
    fn wrong_kind() {
        let f = internal("wk", "[Room(name: \"r1\", scene: \"x\"), Kit(name: \"k1\"), Hatch(name: \"h\", owner: \"kit:k1\")]");
        one(&f, "content_wrong_kind", "s:defs/a.ron#h", "kit:k1");
        assert!(f[0].message.contains("Room") && f[0].message.contains("Kit"));
    }

    #[test]
    fn prefix_is_optional_in_references() {
        let f = internal("pre", "[Kit(name: \"k1\"), Prop(name: \"p\", model: \"m\", kit: \"k1\"), Prop(name: \"q\", model: \"m\", kit: \"kit:k1\")]");
        assert!(f.is_empty(), "{f:?}");
        let f = internal("pre2", "[Kit(name: \"k1\"), Prop(name: \"p\", model: \"m\", kit: \"k2\")]");
        one(&f, "content_unknown_ref", "s:defs/a.ron#p", "k2");
    }

    #[test]
    fn non_string_ref_is_reported_and_absent_is_not() {
        let f = internal("ns", "[Hatch(name: \"h\", owner: 3)]");
        one(&f, "content_unknown_ref", "s:defs/a.ron#h", "owner");
        assert!(internal("abs", "[Hatch(name: \"h\")]").is_empty());
    }

    #[test]
    fn not_distinct() {
        let f = internal("nd", "[Room(name: \"r1\", scene: \"x\"), Hatch(name: \"h\", links: (\"r1\", \"r1\"))]");
        one(&f, "content_not_distinct", "s:defs/a.ron#h", "r1");
        assert!(f[0].message.contains("links"));
    }

    fn external(name: &str, extra: &str, text: &str, keys: &[&str], gp: Option<&str>, create: &[&str]) -> Vec<Finding> {
        let (root, set, els) = load(name, extra, text);
        let keys: BTreeSet<String> = keys.iter().map(|k| k.to_string()).collect();
        for c in create {
            std::fs::create_dir_all(root.join(c).parent().unwrap()).unwrap();
            std::fs::write(root.join(c), "").unwrap();
        }
        let gp = gp.map(|g| root.join(g));
        let mut f = Vec::new();
        check_external("s", &set, &els, &keys, gp.as_deref(), &mut f);
        f
    }

    #[test]
    fn asset_missing_is_error_or_warn_with_greybox() {
        let text = "[Prop(name: \"p\", model: \"m\"), Prop(name: \"q\", model: \"known\")]";
        let f = external("a1", "", text, &["known"], None, &[]);
        one(&f, "content_asset_missing", "s:defs/a.ron#p", "m");
        let f = external("a2", "greybox = true", text, &["known"], None, &[]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!((f[0].code.as_str(), f[0].severity), ("content_asset_missing", Severity::Warn));
    }

    fn scene(name: &str, value: &str, gp: Option<&str>, create: &[&str]) -> Vec<Finding> {
        external(name, "", &format!("[Room(name: \"r\", scene: \"{value}\")]"), &[], gp, create)
    }

    #[test]
    fn scene_existing_or_missing() {
        let ok = scene("s1", "res://m/a.tscn", Some("g"), &["g/m/a.tscn"]);
        assert!(ok.is_empty(), "{ok:?}");
        let f = scene("s2", "res://m/a.tscn", Some("g"), &[]);
        one(&f, "content_scene_missing", "s:defs/a.ron#r", "no existe");
    }

    #[test]
    fn scene_path_must_be_res_and_inside() {
        let f = scene("s3", "res://../a.tscn", Some("g"), &[]);
        one(&f, "content_scene_missing", "s:defs/a.ron#r", "res://");
        let f = scene("s4", "a.tscn", Some("g"), &[]);
        one(&f, "content_scene_missing", "s:defs/a.ron#r", "res://");
    }

    #[test]
    fn scene_without_godot_project() {
        let f = scene("s5", "res://a.tscn", None, &[]);
        one(&f, "content_scene_missing", "s:defs/a.ron#r", "godot_project");
    }
}
