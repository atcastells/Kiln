//! Writes the canonical container of a 64-segment GLB and prints its hash:
//! `cargo run -p kiln-core --example canonical_fixture -- <GLB> <OUT.bin>`.

use kiln_core::{canonical, mesh};
use std::path::Path;
use std::process::ExitCode;

fn run(glb: &str, out: &str) -> Result<String, String> {
    let meshes = mesh::read(Path::new(glb))?;
    let [m] = meshes.as_slice() else {
        return Err(format!("{glb}: {} meshes, expected exactly one", meshes.len()));
    };
    let reject = |r: canonical::Rejection| format!("{} (element {:?}): {}", r.code, r.element, r.detail);
    let g = canonical::build(&m.positions, &m.indices, 64).map_err(reject)?;
    canonical::validate(&g).map_err(reject)?;
    std::fs::write(out, canonical::to_bytes(&g)).map_err(|e| format!("{out}: {e}"))?;
    Ok(canonical::geometry_sha256(&g))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [glb, out] = args.as_slice() else {
        eprintln!("usage: canonical_fixture <GLB> <OUT.bin>");
        return ExitCode::from(2);
    };
    match run(glb, out) {
        Ok(sha) => {
            println!("geometry_sha256 {sha}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
