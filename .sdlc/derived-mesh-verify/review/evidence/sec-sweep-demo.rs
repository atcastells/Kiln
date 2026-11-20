use kiln_core::integrity::{check, weld};
use kiln_core::spec::{MeshRules, Normals};
use serde_json::json;
use std::time::Instant;

fn glb(pos: &[[f32; 3]], ix: &[u32]) -> Vec<u8> {
    let mut bin: Vec<u8> = pos.iter().flatten().flat_map(|c| c.to_le_bytes()).collect();
    let at = bin.len();
    bin.extend(ix.iter().flat_map(|i| i.to_le_bytes()));
    let doc = json!({
        "asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "body", "mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": pos.len(), "type": "VEC3"},
            {"bufferView": 1, "componentType": 5125, "count": ix.len(), "type": "SCALAR"}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": at},
            {"buffer": 0, "byteOffset": at, "byteLength": ix.len() * 4}],
        "buffers": [{"byteLength": bin.len()}]
    });
    let mut j = doc.to_string().into_bytes();
    j.resize(j.len().next_multiple_of(4), b' ');
    bin.resize(bin.len().next_multiple_of(4), 0);
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend((28 + j.len() as u32 + bin.len() as u32).to_le_bytes());
    out.extend((j.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(j);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004E_4942u32.to_le_bytes());
    out.extend(bin);
    out
}

/// Closed prism along x: two rings of k points (radius r mm) at x = 0 and x = len mm,
/// each cap a fan from its centre. `flip` reverses every triangle.
fn prism(k: u32, r: f64, len: f32, flip: bool) -> Vec<u8> {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    for x in [0.0f32, len] {
        for i in 0..k {
            let a = std::f64::consts::TAU * i as f64 / k as f64;
            pos.push([x, (r * a.cos()).round() as f32 / 1000.0, (r * a.sin()).round() as f32 / 1000.0]);
        }
    }
    pos.push([0.0, 0.0, 0.0]);
    pos.push([len, 0.0, 0.0]);
    let (c0, c1) = (2 * k, 2 * k + 1);
    let mut t: Vec<[u32; 3]> = Vec::new();
    for i in 0..k {
        let j = (i + 1) % k;
        let (a, b, c, d) = (i, j, k + j, k + i);
        t.push([a, b, c]);
        t.push([a, c, d]);
        t.push([c0, j, i]);
        t.push([c1, k + i, k + j]);
    }
    if flip { t.iter_mut().for_each(|x| x.swap(1, 2)); }
    glb(&pos, &t.concat())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let k: u32 = args[1].parse().unwrap();
    let flip = args.get(2).is_some_and(|s| s == "flip");
    let bytes = prism(k, std::env::var("R").ok().and_then(|r| r.parse().ok()).unwrap_or(100_000.0), 1.0, flip);
    if let Some(p) = args.get(3) { std::fs::write(p, &bytes).unwrap(); }
    let w = weld(&bytes).expect("readable").unwrap_or_else(|r| panic!("weld {} {}", r.code, r.detail));
    let rules = MeshRules { components: 1, genus: 0, normals: Normals::Outward };
    let t = Instant::now();
    let r = check(&w, &rules);
    println!("k={k} glb_bytes={} tris={} verts={} result={:?} time={:?}",
        bytes.len(), w.geometry.triangles.len(), w.geometry.vertices.len(),
        r.as_ref().map_err(|e| format!("{} {}", e.code, e.detail.chars().take(80).collect::<String>())), t.elapsed());
}
