//! What Kiln reads of a glTF (.glb or .gltf) without any engine: its bounding
//! box in the scene (node transforms applied), triangles, material slots and
//! images. glTF is Y-up and in metres, as Godot.

use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct GlbInfo {
    /// Bounding box of the default scene, metres, glTF axes (y up).
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub tris: u64,
    /// Material slot names, in file order (`material_N` when unnamed).
    pub materials: Vec<String>,
    pub images: usize,
}

impl GlbInfo {
    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

/// Reads a `.glb`, or a `.gltf` (its JSON is enough: accessors carry min and max).
pub fn read(path: &Path) -> Result<GlbInfo, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gltf"))
    {
        let doc = serde_json::from_slice(&bytes).map_err(|e| format!("glTF roto: {e}"))?;
        return from_doc(&doc);
    }
    parse(&bytes)
}

/// The JSON chunk of a GLB.
pub fn json_chunk(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("no es un GLB (falta la cabecera glTF)".into());
    }
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if &bytes[16..20] != b"JSON" || bytes.len() < 20 + len {
        return Err("GLB sin bloque JSON".into());
    }
    serde_json::from_slice(&bytes[20..20 + len]).map_err(|e| format!("JSON del GLB roto: {e}"))
}

pub fn parse(bytes: &[u8]) -> Result<GlbInfo, String> {
    from_doc(&json_chunk(bytes)?)
}

