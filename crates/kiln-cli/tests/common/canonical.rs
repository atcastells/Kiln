//! Runs a resolved recipe through `generate.gd` as `kiln verify` does (same
//! scratch project and job shape as `generate::run_seed`, which a test cannot
//! call: kiln-cli has no lib) and gives the canonical verdict of its GLB.
#![allow(dead_code)]

use kiln_core::canonical;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const SCRIPT: &str = include_str!("../../godot/generate.gd");
const BUILDER: &str = include_str!("../../godot/kiln_generator.gd");
const PROJECT: &str = "config_version=5\n\n[application]\n\nconfig/name=\"kiln-verify\"\n";
const WORKERS: usize = 6;

pub struct Case {
    pub name: String,
    pub expected: &'static str,
    pub material: Value,
    pub ops: Value,
}

/// The verdict of `verify` for one case: `Ok(geometry_sha256)` or the rejection code.
pub fn verdict(root: &Path, case: &Case) -> Result<String, String> {
    let (positions, indices) = export(root, case)?;
    let segments = case.ops[0]["params"]["segments"].as_f64().unwrap() as u32;
    let g = canonical::build(&positions, &indices, segments).map_err(|r| r.code.to_string())?;
    canonical::validate(&g).map_err(|r| r.code.to_string())?;
    Ok(canonical::geometry_sha256(&g))
}

/// The GLB that `generate.gd` exports for one case, read with `mesh::read`
/// after the same node and mesh-count checks as `verify`.
pub fn export(root: &Path, case: &Case) -> Result<(Vec<[f32; 3]>, Vec<u32>), String> {
    let work = root.join(&case.name);
    std::fs::create_dir_all(&work).unwrap();
    for (file, text) in [("project.godot", PROJECT), ("generate.gd", SCRIPT), ("kiln_generator.gd", BUILDER)] {
        std::fs::write(work.join(file), text).unwrap();
    }
    let glb = work.join("seed.glb");
    let job = json!({"id": case.name, "glb": glb, "material": case.material, "ops": case.ops});
    std::fs::write(work.join("job.json"), job.to_string()).unwrap();
    let run = Command::new("godot")
        .arg("--headless")
        .arg("--path")
        .arg(&work)
        .arg("--script")
        .arg(work.join("generate.gd"))
        .arg("--")
        .arg(work.join("job.json"))
        .arg(work.join("out.json"))
        .output()
        .expect("godot on the PATH");
    if !run.status.success() || !glb.is_file() {
        return Err(format!("godot failed: {}", String::from_utf8_lossy(&run.stderr)));
    }
    if let Some(t) = kiln_core::mesh::node_transform(&glb)? {
        return Err(format!("canonical_node_transform ({t})"));
    }
    let meshes = kiln_core::mesh::read(&glb)?;
    let [mesh] = meshes.as_slice() else { return Err(format!("canonical_mesh_count ({})", meshes.len())) };
    Ok((mesh.positions.clone(), mesh.indices.clone()))
}

/// Runs every case on `WORKERS` threads, each case in its own Godot project under `root`.
pub fn run_all(root: &Path, cases: &[Case]) -> Vec<Result<String, String>> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Result<String, String>>>> = Mutex::new(vec![None; cases.len()]);
    std::thread::scope(|s| {
        for _ in 0..WORKERS {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(case) = cases.get(i) else { break };
                let v = verdict(root, case);
                results.lock().unwrap()[i] = Some(v);
            });
        }
    });
    results.into_inner().unwrap().into_iter().map(Option::unwrap).collect()
}
