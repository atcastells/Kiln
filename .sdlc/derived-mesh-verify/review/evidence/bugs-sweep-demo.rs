use kiln_core::canonical::Geometry;
use kiln_core::integrity::{check, intersect::triangles_cross, Welded};
use kiln_core::spec::{MeshRules, Normals};
use std::time::Instant;

fn welded(vertices: Vec<[i32; 3]>, triangles: Vec<[u32; 3]>) -> Welded {
    let origin = (0..triangles.len()).map(|t| (0, t)).collect();
    let first_use = (0..vertices.len() as u32).map(|v| (0, v)).collect();
    Welded { geometry: Geometry { vertices, triangles }, origin, first_use }
}

/// Capped cylinder, k sides, m rings, axis 0=x 1=y 2=z, outward.
fn tube(k: u32, m: u32, axis: usize, radius: f64, len: f64) -> Welded {
    let mut vs: Vec<[i32; 3]> = Vec::new();
    let place = |u: f64, v: f64, w: f64| -> [i32; 3] {
        let p = match axis { 0 => [u, v, w], 1 => [w, u, v], _ => [v, w, u] };
        p.map(|c| c.round() as i32)
    };
    for r in 0..=m {
        for s in 0..k {
            let th = std::f64::consts::TAU * s as f64 / k as f64;
            vs.push(place(len * r as f64 / m as f64, radius * th.cos(), radius * th.sin()));
        }
    }
    let c0 = vs.len() as u32; vs.push(place(0., 0., 0.));
    let c1 = vs.len() as u32; vs.push(place(len, 0., 0.));
    let at = |r: u32, s: u32| r * k + s % k;
    let mut ts = Vec::new();
    for r in 0..m { for s in 0..k {
        let (a, b, c, d) = (at(r, s), at(r, s + 1), at(r + 1, s + 1), at(r + 1, s));
        ts.push([a, b, d]); ts.push([b, c, d]);
    }}
    for s in 0..k {
        ts.push([c0, at(0, s + 1), at(0, s)]);
        ts.push([c1, at(m, s), at(m, s + 1)]);
    }
    welded(vs, ts)
}

fn main() {
    let rules = MeshRules { components: 1, genus: 0, normals: Normals::Outward };
    for (k, m) in [(2500u32, 1u32), (5000, 1), (10000, 1)] {
        for axis in [0usize] {
            let w = tube(k, m, axis, 1_000_000., 1_000_000.);
            let n = w.geometry.triangles.len();
            let t = Instant::now();
            let r = check(&w, &rules);
            println!("tube axis {} n={n}: {:?} in {:?}", ["x","y","z"][axis], r.as_ref().map_err(|e| (e.code, e.detail.chars().take(80).collect::<String>())), t.elapsed());
        }
    }
    // cap flat in x: one big disc plate perpendicular to x is the same worst case; see tube axis y/z
    let _ = triangles_cross;
}
