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
    fn rejects_what_is_not_a_glb() {
        assert!(parse(b"not a glb at all, sorry").is_err());
    }
}
