//! Mesh integrity of a published GLB, enabled per spec with `[mesh]`.

pub mod intersect;

use crate::canonical::{topology, Geometry, Rejection};
use crate::spec::{MeshRules, Normals};

/// The welded mesh: vertices numbered by first use in a triangle, triangles
/// in primitive order. `origin[t]` is (primitive, triangle within it) and
/// `first_use[v]` is (primitive, GLB index) of the first corner at `v`.
#[derive(Debug, Clone)]
pub struct Welded {
    pub geometry: Geometry,
    pub origin: Vec<(usize, usize)>,
    pub first_use: Vec<(usize, u32)>,
}

/// `Err` when the GLB cannot be read; `Ok(Err)` with `mesh_layout`,
/// `mesh_offgrid` or `mesh_degenerate` when it cannot be welded honestly.
pub fn weld(bytes: &[u8]) -> Result<Result<Welded, Rejection>, String> {
    let (meshes, skipped) = crate::mesh::read_bytes_with_skipped(bytes)?;
    let doc = crate::glb::json_chunk(bytes)?;
    Ok(placement(&doc, &skipped).and_then(|()| join(&meshes)))
}

const STORE_TOL_MM: f64 = 0.05;

fn layout(detail: String) -> Result<(), Rejection> {
    Err(Rejection::new("mesh_layout", None, detail))
}

/// One mesh, exactly one node holding it, triangles only, and every node
/// that holds the mesh or has children placed by a translation on the mm
/// grid: anything else would change what the welded mesh means.
fn placement(doc: &serde_json::Value, skipped: &[(usize, usize)]) -> Result<(), Rejection> {
    let meshes = doc["meshes"].as_array().map_or(0, Vec::len);
    if meshes != 1 {
        return layout(format!("{meshes} mallas; se admite una"));
    }
    if let Some(&(m, p)) = skipped.first() {
        let mode = &doc["meshes"][m]["primitives"][p]["mode"];
        return layout(format!("malla {m}, primitiva {p}: mode {mode}; solo se admiten triángulos (4)"));
    }
    let nodes = doc["nodes"].as_array().map(Vec::as_slice).unwrap_or_default();
    let holders = nodes.iter().filter(|n| n.get("mesh").is_some()).count();
    if holders != 1 {
        return layout(format!("{holders} nodos con malla; se admite uno"));
    }
    let identity: [(&str, &[f64]); 3] = [
        ("rotation", &[0., 0., 0., 1.]),
        ("scale", &[1., 1., 1.]),
        ("matrix", &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]),
    ];
    let values = |v: &serde_json::Value| -> Option<Vec<f64>> { v.as_array()?.iter().map(|x| x.as_f64()).collect() };
    for (i, n) in nodes.iter().enumerate() {
        if n.get("mesh").is_none() && n["children"].as_array().is_none_or(Vec::is_empty) {
            continue;
        }
        for (key, id) in identity {
            if n.get(key).is_some_and(|v| values(v).as_deref() != Some(id)) {
                return layout(format!("nodo {i}: {key}; solo se admite una traslación"));
            }
        }
        if let Some(t) = n.get("translation") {
            let ok = values(t).is_some_and(|t| {
                t.len() == 3 && t.iter().all(|c| ((c * 1000.0) - (c * 1000.0).round()).abs() <= STORE_TOL_MM)
            });
            if !ok {
                return layout(format!("nodo {i}: translation {t} fuera de la rejilla de 1 mm"));
            }
        }
    }
    Ok(())
}

/// Quanta of `x` when it lies on the mm grid, within 0.05 mm or half an
/// `f32` ulp of `x`, whichever is larger.
fn on_grid(x: f32) -> Option<i32> {
    let mm = x as f64 * 1000.0;
    let ulp = f32::from_bits(x.abs().to_bits() + 1) - x.abs();
    let tol = STORE_TOL_MM.max(ulp as f64 * 500.0);
    ((mm - mm.round()).abs() <= tol).then(|| crate::canonical::quantize(x).ok()).flatten()
}

