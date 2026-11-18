//! Hash of a recipe: publishing checks that the recipe is the one verified.

use serde_json::{Map, Value};

use super::Generator;

/// SHA-256 (hex) of the canonical JSON of `{"budget_tris", "generator"}`.
///
/// Canonical: object keys sorted at every level, no whitespace, numbers as
/// serde_json prints them; the order of the ops is kept. The same recipe gives
/// the same hash however its TOML was written, so publishing can check that the
/// recipe being published is the one that was verified.
pub fn recipe_sha256(g: &Generator, budget_tris: Option<u64>) -> String {
    let mut root = Map::new();
    root.insert("budget_tris".into(), serde_json::json!(budget_tris));
    root.insert(
        "generator".into(),
        serde_json::to_value(g).expect("a recipe always serializes"),
    );
    let text = serde_json::to_string(&sorted(Value::Object(root)))
        .expect("a JSON value always serializes");
    crate::asset::sha256_hex(text.as_bytes())
}

/// Rebuilds the value with sorted keys, whatever the map implementation.
fn sorted(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(String, Value)> = m.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(sorted).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use crate::asset::sha256_hex;
    use crate::spec;

    use super::*;

    fn recipe(head: &str, ops: &str) -> Generator {
        let text = format!("[asset]\nid = \"a\"\nname = \"A\"\n[generator]\n{head}\n{ops}");
        spec::parse(&text).unwrap().generator.unwrap()
    }

    const HEAD: &str = "seed = 7\nmaterial = \"basalt\"";
    const SPHERE: &str = "[[generator.ops]]\nop = \"sphere\"\nradius_m = 12\nsegments = 16\n";
    const NOISE: &str = "[[generator.ops]]\nop = \"noise\"\namplitude = 0.1\nsize = 0.5\n";

    fn base() -> String {
        recipe(HEAD, SPHERE).pipe_hash(Some(600))
    }

    trait Pipe {
        fn pipe_hash(&self, b: Option<u64>) -> String;
    }
    impl Pipe for Generator {
        fn pipe_hash(&self, b: Option<u64>) -> String {
            recipe_sha256(self, b)
        }
    }

    #[test]
    fn it_is_64_lowercase_hex() {
        let h = base();
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    }

    #[test]
    fn writing_order_does_not_matter() {
        let swapped = "[[generator.ops]]\nsegments = 16\nradius_m = 12\nop = \"sphere\"\n";
        assert_eq!(base(), recipe(HEAD, swapped).pipe_hash(Some(600)));
        let head2 = "material = \"basalt\"\nseed = 7";
        assert_eq!(base(), recipe(head2, SPHERE).pipe_hash(Some(600)));
    }

    #[test]
    fn integer_and_float_spelling_match() {
        let float = SPHERE.replace("radius_m = 12", "radius_m = 12.0");
        assert_eq!(base(), recipe(HEAD, &float).pipe_hash(Some(600)));
    }

    #[test]
    fn it_changes_with_the_recipe() {
        let b = base();
        let h = |head: &str, ops: &str, budget| recipe(head, ops).pipe_hash(budget);
        let value = SPHERE.replace("radius_m = 12", "radius_m = 13");
        let range = SPHERE.replace("radius_m = 12", "radius_m = [12, 13]");
        let two = format!("{SPHERE}{NOISE}");
        let two_swapped = format!("{NOISE}{SPHERE}");
        let changed = [
            h(HEAD, &value, Some(600)),
            h(HEAD, &range, Some(600)),
            h("seed = 8\nmaterial = \"basalt\"", SPHERE, Some(600)),
            h("seed = 7\nmaterial = \"ice\"", SPHERE, Some(600)),
            h("seed = 7", SPHERE, Some(600)),
            h(HEAD, &two, Some(600)),
            h(HEAD, SPHERE, Some(601)),
            h(HEAD, SPHERE, None),
        ];
        for c in &changed {
            assert_ne!(&b, c);
        }
        assert_ne!(h(HEAD, &two, Some(600)), h(HEAD, &two_swapped, Some(600)));
    }

    #[test]
    fn the_canonical_form_is_pinned() {
        let g = recipe(HEAD, SPHERE);
        let expected = br#"{"budget_tris":600,"generator":{"material":"basalt","ops":[{"op":"sphere","radius_m":12.0,"segments":16.0}],"seed":7}}"#;
        assert_eq!(recipe_sha256(&g, Some(600)), sha256_hex(expected));
        let none = br#"{"budget_tris":null,"generator":{"material":"basalt","ops":[{"op":"sphere","radius_m":12.0,"segments":16.0}],"seed":7}}"#;
        assert_eq!(recipe_sha256(&g, None), sha256_hex(none));
    }
}
