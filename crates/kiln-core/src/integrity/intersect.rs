//! Exact triangle-triangle intersection on integer (mm) coordinates, ported
//! from ASTRA `assets/kits/excavations/src/check.py:58-136`.
//!
//! Width: any `i32` input gives differences below 2^32, cross products below
//! 2^65 and `orient` below 3 * 2^97 < 2^99, so `i128` is exact. check.py's
//! `dot(cross, cross)` would reach 2^131; here both uses compare coplanar
//! vectors, so only the sign of one cross component is needed instead.

type P = [i128; 3];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn orient(a: P, b: P, c: P, d: P) -> i128 {
    let n = cross(sub(b, a), sub(c, a));
    let e = sub(d, a);
    n[0] * e[0] + n[1] * e[1] + n[2] * e[2]
}

/// Axis of the largest component of `n`: projecting along it keeps a plane
/// with normal `n` non-degenerate.
fn major(n: P) -> usize {
    (0..3).fold(0, |k, i| if n[i].abs() > n[k].abs() { i } else { k })
}

/// Sign of `dot(n, cross(x, y))` for `x`, `y` in the plane of normal `n`:
/// there `cross(x, y) = l * n`, so it is the sign of `l`.
fn along(n: P, x: P, y: P) -> i128 {
    let k = major(n);
    cross(x, y)[k].signum() * n[k].signum()
}

fn o2(a: [i128; 2], b: [i128; 2], c: [i128; 2]) -> i128 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on2(x: [i128; 2], y: [i128; 2], z: [i128; 2]) -> bool {
    let within = |i: usize| x[i].min(y[i]) <= z[i] && z[i] <= x[i].max(y[i]);
    o2(x, y, z) == 0 && within(0) && within(1)
}

fn seg_seg_2d(p: [i128; 2], q: [i128; 2], a: [i128; 2], b: [i128; 2]) -> bool {
    let apart = |s: i128, t: i128| s != 0 && t != 0 && (s > 0) != (t > 0);
    (apart(o2(a, b, p), o2(a, b, q)) && apart(o2(p, q, a), o2(p, q, b)))
        || on2(a, b, p)
        || on2(a, b, q)
        || on2(p, q, a)
        || on2(p, q, b)
}

fn same_sign(s: [i128; 3]) -> bool {
    s.iter().all(|&x| x >= 0) || s.iter().all(|&x| x <= 0)
}

fn in_tri_2d(p: [i128; 2], a: [i128; 2], b: [i128; 2], c: [i128; 2]) -> bool {
    same_sign([o2(a, b, p), o2(b, c, p), o2(c, a, p)])
}

/// Closed segment `pq` against closed triangle `abc`.
fn seg_tri(p: P, q: P, a: P, b: P, c: P) -> bool {
    let (s1, s2) = (orient(a, b, c, p), orient(a, b, c, q));
    if (s1 > 0 && s2 > 0) || (s1 < 0 && s2 < 0) {
        return false;
    }
    if s1 == 0 && s2 == 0 {
        let k = major(cross(sub(b, a), sub(c, a)));
        let pr = |x: P| {
            let [i, j] = [(k + 1) % 3, (k + 2) % 3];
            [x[i], x[j]]
        };
        let [p, q, a, b, c] = [p, q, a, b, c].map(pr);
        return in_tri_2d(p, a, b, c)
            || in_tri_2d(q, a, b, c)
            || [(a, b), (b, c), (c, a)].iter().any(|&(x, y)| seg_seg_2d(p, q, x, y));
    }
    same_sign([orient(p, q, a, b), orient(p, q, b, c), orient(p, q, c, a)])
}

/// Edge `v -> w`, coplanar with triangle `(v, c, d)`, points into its closed
/// angle at `v`.
fn ray_in_angle(v: P, w: P, c: P, d: P) -> bool {
    if orient(v, c, d, w) != 0 {
        return false;
    }
    let n = cross(sub(c, v), sub(d, v));
    along(n, sub(c, v), sub(w, v)) >= 0 && along(n, sub(w, v), sub(d, v)) >= 0
}

