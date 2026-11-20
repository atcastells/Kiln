use kiln_core::canonical::Geometry;
use kiln_core::integrity::{check, weld, Welded};
use kiln_core::spec::{MeshRules, Normals};
use std::time::Instant;

fn welded(vertices: Vec<[i32; 3]>, triangles: Vec<[u32; 3]>) -> Welded {
    let origin = (0..triangles.len()).map(|t| (0, t)).collect();
    let first_use = (0..vertices.len() as u32).map(|v| (0, v)).collect();
    Welded { geometry: Geometry { vertices, triangles }, origin, first_use }
}

/// Prism over the exactly convex polygon i*u + i^2*v (i in -h..=h), from 0 to
/// len*dir; side triangles as long as the prism; caps as strips or fans.
fn tube(h: i32, len: i32, dir: [i32; 3], u: [i32; 3], v: [i32; 3], fan: bool) -> Welded {
    let p = (2 * h + 1) as u32;
    let mut vs = Vec::new();
    for ring in 0..2 {
        for i in -h..=h {
            vs.push([0, 1, 2].map(|k| i * u[k] + i * i * v[k] + ring * len * dir[k]));
        }
    }
    let mut ts = Vec::new();
    for a in 0..p {
        let b = (a + 1) % p;
        ts.push([a, b, b + p]);
        ts.push([a, b + p, a + p]);
    }
    let (l, r) = (|i: i32| (h - i) as u32, |i: i32| (h + i) as u32);
    let mut cap = Vec::new();
    if fan {
        for j in 1..p - 1 { cap.push([0, j, j + 1]); }
    } else {
        cap.push([l(0), r(1), l(1)]);
        for i in 1..h { cap.push([l(i), r(i), r(i + 1)]); cap.push([l(i), r(i + 1), l(i + 1)]); }
    }
    for t in &cap { ts.push([t[0], t[2], t[1]]); ts.push(t.map(|x| x + p)); }
    welded(vs, ts)
}

fn run(name: &str, w: &Welded) {
    let n = w.geometry.triangles.len();
    let t = Instant::now();
    let mut r = check(w, &MeshRules { components: 1, genus: 0, normals: Normals::Outward });
    if matches!(&r, Err(e) if e.code == "mesh_normals") {
        r = check(w, &MeshRules { components: 1, genus: 0, normals: Normals::Inward });
    }
    println!("{name}: n={n} {:?} in {:?}", r.as_ref().map_err(|e| (e.code, e.detail.chars().take(60).collect::<String>())), t.elapsed());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let hs: Vec<i32> = args.get(1).map_or(vec![1000, 2000, 4000], |s| s.split(',').map(|x| x.parse().unwrap()).collect());
    let axes = [("x", [1, 0, 0], [0, 1, 0], [0, 0, 1]), ("y", [0, 1, 0], [0, 0, 1], [1, 0, 0]), ("z", [0, 0, 1], [1, 0, 0], [0, 1, 0])];
    for &h in &hs {
        let len = 2 * h * h;
        for (name, d, u, v) in axes {
            run(&format!("strip tube {name} h={h}"), &tube(h, len, d, u, v, false));
        }
        run(&format!("fan tube x h={h}"), &tube(h, len, [1, 0, 0], [0, 1, 0], [0, 0, 1], true));
        run(&format!("strip tube diagonal h={h}"), &tube(h, len / 2, [1, 1, 1], [1, -1, 0], [1, 1, -2], false));
    }
    if let Some(path) = args.get(2) {
        let bytes = std::fs::read(path).unwrap();
        let w = weld(&bytes).unwrap().unwrap_or_else(|r| panic!("{}", r.code));
        let t = Instant::now();
        let mut best = None;
        for _ in 0..100 { let r = check(&w, &MeshRules { components: 1, genus: 1, normals: Normals::Inward }); best = Some(r); }
        println!("cavity: n={} {:?} x100 in {:?}", w.geometry.triangles.len(), best.unwrap().map_err(|e| e.code), t.elapsed());
    }
}
