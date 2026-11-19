//! Triangle meshes of a `.glb`, read defensively: the file may be damaged or
//! crafted, so every offset and count is checked before it is used.

use crate::glb::json_chunk;
use serde_json::Value;
use std::path::Path;

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    /// A multiple of 3, every one below `positions.len()`.
    pub indices: Vec<u32>,
}

pub const MAX_VERTICES: usize = 2_000_000;
const BIN: u32 = 0x004E_4942;

pub fn read(path: &Path) -> Result<Vec<Mesh>, String> {
    read_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
}

pub fn read_bytes(bytes: &[u8]) -> Result<Vec<Mesh>, String> {
    read_within(bytes, MAX_VERTICES)
}

pub fn node_transform(path: &Path) -> Result<Option<String>, String> {
    node_transform_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
}

/// `read` takes positions as they are in the mesh and ignores the nodes. This
/// says whether that differs from what the file places: the first node that
/// holds a mesh, or is an ancestor of one, with a transform that is not the
/// identity (`"nodo N: translation"`). A malformed transform counts as one.
pub fn node_transform_bytes(bytes: &[u8]) -> Result<Option<String>, String> {
    let doc = json_chunk(bytes)?;
    let nodes = doc["nodes"].as_array().map(Vec::as_slice).unwrap_or_default();
    let mut parents = vec![Vec::new(); nodes.len()];
    for (i, n) in nodes.iter().enumerate() {
        for c in n["children"].as_array().into_iter().flatten() {
            if let Some(c) = c.as_u64().and_then(|c| usize::try_from(c).ok()).filter(|&c| c < nodes.len()) {
                parents[c].push(i);
            }
        }
    }
    // A node places a mesh when it holds one or a descendant does. One walk up
    // from the mesh nodes, each node visited once, so a crafted file with
    // cycles or many nodes stays linear.
    let mut places_mesh = vec![false; nodes.len()];
    let mut stack: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].get("mesh").is_some()).collect();
    while let Some(i) = stack.pop() {
        if !std::mem::replace(&mut places_mesh[i], true) {
            stack.extend(&parents[i]);
        }
    }
    const IDENTITY: [(&str, &[f64]); 4] = [
        ("translation", &[0., 0., 0.]),
        ("rotation", &[0., 0., 0., 1.]),
        ("scale", &[1., 1., 1.]),
        ("matrix", &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]),
    ];
    for (i, node) in nodes.iter().enumerate() {
        for (key, identity) in IDENTITY {
            let Some(v) = node.get(key) else { continue };
            let values: Option<Vec<f64>> = v.as_array().and_then(|a| a.iter().map(Value::as_f64).collect());
            if values.as_deref() != Some(identity) && places_mesh[i] {
                return Ok(Some(format!("nodo {i}: {key}")));
            }
        }
    }
    Ok(None)
}

/// Primitives may share accessors, so a small file can name the same big
/// buffer many times: the vertices of the whole file are bounded, not only
/// those of each primitive.
fn read_within(bytes: &[u8], max_total: usize) -> Result<Vec<Mesh>, String> {
    let mut total = 0usize;
    let doc = json_chunk(bytes)?;
    if u32_at(bytes, 4) != Some(2) {
        return Err("GLB roto: solo se admite la versión 2".into());
    }
    let bin = bin_chunk(bytes)?;
    let mut meshes = Vec::new();
    for mesh in doc["meshes"].as_array().into_iter().flatten() {
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim.get("mode").is_some_and(|m| m.as_u64() != Some(4)) {
                continue;
            }
            let m = primitive(&doc, bin, prim)?;
            total = total.saturating_add(m.positions.len().max(m.indices.len() / 3));
            if total > max_total {
                return Err(format!("GLB roto: pasa de {max_total} vértices entre todas sus mallas"));
            }
            meshes.push(m);
        }
    }
    Ok(meshes)
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_le_bytes(b.get(at..end)?.try_into().ok()?))
}

fn bin_chunk(bytes: &[u8]) -> Result<&[u8], String> {
    let broken = || "GLB roto: bloques truncados".to_string();
    let mut at = (u32_at(bytes, 12).ok_or_else(broken)? as usize)
        .checked_add(20)
        .ok_or_else(broken)?;
    while at < bytes.len() {
        let len = u32_at(bytes, at).ok_or_else(broken)? as usize;
        let kind = u32_at(bytes, at + 4).ok_or_else(broken)?;
        let start = at.checked_add(8).ok_or_else(broken)?;
        let end = start.checked_add(len).ok_or_else(broken)?;
        let data = bytes.get(start..end).ok_or_else(broken)?;
        if kind == BIN {
            return Ok(data);
        }
        at = end.checked_add(3).ok_or_else(broken)? & !3;
    }
    Err("GLB sin bloque BIN".into())
}

