//! Throwaway T13 runner: for each "<name> <segments>" line of CASES, reads
//! DIR/<name>/seed.glb and prints "<name> <geometry_sha256|ERR code>".
use kiln_core::{canonical, mesh};
use std::path::Path;

fn hash(glb: &Path, segments: u32) -> Result<String, String> {
    if let Some(t) = mesh::node_transform(glb)? {
        return Err(format!("canonical_node_transform({t})"));
    }
    let meshes = mesh::read(glb)?;
    let [m] = meshes.as_slice() else { return Err(format!("canonical_mesh_count({})", meshes.len())) };
    let g = canonical::build(&m.positions, &m.indices, segments).map_err(|r| r.code.to_string())?;
    canonical::validate(&g).map_err(|r| r.code.to_string())?;
    let bytes = canonical::to_bytes(&g);
    let sha = canonical::geometry_sha256(&g);
    Ok(format!("{sha} bytes={}", bytes.len()))
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (dir, cases) = (Path::new(&a[0]), std::fs::read_to_string(&a[1]).unwrap());
    for line in cases.lines().filter(|l| !l.is_empty()) {
        let (name, seg) = line.split_once(' ').unwrap();
        let seg = seg.parse::<f64>().unwrap() as u32;
        match hash(&dir.join(name).join("seed.glb"), seg) {
            Ok(s) => println!("{name} {s}"),
            Err(e) => println!("{name} ERR {e}"),
        }
    }
}
