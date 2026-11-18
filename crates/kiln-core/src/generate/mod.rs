//! Assets generated in the game from a recipe: the types of the `[generator]`
//! table of a spec.

pub mod check;
pub mod resolve;
pub mod vocab;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A parameter: a fixed number or a range `[min, max]` drawn by the seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Fixed(f64),
    Range([f64; 2]),
}

/// One step of the recipe: `op = "<id>"` plus its parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Op {
    pub op: String,
    #[serde(flatten)]
    pub params: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Generator {
    #[serde(default)]
    pub seed: u32,
    /// Palette key of the material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    #[serde(default)]
    pub ops: Vec<Op>,
}

#[cfg(test)]
mod tests {
    use crate::spec;

    use super::*;

    const RECIPE: &str = r#"
[asset]
id = "asteroid"
name = "Asteroide"
budget_tris = 6000

[generator]
seed = 42
material = "basalt"

[[generator.ops]]
op = "sphere"
radius_m = [8, 40]
segments = 64

[[generator.ops]]
op = "stretch"
x = [1.0, 1.8]

[[generator.ops]]
op = "noise"
amplitude = [0.2, 0.5]
size = 0.6

[[generator.ops]]
op = "noise"
amplitude = [0.05, 0.15]
size = 0.1

[[generator.ops]]
op = "facet"
"#;

    #[test]
    fn reads_and_round_trips_the_recipe() {
        let s = spec::parse(RECIPE).unwrap();
        let g = s.generator.clone().unwrap();
        assert_eq!(g.seed, 42);
        assert_eq!(g.material.as_deref(), Some("basalt"));
        assert_eq!(g.ops.len(), 5);
        assert_eq!(g.ops[0].params["radius_m"], Value::Range([8.0, 40.0]));
        assert_eq!(g.ops[0].params["segments"], Value::Fixed(64.0));
        let text = toml::to_string(&s).unwrap();
        let again = spec::parse(&text).unwrap().generator.unwrap();
        assert_eq!(again, g);
    }

    #[test]
    fn a_spec_without_recipe_does_not_write_one() {
        let s = spec::parse("[asset]\nid = \"a\"\nname = \"A\"\n").unwrap();
        assert!(!toml::to_string(&s).unwrap().contains("generator"));
    }

    fn findings(text: &str) -> Vec<String> {
        let mut cat = crate::Catalog::default();
        spec::check(&spec::parse(text).unwrap(), &Default::default(), &mut cat);
        cat.findings.into_iter().map(|f| f.code).collect()
    }

    #[test]
    fn a_recipe_alone_is_not_without_parts() {
        assert!(!findings(RECIPE).contains(&"no_parts".to_string()));
    }

    #[test]
    fn a_recipe_with_parts_is_refused() {
        let text = format!(
            "{RECIPE}\n[[parts]]\nid = \"p\"\nwhat = \"p\"\n[parts.source]\nkind = \"reuse\"\nfrom = \"k/p\"\n"
        );
        assert!(findings(&text).contains(&"generator_and_parts".to_string()));
    }
}
