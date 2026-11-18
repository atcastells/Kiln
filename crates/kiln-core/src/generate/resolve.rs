//! Draws the ranges of a recipe from its seed. The draw is pure: the same
//! recipe and seed always give the same values.

use super::{vocab, Generator, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedOp {
    pub op: String,
    pub params: BTreeMap<String, f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_seed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolved {
    pub seed: u32,
    pub material: Option<String>,
    pub ops: Vec<ResolvedOp>,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// FNV-1a 64 of the key followed by one SplitMix64 finalizer step.
pub fn mix(key: &str) -> u64 {
    let mut z = fnv1a(key.as_bytes()).wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn unit(key: &str) -> f64 {
    (mix(key) >> 11) as f64 / (1u64 << 53) as f64
}

fn draw(v: &Value, key: &str, integer: bool) -> f64 {
    let x = match v {
        Value::Fixed(x) => *x,
        Value::Range([min, max]) => min + (max - min) * unit(key),
    };
    if integer {
        x.floor()
    } else {
        x
    }
}

/// The seeds a verification generates: element 0 is `seed`, element k is the low 32 bits
/// of `mix("<seed>/verify/<k>")`; a repeat is redrawn with `"<seed>/verify/<k>/<j>"`, j = 1, 2, ...
pub fn batch(seed: u32, n: u32) -> Vec<u32> {
    let mut seeds: Vec<u32> = Vec::new();
    for k in 0..n {
        if k == 0 {
            seeds.push(seed);
            continue;
        }
        let mut s = (mix(&format!("{seed}/verify/{k}")) & 0xffff_ffff) as u32;
        let mut j = 1;
        while seeds.contains(&s) {
            s = (mix(&format!("{seed}/verify/{k}/{j}")) & 0xffff_ffff) as u32;
            j += 1;
        }
        seeds.push(s);
    }
    seeds
}

pub fn resolve(g: &Generator, seed: u32) -> Resolved {
    let ops = g
        .ops
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let key = |p: &str| format!("{seed}/{i}/{p}");
            let mut params = BTreeMap::new();
            match vocab::op(&op.op) {
                Some(def) => {
                    for p in def.params {
                        let v = op.params.get(p.id).cloned().unwrap_or(Value::Fixed(p.default));
                        params.insert(p.id.to_string(), draw(&v, &key(p.id), p.integer));
                    }
                }
                None => {
                    for (id, v) in &op.params {
                        let low = match v {
                            Value::Fixed(x) => *x,
                            Value::Range([min, _]) => *min,
                        };
                        params.insert(id.clone(), low);
                    }
                }
            }
            let noise_seed = (op.op == "noise").then(|| (mix(&key("noise")) & 0xffff_ffff) as u32);
            ResolvedOp { op: op.op.clone(), params, noise_seed }
        })
        .collect();
    Resolved { seed, material: g.material.clone(), ops }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec;

    fn recipe(extra: &str) -> Generator {
        let text = format!(
            "[asset]\nid = \"a\"\nname = \"A\"\n[generator]\nseed = 42\n{extra}"
        );
        spec::parse(&text).unwrap().generator.unwrap()
    }

    fn asteroid() -> Generator {
        recipe(
            "[[generator.ops]]\nop = \"sphere\"\nradius_m = [8, 40]\nsegments = [32, 64]\n\
             [[generator.ops]]\nop = \"stretch\"\nx = [1.0, 1.8]\n\
             [[generator.ops]]\nop = \"noise\"\namplitude = [0.2, 0.5]\nsize = 0.6\n\
             [[generator.ops]]\nop = \"noise\"\namplitude = [0.05, 0.15]\nsize = 0.1\n\
             [[generator.ops]]\nop = \"facet\"\n",
        )
    }

    #[test]
    fn fnv_matches_the_known_vector() {
        assert_eq!(fnv1a(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(fnv1a(b""), 0xcbf29ce484222325);
    }

    #[test]
    fn mix_is_pinned() {
        assert_eq!(mix("42/0/radius_m"), 7067743086734528551);
        assert_eq!(mix("42/2/noise"), 4473741229839884888);
    }

    /// Values worked out apart from this code, from the written algorithm:
    /// what another program has to get for the same recipe and seed.
    #[test]
    fn resolved_values_are_pinned() {
        let r = resolve(&asteroid(), 42);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(near(r.ops[0].params["radius_m"], 20.26057985473117));
        assert_eq!(r.ops[0].params["segments"], 41.0);
        assert!(near(r.ops[1].params["x"], 1.2931135058245657));
        assert!(near(r.ops[2].params["amplitude"], 0.4440197061243818));
        assert_eq!(r.ops[2].noise_seed, Some(362816088));
        assert_eq!(r.ops[3].noise_seed, Some(1464571727));
    }

    #[test]
    fn fixed_stays_and_absent_takes_the_default() {
        let r = resolve(&asteroid(), 42);
        assert_eq!(r.ops[2].params["size"], 0.6);
        assert_eq!(r.ops[1].params["y"], 1.0);
        assert_eq!(r.ops[1].params["z"], 1.0);
        assert_eq!(r.material, None);
    }

    #[test]
    fn ranges_stay_inside_and_integers_are_whole() {
        for seed in 0..50 {
            let r = resolve(&asteroid(), seed);
            let radius = r.ops[0].params["radius_m"];
            assert!((8.0..=40.0).contains(&radius));
            let seg = r.ops[0].params["segments"];
            assert_eq!(seg, seg.floor());
            assert!((32.0..=64.0).contains(&seg));
            assert!((1.0..=1.8).contains(&r.ops[1].params["x"]));
        }
    }

    #[test]
    fn resolving_is_repeatable_and_the_seed_matters() {
        let g = asteroid();
        assert_eq!(resolve(&g, 42), resolve(&g, 42));
        assert_ne!(resolve(&g, 42), resolve(&g, 43));
    }

    #[test]
    fn batch_is_pinned() {
        assert_eq!(batch(42, 4), vec![42, 1227191235, 4096378883, 621036976]);
        assert_eq!(batch(7, 3), vec![7, 256104206, 3962503091]);
    }

    #[test]
    fn batch_is_distinct_extends_and_handles_the_edges() {
        let long = batch(42, 16);
        assert_eq!(long.len(), 16);
        assert_eq!(long[0], 42);
        let mut sorted = long.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 16);
        assert_eq!(long[..4], batch(42, 4)[..]);
        assert!(batch(42, 0).is_empty());
        assert_eq!(batch(5, 1), vec![5]);
    }

    #[test]
    fn noise_ops_get_their_own_seed_and_facet_none() {
        let r = resolve(&asteroid(), 42);
        assert!(r.ops[2].noise_seed.is_some());
        assert_ne!(r.ops[2].noise_seed, r.ops[3].noise_seed);
        assert_eq!(r.ops[4].noise_seed, None);
        assert_eq!(r.ops[2].noise_seed, Some((mix("42/2/noise") & 0xffff_ffff) as u32));
    }
}