struct Located<'a> {
    count: usize,
    component: u64,
    data: &'a [u8],
    stride: usize,
}

fn field(v: &Value, key: &str) -> Result<usize, String> {
    match v.get(key) {
        None => Ok(0),
        Some(n) => n
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| format!("GLB roto: «{key}» no es un entero válido")),
    }
}

/// Finds an accessor and checks that all its elements fit in the BIN chunk.
fn locate<'a>(
    doc: &Value,
    bin: &'a [u8],
    index: &Value,
    ty: &str,
    components: &[u64],
    max_count: usize,
) -> Result<Located<'a>, String> {
    let acc = index
        .as_u64()
        .and_then(|i| doc["accessors"].get(usize::try_from(i).ok()?))
        .ok_or("GLB roto: accessor inexistente")?;
    if acc.get("sparse").is_some() {
        return Err("no se admite un accessor sparse".into());
    }
    if acc["type"] != ty {
        return Err(format!("GLB roto: se esperaba un accessor {ty}"));
    }
    let component = acc["componentType"].as_u64().unwrap_or(0);
    if !components.contains(&component) {
        return Err(format!("no se admite componentType {component} aquí"));
    }
    let count = field(acc, "count")?;
    if count > max_count {
        return Err(format!("GLB roto: {count} elementos pasan del máximo {max_count}"));
    }
    let view = acc["bufferView"]
        .as_u64()
        .and_then(|i| doc["bufferViews"].get(usize::try_from(i).ok()?))
        .ok_or("GLB roto: accessor sin bufferView")?;
    if field(view, "buffer")? != 0 || doc["buffers"][0].get("uri").is_some() {
        return Err("no se admite un buffer fuera del BIN del GLB".into());
    }
    let size = match component {
        5121 => 1,
        5123 => 2,
        _ => 4,
    } * if ty == "VEC3" { 3 } else { 1 };
    let stride = match field(view, "byteStride")? {
        0 => size,
        s => s,
    };
    let view_start = field(view, "byteOffset")?;
    let view_end = view_start
        .checked_add(field(view, "byteLength")?)
        .filter(|e| *e <= bin.len())
        .ok_or("GLB roto: un bufferView pasa del final del BIN")?;
    let start = view_start
        .checked_add(field(acc, "byteOffset")?)
        .ok_or("GLB roto: byteOffset desbordado")?;
    let span = match count {
        0 => 0,
        n => (n - 1)
            .checked_mul(stride)
            .and_then(|s| s.checked_add(size))
            .ok_or("GLB roto: accessor desbordado")?,
    };
    let end = start
        .checked_add(span)
        .filter(|e| *e <= view_end)
        .ok_or("GLB roto: un accessor pasa del final de su bufferView")?;
    Ok(Located {
        count,
        component,
        data: &bin[start..end],
        stride,
    })
}