fn join(meshes: &[crate::mesh::Mesh]) -> Result<Welded, Rejection> {
    let mut ids = std::collections::HashMap::new();
    let (mut vertices, mut triangles, mut origin, mut first_use) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (p, m) in meshes.iter().enumerate() {
        for (t, tri) in m.indices.chunks_exact(3).enumerate() {
            let mut v = [0u32; 3];
            for (k, &i) in tri.iter().enumerate() {
                let pos = m.positions[i as usize];
                let Some(q) = pos.iter().map(|&c| on_grid(c)).collect::<Option<Vec<i32>>>() else {
                    let detail = format!("primitiva {p}, vértice {i} en {pos:?} m: fuera de la rejilla de 1 mm");
                    return Err(Rejection::new("mesh_offgrid", Some(i as usize), detail));
                };
                v[k] = *ids.entry([q[0], q[1], q[2]]).or_insert_with(|| {
                    vertices.push([q[0], q[1], q[2]]);
                    first_use.push((p, i));
                    vertices.len() as u32 - 1
                });
            }
            triangles.push(v);
            origin.push((p, t));
        }
    }
    let cross = |t: &[u32; 3]| {
        let [a, b, c] = t.map(|i| vertices[i as usize].map(i128::from));
        let (u, w) = ([0, 1, 2].map(|k| b[k] - a[k]), [0, 1, 2].map(|k| c[k] - a[k]));
        [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]
    };
    let bad: Vec<usize> = (0..triangles.len()).filter(|&t| cross(&triangles[t]) == [0; 3]).collect();
    if let Some(&first) = bad.first() {
        let ([a, b, c], (p, t)) = (triangles[first], origin[first]);
        let why = if a == b || b == c || a == c { "repite un vértice" } else { "tiene área nula" };
        let at = vertices[a as usize].map(|q| q as f64 / 1000.0);
        let detail = format!(
            "{} de {} triángulos degenerados; el triángulo {first} (primitiva {p}, triángulo {t}, en {at:?} m) {why}; primeros: {:?}",
            bad.len(),
            triangles.len(),
            &bad[..bad.len().min(5)]
        );
        return Err(Rejection::new("mesh_degenerate", Some(first), detail));
    }
    Ok(Welded { geometry: Geometry { vertices, triangles }, origin, first_use })
}

/// The rules of `[mesh]` on a welded mesh, first failure wins: I1–I6 as
/// `mesh_*`, then pieces and V - E + F against `components` and `genus`
/// (χ = 2c - 2g), then the sign of the volume against `normals`.
pub fn check(w: &Welded, rules: &MeshRules) -> Result<(), Rejection> {
    let g = &w.geometry;
    topology::check_i1_i5(g).and_then(|()| topology::check_i6(g)).map_err(|r| translate(w, r))?;
    let (pieces, chi, outside) = topology::pieces_and_euler(g);
    let (c, genus) = (rules.components, rules.genus);
    let want = 2 * i64::from(c) - 2 * i64::from(genus);
    if pieces != c as usize || chi != want {
        let mut detail = format!("{pieces} piezas, V − E + F = {chi}; la ficha declara {c} componentes y género {genus}, que dan {want}");
        if let Some(t) = outside {
            detail += &format!("; fuera de la pieza del triángulo 0: {}", triangle_at(w, t));
        }
        return Err(Rejection::new("mesh_topology", outside, detail));
    }
    // Six times the signed volume, exact: |c| < 2^30 keeps each term under 2^93.
    let volume6: i128 = g
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| g.vertices[i as usize].map(i128::from));
            a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2]) + a[2] * (b[0] * c[1] - b[1] * c[0])
        })
        .sum();
    let (name, ok, sign) = match rules.normals {
        Normals::Outward => ("outward", volume6 > 0, "positivo"),
        Normals::Inward => ("inward", volume6 < 0, "negativo"),
    };
    if !ok {
        let m3 = volume6 as f64 / 6.0 / 1e9;
        let detail = format!("volumen con signo {m3} m³; con normals = \"{name}\" debe ser {sign}");
        return Err(Rejection::new("mesh_normals", None, detail));
    }
    if let (Some((a, b)), hits, _, _) = crossings(g) {
        let detail = format!("{hits} pares de triángulos se cruzan; el primero, {} con {}", triangle_at(w, a), triangle_at(w, b));
        return Err(Rejection::new("mesh_self_intersection", Some(a), detail));
    }
    Ok(())
}

