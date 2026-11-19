//! Invariants I8 and I9 (S1 §3.1): the class `radial` about the centre (0,0,0).

use super::{Geometry, Rejection};

/// I9 codes by count; the first triangle always contains its own direction,
/// so the count is at least 1 and a rejection starts at 2.
const I9_CODES: [&str; 8] = [
    "I9_covers_a_direction_2_times",
    "I9_covers_a_direction_3_times",
    "I9_covers_a_direction_4_times",
    "I9_covers_a_direction_5_times",
    "I9_covers_a_direction_6_times",
    "I9_covers_a_direction_7_times",
    "I9_covers_a_direction_8_times",
    "I9_covers_a_direction_9_times",
];
const I9_MANY: &str = "I9_covers_a_direction_10_or_more_times";
const NOT_ADMITTED: &str = "a well-formed shape that v0 does not admit (not radial), not a corrupt file";

/// Checks I8 then I9. Precondition: the geometry already passes I1–I7
/// (indices in range, |c| < 2^30, closed, coherently oriented, one sphere);
/// on other input it may panic on an index or give a meaningless verdict.
pub fn check_i8_i9(g: &Geometry) -> Result<(), Rejection> {
    check_i8(g)?;
    check_i9(g)
}

fn corners(g: &Geometry, t: &[u32; 3]) -> [[i128; 3]; 3] {
    t.map(|i| g.vertices[i as usize].map(i128::from))
}

/// det(u, v, w) = u · (v × w). With |c| < 2^30 (I1) each of its six products
/// is below 2^90, so |det| < 2^93; with u = a + b + c (|u_i| < 2^32) it stays
/// below 2^95. Both fit an i128 exactly.
fn det(u: [i128; 3], v: [i128; 3], w: [i128; 3]) -> i128 {
    u[0] * (v[1] * w[2] - v[2] * w[1]) - u[1] * (v[0] * w[2] - v[2] * w[0]) + u[2] * (v[0] * w[1] - v[1] * w[0])
}

/// I8: det(a − c0, b − c0, c − c0) > 0 with c0 = (0,0,0); first failing triangle.
fn check_i8(g: &Geometry) -> Result<(), Rejection> {
    for (t, tri) in g.triangles.iter().enumerate() {
        let [a, b, c] = corners(g, tri);
        let d = det(a, b, c);
        if d <= 0 {
            let detail = format!("triangle {t}: det(a, b, c) = {d} <= 0 about the centre; {NOT_ADMITTED}");
            return Err(Rejection::new("I8_not_radial", Some(t), detail));
        }
    }
    Ok(())
}

/// I9: the direction d = m + εb + ε²c, with m = a + b + c from the first
/// triangle (a, b, c) and ε > 0 infinitesimal, must lie in exactly one cone.
/// d is strictly inside the first cone and, unlike m alone, never on the
/// boundary plane of another cone (see `side`), so no retry is needed.
/// Element: the first other triangle whose cone contains d.
fn check_i9(g: &Geometry) -> Result<(), Rejection> {
    let Some(first) = g.triangles.first() else { return Ok(()) };
    let [a, b, c] = corners(g, first);
    let dir = [[0, 1, 2].map(|k| a[k] + b[k] + c[k]), b, c];
    let hits: Vec<usize> =
        (0..g.triangles.len()).filter(|&t| cone_contains(&dir, corners(g, &g.triangles[t]))).collect();
    if hits.len() == 1 {
        return Ok(());
    }
    let n = hits.len();
    let code = I9_CODES.get(n.wrapping_sub(2)).copied().unwrap_or(I9_MANY);
    let other = hits.iter().copied().find(|&t| t != 0);
    let detail = format!("a direction inside triangle 0 lies in {n} triangle cones; {NOT_ADMITTED}");
    Err(Rejection::new(code, other, detail))
}

/// Sign of det(m + εb + ε²c, u, v) for small ε > 0: the first non-zero of
/// det(m, u, v), det(b, u, v), det(c, u, v). They cannot all be zero, since
/// m, b, c are independent (det(a, b, c) > 0) and u, v are too (I8).
fn side(dir: &[[i128; 3]; 3], u: [i128; 3], v: [i128; 3]) -> bool {
    dir.iter().map(|&p| det(p, u, v)).find(|&s| s != 0).is_some_and(|s| s > 0)
}