fn primitive(doc: &Value, bin: &[u8], prim: &Value) -> Result<Mesh, String> {
    let pos = locate(doc, bin, &prim["attributes"]["POSITION"], "VEC3", &[5126], MAX_VERTICES)?;
    let mut positions = Vec::with_capacity(pos.count);
    for i in 0..pos.count {
        let at = i * pos.stride;
        let mut p = [0f32; 3];
        for (k, v) in p.iter_mut().enumerate() {
            let raw = pos.data.get(at + 4 * k..at + 4 * k + 4).ok_or("GLB roto: POSITION truncado")?;
            *v = f32::from_le_bytes(raw.try_into().map_err(|_| "GLB roto: POSITION truncado")?);
            if !v.is_finite() {
                return Err("GLB roto: posición no finita".into());
            }
        }
        positions.push(p);
    }
    let indices: Vec<u32> = match prim.get("indices") {
        None => (0..positions.len() as u32).collect(),
        Some(i) => {
            let ix = locate(doc, bin, i, "SCALAR", &[5121, 5123, 5125], 3 * MAX_VERTICES)?;
            let width = match ix.component {
                5121 => 1,
                5123 => 2,
                _ => 4,
            };
            (0..ix.count)
                .map(|n| {
                    let raw = ix.data.get(n * ix.stride..n * ix.stride + width)?;
                    let mut b = [0u8; 4];
                    b[..width].copy_from_slice(raw);
                    Some(u32::from_le_bytes(b))
                })
                .collect::<Option<_>>()
                .ok_or("GLB roto: índices truncados")?
        }
    };
    if !indices.len().is_multiple_of(3) {
        return Err("GLB roto: el número de índices no es múltiplo de 3".into());
    }
    if indices.iter().any(|&i| i as usize >= positions.len()) {
        return Err("GLB roto: un índice pasa del último vértice".into());
    }
    Ok(Mesh { positions, indices })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut j = json.as_bytes().to_vec();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut b = bin.to_vec();
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((12 + 8 + j.len() as u32 + 8 + b.len() as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(j);
        if !bin.is_empty() {
            out.extend((b.len() as u32).to_le_bytes());
            out.extend(BIN.to_le_bytes());
            out.extend(b);
        }
        out
    }

    const V: [[f32; 3]; 4] = [[1., 1., 1.], [1., -1., -1.], [-1., 1., -1.], [-1., -1., 1.]];
    const T: [[usize; 3]; 4] = [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];

    fn floats(p: &[[f32; 3]], pad: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for v in p {
            for c in v {
                out.extend(c.to_le_bytes());
            }
            out.extend(vec![0u8; pad]);
        }
        out
    }

    /// Positions `[0, 48)`, indices after them; `stride` pads each vertex.
    fn tetra(indexed: bool, wide: bool) -> Vec<u8> {
        if !indexed {
            let p: Vec<[f32; 3]> = T.iter().flatten().map(|&i| V[i]).collect();
            let json = r#"{"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
                "accessors":[{"bufferView":0,"componentType":5126,"count":12,"type":"VEC3"}],
                "bufferViews":[{"buffer":0,"byteLength":144}],"buffers":[{"byteLength":144}]}"#;
            return glb(json, &floats(&p, 0));
        }
        let mut bin = floats(&V, 0);
        for i in T.iter().flatten() {
            if wide {
                bin.extend((*i as u32).to_le_bytes());
            } else {
                bin.extend((*i as u16).to_le_bytes());
            }
        }
        let (ct, len) = if wide { (5125, 48) } else { (5123, 24) };
        let json = format!(
            r#"{{"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1}}]}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3"}},
                {{"bufferView":1,"componentType":{ct},"count":12,"type":"SCALAR"}}],
            "bufferViews":[{{"buffer":0,"byteLength":48}},{{"buffer":0,"byteOffset":48,"byteLength":{len}}}],
            "buffers":[{{"byteLength":{}}}]}}"#,
            48 + len
        );
        glb(&json, &bin)
    }

    fn check_tetra(meshes: Vec<Mesh>) {
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        assert_eq!(m.indices.len(), 12);
        for (t, tri) in T.iter().enumerate() {
            for (k, &v) in tri.iter().enumerate() {
                assert_eq!(m.positions[m.indices[t * 3 + k] as usize], V[v]);
            }
        }
    }

    #[test]
    fn reads_every_form_of_tetrahedron() {
        for (indexed, wide) in [(true, false), (true, true), (false, false)] {
            check_tetra(read_bytes(&tetra(indexed, wide)).unwrap());
        }
    }

    #[test]
    fn the_whole_file_is_bounded_not_each_primitive() {
        let bytes = tetra(true, false);
        // 4 vertices in the file: within a total of 4, over a total of 3
        assert!(read_within(&bytes, 4).is_ok());
        assert!(read_within(&bytes, 3).is_err_and(|e| e.contains("entre todas sus mallas")));
    }

    #[test]
    fn honours_byte_stride() {
        let json = r#"{"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],
            "bufferViews":[{"buffer":0,"byteLength":48,"byteStride":16}],"buffers":[{"byteLength":48}]}"#;
        let m = read_bytes(&glb(json, &floats(&V[..3], 4))).unwrap();
        assert_eq!(m[0].positions, V[..3].to_vec());
    }

    #[test]
    fn skips_other_modes() {
        let json = r#"{"meshes":[{"primitives":[{"attributes":{"POSITION":0},"mode":1}]}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3"}],
            "bufferViews":[{"buffer":0,"byteLength":48}],"buffers":[{"byteLength":48}]}"#;
        assert!(read_bytes(&glb(json, &floats(&V, 0))).unwrap().is_empty());
    }

    /// One accessor over `bin`, with `extra` JSON fields in the accessor.
    fn one(acc: &str, view: &str, bin: &[u8]) -> Vec<u8> {
        let json = format!(
            r#"{{"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}{}}}]}}],
            "accessors":[{acc}],"bufferViews":[{view}],"buffers":[{{"byteLength":48}}]}}"#,
            ""
        );
        glb(&json, bin)
    }

    const ACC: &str = r#"{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3"}"#;
    const VIEW: &str = r#"{"buffer":0,"byteLength":48}"#;

    /// The indexed tetrahedron, with `nodes` and a scene over node 0.
    fn tetra_with_nodes(nodes: &str) -> Vec<u8> {
        let base = tetra(true, false);
        let doc = json_chunk(&base).unwrap();
        let mut doc = doc.as_object().unwrap().clone();
        doc.insert("nodes".into(), serde_json::from_str(nodes).unwrap());
        doc.insert("scenes".into(), serde_json::json!([{"nodes": [0]}]));
        let bin = bin_chunk(&base).unwrap().to_vec();
        glb(&Value::Object(doc).to_string(), &bin)
    }

    #[test]
    fn detects_a_transform_on_the_mesh_node() {
        for node in [
            r#"[{"mesh":0,"translation":[0,0.5,0]}]"#,
            r#"[{"mesh":0,"rotation":[0,0.7071068,0,0.7071068]}]"#,
            r#"[{"mesh":0,"scale":[2,2,2]}]"#,
            r#"[{"mesh":0,"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,3,1]}]"#,
            r#"[{"mesh":0,"translation":"x"}]"#,
            r#"[{"children":[1],"translation":[1,0,0]},{"mesh":0}]"#,
        ] {
            let found = node_transform_bytes(&tetra_with_nodes(node)).unwrap();
            assert!(found.is_some(), "{node} must be detected");
        }
        let found = node_transform_bytes(&tetra_with_nodes(r#"[{"mesh":0,"translation":[0,0.5,0]}]"#))
            .unwrap()
            .unwrap();
        assert!(found.contains("translation"), "{found}");
    }

    #[test]
    fn an_identity_or_absent_transform_is_not_detected() {
        for node in [
            r#"[{"mesh":0}]"#,
            r#"[{"mesh":0,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]}]"#,
            r#"[{"mesh":0,"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}]"#,
            r#"[{"mesh":0},{"translation":[5,0,0]}]"#,
        ] {
            assert_eq!(node_transform_bytes(&tetra_with_nodes(node)).unwrap(), None, "{node}");
        }
        assert_eq!(node_transform_bytes(&tetra(true, false)).unwrap(), None);
        // read keeps ignoring the transform
        check_tetra(read_bytes(&tetra_with_nodes(r#"[{"mesh":0,"translation":[0,0.5,0]}]"#)).unwrap());
    }

    #[test]
    fn damaged_files_are_errors() {
        let good = tetra(true, false);
        let mut cases: Vec<(&str, Vec<u8>)> = vec![
            ("truncated in BIN", good[..good.len() - 10].to_vec()),
            ("view past the BIN", one(ACC, r#"{"buffer":0,"byteOffset":8,"byteLength":48}"#, &floats(&V, 0))),
            ("huge count", one(&ACC.replace('4', "4000000000"), VIEW, &floats(&V, 0))),
            ("VEC2 position", one(&ACC.replace("VEC3", "VEC2"), VIEW, &floats(&V, 0))),
            ("no BIN", glb(r#"{"meshes":[]}"#, &[])),
            ("empty", Vec::new()),
            ("random", (0..64u8).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect()),
        ];
        let mut nan = V;
        nan[2][1] = f32::NAN;
        cases.push(("NaN", one(ACC, VIEW, &floats(&nan, 0))));
        let wrong = |tweak: &dyn Fn(&mut Vec<u8>), json: &str| {
            let mut bin = floats(&V, 0);
            tweak(&mut bin);
            glb(json, &bin)
        };
        let idx = |count: u32| {
            format!(
                r#"{{"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1}}]}}],
                "accessors":[{ACC},{{"bufferView":1,"componentType":5123,"count":{count},"type":"SCALAR"}}],
                "bufferViews":[{VIEW},{{"buffer":0,"byteOffset":48,"byteLength":24}}],"buffers":[{{"byteLength":72}}]}}"#
            )
        };
        let put = |ix: [u16; 12]| {
            move |b: &mut Vec<u8>| ix.iter().for_each(|i| b.extend(i.to_le_bytes()))
        };
        cases.push(("index = vertex count", wrong(&put([0, 1, 4, 0, 1, 2, 0, 1, 2, 0, 1, 2]), &idx(12))));
        cases.push(("count not multiple of 3", wrong(&put([0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2]), &idx(11))));
        for (name, bytes) in cases {
            assert!(read_bytes(&bytes).is_err(), "{name} must be an error");
        }
    }
}