/// Crossing pairs of triangles: the first, as (lower, higher) index, the
/// number of crossing pairs, the pairs scanned by the sweep (their intervals
/// overlap on the sweep axis) and those whose boxes overlap (tested exactly).
///
/// Sweep over integer boxes sorted by their min on one axis. Two triangles
/// that cross share a point, so their closed boxes overlap on every axis; of
/// the two, the one sorted first starts at or before the other and reaches it
/// on that axis, so its scan over the following boxes (until one starts past
/// its max) meets the other. Any axis is complete; `sweep_axis` picks the one
/// expected to scan the fewest pairs.
fn crossings(g: &Geometry) -> (Option<(usize, usize)>, usize, usize, usize) {
    let boxes: Vec<[[i32; 3]; 2]> = g
        .triangles
        .iter()
        .map(|t| {
            let p = t.map(|i| g.vertices[i as usize]);
            let span = |k: usize| (p.iter().map(|q| q[k]).min().unwrap_or(0), p.iter().map(|q| q[k]).max().unwrap_or(0));
            let (x, y, z) = (span(0), span(1), span(2));
            [[x.0, y.0, z.0], [x.1, y.1, z.1]]
        })
        .collect();
    let s = sweep_axis(&boxes);
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_unstable_by_key(|&t| (boxes[t][0][s], t));
    let (mut first, mut hits, mut scanned, mut tested) = (None::<(usize, usize)>, 0, 0, 0);
    for (n, &a) in order.iter().enumerate() {
        for &b in order[n + 1..].iter().take_while(|&&b| boxes[b][0][s] <= boxes[a][1][s]) {
            scanned += 1;
            if (0..3).any(|k| k != s && (boxes[a][0][k] > boxes[b][1][k] || boxes[b][0][k] > boxes[a][1][k])) {
                continue;
            }
            tested += 1;
            if intersect::triangles_cross(g.triangles[a], g.triangles[b], &g.vertices) {
                hits += 1;
                let pair = (a.min(b), a.max(b));
                first = Some(first.map_or(pair, |f| f.min(pair)));
            }
        }
    }
    (first, hits, scanned, tested)
}

/// The axis whose box intervals are shortest relative to the mesh's extent
/// on it: n boxes of mean length L spread over S overlap in about n²·L/S
/// pairs, so the least sum of lengths over extent scans the fewest. Compared
/// as fractions in integers; an axis of zero extent is the worst, and ties go
/// to the lower axis.
fn sweep_axis(boxes: &[[[i32; 3]; 2]]) -> usize {
    let cost = |k: usize| -> (u128, u128) {
        let lengths: u128 = boxes.iter().map(|b| (i64::from(b[1][k]) - i64::from(b[0][k])) as u128).sum();
        let lo = boxes.iter().map(|b| b[0][k]).min().unwrap_or(0);
        let hi = boxes.iter().map(|b| b[1][k]).max().unwrap_or(0);
        let extent = (i64::from(hi) - i64::from(lo)) as u128;
        if extent == 0 { (1, 0) } else { (lengths, extent) }
    };
    let costs = [cost(0), cost(1), cost(2)];
    (1..3).fold(0, |best, k| {
        let ((a, b), (c, d)) = (costs[k], costs[best]);
        if a * d < c * b { k } else { best }
    })
}

/// An `I*` rejection of the welded mesh as its `mesh_*` finding, with the
/// primitive, the GLB index and the position of its element.
fn translate(w: &Welded, r: Rejection) -> Rejection {
    let (code, what, vertex) = match r.code {
        "I1_too_small" => ("mesh_too_small", "menos de 4 vértices o 4 triángulos", false),
        "I1_out_of_range" => ("mesh_offgrid", "coordenada fuera de ±2^30 mm", true),
        "I4_open_edge" => ("mesh_open_edge", "borde abierto", false),
        "I4_edge_shared_by_more_than_two" => ("mesh_non_manifold_edge", "arista en más de dos triángulos", false),
        "I5_inconsistent_orientation" => ("mesh_orientation", "orientación incoherente con un triángulo anterior", false),
        "I6_pinched_vertex" => ("mesh_pinched_vertex", "vértice pellizcado", true),
        // I2 and I3 cannot follow a weld: kept as they are.
        other => return Rejection::new(other, r.element, r.detail),
    };
    let g = &w.geometry;
    let detail = match r.element {
        Some(e) if vertex => {
            let (p, i) = w.first_use[e];
            format!("{what}: vértice soldado {e} (primitiva {p}, índice {i}, en {:?} m)", metres(g.vertices[e]))
        }
        Some(e) => format!("{what}: {}", triangle_at(w, e)),
        None => format!("{what}: {} vértices y {} triángulos", g.vertices.len(), g.triangles.len()),
    };
    Rejection::new(code, r.element, detail)
}

