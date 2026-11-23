//! A preview of a GLB without Blender or Godot: the triangles of every mesh,
//! placed by their nodes and coloured by their material, drawn flat-shaded
//! from the front, the side, above and at three quarters, on one PNG sheet.
//! It is for the person to see the asset in the conversation, not a render.

use crate::glb::json_chunk;
use crate::mesh::{MAX_VERTICES, bin_chunk, primitive, u32_at};
use serde_json::Value;

type M4 = [[f64; 4]; 4];

const IDENTITY: M4 = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
const MAX_NODES: usize = 100_000;

fn mul(a: &M4, b: &M4) -> M4 {
    let mut r = [[0.; 4]; 4];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

fn nums(v: &Value, n: usize) -> Option<Vec<f64>> {
    let a = v.as_array()?;
    (a.len() == n).then(|| a.iter().map(|x| x.as_f64().filter(|f| f.is_finite())).collect::<Option<Vec<_>>>())?
}

/// The local matrix of a node (row-major, column vectors).
fn local(node: &Value) -> M4 {
    if let Some(m) = nums(&node["matrix"], 16) {
        // glTF stores the matrix column-major.
        let mut r = [[0.; 4]; 4];
        for c in 0..4 {
            for (row, line) in r.iter_mut().enumerate() {
                line[c] = m[c * 4 + row];
            }
        }
        return r;
    }
    let t = nums(&node["translation"], 3).unwrap_or(vec![0., 0., 0.]);
    let q = nums(&node["rotation"], 4).unwrap_or(vec![0., 0., 0., 1.]);
    let s = nums(&node["scale"], 3).unwrap_or(vec![1., 1., 1.]);
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [1. - 2. * (y * y + z * z), 2. * (x * y - z * w), 2. * (x * z + y * w)],
        [2. * (x * y + z * w), 1. - 2. * (x * x + z * z), 2. * (y * z - x * w)],
        [2. * (x * z - y * w), 2. * (y * z + x * w), 1. - 2. * (x * x + y * y)],
    ];
    let mut m = IDENTITY;
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = r[i][j] * s[j];
        }
        m[i][3] = t[i];
    }
    m
}

/// A triangle in the asset's space, with its colour (linear 0..1).
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pub p: [[f64; 3]; 3],
    pub color: [f64; 3],
    /// Which primitive it came from, to draw the lines between pieces.
    pub group: u32,
}

/// The triangles of a GLB, placed by the nodes of its scene.
pub fn triangles(bytes: &[u8]) -> Result<Vec<Tri>, String> {
    let doc = json_chunk(bytes)?;
    if u32_at(bytes, 4) != Some(2) {
        return Err("GLB roto: solo se admite la versión 2".into());
    }
    let bin = bin_chunk(bytes).unwrap_or(&[]);
    let nodes = doc["nodes"].as_array().map(Vec::as_slice).unwrap_or_default();
    if nodes.len() > MAX_NODES {
        return Err(format!("GLB con más de {MAX_NODES} nodos"));
    }
    let colors: Vec<[f64; 3]> = doc["materials"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| {
            nums(&m["pbrMetallicRoughness"]["baseColorFactor"], 4)
                .map(|c| [c[0], c[1], c[2]].map(|v| v.clamp(0., 1.)))
                .unwrap_or([0.8, 0.8, 0.8])
        })
        .collect();
    // Roots: the nodes of the scene, or every node nobody holds.
    let scene = doc["scene"].as_u64().unwrap_or(0) as usize;
    let mut roots: Vec<usize> = doc["scenes"][scene]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_u64().map(|n| n as usize))
        .filter(|&n| n < nodes.len())
        .collect();
    if roots.is_empty() {
        let mut held = vec![false; nodes.len()];
        for n in nodes {
            for c in n["children"].as_array().into_iter().flatten().filter_map(Value::as_u64) {
                if let Some(h) = held.get_mut(c as usize) {
                    *h = true;
                }
            }
        }
        roots = (0..nodes.len()).filter(|&i| !held[i]).collect();
    }
    let mut out = Vec::new();
    let mut seen = vec![false; nodes.len()];
    let mut stack: Vec<(usize, M4)> = roots.into_iter().map(|r| (r, IDENTITY)).collect();
    let mut verts = 0usize;
    let mut group = 0u32;
    while let Some((i, parent)) = stack.pop() {
        if std::mem::replace(&mut seen[i], true) {
            continue; // a node held twice, or a cycle
        }
        let m = mul(&parent, &local(&nodes[i]));
        for c in nodes[i]["children"].as_array().into_iter().flatten().filter_map(Value::as_u64) {
            if (c as usize) < nodes.len() {
                stack.push((c as usize, m));
            }
        }
        let Some(mesh) = nodes[i]["mesh"].as_u64().and_then(|k| doc["meshes"].get(k as usize)) else { continue };
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim.get("mode").is_some_and(|m| m.as_u64() != Some(4)) {
                continue;
            }
            let pm = primitive(&doc, bin, prim)?;
            verts = verts.saturating_add(pm.positions.len().max(pm.indices.len() / 3));
            if verts > MAX_VERTICES {
                return Err(format!("GLB con más de {MAX_VERTICES} vértices"));
            }
            let color = prim["material"].as_u64().and_then(|k| colors.get(k as usize)).copied().unwrap_or([0.8; 3]);
            let place = |v: [f32; 3]| -> [f64; 3] {
                let v = [v[0] as f64, v[1] as f64, v[2] as f64];
                [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2] + m[r][3])
            };
            for t in pm.indices.chunks_exact(3) {
                out.push(Tri { p: [0, 1, 2].map(|k| place(pm.positions[t[k] as usize])), color, group });
            }
            group += 1;
        }
    }
    Ok(out)
}

