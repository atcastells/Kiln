//! Canonical geometry, via A of K013 (spec R6, table *Casos*): a resolved
//! recipe goes through `generate.gd` and the real Godot export, the GLB is
//! read with `mesh::read`, built with `canonical::build` and checked with
//! `canonical::validate`, as `kiln verify` does. Expected verdicts are those
//! of ASTRA `.sdlc/canonical-geometry/measurements.md` §8. Via B, at the
//! end, feeds real GLBs to `canonical::validate` without `build`.
//! Needs Godot on the PATH and `KILN_CANONICAL_RECIPES` pointing at a folder
//! with `asteroid_{32,64,128}/recipe.json` (ASTRA `godot/assets/kiln`):
//! `cargo test -p kiln-cli --test godot_canonical -- --ignored --nocapture`.
mod common;

#[path = "common/canonical.rs"]
mod canonical_run;

use canonical_run::{run_all, Case};
use common::empty_dir;
use kiln_core::canonical;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// `asteroid_64` seed 42, the fixture of `kiln-core/tests/fixtures/canonical`.
const SEED42_SHA: &str = "2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a";

fn recipe(dir: &Path, n: u32) -> Value {
    let path = dir.join(format!("asteroid_{n}/recipe.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// The 66 positives and 2 negatives, built like ASTRA `measure/dump.gd` but
/// keeping `facet`: via A is the real recipe. Ops: 0 sphere, 1 stretch, 2..=4 noise.
fn cases() -> Vec<Case> {
    let dir = PathBuf::from(std::env::var("KILN_CANONICAL_RECIPES").expect(
        "KILN_CANONICAL_RECIPES must name the folder with asteroid_{32,64,128}/recipe.json (ASTRA godot/assets/kiln)",
    ));
    let mut out = Vec::new();
    let (mut base, mut material) = (Value::Null, Value::Null);
    for n in [32, 64, 128] {
        for s in recipe(&dir, n)["seeds"].as_array().unwrap() {
            let seed = s["seed"].as_u64().unwrap();
            if n == 64 && seed == 42 {
                (base, material) = (s["ops"].clone(), s["material"].clone());
            }
            let (material, ops) = (s["material"].clone(), s["ops"].clone());
            out.push(Case { name: format!("pub_{n}_{seed}"), expected: "ok", material, ops });
        }
    }
    let derived = |f: &dyn Fn(&mut Value)| {
        let mut ops = base.clone();
        f(&mut ops);
        ops
    };
    for radius in [10.0, 100.0, 1000.0, 10000.0] {
        for segments in [32.0, 64.0, 128.0, 256.0] {
            let ops = derived(&|o| o[0]["params"] = json!({"radius_m": radius, "segments": segments}));
            out.push(Case { name: format!("scale_r{radius}_s{segments}"), expected: "ok", material: material.clone(), ops });
        }
    }
    let ops = derived(&|o| {
        o[0]["params"] = json!({"radius_m": 10000.0, "segments": 256.0});
        o[1]["params"] = json!({"x": 5.0, "y": 5.0, "z": 5.0});
    });
    out.push(Case { name: "max_r10000_k5_s256".into(), expected: "ok", material: material.clone(), ops });
    let ops = derived(&|o| {
        o[1]["params"] = json!({"x": 1.0, "y": 0.5, "z": 1.0});
        for (i, a) in [(2, 0.3), (3, 0.1), (4, 0.05)] {
            o[i]["params"]["amplitude"] = json!(a);
        }
    });
    out.push(Case { name: "edge_y05_a045".into(), expected: "ok", material: material.clone(), ops });
    // The two negatives of S1 §4.2. Kiln's static `generator_radius` rule
    // rejects them before Godot runs; they go straight to generate.gd on
    // purpose, so that the validator itself is what turns them down.
    let ops = derived(&|o| {
        o[1]["params"] = json!({"x": 0.2, "y": 0.2, "z": 0.2});
        for i in 2..=4 {
            o[i]["params"]["amplitude"] = json!(1.0);
        }
    });
    out.push(Case { name: "neg_k02_a1x3".into(), expected: "I8_not_radial", material: material.clone(), ops });
    let ops = derived(&|o| {
        o[1]["params"] = json!({"x": 1.0, "y": 0.2, "z": 1.0});
        o[2]["params"]["amplitude"] = json!(0.6);
    });
    out.push(Case { name: "neg_y02_a06".into(), expected: "I8_not_radial", material: material.clone(), ops });
    out
}

#[test]
#[ignore]
fn recipes_through_godot_give_the_verdicts_of_m8() {
    let cases = cases();
    assert_eq!(cases.len(), 68);
    let root = empty_dir();
    let start = std::time::Instant::now();
    let results = run_all(root.path(), &cases);
    let mut wrong = Vec::new();
    println!("| case | expected | got | geometry_sha256 |");
    for (case, got) in cases.iter().zip(&results) {
        let (verdict, sha) = match got {
            Ok(sha) => ("ok", sha.as_str()),
            Err(code) => (code.as_str(), ""),
        };
        let sha = if case.name.starts_with("pub_") { sha } else { "" };
        println!("| {} | {} | {verdict} | {sha} |", case.name, case.expected);
        if verdict != case.expected {
            wrong.push(case.name.clone());
        }
    }
    println!("wall time: {:.1} s", start.elapsed().as_secs_f64());
    let seed42 = cases.iter().position(|c| c.name == "pub_64_42").unwrap();
    assert_eq!(results[seed42].as_deref(), Ok(SEED42_SHA), "seed 42 differs from the T1 fixture");
    assert!(wrong.is_empty(), "verdicts that differ from M§8: {wrong:?}");
}

// Via B (spec, table *Casos*): a real GLB of `generate.gd` goes straight to
// `canonical::validate`, without `build`. The only conversion is the one a
// `Geometry` needs: each position quantized as its own vertex, each index kept.

fn direct(positions: &[[f32; 3]], indices: &[u32]) -> canonical::Geometry {
    let vertices = positions.iter().map(|p| p.map(|c| canonical::quantize(c).unwrap())).collect();
    let triangles = indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect();
    canonical::Geometry { vertices, triangles }
}

/// Joins only bit-identical `f32` positions (M §2), numbering by first
/// appearance and dropping triangles that repeat a vertex. `-0.0` and `0.0`
/// stay apart: that is the defect this case shows.
fn weld_bits(positions: &[[f32; 3]], indices: &[u32]) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut ids = std::collections::HashMap::new();
    let mut welded = Vec::new();
    let remap: Vec<u32> = positions
        .iter()
        .map(|p| {
            *ids.entry(p.map(f32::to_bits)).or_insert_with(|| {
                welded.push(*p);
                welded.len() as u32 - 1
            })
        })
        .collect();
    let mut out = Vec::new();
    for t in indices.chunks(3) {
        let [a, b, c] = [remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]];
        if a != b && b != c && a != c {
            out.extend([a, b, c]);
        }
    }
    (welded, out)
}

fn code(r: Result<(), canonical::Rejection>) -> String {
    r.map_or_else(|e| e.code.to_string(), |()| "ok".into())
}

#[test]
#[ignore]
fn glbs_straight_to_the_validator_give_the_verdicts_of_m8() {
    let dir = PathBuf::from(std::env::var("KILN_CANONICAL_RECIPES").expect(
        "KILN_CANONICAL_RECIPES must name the folder with asteroid_{32,64,128}/recipe.json (ASTRA godot/assets/kiln)",
    ));
    let seed42 = recipe(&dir, 64)["seeds"].as_array().unwrap().iter().find(|s| s["seed"] == 42).unwrap().clone();
    let material = seed42["material"].clone();
    let faceted = seed42["ops"].clone();
    let smooth: Vec<Value> = faceted.as_array().unwrap().iter().filter(|o| o["op"] != "facet").cloned().collect();
    assert_eq!(smooth.len() + 1, faceted.as_array().unwrap().len(), "seed 42 has one facet op");
    let sphere = json!([{"op": "sphere", "params": {"radius_m": 1000.0, "segments": 64.0}}]);
    let root = empty_dir();
    let glb = |name: &str, ops: Value| {
        let case = Case { name: name.into(), expected: "", material: material.clone(), ops };
        canonical_run::export(root.path(), &case).unwrap()
    };
    let (fac_v, fac_i) = glb("b_seed42_facet", faceted.clone());
    let (smo_v, smo_i) = glb("b_seed42_smooth", Value::Array(smooth));
    let (sph_v, sph_i) = glb("b_sphere_r1000_s64", sphere);

    let mut rows: Vec<(&str, &str, &str, String)> = Vec::new();
    // 1. asteroid_64 seed 42 without its `facet` op: the smooth indexed arrays
    // as exported (2210 positions, seam column and pole rows unjoined).
    rows.push(("smooth_unprepared", "asteroid_64 seed 42, 64 seg, no facet", "I3_duplicate_position", code(canonical::validate(&direct(&smo_v, &smo_i)))));
    // 2. asteroid_64 seed 42 as published, with `facet`: three positions per
    // triangle, none shared (12672 positions).
    rows.push(("faceted_unshared", "asteroid_64 seed 42, 64 seg, facet", "I3_duplicate_position", code(canonical::validate(&direct(&fac_v, &fac_i)))));
    // 3. Bare `sphere` r 1000 m, 64 seg, no facet: the SphereMesh lattice as
    // exported, unwelded. M built it from seed 42 in lattice order; through a
    // GLB that is byte for byte case 1, so this case uses the base sphere.
    rows.push(("lattice_unwelded", "sphere r1000, 64 seg, no facet", "I3_duplicate_position", code(canonical::validate(&direct(&sph_v, &sph_i)))));
    // 4. The same base-sphere GLB welded bit by bit (M §2: 2210 -> 2056, the
    // poles keep signed zeros apart). No winding flip: the GLB faces outwards.
    let (wv, wi) = weld_bits(&sph_v, &sph_i);
    assert_eq!((sph_v.len(), wv.len()), (2210, 2056), "bit-by-bit weld counts of M §2");
    rows.push(("sphere_welded_bitwise", "sphere r1000, 64 seg, no facet", "I3_duplicate_position", code(canonical::validate(&direct(&wv, &wi)))));
    // 5. Godot's triangle order. M measured it on the raw arrays, which face
    // inwards; per decision 001 the GLB already faces outwards, so the case is
    // built from asteroid_64 seed 42 with `facet`: normal `build`, then every
    // triangle's winding reversed.
    let mut g = canonical::build(&fac_v, &fac_i, 64).unwrap();
    assert_eq!(canonical::validate(&g).map_err(|r| r.code), Ok(()), "the unreversed build passes");
    for t in &mut g.triangles {
        t.swap(1, 2);
    }
    rows.push(("godot_order_reversed", "asteroid_64 seed 42, 64 seg, facet; build + reverse", "I8_not_radial", code(canonical::validate(&g))));

    println!("| case | source GLB | expected | got |");
    for (case, source, expected, got) in &rows {
        println!("| {case} | {source} | {expected} | {got} |");
    }
    let wrong: Vec<_> = rows.iter().filter(|r| r.2 != r.3).map(|r| r.0).collect();
    assert!(wrong.is_empty(), "verdicts that differ from M§8: {wrong:?}");
}