fn metres(q: [i32; 3]) -> [f64; 3] {
    q.map(|c| c as f64 / 1000.0)
}

/// Welded triangle `t` with its primitive, its index there and its first corner.
fn triangle_at(w: &Welded, t: usize) -> String {
    let (p, k) = w.origin[t];
    let corner = metres(w.geometry.vertices[w.geometry.triangles[t][0] as usize]);
    format!("triángulo soldado {t} (primitiva {p}, triángulo {k}, en {corner:?} m)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::topology::check_i1_i5;
    use serde_json::json;

    const K: f32 = 1.0;
    const OCTA_V: [[f32; 3]; 6] = [[K, 0., 0.], [-K, 0., 0.], [0., K, 0.], [0., -K, 0.], [0., 0., K], [0., 0., -K]];
    const OCTA_T: [[u32; 3]; 8] = [[0, 2, 4], [1, 4, 2], [0, 4, 3], [0, 5, 2], [1, 3, 4], [1, 2, 5], [0, 3, 5], [1, 5, 3]];

    /// Positions, indices and mode of one primitive.
    type Prim = (Vec<[f32; 3]>, Vec<u32>, Option<u32>);

    /// One mesh with `prims`, and `nodes`.
    fn glb(prims: &[Prim], nodes: serde_json::Value) -> Vec<u8> {
        let (mut bin, mut accessors, mut views, mut primitives) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (pos, ix, mode) in prims {
            let mut view = |data: Vec<u8>, count: usize, ct: u32, ty: &str| {
                views.push(json!({"buffer": 0, "byteOffset": bin.len(), "byteLength": data.len()}));
                accessors.push(json!({"bufferView": views.len() - 1, "componentType": ct, "count": count, "type": ty}));
                bin.extend(data);
                accessors.len() - 1
            };
            let p = view(pos.iter().flatten().flat_map(|c| c.to_le_bytes()).collect(), pos.len(), 5126, "VEC3");
            let i = view(ix.iter().flat_map(|c| c.to_le_bytes()).collect(), ix.len(), 5125, "SCALAR");
            let mut prim = json!({"attributes": {"POSITION": p}, "indices": i});
            if let Some(m) = mode {
                prim["mode"] = json!(m);
            }
            primitives.push(prim);
        }
        let doc = json!({"meshes": [{"primitives": primitives}], "accessors": accessors, "bufferViews": views,
            "buffers": [{"byteLength": bin.len()}], "nodes": nodes, "scenes": [{"nodes": [0]}]});
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

    /// The octahedron as unindexed corners (every corner its own position),
    /// split into primitives of `sizes` triangles.
    fn octa(sizes: &[usize], v: [[f32; 3]; 6]) -> Vec<Prim> {
        let mut at = 0;
        sizes
            .iter()
            .map(|&n| {
                let pos: Vec<[f32; 3]> = OCTA_T[at..at + n].iter().flatten().map(|&i| v[i as usize]).collect();
                at += n;
                ((0..pos.len() as u32).collect(), pos)
            })
            .map(|(ix, pos)| (pos, ix, None))
            .collect()
    }

    #[test]
    fn welding_reports_what_it_cannot_weld_honestly() {
        let node = json!([{"mesh": 0, "translation": [-260, -455, -2.5]}]);
        let mut offgrid = OCTA_V;
        offgrid[0][0] += 0.0004;
        let flat = vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [0., 1., 0.]];
        let twin = vec![[0., 0., 0.], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mut lines = octa(&[8], OCTA_V);
        lines.push((OCTA_V.to_vec(), vec![0, 1], Some(1)));
        type Case = (&'static str, Vec<u8>, Option<(&'static str, &'static str)>);
        let cases: Vec<Case> = vec![
            ("octa_3_primitives", glb(&octa(&[3, 3, 2], OCTA_V), node.clone()), None),
            ("offgrid", glb(&octa(&[8], offgrid), node.clone()), Some(("mesh_offgrid", "primitiva 0, vértice 0"))),
            ("zero_area", glb(&[(flat, vec![0, 1, 2, 0, 1, 3], None)], node.clone()), Some(("mesh_degenerate", "1 de 2"))),
            ("repeated", glb(&[(twin, vec![0, 3, 2, 0, 1, 2], None)], node.clone()), Some(("mesh_degenerate", "triángulo 1"))),
            ("rotated", glb(&octa(&[8], OCTA_V), json!([{"mesh": 0, "rotation": [0, 0.6, 0, 0.8]}])), Some(("mesh_layout", "rotation"))),
            ("translation_offgrid", glb(&octa(&[8], OCTA_V), json!([{"mesh": 0, "translation": [0, 0.0004, 0]}])), Some(("mesh_layout", "translation"))),
            ("two_nodes", glb(&octa(&[8], OCTA_V), json!([{"mesh": 0}, {"mesh": 0}])), Some(("mesh_layout", "2 nodos"))),
            ("no_node", glb(&octa(&[8], OCTA_V), json!([{"name": "empty"}])), Some(("mesh_layout", "0 nodos"))),
            ("mode_1", glb(&lines, node.clone()), Some(("mesh_layout", "primitiva 1: mode 1"))),
        ];
        for (name, bytes, want) in cases {
            let got = weld(&bytes).unwrap_or_else(|e| panic!("{name}: unreadable: {e}"));
            match (got, want) {
                (Ok(w), None) => {
                    assert_eq!((w.geometry.vertices.len(), w.geometry.triangles.len()), (6, 8), "{name}");
                    assert_eq!(w.geometry.vertices[0], [1000, 0, 0], "{name}");
                    assert_eq!(w.origin[5], (1, 2), "{name}");
                    assert_eq!(w.first_use[5], (1, 1), "{name}: -z first appears in primitive 1, corner 1");
                    assert_eq!(check_i1_i5(&w.geometry), Ok(()), "{name}");
                }
                (Err(r), Some((code, detail))) => {
                    assert_eq!(r.code, code, "{name}: {}", r.detail);
                    assert!(r.detail.contains(detail), "{name}: {}", r.detail);
                }
                (Ok(_), Some((code, _))) => panic!("{name}: welded, want {code}"),
                (Err(r), None) => panic!("{name}: {} {}", r.code, r.detail),
            }
        }
    }

    /// Welded as if every vertex and triangle came from primitive 0 in order.
    fn welded(vertices: Vec<[i32; 3]>, triangles: Vec<[u32; 3]>) -> Welded {
        let origin = (0..triangles.len()).map(|t| (0, t)).collect();
        let first_use = (0..vertices.len() as u32).map(|v| (0, v)).collect();
        Welded { geometry: Geometry { vertices, triangles }, origin, first_use }
    }

    fn octa_mm(scale: i32, at: [i32; 3]) -> Welded {
        let vs = OCTA_V.iter().map(|p| [0, 1, 2].map(|k| p[k] as i32 * scale + at[k])).collect();
        welded(vs, OCTA_T.to_vec())
    }

    /// Two meshes as one, welding the vertices of `b` that coincide with `a`.
    fn union(a: Welded, b: Welded) -> Welded {
        let (mut vs, mut ts) = (a.geometry.vertices, a.geometry.triangles);
        let map: Vec<u32> = b.geometry.vertices.iter().map(|p| match vs.iter().position(|q| q == p) {
            Some(i) => i as u32,
            None => { vs.push(*p); vs.len() as u32 - 1 }
        }).collect();
        ts.extend(b.geometry.triangles.iter().map(|t| t.map(|i| map[i as usize])));
        welded(vs, ts)
    }

    /// 4 x 4 torus, radii 3000 and 1000 mm, outward: (i, j) is (major, minor).
    fn torus() -> Welded {
        let (cos, sin) = ([1, 0, -1, 0], [0, 1, 0, -1]);
        let at = |i: u32, j: u32| (i % 4) * 4 + j % 4;
        let vs = (0..16).map(|v| {
            let (i, j) = (v / 4, v % 4);
            let r = 3000 + 1000 * cos[j];
            [r * cos[i], r * sin[i], 1000 * sin[j]]
        }).collect();
        let mut ts = Vec::new();
        for i in 0..4 {
            for j in 0..4 {
                let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                ts.extend([[a, b, c], [a, c, d]]);
            }
        }
        welded(vs, ts)
    }

    #[test]
    fn mesh_rules_find_one_code_per_mutation() {
        let rules = |components, genus, normals| MeshRules { components, genus, normals };
        let solid = rules(1, 0, Normals::Outward);
        let mut removed = octa_mm(1000, [0; 3]);
        removed.geometry.triangles.remove(0);
        let mut flipped = octa_mm(1000, [0; 3]);
        flipped.geometry.triangles[0].swap(1, 2);
        let mut inverted = octa_mm(1000, [0; 3]);
        inverted.geometry.triangles.iter_mut().for_each(|t| t.swap(1, 2));
        let tetra = welded(vec![[100, 0, 0], [0, 100, 0], [0, 0, 100], [-50, -50, -50]], vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]]);
        let extra = union(octa_mm(1000, [0; 3]), tetra);
        let two = union(octa_mm(1000, [0; 3]), octa_mm(1000, [5000, 0, 0]));
        let edge_x4 = union(octa_mm(1000, [0; 3]), octa_mm(1000, [1000, 1000, 0]));
        let pinched = union(octa_mm(1000, [0; 3]), octa_mm(1000, [2000, 0, 0]));
        let mut small = octa_mm(1000, [0; 3]);
        small.geometry.triangles.truncate(3);
        // (name, mesh, rules, code and element, or None when it passes).
        type Case = (&'static str, Welded, MeshRules, Option<(&'static str, Option<usize>)>);
        let cases: Vec<Case> = vec![
            ("octa", octa_mm(1000, [0; 3]), solid.clone(), None),
            ("torus_genus_1", torus(), rules(1, 1, Normals::Outward), None),
            ("two_octa_declared", two.clone(), rules(2, 0, Normals::Outward), None),
            ("inverted_inward", inverted.clone(), rules(1, 0, Normals::Inward), None),
            ("too_small", small, solid.clone(), Some(("mesh_too_small", None))),
            // check.py removed: [1, 4, 2], now first, borders the hole on 4-2.
            ("removed", removed, solid.clone(), Some(("mesh_open_edge", Some(0)))),
            // check.py flipped: [0, 4, 2] repeats 4->2 of [1, 4, 2].
            ("flipped", flipped, solid.clone(), Some(("mesh_orientation", Some(1)))),
            ("edge_x4", edge_x4, solid.clone(), Some(("mesh_non_manifold_edge", Some(0)))),
            ("pinched", pinched, solid.clone(), Some(("mesh_pinched_vertex", Some(0)))),
            // check.py extra_component: a loose tetrahedron inside.
            ("extra_component", extra, solid.clone(), Some(("mesh_topology", Some(8)))),
            ("two_octa_undeclared", two, solid.clone(), Some(("mesh_topology", Some(8)))),
            ("torus_genus_0", torus(), solid.clone(), Some(("mesh_topology", None))),
            ("inverted_outward", inverted, solid.clone(), Some(("mesh_normals", None))),
            ("octa_inward", octa_mm(1000, [0; 3]), rules(1, 0, Normals::Inward), Some(("mesh_normals", None))),
        ];
        for (name, w, rules, want) in cases {
            match (check(&w, &rules), want) {
                (Ok(()), None) => {}
                (Err(r), Some((code, element))) => {
                    assert_eq!((r.code, r.element), (code, element), "{name}: {}", r.detail);
                    if element.is_some() {
                        assert!(r.detail.contains("primitiva 0") && r.detail.contains(" m)"), "{name}: {}", r.detail);
                    }
                }
                (Ok(()), Some((code, _))) => panic!("{name}: passed, want {code}"),
                (Err(r), None) => panic!("{name}: {} {}", r.code, r.detail),
            }
        }
        let topology = check(&torus(), &solid).unwrap_err().detail;
        assert!(topology.contains("1 piezas, V − E + F = 0") && topology.contains("que dan 2"), "{topology}");
    }

    /// Every pair tested exactly: the reference for the sweep.
    fn all_pairs(g: &Geometry) -> (Option<(usize, usize)>, usize) {
        let n = g.triangles.len();
        let hits: Vec<(usize, usize)> = (0..n)
            .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
            .filter(|&(a, b)| intersect::triangles_cross(g.triangles[a], g.triangles[b], &g.vertices))
            .collect();
        (hits.first().copied(), hits.len())
    }

    /// Torus of `n` x `m` quads around z, radii `big` and `small` mm, outward.
    fn fine_torus(n: u32, m: u32, big: f64, small: f64) -> Welded {
        let at = |i: u32, j: u32| (i % n) * m + j % m;
        let vs = (0..n * m).map(|v| {
            let (u, w) = (std::f64::consts::TAU * (v / m) as f64 / n as f64, std::f64::consts::TAU * (v % m) as f64 / m as f64);
            let r = big + small * w.cos();
            [r * u.cos(), r * u.sin(), small * w.sin()].map(|c| c.round() as i32)
        }).collect();
        let ts = (0..n).flat_map(|i| (0..m).flat_map(move |j| {
            let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            [[a, b, c], [a, c, d]]
        })).collect();
        welded(vs, ts)
    }

    /// Closed prism over the exactly convex polygon `i*u + i²*v`, `i` in
    /// `-h..=h`, from 0 to `len * dir`: every side triangle is as long as the
    /// prism. Caps as strips of thin slivers, or as fans from one vertex.
    fn prism(h: i32, len: i32, dir: [i32; 3], u: [i32; 3], v: [i32; 3], fan: bool) -> Welded {
        let p = (2 * h + 1) as u32;
        let vs = (0..2).flat_map(|ring| (-h..=h).map(move |i| [0, 1, 2].map(|k| i * u[k] + i * i * v[k] + ring * len * dir[k])));
        let sides = (0..p).flat_map(|a| [[a, (a + 1) % p, (a + 1) % p + p], [a, (a + 1) % p + p, a + p]]);
        let (l, r) = (|i: i32| (h - i) as u32, |i: i32| (h + i) as u32);
        let cap: Vec<[u32; 3]> = if fan {
            (1..p - 1).map(|j| [0, j, j + 1]).collect()
        } else {
            std::iter::once([l(0), r(1), l(1)])
                .chain((1..h).flat_map(|i| [[l(i), r(i), r(i + 1)], [l(i), r(i + 1), l(i + 1)]]))
                .collect()
        };
        let caps = cap.into_iter().flat_map(|t| [[t[0], t[2], t[1]], t.map(|x| x + p)]);
        welded(vs.collect(), sides.chain(caps).collect())
    }

    /// (name, direction, u, v) of a prism along x, y, z and the diagonal.
    type PrismAxis = (&'static str, [i32; 3], [i32; 3], [i32; 3]);
    const PRISM_AXES: [PrismAxis; 4] = [
        ("x", [1, 0, 0], [0, 1, 0], [0, 0, 1]),
        ("y", [0, 1, 0], [0, 0, 1], [1, 0, 0]),
        ("z", [0, 0, 1], [1, 0, 0], [0, 1, 0]),
        ("diagonal", [1, 1, 1], [1, -1, 0], [1, 1, -2]),
    ];

    #[test]
    fn self_intersection_sweep_misses_no_pair() {
        // check.py moved, here through the face [1, 4, 2] opposite +x:
        // vertex 0 from (1, 0, 0) to (-0.8, 0.8, 0.8) m, beyond x - y - z = -1 m.
        let mut moved = octa_mm(1000, [0; 3]);
        moved.geometry.vertices[0] = [-800, 800, 800];
        let crossing = union(octa_mm(1000, [0; 3]), octa_mm(1000, [500, 300, 100]));
        let edge = union(octa_mm(1000, [0; 3]), octa_mm(1000, [1000, 1000, 0]));
        let vertex = union(octa_mm(1000, [0; 3]), octa_mm(1000, [2000, 0, 0]));
        let two = union(octa_mm(1000, [0; 3]), octa_mm(1000, [5000, 0, 0]));
        let mut pushed = prism(6, 100, [1, 0, 0], [0, 1, 0], [0, 0, 1], false);
        pushed.geometry.vertices[6][0] = 150; // the vertex at i = 0 of the near cap, through the far one
        // (name, mesh, rules for `check`, whether some pair crosses).
        let mut cases: Vec<(String, Welded, Option<MeshRules>, bool)> = vec![
            ("octa", octa_mm(1000, [0; 3]), None, false),
            ("torus", torus(), None, false),
            ("fine_torus", fine_torus(24, 8, 4000., 1000.), None, false),
            ("two_apart", two, Some(MeshRules { components: 2, genus: 0, normals: Normals::Outward }), false),
            ("touch_on_edge", edge, None, false),
            ("touch_on_vertex", vertex, None, false),
            ("moved", moved, Some(MeshRules { components: 1, genus: 0, normals: Normals::Outward }), true),
            ("two_crossing", crossing, Some(MeshRules { components: 2, genus: 0, normals: Normals::Outward }), true),
            ("prism_pushed_through", pushed, None, true),
        ]
        .into_iter()
        .map(|(name, w, rules, crosses)| (name.to_string(), w, rules, crosses))
        .collect();
        for (axis, d, u, v) in PRISM_AXES {
            for fan in [false, true] {
                cases.push((format!("prism {axis} fan={fan}"), prism(6, 100, d, u, v, fan), None, false));
            }
        }
        for (name, w, rules, crosses) in cases {
            let (first, hits, _, _) = crossings(&w.geometry);
            assert_eq!((first, hits), all_pairs(&w.geometry), "{name}: sweep against every pair");
            assert_eq!(first.is_some(), crosses, "{name}");
            let Some(rules) = rules else { continue };
            match (check(&w, &rules), first) {
                (Ok(()), None) => {}
                (Err(r), Some((a, _))) => {
                    assert_eq!((r.code, r.element), ("mesh_self_intersection", Some(a)), "{name}: {}", r.detail);
                    assert!(r.detail.starts_with(&format!("{hits} pares")) && r.detail.matches("primitiva 0").count() == 2, "{name}: {}", r.detail);
                }
                (got, _) => panic!("{name}: {got:?}"),
            }
        }
    }

    /// Run with `--nocapture` to read the time.
    #[test]
    fn self_intersection_sweep_prunes_a_fine_torus() {
        let w = fine_torus(1000, 8, 60_000., 1000.);
        let n = w.geometry.triangles.len();
        let start = std::time::Instant::now();
        let (first, _, scanned, tested) = crossings(&w.geometry);
        let took = start.elapsed();
        eprintln!("fine torus: {n} triangles, {} pairs, {scanned} scanned, {tested} boxes overlap, {took:?}", n * (n - 1) / 2);
        assert_eq!(first, None);
        assert!(tested > 0 && tested < 20 * n, "{tested} boxes overlap for {n} triangles");
    }

    /// Long prisms whose side triangles are as long as the prism, along x, y
    /// and z: every side overlaps every other on that axis, so a sweep along
    /// it scans n² pairs. Run with `--nocapture` to read the times.
    #[test]
    fn self_intersection_sweep_avoids_the_axis_of_long_triangles() {
        let h = 6000;
        let mut report = Vec::new();
        for (axis, d, u, v) in &PRISM_AXES[..3] {
            let w = prism(h, 2 * h * h, *d, *u, *v, false);
            let n = w.geometry.triangles.len();
            let start = std::time::Instant::now();
            let (first, _, scanned, _) = crossings(&w.geometry);
            let took = start.elapsed();
            eprintln!("prism along {axis}: {n} triangles, {scanned} pairs scanned, {took:?}");
            report.push((*axis, first, n, scanned, took));
        }
        for (axis, first, n, scanned, took) in report {
            assert_eq!(first, None, "{axis}");
            assert!(scanned < 20 * n, "along {axis}: {scanned} pairs scanned for {n} triangles");
            assert!(took < std::time::Duration::from_secs(3), "along {axis}: {took:?}");
        }
    }
}