/// Whether two triangles of a welded mesh intersect beyond what their shared
/// vertices allow. Triangles are indices into `points`; one that repeats an
/// index is degenerate, reported elsewhere, and never crosses here.
pub fn triangles_cross(t1: [u32; 3], t2: [u32; 3], points: &[[i32; 3]]) -> bool {
    let repeats = |t: [u32; 3]| t[0] == t[1] || t[1] == t[2] || t[2] == t[0];
    if repeats(t1) || repeats(t2) {
        return false;
    }
    let at = |i: u32| points[i as usize].map(i128::from);
    let shared: Vec<u32> = t1.iter().copied().filter(|i| t2.contains(i)).collect();
    let rest = |t: [u32; 3]| -> Vec<P> { t.iter().filter(|i| !shared.contains(i)).map(|&i| at(i)).collect() };
    let (a, b) = (rest(t1), rest(t2));
    match shared.len() {
        3.. => true,
        2 => {
            let (u, w) = (at(shared[0]), at(shared[1]));
            let n = cross(sub(w, u), sub(a[0], u));
            orient(u, w, a[0], b[0]) == 0 && along(n, sub(w, u), sub(b[0], u)) > 0
        }
        1 => {
            let v = at(shared[0]);
            seg_tri(a[0], a[1], v, b[0], b[1])
                || seg_tri(b[0], b[1], v, a[0], a[1])
                || a.iter().any(|&x| ray_in_angle(v, x, b[0], b[1]))
                || b.iter().any(|&x| ray_in_angle(v, x, a[0], a[1]))
        }
        _ => {
            let edges = |t: &[P]| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            edges(&a).iter().any(|&(p, q)| seg_tri(p, q, b[0], b[1], b[2]))
                || edges(&b).iter().any(|&(p, q)| seg_tri(p, q, a[0], a[1], a[2]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::triangles_cross;

    type Case<'a> = (&'a str, &'a [[i32; 3]], [u32; 3], bool);

    #[test]
    fn intersect_pairs_with_known_answers() {
        // 0..=2: base triangle in z = 0, right angle at the origin.
        let base: [[i32; 3]; 3] = [[0, 0, 0], [10, 0, 0], [0, 10, 0]];
        let big = 50_000_000; // coordinates up to 1.5e9 mm, near the i32 limit
        let cases: &[Case] = &[
            ("cross", &[[1, 1, -5], [1, 1, 5], [20, 20, 0]], [3, 4, 5], true),
            ("parallel", &[[0, 0, 5], [10, 0, 5], [0, 10, 5]], [3, 4, 5], false),
            ("planes_cross_far", &[[20, 20, -5], [20, 20, 5], [30, 25, 0]], [3, 4, 5], false),
            ("touch_on_edge", &[[5, 0, 0], [5, -5, 5], [5, -5, -5]], [3, 4, 5], true),
            ("same_triangle", &[], [0, 2, 1], true),
            ("shared_edge_folded", &[[5, 5, 0]], [0, 1, 3], true),
            ("shared_edge_flat", &[[5, -5, 0]], [0, 1, 3], false),
            ("shared_edge_bent", &[[5, 5, 5]], [0, 1, 3], false),
            ("shared_vertex_ray_inside", &[[30, 30, 0], [30, 30, 10]], [0, 3, 4], true),
            ("shared_vertex_ray_outside", &[[-3, -3, 0], [-3, -3, 10]], [0, 3, 4], false),
            ("shared_vertex_coplanar_out", &[[-10, 0, 0], [0, -10, 0]], [0, 3, 4], false),
            ("shared_vertex_opposite_edge_pierces", &[[2, 2, -5], [2, 2, 5]], [0, 3, 4], true),
            ("coplanar_overlap", &[[2, 2, 0], [12, 2, 0], [2, 12, 0]], [3, 4, 5], true),
            ("coplanar_inside", &[[1, 1, 0], [2, 1, 0], [1, 2, 0]], [3, 4, 5], true),
            ("coplanar_disjoint", &[[20, 20, 0], [30, 20, 0], [20, 30, 0]], [3, 4, 5], false),
            ("coplanar_disjoint_near", &[[6, 6, 0], [16, 6, 0], [6, 16, 0]], [3, 4, 5], false),
        ];
        for &(name, extra, t2, want) in cases {
            for scale in [1, big] {
                let mut p: Vec<[i32; 3]> = base.to_vec();
                p.extend_from_slice(extra);
                let p: Vec<[i32; 3]> = p.iter().map(|v| v.map(|c| c * scale)).collect();
                let t1 = [0, 1, 2];
                assert_eq!(triangles_cross(t1, t2, &p), want, "{name} x{scale}");
                assert_eq!(triangles_cross(t2, t1, &p), want, "{name} x{scale} swapped");
            }
        }
        // One millimetre apart at 5e8 mm: parallel planes and a near miss.
        let mut p: Vec<[i32; 3]> = base.iter().map(|v| v.map(|c| c * big)).collect();
        p.extend([[0, 0, 1], [10 * big, 0, 1], [0, 10 * big, 1]]);
        assert!(!triangles_cross([0, 1, 2], [3, 4, 5], &p), "parallel 1 mm at 5e8");
        p.extend([[5 * big + 1, 5 * big, -5], [5 * big + 1, 5 * big, 5], [9 * big, 9 * big, 0]]);
        assert!(!triangles_cross([0, 1, 2], [6, 7, 8], &p), "1 mm past the hypotenuse");
    }
}