/// One view: screen right, screen up and the axis towards the viewer.
struct View {
    right: [f64; 3],
    up: [f64; 3],
    toward: [f64; 3],
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 0. { a.map(|v| v / l) } else { a }
}

/// Front (the face of a piece looks to +Z in glTF), side (from +X), top
/// (from +Y, the front at the bottom) and three quarters.
fn views() -> [View; 4] {
    let toward = norm([0.6, 0.45, 0.75]);
    let right = norm(cross([0., 1., 0.], toward));
    let up = cross(toward, right);
    [
        View { right: [1., 0., 0.], up: [0., 1., 0.], toward: [0., 0., 1.] },
        View { right: [0., 0., -1.], up: [0., 1., 0.], toward: [1., 0., 0.] },
        View { right: [1., 0., 0.], up: [0., 0., -1.], toward: [0., 1., 0.] },
        View { right, up, toward },
    ]
}

/// The labels of the four views, in the order of the sheet (left to right, top to bottom).
pub const VIEWS: [&str; 4] = ["frente", "perfil", "planta", "tres cuartos"];

/// What the sheet shows.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Sheet {
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
    pub triangles: usize,
    /// Size of the asset in metres (x, y, z).
    pub size_m: [f64; 3],
}

/// Draws the four views of `tris` on a 2x2 sheet of `tile` pixels a side.
pub fn sheet(tris: &[Tri], tile: u32) -> Sheet {
    let tile = tile.clamp(64, 1024) as usize;
    let (w, h) = (tile * 2, tile * 2);
    let bg = [236u8, 236, 232];
    let mut rgb = vec![0u8; w * h * 3];
    for px in rgb.chunks_exact_mut(3) {
        px.copy_from_slice(&bg);
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in tris {
        for p in &t.p {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let size_m = if tris.is_empty() { [0.; 3] } else { [0, 1, 2].map(|k| hi[k] - lo[k]) };
    let center = [0, 1, 2].map(|k| if tris.is_empty() { 0. } else { (lo[k] + hi[k]) / 2. });
    for (vi, v) in views().iter().enumerate() {
        let (ox, oy) = ((vi % 2) * tile, (vi / 2) * tile);
        let proj = |p: [f64; 3]| {
            let d = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
            (dot(d, v.right), dot(d, v.up), dot(d, v.toward))
        };
        // Each view fills its tile: a tall lamp is not lost in the plan.
        let mut extent = 1e-6f64;
        for t in tris {
            for p in &t.p {
                let (x, y, _) = proj(*p);
                extent = extent.max(2. * x.abs()).max(2. * y.abs());
            }
        }
        let scale = tile as f64 * 0.84 / extent;
        let mut depth = vec![f64::NEG_INFINITY; tile * tile];
        let mut group = vec![u32::MAX; tile * tile];
        let light = norm([-0.35, 0.6, 0.72]);
        for t in tris {
            let s = t.p.map(|p| {
                let (x, y, z) = proj(p);
                (tile as f64 / 2. + x * scale, tile as f64 / 2. - y * scale, z)
            });
            // Normal in view space from the world normal, two-sided.
            let wn = norm(cross(
                [t.p[1][0] - t.p[0][0], t.p[1][1] - t.p[0][1], t.p[1][2] - t.p[0][2]],
                [t.p[2][0] - t.p[0][0], t.p[2][1] - t.p[0][1], t.p[2][2] - t.p[0][2]],
            ));
            let vn = [dot(wn, v.right), dot(wn, v.up), dot(wn, v.toward)];
            let vn = if vn[2] < 0. { vn.map(|c| -c) } else { vn };
            let shade = 0.38 + 0.62 * dot(vn, light).max(0.);
            let color = t.color.map(|c| (linear_to_srgb(c * shade) * 255.).round().clamp(0., 255.) as u8);
            let min_x = s.iter().map(|p| p.0).fold(f64::INFINITY, f64::min).floor().max(0.) as usize;
            let max_x = s.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max).ceil().min(tile as f64 - 1.);
            let min_y = s.iter().map(|p| p.1).fold(f64::INFINITY, f64::min).floor().max(0.) as usize;
            let max_y = s.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max).ceil().min(tile as f64 - 1.);
            if max_x < 0. || max_y < 0. {
                continue;
            }
            let area = (s[1].0 - s[0].0) * (s[2].1 - s[0].1) - (s[2].0 - s[0].0) * (s[1].1 - s[0].1);
            if area.abs() < 1e-12 {
                continue;
            }
            for y in min_y..=max_y as usize {
                for x in min_x..=max_x as usize {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let e = |a: (f64, f64, f64), b: (f64, f64, f64)| (b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0);
                    let (w0, w1, w2) = (e(s[1], s[2]) / area, e(s[2], s[0]) / area, e(s[0], s[1]) / area);
                    if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                        continue;
                    }
                    let z = w0 * s[0].2 + w1 * s[1].2 + w2 * s[2].2;
                    let k = y * tile + x;
                    if z > depth[k] {
                        depth[k] = z;
                        group[k] = t.group;
                        let at = ((oy + y) * w + ox + x) * 3;
                        rgb[at..at + 3].copy_from_slice(&color);
                    }
                }
            }
        }
        // Dark lines where the asset meets the background or one piece another.
        let line = |a: usize, b: usize| {
            let (ga, gb) = (group[a], group[b]);
            ga != gb && (ga != u32::MAX || gb != u32::MAX)
        };
        for y in 0..tile {
            for x in 0..tile {
                let k = y * tile + x;
                let edge = (x + 1 < tile && line(k, k + 1))
                    || (y + 1 < tile && line(k, k + tile))
                    || (x > 0 && line(k, k - 1))
                    || (y > 0 && line(k, k - tile));
                if edge && group[k] != u32::MAX {
                    let at = ((oy + y) * w + ox + x) * 3;
                    for c in &mut rgb[at..at + 3] {
                        *c = (*c as f64 * 0.45) as u8;
                    }
                }
            }
        }
    }
    // A thin grid between the views.
    for i in 0..w {
        for (x, y) in [(i, tile), (tile, i)] {
            let at = (y.min(h - 1) * w + x.min(w - 1)) * 3;
            rgb[at..at + 3].copy_from_slice(&[200, 200, 196]);
        }
    }
    Sheet { width: w as u32, height: h as u32, png: crate::png::encode(w as u32, h as u32, &rgb), triangles: tris.len(), size_m }
}

