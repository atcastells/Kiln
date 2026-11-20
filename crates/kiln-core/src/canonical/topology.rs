//! Topological invariants I1–I7 of S1 §3.1, integer only. Each code is a
//! full pass over its elements, in the order of the reference validator
//! (`geo.py` `validate`); the first failing pass is reported.
//!
//! Element reported, always the lowest index that fails:
//! - `I1_too_small`: none. `I1_out_of_range`: vertex.
//! - `I2_*`: triangle.
//! - `I3_unused_vertex`: vertex. `I3_duplicate_position`: the later vertex of the pair.
//! - `I4_*`: first triangle with an edge used once / more than twice.
//! - `I5_inconsistent_orientation`: first triangle with a directed edge
//!   already used by an earlier triangle.
//! - `I6_pinched_vertex`: first vertex whose fan is more than one cycle.
//! - `I7_not_connected`: first triangle outside the piece of triangle 0.
//!   `I7_not_a_sphere`: none; the detail holds V - E + F.

use super::{Geometry, Rejection};
use std::collections::HashMap;

/// Coordinates must satisfy |c| < 2^30 quanta.
pub const COORD_LIMIT: u32 = 1 << 30;

fn first<T>(items: &[T], bad: impl Fn(&T) -> bool) -> Option<usize> {
    items.iter().position(bad)
}

