//! The content schema a project declares under `[content.<set>]` in `kiln.toml`.

use crate::config::Config;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSet {
    /// Globs, relative to the project root, of the definition files.
    pub definitions: Vec<String>,
    #[serde(default)]
    pub manifest: Option<String>,
    /// A missing asset is a warning instead of an error.
    #[serde(default)]
    pub greybox: bool,
    /// Godot node metadata that holds the node id.
    #[serde(default = "default_id_meta")]
    pub id_meta: String,
    /// The file is a record and its elements are this list field.
    #[serde(default)]
    pub root: Option<String>,
    /// Kind of the unnamed elements.
    #[serde(default)]
    pub default_kind: Option<String>,
    #[serde(default)]
    pub kinds: BTreeMap<String, Kind>,
}

fn default_id_meta() -> String {
    "content_id".into()
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kind {
    /// Field that holds the element id.
    pub id: String,
    /// The id in the scene is prefix + value.
    #[serde(default)]
    pub id_prefix: Option<String>,
    /// Field with a `res://` scene that must exist and contain the node.
    #[serde(default)]
    pub scene: Option<String>,
    /// Field whose value must be a catalog piece key.
    #[serde(default)]
    pub asset: Option<String>,
    /// Field -> kind that every value (single, tuple or list) must refer to.
    #[serde(default)]
    pub refs: BTreeMap<String, String>,
    /// Fields whose values must all differ.
    #[serde(default)]
    pub distinct: Vec<String>,
    /// Field -> Godot metadata key compared with the scene node.
    #[serde(default)]
    pub node_meta: BTreeMap<String, String>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub fields: Vec<FieldRule>,
}

/// A list field compared with child nodes of the scene node.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub field: String,
    /// Item field with the marker name.
    pub name: String,
    /// Expected child node name; `{name}` stands for the marker name.
    pub node: String,
    /// Item fields compared with the child's metadata of the same name.
    #[serde(default)]
    pub meta: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Ident,
    List,
    Tuple,
    Struct,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRule {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, rename = "type")]
    pub ty: Option<FieldType>,
    #[serde(default)]
    pub one_of: Option<Vec<String>>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

/// Why a set of the schema cannot be used; becomes a `content_schema` finding.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaProblem {
    pub set: String,
    pub message: String,
}

impl Config {
    /// The valid content sets, and a problem for each one that is not.
    pub fn content_sets(&self) -> (BTreeMap<String, ContentSet>, Vec<SchemaProblem>) {
        let mut sets = BTreeMap::new();
        let mut problems = Vec::new();
        let mut problem = |set: &str, message: String| {
            problems.push(SchemaProblem {
                set: set.into(),
                message,
            })
        };
        let table = match &self.content {
            None => return (sets, problems),
            Some(toml::Value::Table(t)) => t,
            Some(_) => {
                problem(
                    "content",
                    "[content] debe ser una tabla de conjuntos".into(),
                );
                return (sets, problems);
            }
        };
        for (name, raw) in table {
            match raw.clone().try_into::<ContentSet>() {
                Err(e) => problem(name, e.message().to_string()),
                Ok(set) => match set.check() {
                    Some(m) => problem(name, m),
                    None => {
                        sets.insert(name.clone(), set);
                    }
                },
            }
        }
        (sets, problems)
    }
}