fn linear_to_srgb(c: f64) -> f64 {
    let c = c.clamp(0., 1.);
    if c <= 0.003_130_8 { 12.92 * c } else { 1.055 * c.powf(1. / 2.4) - 0.055 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut j = json.as_bytes().to_vec();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((12 + 8 + j.len() as u32 + 8 + bin.len() as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(j);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(0x004E_4942u32.to_le_bytes());
        out.extend(bin);
        out
    }

    /// One triangle, in a node moved 2 m up and scaled twice, red.
    fn one() -> Vec<u8> {
        let mut bin = Vec::new();
        for v in [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
            for c in v {
                bin.extend(c.to_le_bytes());
            }
        }
        let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
            "nodes":[{"children":[1],"translation":[0,2,0]},{"mesh":0,"scale":[2,2,2]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],
            "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1]}}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],
            "bufferViews":[{"buffer":0,"byteLength":36}],"buffers":[{"byteLength":36}]}"#;
        glb(json, &bin)
    }

    #[test]
    fn places_triangles_by_their_nodes_and_colours_them() {
        let t = triangles(&one()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].p, [[0., 2., 0.], [2., 2., 0.], [0., 4., 0.]]);
        assert_eq!(t[0].color, [1., 0., 0.]);
    }

    #[test]
    fn a_cycle_of_nodes_does_not_hang() {
        let json = r#"{"asset":{"version":"2.0"},"nodes":[{"children":[1]},{"children":[0]}]}"#;
        assert!(triangles(&glb(json, &[])).unwrap().is_empty());
    }

    #[test]
    fn the_sheet_draws_the_asset_in_its_colour() {
        let s = sheet(&triangles(&one()).unwrap(), 64);
        assert_eq!((s.width, s.height), (128, 128));
        assert_eq!(s.size_m, [2., 2., 0.]);
        // Decode the PNG back and look for red pixels in the front view.
        let idat = s.png.windows(4).position(|w| w == b"IDAT").unwrap();
        let len = u32::from_be_bytes(s.png[idat - 4..idat].try_into().unwrap()) as usize;
        let z = &s.png[idat + 4..idat + 4 + len];
        let raw = crate::png::tests::inflate_fixed(&z[2..z.len() - 4]);
        let row = 128 * 3;
        let mut rgb = vec![0u8; 128 * row];
        for y in 0..128 {
            let f = raw[y * (row + 1)];
            for i in 0..row {
                let x = raw[y * (row + 1) + 1 + i];
                let left = if i >= 3 { rgb[y * row + i - 3] } else { 0 };
                let up = if y > 0 { rgb[(y - 1) * row + i] } else { 0 };
                rgb[y * row + i] = x.wrapping_add(if f == 1 { left } else { up });
            }
        }
        let red = (0..64 * 64).filter(|k| {
            let (x, y) = (k % 64, k / 64);
            let p = &rgb[y * row + x * 3..y * row + x * 3 + 3];
            p[0] > 150 && p[1] < 60 && p[2] < 60
        });
        assert!(red.count() > 200, "the front view shows the red triangle");
    }
}