fn from_doc(doc: &Value) -> Result<GlbInfo, String> {
    let empty = Vec::new();
    let arr = |k: &str| doc.get(k).and_then(Value::as_array).unwrap_or(&empty);
    let (accessors, meshes, nodes) = (arr("accessors"), arr("meshes"), arr("nodes"));

    let materials = arr("materials")
        .iter()
        .enumerate()
        .map(|(i, m)| {
            m.get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or(format!("material_{i}"))
        })
        .collect();

    let mut bounds = Bounds::default();
    let mut tris = 0u64;
    let scene = doc.get("scene").and_then(Value::as_u64).unwrap_or(0) as usize;
    let roots: Vec<usize> = match arr("scenes").get(scene) {
        Some(s) => s
            .get("nodes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_default(),
        None => (0..nodes.len()).collect(),
    };
    let mut stack: Vec<(usize, Mat)> = roots.into_iter().map(|n| (n, IDENTITY)).collect();
    let mut guard = 0;
    while let Some((n, parent)) = stack.pop() {
        guard += 1;
        if guard > 100_000 {
            return Err("grafo de nodos con ciclos".into());
        }
        let Some(node) = nodes.get(n) else { continue };
        let world = mul(&parent, &local(node));
        if let Some(mesh) = node
            .get("mesh")
            .and_then(Value::as_u64)
            .and_then(|m| meshes.get(m as usize))
        {
            for p in mesh
                .get("primitives")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
            {
                let pos = p
                    .pointer("/attributes/POSITION")
                    .and_then(Value::as_u64)
                    .and_then(|a| accessors.get(a as usize));
                if let Some(acc) = pos
                    && let (Some(lo), Some(hi)) = (vec3(acc.get("min")), vec3(acc.get("max")))
                {
                    for c in 0..8 {
                        let corner = [
                            if c & 1 == 0 { lo[0] } else { hi[0] },
                            if c & 2 == 0 { lo[1] } else { hi[1] },
                            if c & 4 == 0 { lo[2] } else { hi[2] },
                        ];
                        bounds.add(apply(&world, corner));
                    }
                }
                let mode = p.get("mode").and_then(Value::as_u64).unwrap_or(4);
                let count = |a: Option<&Value>| {
                    a.and_then(Value::as_u64)
                        .and_then(|i| accessors.get(i as usize))
                        .and_then(|a| a.get("count"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                };
                let verts = match p.get("indices") {
                    Some(i) => count(Some(i)),
                    None => count(p.pointer("/attributes/POSITION")),
                };
                tris += match mode {
                    4 => verts / 3,
                    5 | 6 => verts.saturating_sub(2),
                    _ => 0,
                };
            }
        }
        for c in node
            .get("children")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            if let Some(c) = c.as_u64() {
                stack.push((c as usize, world));
            }
        }
    }
    let (min, max) = bounds.get().ok_or("el GLB no tiene mallas en su escena")?;
    Ok(GlbInfo {
        min,
        max,
        tris,
        materials,
        images: arr("images").len(),
    })
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Node {
    pub name: String,
    pub parent: Option<String>,
    pub translation: [f64; 3],
    /// Quaternion x, y, z, w.
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
    pub extras: std::collections::BTreeMap<String, Value>,
    pub mesh: Option<MeshInfo>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MeshInfo {
    pub tris: u64,
    /// Material name per primitive, in order; empty when it has none.
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Structure {
    pub nodes: Vec<Node>,
}

/// Reads the node structure of a `.glb` or `.gltf`.
pub fn structure(path: &Path) -> Result<Structure, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let doc = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gltf"))
    {
        serde_json::from_slice(&bytes).map_err(|e| format!("glTF roto: {e}"))?
    } else {
        json_chunk(&bytes)?
    };
    Ok(structure_of(&doc))
}

/// Every node of the document, in file order (not only those of the default scene).
pub fn structure_of(doc: &Value) -> Structure {
    let empty = Vec::new();
    let arr = |k: &str| doc.get(k).and_then(Value::as_array).unwrap_or(&empty);
    let (accessors, meshes, materials, nodes) = (
        arr("accessors"),
        arr("meshes"),
        arr("materials"),
        arr("nodes"),
    );
    let name_of = |v: Option<&Value>| {
        v.and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };

    let mut parents: Vec<Option<usize>> = vec![None; nodes.len()];
    for (i, n) in nodes.iter().enumerate() {
        for c in n
            .get("children")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            if let Some(slot) = c.as_u64().and_then(|c| parents.get_mut(c as usize)) {
                *slot = Some(i);
            }
        }
    }

    let count = |a: Option<&Value>| {
        a.and_then(Value::as_u64)
            .and_then(|i| accessors.get(i as usize))
            .and_then(|a| a.get("count"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let mesh_info = |m: &Value| {
        let (mut tris, mut mats) = (0u64, Vec::new());
        for p in m
            .get("primitives")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            if p.get("mode").and_then(Value::as_u64).unwrap_or(4) == 4 {
                let verts = match p.get("indices") {
                    Some(i) => count(Some(i)),
                    None => count(p.pointer("/attributes/POSITION")),
                };
                tris += verts / 3;
            }
            let mat = p
                .get("material")
                .and_then(Value::as_u64)
                .and_then(|i| materials.get(i as usize));
            mats.push(name_of(mat));
        }
        MeshInfo {
            tris,
            materials: mats,
        }
    };

    let out = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (mut translation, mut scale, mut rotation) = (
                vec3(n.get("translation")).unwrap_or([0.; 3]),
                [1.; 3],
                [0., 0., 0., 1.],
            );
            if let Some(s) = vec3(n.get("scale")) {
                scale = s;
            }
            if let Some(r) = n
                .get("rotation")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
                .filter(|v| v.len() == 4)
            {
                rotation = [r[0], r[1], r[2], r[3]];
            }
            if n.get("matrix").is_some() {
                let m = local(n);
                translation = [m[12], m[13], m[14]];
                let len = |c: usize| {
                    (m[c * 4].powi(2) + m[c * 4 + 1].powi(2) + m[c * 4 + 2].powi(2)).sqrt()
                };
                scale = [len(0), len(1), len(2)];
                rotation = [0., 0., 0., 1.];
            }
            Node {
                name: name_of(Some(n)),
                parent: parents[i].map(|p| name_of(nodes.get(p))),
                translation,
                rotation,
                scale,
                extras: n
                    .get("extras")
                    .and_then(Value::as_object)
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
                mesh: n
                    .get("mesh")
                    .and_then(Value::as_u64)
                    .and_then(|m| meshes.get(m as usize))
                    .map(mesh_info),
            }
        })
        .collect();
    Structure { nodes: out }
}

fn vec3(v: Option<&Value>) -> Option<[f64; 3]> {
    let a = v?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

#[derive(Default)]
struct Bounds(Option<([f64; 3], [f64; 3])>);
impl Bounds {
    fn add(&mut self, p: [f64; 3]) {
        let (lo, hi) = self.0.get_or_insert((p, p));
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    fn get(&self) -> Option<([f64; 3], [f64; 3])> {
        self.0
    }
}

/// Column-major 4x4, as glTF writes `matrix`.
type Mat = [f64; 16];
const IDENTITY: Mat = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

fn local(node: &Value) -> Mat {
    if let Some(m) = node.get("matrix").and_then(Value::as_array) {
        let v: Vec<f64> = m.iter().filter_map(Value::as_f64).collect();
        if v.len() == 16 {
            return v.try_into().unwrap();
        }
    }
    let t = vec3(node.get("translation")).unwrap_or([0.; 3]);
    let s = vec3(node.get("scale")).unwrap_or([1.; 3]);
    let r = node
        .get("rotation")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
        .filter(|v| v.len() == 4)
        .unwrap_or(vec![0., 0., 0., 1.]);
    let (x, y, z, w) = (r[0], r[1], r[2], r[3]);
    let rot = [
        1. - 2. * (y * y + z * z),
        2. * (x * y + z * w),
        2. * (x * z - y * w),
        2. * (x * y - z * w),
        1. - 2. * (x * x + z * z),
        2. * (y * z + x * w),
        2. * (x * z + y * w),
        2. * (y * z - x * w),
        1. - 2. * (x * x + y * y),
    ];
    [
        rot[0] * s[0],
        rot[1] * s[0],
        rot[2] * s[0],
        0.,
        rot[3] * s[1],
        rot[4] * s[1],
        rot[5] * s[1],
        0.,
        rot[6] * s[2],
        rot[7] * s[2],
        rot[8] * s[2],
        0.,
        t[0],
        t[1],
        t[2],
        1.,
    ]
}

fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut o = [0.; 16];
    for c in 0..4 {
        for r in 0..4 {
            o[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    o
}

fn apply(m: &Mat, p: [f64; 3]) -> [f64; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn fmt4(x: f64) -> String {
    let s = format!("{x:.4}");
    if s == "-0.0000" { "0.0000".into() } else { s }
}

/// Hash of the structure that two equivalent builds share: order of nodes,
/// the sign of quaternions and noise below 1e-4 do not change it.
pub fn structure_hash(s: &Structure) -> String {
    let mut nodes: Vec<&Node> = s.nodes.iter().collect();
    nodes.sort_by(|a, b| (&a.name, &a.parent).cmp(&(&b.name, &b.parent)));
    let mut text = String::new();
    for n in nodes {
        let mut q = n.rotation.map(|v| fmt4(v).parse::<f64>().unwrap_or(0.0));
        let flip = if q[3] != 0.0 {
            q[3] < 0.0
        } else {
            q[..3].iter().find(|v| **v != 0.0).is_some_and(|v| *v < 0.0)
        };
        if flip {
            q = q.map(|v| -v);
        }
        let list = |v: &[f64]| v.iter().map(|x| fmt4(*x)).collect::<Vec<_>>().join(",");
        text.push_str(&format!(
            "{}|{}|{}|{}|{}",
            n.name,
            n.parent.as_deref().unwrap_or(""),
            list(&n.translation),
            list(&q),
            list(&n.scale)
        ));
        if let Some(m) = &n.mesh {
            text.push_str(&format!("|mesh:{}:{}", m.tris, m.materials.join(",")));
        }
        for (k, v) in n.extras.iter().filter(|(k, _)| k.starts_with("kiln_")) {
            text.push_str(&format!("|{k}={v}"));
        }
        text.push('\n');
    }
    crate::asset::sha256_hex(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glb(json: &str) -> Vec<u8> {
        let mut j = json.as_bytes().to_vec();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((20 + j.len() as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(j);
        out
    }

    #[test]
    fn applies_node_scale_and_translation() {
        let info = parse(&glb(r#"{"scene":0,"scenes":[{"nodes":[0]}],
            "nodes":[{"scale":[2,2,2],"translation":[0,1,0],"children":[1]},{"mesh":0}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0}]}],
            "accessors":[{"count":4,"min":[-0.5,0,-0.5],"max":[0.5,1,0.5]},{"count":6}],
            "materials":[{"name":"wood"}]}"#))
        .unwrap();
        assert_eq!(info.min, [-1.0, 1.0, -1.0]);
        assert_eq!(info.max, [1.0, 3.0, 1.0]);
        assert_eq!(info.tris, 2);
        assert_eq!(info.materials, ["wood"]);
    }

    #[test]
    fn reads_node_structure() {
        let s = structure_of(
            &json_chunk(&glb(r#"{"nodes":[
                {"name":"door","translation":[1,2,3],"children":[1]},
                {"name":"leaf","rotation":[0,0.6,0,0.8],"mesh":0,
                 "extras":{"kiln_part":"leaf","kiln_moves":"slides"}}],
              "meshes":[{"primitives":[
                {"attributes":{"POSITION":0},"indices":1,"material":0},
                {"attributes":{"POSITION":2}}]}],
              "accessors":[{"count":4},{"count":6},{"count":3}],
              "materials":[{"name":"steel"}]}"#))
            .unwrap(),
        );
        let (door, leaf) = (&s.nodes[0], &s.nodes[1]);
        assert_eq!((door.name.as_str(), door.parent.clone()), ("door", None));
        assert_eq!(door.translation, [1.0, 2.0, 3.0]);
        assert_eq!(door.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(door.scale, [1.0; 3]);
        assert!(door.mesh.is_none() && door.extras.is_empty());
        assert_eq!(leaf.parent.as_deref(), Some("door"));
        assert_eq!(leaf.translation, [0.0; 3]);
        assert_eq!(leaf.rotation, [0.0, 0.6, 0.0, 0.8]);
        assert_eq!(leaf.extras["kiln_part"], "leaf");
        assert_eq!(leaf.extras["kiln_moves"], "slides");
        let mesh = leaf.mesh.as_ref().unwrap();
        assert_eq!(mesh.tris, 3);
        assert_eq!(mesh.materials, ["steel", ""]);
    }

    #[test]
    fn node_matrix_gives_translation_and_scale() {
        let s = structure_of(&serde_json::json!({"nodes":[{"name":"m","matrix":
            [2,0,0,0, 0,3,0,0, 0,0,4,0, 5,6,7,1]}]}));
        let n = &s.nodes[0];
        assert_eq!(n.translation, [5.0, 6.0, 7.0]);
        assert_eq!(n.scale, [2.0, 3.0, 4.0]);
        assert_eq!(n.rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    fn hnode(name: &str, parent: Option<&str>) -> Node {
        Node {
            name: name.into(),
            parent: parent.map(String::from),
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            extras: Default::default(),
            mesh: None,
        }
    }

    fn hash_of(nodes: Vec<Node>) -> String {
        structure_hash(&Structure { nodes })
    }

    fn base() -> Vec<Node> {
        let mut leaf = hnode("leaf", Some("door"));
        leaf.mesh = Some(MeshInfo {
            tris: 12,
            materials: vec!["steel".into()],
        });
        leaf.extras.insert("kiln_part".into(), "leaf".into());
        vec![hnode("door", None), leaf]
    }

    #[test]
    fn structure_hash_ignores_order_sign_and_noise() {
        let h = hash_of(base());
        let mut rev = base();
        rev.reverse();
        assert_eq!(hash_of(rev), h);

        let mut a = base();
        a[0].rotation = [0.0, 0.6, 0.0, 0.8];
        let mut b = base();
        b[0].rotation = [0.0, -0.6, 0.0, -0.8];
        assert_eq!(hash_of(a), hash_of(b));

        let mut a = base();
        a[0].rotation = [0.0, 0.0, 1.0, 0.0];
        let mut b = base();
        b[0].rotation = [0.0, 0.0, -1.0, -0.0];
        assert_eq!(hash_of(a), hash_of(b));

        let mut n = base();
        n[0].translation = [0.00001, -0.0, 0.0];
        assert_eq!(hash_of(n), h);

        let mut n = base();
        n[1].extras.insert("other".into(), "x".into());
        assert_eq!(hash_of(n), h);
    }

    #[test]
    fn structure_hash_sees_real_changes() {
        let h = hash_of(base());
        let mut n = base();
        n[0].translation[0] = 0.001;
        assert_ne!(hash_of(n), h);
        let mut n = base();
        n[1].mesh.as_mut().unwrap().tris = 13;
        assert_ne!(hash_of(n), h);
        let mut n = base();
        n[1].mesh.as_mut().unwrap().materials[0] = "wood".into();
        assert_ne!(hash_of(n), h);
        let mut n = base();
        n[1].parent = None;
        assert_ne!(hash_of(n), h);
        let mut n = base();
        n[1].extras.insert("kiln_part".into(), "panel".into());
        assert_ne!(hash_of(n), h);
    }

    #[test]
    fn rejects_what_is_not_a_glb() {
        assert!(parse(b"not a glb at all, sorry").is_err());
    }
}