impl ContentSet {
    /// The first semantic problem of the set, if any.
    fn check(&self) -> Option<String> {
        if self.definitions.is_empty() {
            return Some("definitions no puede estar vacío".into());
        }
        let declared = |k: &str| self.kinds.contains_key(k);
        match (&self.default_kind, &self.root) {
            (Some(k), _) if !declared(k) => {
                return Some(format!("default_kind `{k}` no es un tipo declarado"))
            }
            (None, Some(_)) => return Some("root exige default_kind".into()),
            _ => {}
        }
        for (name, kind) in &self.kinds {
            if let Some(t) = kind.refs.values().find(|t| !declared(t)) {
                return Some(format!(
                    "{name}: refs apunta al tipo `{t}`, que no está declarado"
                ));
            }
            if let Some(m) = kind.markers.iter().find(|m| !m.node.contains("{name}")) {
                return Some(format!(
                    "{name}: el patrón node `{}` no contiene {{name}}",
                    m.node
                ));
            }
            for f in &kind.fields {
                if matches!((f.min, f.max), (Some(a), Some(b)) if a > b) {
                    return Some(format!("{name}.{}: min es mayor que max", f.name));
                }
                if f.one_of.as_ref().is_some_and(Vec::is_empty) {
                    return Some(format!("{name}.{}: one_of no puede estar vacío", f.name));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "[project]\nname = \"demo\"\n";

    fn sets(toml: &str) -> (BTreeMap<String, ContentSet>, Vec<SchemaProblem>) {
        Config::parse(&format!("{BASE}{toml}"))
            .unwrap()
            .content_sets()
    }

    /// One set `s` with a valid kind `K` plus the given extra lines.
    fn one(
        set_extra: &str,
        kind_extra: &str,
    ) -> (BTreeMap<String, ContentSet>, Vec<SchemaProblem>) {
        sets(&format!(
            "[content.s]\ndefinitions = [\"a/*.ron\"]\n{set_extra}\n[content.s.kinds.K]\nid = \"key\"\n{kind_extra}\n"
        ))
    }

    fn problem(r: (BTreeMap<String, ContentSet>, Vec<SchemaProblem>), needle: &str) {
        assert!(r.0.is_empty(), "set should be rejected: {:?}", r.0);
        assert_eq!(r.1.len(), 1, "{:?}", r.1);
        assert_eq!(r.1[0].set, "s");
        assert!(
            r.1[0].message.contains(needle),
            "{} lacks {needle}",
            r.1[0].message
        );
    }

    const FULL: &str = r#"
[content.station]
definitions = ["content/astra/**/*.ron"]
manifest = "kiln/content_manifest.json"
greybox = true
id_meta = "astra_id"
root = "modules"
default_kind = "Module"

[content.station.kinds.Module]
id = "key"
id_prefix = "sector:"
scene = "scene"
asset = "asset"
refs = { sector = "Sector", objects = "Sector" }
distinct = ["connects"]
node_meta = { connects = "astra_connects" }

[[content.station.kinds.Module.markers]]
field = "connectors"
name = "name"
node = "conn_{name}"
meta = ["side", "offset"]

[[content.station.kinds.Module.fields]]
name = "size"
required = true
type = "tuple"
[[content.station.kinds.Module.fields]]
name = "kind"
required = true
type = "ident"
one_of = ["Access", "Workshop"]
[[content.station.kinds.Module.fields]]
name = "capacity"
type = "int"
min = 0
max = 500

[content.station.kinds.Sector]
id = "id"
"#;

    #[test]
    fn full_example_parses() {
        let (map, problems) = sets(FULL);
        assert!(problems.is_empty(), "{problems:?}");
        let s = &map["station"];
        assert_eq!(s.definitions, vec!["content/astra/**/*.ron".to_string()]);
        assert_eq!(s.manifest.as_deref(), Some("kiln/content_manifest.json"));
        assert!(s.greybox);
        assert_eq!(
            (s.id_meta.as_str(), s.root.as_deref()),
            ("astra_id", Some("modules"))
        );
        let k = &s.kinds["Module"];
        assert_eq!(k.id_prefix.as_deref(), Some("sector:"));
        assert_eq!(k.refs["objects"], "Sector");
        assert_eq!(k.node_meta["connects"], "astra_connects");
        assert_eq!(k.markers[0].node, "conn_{name}");
        assert_eq!(
            k.markers[0].meta,
            vec!["side".to_string(), "offset".to_string()]
        );
        assert_eq!(k.fields.len(), 3);
        assert_eq!(k.fields[0].ty, Some(FieldType::Tuple));
        assert_eq!(k.fields[1].one_of.as_ref().map(Vec::len), Some(2));
        assert_eq!((k.fields[2].min, k.fields[2].max), (Some(0.0), Some(500.0)));
    }

    #[test]
    fn defaults_apply() {
        let (map, problems) = one("", "");
        assert!(problems.is_empty(), "{problems:?}");
        let s = &map["s"];
        assert!(!s.greybox);
        assert_eq!(s.id_meta, "content_id");
        assert!(s.manifest.is_none() && s.root.is_none() && s.default_kind.is_none());
    }

    #[test]
    fn no_content_is_empty() {
        let (map, problems) = sets("");
        assert!(map.is_empty() && problems.is_empty());
    }

    #[test]
    fn example_project_still_loads() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kiln.toml");
        let c = Config::load_file(&path).unwrap();
        let (map, problems) = c.content_sets();
        assert!(map.is_empty() && problems.is_empty());
    }

    #[test]
    fn unknown_keys_are_problems() {
        problem(one("typo = 1", ""), "typo");
        problem(one("", "typo = 1"), "typo");
        problem(
            one(
                "",
                "markers = [{ field = \"f\", name = \"n\", node = \"{name}\", typo = 1 }]",
            ),
            "typo",
        );
        problem(one("", "fields = [{ name = \"f\", typo = 1 }]"), "typo");
    }

    #[test]
    fn semantic_problems() {
        problem(sets("[content.s]\ndefinitions = []\n"), "definitions");
        problem(one("default_kind = \"Nope\"", ""), "Nope");
        problem(one("", "refs = { a = \"Nope\" }"), "Nope");
        problem(one("root = \"items\"", ""), "default_kind");
        problem(
            one(
                "",
                "markers = [{ field = \"f\", name = \"n\", node = \"conn\" }]",
            ),
            "{name}",
        );
        problem(
            one("", "fields = [{ name = \"f\", min = 5, max = 1 }]"),
            "min",
        );
        problem(
            one("", "fields = [{ name = \"f\", one_of = [] }]"),
            "one_of",
        );
        problem(
            one("", "fields = [{ name = \"f\", type = \"blob\" }]"),
            "blob",
        );
    }

    #[test]
    fn broken_set_does_not_hide_good_one() {
        let (map, problems) = sets(
            "[content.good]\ndefinitions = [\"a\"]\n[content.bad]\ndefinitions = [\"a\"]\ntypo = 1\n",
        );
        assert_eq!(map.keys().collect::<Vec<_>>(), vec!["good"]);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].set, "bad");
    }
}