fn edges(t: &[u32; 3]) -> [(u32, u32); 3] {
    [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
}

fn undirected((a, b): (u32, u32)) -> (u32, u32) {
    (a.min(b), a.max(b))
}

pub fn check_i1_i5(g: &Geometry) -> Result<(), Rejection> {
    let (vs, ts) = (&g.vertices, &g.triangles);
    let fail = |code, element: Option<usize>, detail: String| Err(Rejection::new(code, element, detail));
    if vs.len() < 4 || ts.len() < 4 {
        return fail("I1_too_small", None, format!("{} vertices and {} triangles, need 4 and 4", vs.len(), ts.len()));
    }
    if let Some(v) = first(vs, |p| p.iter().any(|c| c.unsigned_abs() >= COORD_LIMIT)) {
        return fail("I1_out_of_range", Some(v), format!("vertex {v} {:?} has |c| >= 2^30", vs[v]));
    }
    if let Some(t) = first(ts, |t| t.iter().any(|&i| i as usize >= vs.len())) {
        return fail("I2_index_out_of_range", Some(t), format!("triangle {t} {:?} of {} vertices", ts[t], vs.len()));
    }
    if let Some(t) = first(ts, |t| t[0] == t[1] || t[1] == t[2] || t[0] == t[2]) {
        return fail("I2_repeated_index", Some(t), format!("triangle {t} {:?} repeats an index", ts[t]));
    }
    let mut used = vec![false; vs.len()];
    ts.iter().flatten().for_each(|&i| used[i as usize] = true);
    if let Some(v) = first(&used, |u| !u) {
        return fail("I3_unused_vertex", Some(v), format!("vertex {v} is in no triangle"));
    }
    let mut seen: HashMap<[i32; 3], usize> = HashMap::with_capacity(vs.len());
    for (v, p) in vs.iter().enumerate() {
        if let Some(earlier) = seen.insert(*p, v) {
            return fail("I3_duplicate_position", Some(v), format!("vertex {v} is at {p:?}, like vertex {earlier}"));
        }
    }
    let mut count: HashMap<(u32, u32), u32> = HashMap::with_capacity(3 * ts.len());
    ts.iter().flat_map(edges).for_each(|e| *count.entry(undirected(e)).or_default() += 1);
    let uses = |e| count[&undirected(e)];
    if let Some(t) = first(ts, |t| edges(t).into_iter().any(|e| uses(e) == 1)) {
        return fail("I4_open_edge", Some(t), format!("triangle {t} {:?} has an edge in no other triangle", ts[t]));
    }
    if let Some(t) = first(ts, |t| edges(t).into_iter().any(|e| uses(e) > 2)) {
        return fail("I4_edge_shared_by_more_than_two", Some(t), format!("triangle {t} {:?} has an edge in 3+ triangles", ts[t]));
    }
    let mut directed: HashMap<(u32, u32), usize> = HashMap::with_capacity(3 * ts.len());
    for (t, tri) in ts.iter().enumerate() {
        for e in edges(tri) {
            if let Some(earlier) = directed.insert(e, t) {
                let detail = format!("triangle {t} repeats directed edge {e:?} of triangle {earlier}");
                return fail("I5_inconsistent_orientation", Some(t), detail);
            }
        }
    }
    Ok(())
}

/// I6 and I7, assuming I1–I5 hold (closed, consistently oriented, every
/// vertex used).
pub fn check_i6_i7(g: &Geometry) -> Result<(), Rejection> {
    check_i6(g)?;
    let (_, chi, outside) = pieces_and_euler(g);
    if let Some(t) = outside {
        let detail = format!("triangle {t} {:?} is not connected to triangle 0", g.triangles[t]);
        return Err(Rejection::new("I7_not_connected", Some(t), detail));
    }
    if chi != 2 {
        let edges = g.vertices.len() as i64 + g.triangles.len() as i64 - chi;
        let detail = format!("V - E + F = {} - {edges} + {} = {chi}, need 2", g.vertices.len(), g.triangles.len());
        return Err(Rejection::new("I7_not_a_sphere", None, detail));
    }
    Ok(())
}

/// I6, assuming I1–I5 hold: every vertex fan is a single cycle.
/// Precondition: call it only after `check_i1_i5` passed; on an open mesh or
/// one with an unused vertex it panics.
pub fn check_i6(g: &Geometry) -> Result<(), Rejection> {
    let (vs, ts) = (&g.vertices, &g.triangles);
    // Fan of each vertex: the link edge b->c of every triangle around it.
    let mut fans: Vec<Vec<(u32, u32)>> = vec![Vec::new(); vs.len()];
    for &[a, b, c] in ts {
        fans[a as usize].push((b, c));
        fans[b as usize].push((c, a));
        fans[c as usize].push((a, b));
    }
    for (v, fan) in fans.iter().enumerate() {
        let next: HashMap<u32, u32> = fan.iter().copied().collect();
        let (start, mut cur, mut steps) = (fan[0].0, fan[0].0, 0);
        loop {
            cur = next[&cur];
            steps += 1;
            if cur == start || steps >= fan.len() {
                break;
            }
        }
        if cur != start || steps != fan.len() {
            let detail = format!("vertex {v}: the first cycle of its fan holds {steps} of {} triangles", fan.len());
            return Err(Rejection::new("I6_pinched_vertex", Some(v), detail));
        }
    }
    Ok(())
}

/// Connected pieces of the triangles (through shared vertices), V - E + F,
/// and the first triangle outside the piece of triangle 0.
pub fn pieces_and_euler(g: &Geometry) -> (usize, i64, Option<usize>) {
    let (vs, ts) = (&g.vertices, &g.triangles);
    let mut parent: Vec<usize> = (0..vs.len()).collect();
    fn root(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for &[a, b, c] in ts {
        for (x, y) in [(a, b), (b, c)] {
            let (rx, ry) = (root(&mut parent, x as usize), root(&mut parent, y as usize));
            parent[rx] = ry;
        }
    }
    let roots: Vec<usize> = ts.iter().map(|t| root(&mut parent, t[0] as usize)).collect();
    let outside = roots.first().and_then(|&piece| roots.iter().position(|&r| r != piece));
    let pieces = roots.iter().collect::<std::collections::HashSet<_>>().len();
    let edges = ts.iter().flat_map(edges).map(undirected).collect::<std::collections::HashSet<_>>().len();
    let chi = vs.len() as i64 - edges as i64 + ts.len() as i64;
    (pieces, chi, outside)
}

pub fn check_i1_i7(g: &Geometry) -> Result<(), Rejection> {
    check_i1_i5(g)?;
    check_i6_i7(g)
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

    /// Vertices +x, -x, +y, -y, +z, -z; one outward triangle per octant.
    fn octa() -> Geometry {
        let k = 1000;
        Geometry {
            vertices: vec![[k, 0, 0], [-k, 0, 0], [0, k, 0], [0, -k, 0], [0, 0, k], [0, 0, -k]],
            triangles: vec![[0, 2, 4], [1, 4, 2], [0, 4, 3], [0, 5, 2], [1, 3, 4], [1, 2, 5], [0, 3, 5], [1, 5, 3]],
        }
    }

    fn rejects(g: &Geometry, code: &str, element: Option<usize>) {
        let r = check_i1_i5(g).unwrap_err();
        assert_eq!((r.code, r.element), (code, element), "{}", r.detail);
    }

    #[test]
    fn canonical_topology_accepts_tetra_and_octa() {
        assert_eq!(check_i1_i5(&tetra()), Ok(()));
        assert_eq!(check_i1_i5(&octa()), Ok(()));
    }

    #[test]
    fn canonical_topology_i1_too_small() {
        rejects(&Geometry { vertices: vec![], triangles: vec![] }, "I1_too_small", None);
        let mut g = tetra();
        g.triangles.pop();
        rejects(&g, "I1_too_small", None);
    }

    #[test]
    fn canonical_topology_i1_out_of_range() {
        for c in [1 << 30, -(1 << 30), i32::MIN, i32::MAX] {
            let mut g = tetra();
            g.vertices[2][2] = c;
            rejects(&g, "I1_out_of_range", Some(2));
        }
        let mut g = tetra();
        g.vertices[0][0] = (1 << 30) - 1;
        g.vertices[3][1] = -(1 << 30) + 1;
        assert_eq!(check_i1_i5(&g), Ok(()));
    }

    #[test]
    fn canonical_topology_i2_index_out_of_range() {
        let mut g = tetra();
        g.triangles[3] = [2, 3, 4];
        rejects(&g, "I2_index_out_of_range", Some(3));
        // The range pass runs before the repeated-index pass.
        g.triangles[0] = [0, 0, 1];
        rejects(&g, "I2_index_out_of_range", Some(3));
    }

    #[test]
    fn canonical_topology_i2_repeated_index() {
        let mut g = tetra();
        g.triangles[1] = [0, 3, 3];
        rejects(&g, "I2_repeated_index", Some(1));
    }

    #[test]
    fn canonical_topology_i3_unused_vertex() {
        let mut g = tetra();
        g.vertices.push([5, 5, 5]);
        rejects(&g, "I3_unused_vertex", Some(4));
    }

    #[test]
    fn canonical_topology_i3_duplicate_position() {
        let mut g = tetra();
        g.vertices[3] = g.vertices[1];
        rejects(&g, "I3_duplicate_position", Some(3));
    }

    #[test]
    fn canonical_topology_i4_open_edge() {
        let mut g = octa();
        g.triangles.pop();
        // Edges of the removed [1, 5, 3]: [1, 3, 4] (index 4) is the first to touch one.
        rejects(&g, "I4_open_edge", Some(4));
    }

    #[test]
    fn canonical_topology_i4_edge_shared_by_more_than_two() {
        let mut g = tetra();
        g.triangles.push(g.triangles[0]);
        rejects(&g, "I4_edge_shared_by_more_than_two", Some(0));
    }

    #[test]
    fn canonical_topology_i5_inconsistent_orientation() {
        let mut g = tetra();
        g.triangles[0] = [0, 2, 1];
        // Flipped triangle 0 holds 1->0; triangle 1 [0, 3, 1] holds it too.
        rejects(&g, "I5_inconsistent_orientation", Some(1));
    }

    /// Distinct positions on the x axis: I1–I7 never look at geometry.
    fn line(n: usize) -> Vec<[i32; 3]> {
        (0..n as i32).map(|i| [i, 0, 0]).collect()
    }

    /// Top pole 0, `rings` rings of `segs` vertices, bottom pole last;
    /// consistently oriented.
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

    fn rejects_i1_i7(g: &Geometry, code: &str, element: Option<usize>) -> String {
        let r = check_i1_i7(g).unwrap_err();
        assert_eq!((r.code, r.element), (code, element), "{}", r.detail);
        r.detail
    }

    #[test]
    fn canonical_topology_i6_i7_accepts_closed_spheres() {
        assert_eq!(check_i1_i7(&tetra()), Ok(()));
        assert_eq!(check_i1_i7(&octa()), Ok(()));
        assert_eq!(check_i1_i7(&uv_sphere(3, 6)), Ok(()));
    }

    #[test]
    fn canonical_topology_i6_pinched_vertex() {
        // Two far vertices fused into one: the bottom pole (last) becomes the top pole 0.
        let mut g = uv_sphere(3, 6);
        let bottom = g.vertices.len() as u32 - 1;
        g.vertices.pop();
        g.triangles.iter_mut().flatten().filter(|i| **i == bottom).for_each(|i| *i = 0);
        rejects_i1_i7(&g, "I6_pinched_vertex", Some(0));
    }

    #[test]
    fn canonical_topology_i7_not_connected() {
        let mut g = octa();
        let inner = octa();
        g.vertices.extend(inner.vertices.iter().map(|p| p.map(|c| c / 10)));
        g.triangles.extend(inner.triangles.iter().map(|t| t.map(|i| i + 6)));
        rejects_i1_i7(&g, "I7_not_connected", Some(8));
    }

    #[test]
    fn canonical_topology_i7_not_a_sphere() {
        let detail = rejects_i1_i7(&torus(3, 4), "I7_not_a_sphere", None);
        assert!(detail.contains("= 0"), "{detail}");
    }

    #[test]
    fn pieces_and_euler_counts_a_torus() {
        assert_eq!(pieces_and_euler(&torus(4, 4)), (1, 0, None));
        assert_eq!(pieces_and_euler(&octa()), (1, 2, None));
        let mut g = octa();
        g.vertices.extend(octa().vertices.iter().map(|p| p.map(|c| c / 10)));
        g.triangles.extend(octa().triangles.iter().map(|t| t.map(|i| i + 6)));
        assert_eq!(pieces_and_euler(&g), (2, 4, Some(8)));
    }
}
