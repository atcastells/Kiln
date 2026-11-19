//! Via C of the K013 spec (table *Casos*): synthetic canonical geometries and
//! mutations of a real one, straight into `canonical::validate`, each with
//! the verdict of ASTRA `.sdlc/canonical-geometry/measurements.md` §8.

use kiln_core::canonical::{self, Geometry, from_bytes, to_bytes, topology, validate};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/canonical/asteroid_64_seed42.bin")
}

/// asteroid_64 seed 42 as exported by `generate.gd` and built (see the README).
fn real() -> Geometry {
    from_bytes(&std::fs::read(fixture_path()).expect("fixture")).expect("fixture container")
}

fn tetra() -> Geometry {
    Geometry {
        vertices: vec![[1000, 0, 0], [0, 1000, 0], [0, 0, 1000], [-577, -577, -577]],
        triangles: vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]],
    }
}

/// Vertices +x, -x, +y, -y, +z, -z; one outward triangle per octant.
fn octa() -> Geometry {
    let k = 1000;
    Geometry {
        vertices: vec![[k, 0, 0], [-k, 0, 0], [0, k, 0], [0, -k, 0], [0, 0, k], [0, 0, -k]],
        triangles: vec![[0, 2, 4], [1, 4, 2], [0, 4, 3], [0, 5, 2], [1, 3, 4], [1, 2, 5], [0, 3, 5], [1, 5, 3]],
    }
}

/// Distinct positions on the x axis: I1–I7 never look at geometry.
fn line(n: usize) -> Vec<[i32; 3]> {
    (0..n as i32).map(|i| [i, 0, 0]).collect()
}

/// Top pole 0, `rings` rings of `segs` vertices, bottom pole last.
fn uv_sphere(rings: u32, segs: u32) -> Geometry {
    let at = |r: u32, j: u32| 1 + r * segs + j % segs;
    let bottom = 1 + rings * segs;
    let mut triangles = Vec::new();
    for j in 0..segs {
        triangles.push([0, at(0, j + 1), at(0, j)]);
        for r in 0..rings - 1 {
            let (a, b, c, d) = (at(r, j), at(r, j + 1), at(r + 1, j + 1), at(r + 1, j));
            triangles.extend([[a, b, c], [a, c, d]]);
        }
        triangles.push([bottom, at(rings - 1, j), at(rings - 1, j + 1)]);
    }
    Geometry { vertices: line(bottom as usize + 1), triangles }
}

/// Closed `n` x `m` grid with wrap-around in both directions.
fn torus(n: u32, m: u32) -> Geometry {
    let at = |i: u32, j: u32| (i % n) * m + j % m;
    let mut triangles = Vec::new();
    for i in 0..n {
        for j in 0..m {
            let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            triangles.extend([[a, b, c], [a, c, d]]);
        }
    }
    Geometry { vertices: line((n * m) as usize), triangles }
}

/// M§8 double wrap (port of ASTRA `measure/a7_extra.py`, S = 16, R = 1e6).
fn double_wrap() -> Geometry {
    use std::f64::consts::PI;
    let (s, r0) = (16usize, 1_000_000f64);
    let (rings, cols) = (s / 2, 2 * s);
    let mut vertices = vec![[0, r0 as i32, 0], [0, -(r0 as i32), 0]];
    for j in 1..rings {
        let lat = PI * j as f64 / rings as f64;
        for i in 0..cols {
            let lon = 2.0 * PI * i as f64 / s as f64;
            let r = r0 * (1.0 + 0.3 * (PI * (i as f64 + 0.5) / s as f64).cos());
            let q = |x: f64| x.round() as i32;
            vertices.push([q(r * lat.sin() * lon.sin()), q(r * lat.cos()), q(r * lat.sin() * lon.cos())]);
        }
    }
    let vid = |j: usize, i: usize| (2 + (j - 1) * cols + (i % cols)) as u32;
    let mut triangles = Vec::new();
    for i in 0..cols {
        triangles.push([0, vid(1, i), vid(1, i + 1)]);
        triangles.push([1, vid(rings - 1, i + 1), vid(rings - 1, i)]);
        for j in 1..rings - 1 {
            triangles.push([vid(j, i), vid(j + 1, i), vid(j + 1, i + 1)]);
            triangles.push([vid(j, i), vid(j + 1, i + 1), vid(j, i + 1)]);
        }
    }
    Geometry { vertices, triangles }
}

fn code(g: &Geometry) -> &'static str {
    validate(g).err().map_or("ok", |r| r.code)
}

/// A radial-only rejection: the shape still passes I1–I7.
fn not_radial(g: &Geometry) {
    assert_eq!(topology::check_i1_i7(g), Ok(()));
    let r = validate(g).unwrap_err();
    assert_eq!(r.code, "I8_not_radial", "{}", r.detail);
    assert!(r.detail.contains("v0 does not admit"), "{}", r.detail);
}