/// Whether the direction m + εb + ε²c, for every small enough ε > 0, lies
/// strictly inside the cone of (u, v, w), given det(u, v, w) > 0.
fn cone_contains(dir: &[[i128; 3]; 3], [u, v, w]: [[i128; 3]; 3]) -> bool {
    side(dir, v, w) && side(dir, w, u) && side(dir, u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra() -> Geometry {
        Geometry {
            vertices: vec![[1000, 0, 0], [0, 1000, 0], [0, 0, 1000], [-577, -577, -577]],
            triangles: vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]],
        }
    }

    fn octahedron() -> Geometry {
        let vertices = vec![[1000, 0, 0], [-1000, 0, 0], [0, 1000, 0], [0, -1000, 0], [0, 0, 1000], [0, 0, -1000]];
        let mut triangles = Vec::new();
        for (x, y, z) in [(0, 2, 4), (1, 2, 4), (0, 3, 4), (1, 3, 4), (0, 2, 5), (1, 2, 5), (0, 3, 5), (1, 3, 5)] {
            let negative = (x == 1) ^ (y == 3) ^ (z == 5);
            triangles.push(if negative { [x, z, y] } else { [x, y, z] });
        }
        Geometry { vertices, triangles }
    }

    /// Port of ASTRA `.sdlc/canonical-geometry/measure/a7_extra.py`: a lat-long
    /// grid of 2S columns that goes round the longitude twice, with a
    /// different radius on each turn and one vertex per pole.
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

    #[test]
    fn radial_accepts_centred_tetrahedron_and_octahedron() {
        assert_eq!(check_i8_i9(&tetra()), Ok(()));
        assert_eq!(check_i8_i9(&octahedron()), Ok(()));
    }

    #[test]
    fn radial_rejects_tetrahedron_not_containing_centre() {
        let mut g = tetra();
        for v in &mut g.vertices {
            v[0] += 3000;
        }
        let r = check_i8_i9(&g).unwrap_err();
        assert_eq!((r.code, r.element), ("I8_not_radial", Some(2)));
        assert!(r.detail.contains("v0 does not admit") && r.detail.contains("not a corrupt file"), "{}", r.detail);
    }

    #[test]
    fn radial_rejects_vertex_pushed_through_centre() {
        let mut g = tetra();
        g.vertices[3] = [100, 100, 100];
        let r = check_i8_i9(&g).unwrap_err();
        assert_eq!((r.code, r.element), ("I8_not_radial", Some(1)));
    }

    #[test]
    fn radial_rejects_double_wrap_with_i9() {
        let g = double_wrap();
        assert_eq!((g.vertices.len(), g.triangles.len()), (226, 448));
        assert_eq!(check_i8(&g), Ok(()));
        let r = check_i8_i9(&g).unwrap_err();
        assert_eq!(r.code, "I9_covers_a_direction_2_times");
        assert!(r.element.is_some_and(|t| t > 0));
        assert!(r.detail.contains("v0 does not admit") && r.detail.contains("not a corrupt file"), "{}", r.detail);
        assert_eq!(check_i9(&octahedron()), Ok(()));
    }

    /// Directions exactly on a shared edge or on a vertex ray of the
    /// octahedron fall, after the perturbation, in exactly one face.
    #[test]
    fn radial_perturbed_direction_on_a_boundary_hits_one_face() {
        let o = octahedron();
        let faces: Vec<[[i128; 3]; 3]> =
            o.triangles.iter().map(|t| t.map(|i| o.vertices[i as usize].map(i128::from))).collect();
        let dirs = [
            [[0, 0, 1], [1, 2, 0], [-3, 1, 0]],
            [[0, 0, 1], [-1, -2, 0], [3, -1, 1]],
            [[1, 1, 0], [0, 0, 1], [1, -1, 0]],
            [[1, 1, 0], [0, 0, -1], [-1, 1, 0]],
            [[1, 1, 1], [1, 0, 0], [0, 1, 0]],
        ];
        for dir in dirs {
            let hits = faces.iter().filter(|f| cone_contains(&dir, **f)).count();
            assert_eq!(hits, 1, "{dir:?}");
        }
        // m and b both lie in the plane y = 0: only the ε² term decides it.
        let dir = [[0, 0, 1], [1, 0, 0], [0, 1, 0]];
        let hit: Vec<usize> = (0..faces.len()).filter(|&t| cone_contains(&dir, faces[t])).collect();
        assert_eq!(hit, vec![0]);
    }
}