#[test]
fn canonical_cases_positive_tetrahedron() {
    assert_eq!(validate(&tetra()), Ok(()));
}

#[test]
fn canonical_cases_positive_real_fixture() {
    let bytes = std::fs::read(fixture_path()).unwrap();
    let g = real();
    assert_eq!((g.vertices.len(), g.triangles.len()), (2050, 4096));
    assert_eq!(bytes.len(), canonical::HEADER_LEN + 12 * (2050 + 4096));
    assert_eq!(to_bytes(&g), bytes);
    assert_eq!(validate(&g), Ok(()));
}

#[test]
fn canonical_cases_real_triangle_removed() {
    let mut g = real();
    g.triangles.pop();
    assert_eq!(code(&g), "I4_open_edge");
}

#[test]
fn canonical_cases_real_triangle_flipped() {
    let mut g = real();
    g.triangles[100].swap(1, 2);
    assert_eq!(code(&g), "I5_inconsistent_orientation");
}

#[test]
fn canonical_cases_real_triangle_repeated() {
    let mut g = real();
    g.triangles.push(g.triangles[100]);
    assert_eq!(code(&g), "I4_edge_shared_by_more_than_two");
}

#[test]
fn canonical_cases_real_duplicate_position() {
    let mut g = real();
    g.vertices[500] = g.vertices[1000];
    let r = validate(&g).unwrap_err();
    assert_eq!((r.code, r.element), ("I3_duplicate_position", Some(1000)));
}

#[test]
fn canonical_cases_real_out_of_range() {
    let mut g = real();
    g.vertices[7][1] = 1 << 30;
    let r = validate(&g).unwrap_err();
    assert_eq!((r.code, r.element), ("I1_out_of_range", Some(7)));
}

#[test]
fn canonical_cases_real_index_out_of_range() {
    let mut g = real();
    g.triangles[10][2] = g.vertices.len() as u32;
    let r = validate(&g).unwrap_err();
    assert_eq!((r.code, r.element), ("I2_index_out_of_range", Some(10)));
}

#[test]
fn canonical_cases_real_unused_vertex() {
    let mut g = real();
    g.vertices.push([1, 2, 3]);
    let r = validate(&g).unwrap_err();
    assert_eq!((r.code, r.element), ("I3_unused_vertex", Some(2050)));
}

#[test]
fn canonical_cases_real_overhang() {
    // Vertex a of a triangle slid past its opposite edge: a' = b + c - a,
    // so det(a', b, c) = -det(a, b, c) < 0.
    let mut g = real();
    let [a, b, c] = g.triangles[100].map(|i| g.vertices[i as usize]);
    g.vertices[g.triangles[100][0] as usize] = [0, 1, 2].map(|k| b[k] + c[k] - a[k]);
    not_radial(&g);
}

#[test]
fn canonical_cases_real_vertex_through_centre() {
    let mut g = real();
    let v = g.triangles[100][0] as usize;
    g.vertices[v] = g.vertices[v].map(|c| -c);
    not_radial(&g);
}

#[test]
fn canonical_cases_pinched_vertex() {
    // Two far vertices fused into one: the bottom pole becomes the top pole 0.
    let mut g = uv_sphere(3, 6);
    let bottom = g.vertices.len() as u32 - 1;
    g.vertices.pop();
    g.triangles.iter_mut().flatten().filter(|i| **i == bottom).for_each(|i| *i = 0);
    assert_eq!(code(&g), "I6_pinched_vertex");
}

#[test]
fn canonical_cases_second_body_inside() {
    let mut g = octa();
    let inner = octa();
    g.vertices.extend(inner.vertices.iter().map(|p| p.map(|c| c / 10)));
    g.triangles.extend(inner.triangles.iter().map(|t| t.map(|i| i + 6)));
    assert_eq!(code(&g), "I7_not_connected");
}

#[test]
fn canonical_cases_tetrahedron_off_centre() {
    let mut g = tetra();
    for v in &mut g.vertices {
        v[0] += 3000;
    }
    not_radial(&g);
}

#[test]
fn canonical_cases_torus() {
    assert_eq!(code(&torus(3, 4)), "I7_not_a_sphere");
}

#[test]
fn canonical_cases_empty() {
    assert_eq!(code(&Geometry { vertices: vec![], triangles: vec![] }), "I1_too_small");
}

#[test]
fn canonical_cases_double_wrap() {
    let g = double_wrap();
    assert_eq!((g.vertices.len(), g.triangles.len()), (226, 448));
    assert_eq!(topology::check_i1_i7(&g), Ok(()));
    let r = validate(&g).unwrap_err();
    assert_eq!(r.code, "I9_covers_a_direction_2_times", "{}", r.detail);
    assert!(r.detail.contains("v0 does not admit"), "{}", r.detail);
}
