//! Checks on a GLB that need no external tool, and the verification report.

use crate::catalog::{Finding, Severity};
use crate::glb;
use serde_json::Value;
use std::collections::BTreeMap;

const BIN_CHUNK: u32 = 0x004E_4942;

/// Length of the binary chunk of a GLB; 0 when it has none.
fn bin_len(bytes: &[u8]) -> usize {
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap());
        if kind == BIN_CHUNK {
            return len.min(bytes.len() - at - 8);
        }
        at = at.saturating_add(8).saturating_add(len);
    }
    0
}

/// What is wrong with the GLB itself, whatever asset it is.
pub fn glb_findings(subject: &str, bytes: &[u8]) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut push = |severity, code: &str, message: String| {
        out.push(Finding {
            severity,
            subject: subject.into(),
            code: code.into(),
            message,
        })
    };
    let doc = match glb::json_chunk(bytes) {
        Ok(d) => d,
        Err(e) => {
            push(Severity::Error, "glb_invalid", format!("GLB inválido: {e}"));
            return out;
        }
    };

    let empty = Vec::new();
    let arr = |v: &'_ Value, k: &str| v.get(k).and_then(Value::as_array).unwrap_or(&empty).clone();
    let len_of = |k: &str| arr(&doc, k).len();
    let mut broken = |ptr: String, target: &str, idx: &Value| {
        if let Some(i) = idx.as_u64()
            && i as usize >= len_of(target)
        {
            push(
                Severity::Error,
                "glb_broken_ref",
                format!("{ptr} apunta a {target}[{i}], que no existe"),
            );
        }
    };

    if let Some(s) = doc.get("scene") {
        broken("/scene".into(), "scenes", s);
    }
    for (i, s) in arr(&doc, "scenes").iter().enumerate() {
        for (j, n) in arr(s, "nodes").iter().enumerate() {
            broken(format!("/scenes/{i}/nodes/{j}"), "nodes", n);
        }
    }
    for (i, n) in arr(&doc, "nodes").iter().enumerate() {
        for (j, c) in arr(n, "children").iter().enumerate() {
            broken(format!("/nodes/{i}/children/{j}"), "nodes", c);
        }
        if let Some(m) = n.get("mesh") {
            broken(format!("/nodes/{i}/mesh"), "meshes", m);
        }
    }
    for (i, m) in arr(&doc, "meshes").iter().enumerate() {
        for (j, p) in arr(m, "primitives").iter().enumerate() {
            let base = format!("/meshes/{i}/primitives/{j}");
            if let Some(v) = p.get("material") {
                broken(format!("{base}/material"), "materials", v);
            }
            if let Some(v) = p.get("indices") {
                broken(format!("{base}/indices"), "accessors", v);
            }
            if let Some(attrs) = p.get("attributes").and_then(Value::as_object) {
                for (k, v) in attrs {
                    broken(format!("{base}/attributes/{k}"), "accessors", v);
                }
            }
        }
    }
    for (i, a) in arr(&doc, "accessors").iter().enumerate() {
        if let Some(v) = a.get("bufferView") {
            broken(format!("/accessors/{i}/bufferView"), "bufferViews", v);
        }
    }
    let bin = bin_len(bytes);
    let mut overruns = Vec::new();
    for (i, b) in arr(&doc, "bufferViews").iter().enumerate() {
        let buffer = b.get("buffer");
        if let Some(v) = buffer {
            broken(format!("/bufferViews/{i}/buffer"), "buffers", v);
        }
        if buffer.and_then(Value::as_u64) == Some(0) {
            let off = b.get("byteOffset").and_then(Value::as_u64).unwrap_or(0);
            let len = b.get("byteLength").and_then(Value::as_u64).unwrap_or(0);
            if off.saturating_add(len) > bin as u64 {
                overruns.push(format!(
                    "/bufferViews/{i} llega al byte {} pero el bloque binario del GLB mide {bin}",
                    off.saturating_add(len)
                ));
            }
        }
    }
    for (i, t) in arr(&doc, "textures").iter().enumerate() {
        if let Some(v) = t.get("source") {
            broken(format!("/textures/{i}/source"), "images", v);
        }
    }
    let mut external = Vec::new();
    for (i, im) in arr(&doc, "images").iter().enumerate() {
        if let Some(v) = im.get("bufferView") {
            broken(format!("/images/{i}/bufferView"), "bufferViews", v);
        }
        if let Some(uri) = im.get("uri").and_then(Value::as_str)
            && !uri.starts_with("data:")
        {
            external.push(format!(
                "/images/{i}/uri apunta a un archivo externo que el GLB no incluye"
            ));
        }
    }
    const TEXTURE_SLOTS: [&str; 5] = [
        "pbrMetallicRoughness/baseColorTexture",
        "pbrMetallicRoughness/metallicRoughnessTexture",
        "normalTexture",
        "occlusionTexture",
        "emissiveTexture",
    ];
    for (i, m) in arr(&doc, "materials").iter().enumerate() {
        for slot in TEXTURE_SLOTS {
            let ptr = format!("/{slot}/index");
            if let Some(v) = m.pointer(&ptr) {
                broken(format!("/materials/{i}{ptr}"), "textures", v);
            }
        }
    }
    for m in overruns.into_iter().chain(external) {
        push(Severity::Error, "glb_broken_ref", m);
    }

    let st = glb::structure_of(&doc);
    let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in st.nodes.iter().enumerate() {
        if n.name.is_empty() {
            push(
                Severity::Warn,
                "glb_node_name",
                format!("/nodes/{i} no tiene nombre"),
            );
        } else {
            *by_name.entry(n.name.as_str()).or_default() += 1;
        }
    }
    for (name, count) in by_name {
        if count > 1 {
            push(
                Severity::Error,
                "glb_node_name",
                format!("el nombre de nodo '{name}' se repite en {count} nodos"),
            );
        }
    }

    let scene = doc.get("scene").and_then(Value::as_u64).unwrap_or(0) as usize;
    let scenes = arr(&doc, "scenes");
    let roots = scenes
        .get(scene)
        .map(|s| arr(s, "nodes"))
        .unwrap_or_default();
    for r in roots.iter().filter_map(Value::as_u64) {
        let Some(n) = st.nodes.get(r as usize) else {
            continue;
        };
        let label = if n.name.is_empty() {
            format!("/nodes/{r}")
        } else {
            format!("'{}' (/nodes/{r})", n.name)
        };
        if n.scale.iter().any(|c| *c < 0.0) {
            push(
                Severity::Error,
                "glb_root_scale",
                format!("el nodo raíz {label} tiene escala negativa"),
            );
        } else {
            let (lo, hi) = n
                .scale
                .iter()
                .fold((f64::MAX, f64::MIN), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
            if hi - lo > 1e-4 {
                push(
                    Severity::Warn,
                    "glb_root_scale",
                    format!("el nodo raíz {label} tiene escala no uniforme"),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn glb(doc: &Value, bin: Option<&[u8]>) -> Vec<u8> {
        let mut j = serde_json::to_vec(doc).unwrap();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut body = Vec::new();
        body.extend((j.len() as u32).to_le_bytes());
        body.extend(b"JSON");
        body.extend(&j);
        if let Some(b) = bin {
            let mut b = b.to_vec();
            while !b.len().is_multiple_of(4) {
                b.push(0);
            }
            body.extend((b.len() as u32).to_le_bytes());
            body.extend(BIN_CHUNK.to_le_bytes());
            body.extend(b);
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend(((12 + body.len()) as u32).to_le_bytes());
        out.extend(body);
        out
    }

    fn clean() -> Value {
        json!({
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"name": "root", "children": [1]}, {"name": "body", "mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],
            "materials": [{"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}}],
            "textures": [{"source": 0}],
            "images": [{"bufferView": 1}],
            "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 4}
            ],
            "buffers": [{"byteLength": 40}]
        })
    }

    type Mutation = Box<dyn Fn(&mut Value)>;

    const BIN: [u8; 40] = [0; 40];

    fn run(doc: &Value) -> Vec<Finding> {
        glb_findings("p", &glb(doc, Some(&BIN)))
    }

    fn codes(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.code.as_str()).collect()
    }

    fn mutated(f: impl FnOnce(&mut Value)) -> Vec<Finding> {
        let mut d = clean();
        f(&mut d);
        run(&d)
    }

    #[test]
    fn clean_glb_has_no_findings() {
        assert!(run(&clean()).is_empty());
    }

    #[test]
    fn not_a_glb_is_invalid() {
        let f = glb_findings("p", b"nothing at all here, not a glb");
        assert_eq!(codes(&f), ["glb_invalid"]);
        assert!(matches!(f[0].severity, Severity::Error));
        let mut bad = glb(&clean(), None);
        bad[20] = b'{';
        bad[21] = b'{';
        assert_eq!(codes(&glb_findings("p", &bad)), ["glb_invalid"]);
    }

    #[test]
    fn broken_references_are_reported_with_their_pointer() {
        let cases: [(&str, Mutation); 12] = [
            ("/scene", Box::new(|d| d["scene"] = json!(3))),
            (
                "/scenes/0/nodes/0",
                Box::new(|d| d["scenes"][0]["nodes"][0] = json!(9)),
            ),
            (
                "/nodes/0/children/0",
                Box::new(|d| d["nodes"][0]["children"][0] = json!(9)),
            ),
            (
                "/nodes/1/mesh",
                Box::new(|d| d["nodes"][1]["mesh"] = json!(2)),
            ),
            (
                "/primitives/0/material",
                Box::new(|d| d["meshes"][0]["primitives"][0]["material"] = json!(4)),
            ),
            (
                "/primitives/0/indices",
                Box::new(|d| d["meshes"][0]["primitives"][0]["indices"] = json!(4)),
            ),
            (
                "/attributes/POSITION",
                Box::new(|d| d["meshes"][0]["primitives"][0]["attributes"]["POSITION"] = json!(7)),
            ),
            (
                "/accessors/0/bufferView",
                Box::new(|d| d["accessors"][0]["bufferView"] = json!(5)),
            ),
            (
                "/bufferViews/0/buffer",
                Box::new(|d| d["bufferViews"][0]["buffer"] = json!(1)),
            ),
            (
                "/textures/0/source",
                Box::new(|d| d["textures"][0]["source"] = json!(3)),
            ),
            (
                "/images/0/bufferView",
                Box::new(|d| d["images"][0]["bufferView"] = json!(8)),
            ),
            (
                "/baseColorTexture/index",
                Box::new(|d| {
                    d["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"] =
                        json!(6)
                }),
            ),
        ];
        for (ptr, mutate) in cases {
            let f = mutated(mutate);
            assert_eq!(codes(&f), ["glb_broken_ref"], "{ptr}");
            assert!(f[0].message.contains(ptr), "{ptr}: {}", f[0].message);
            assert!(matches!(f[0].severity, Severity::Error));
        }
    }

    #[test]
    fn other_texture_slots_are_checked() {
        for slot in ["normalTexture", "occlusionTexture", "emissiveTexture"] {
            let f = mutated(|d| d["materials"][0][slot] = json!({"index": 5}));
            assert_eq!(codes(&f), ["glb_broken_ref"], "{slot}");
        }
        let f = mutated(|d| {
            d["materials"][0]["pbrMetallicRoughness"]["metallicRoughnessTexture"] =
                json!({"index": 5})
        });
        assert_eq!(codes(&f), ["glb_broken_ref"]);
    }

    #[test]
    fn buffer_view_beyond_the_binary_chunk() {
        let f = mutated(|d| d["bufferViews"][1]["byteLength"] = json!(8));
        assert_eq!(codes(&f), ["glb_broken_ref"]);
        assert!(f[0].message.contains("/bufferViews/1"));
        let none = glb_findings("p", &glb(&clean(), None));
        assert!(codes(&none).contains(&"glb_broken_ref"));
    }

    #[test]
    fn external_image_uri() {
        let f = mutated(|d| d["images"][0] = json!({"uri": "textures/wood.png"}));
        assert_eq!(codes(&f), ["glb_broken_ref"]);
        assert!(f[0].message.contains("/images/0/uri"));
        let ok = mutated(|d| d["images"][0] = json!({"uri": "data:image/png;base64,AAAA"}));
        assert!(ok.is_empty());
    }

    #[test]
    fn node_names() {
        let f = mutated(|d| d["nodes"][1]["name"] = json!(""));
        assert_eq!(codes(&f), ["glb_node_name"]);
        assert!(matches!(f[0].severity, Severity::Warn));
        let f = mutated(|d| {
            d["nodes"][1].as_object_mut().unwrap().remove("name");
        });
        assert_eq!(codes(&f), ["glb_node_name"]);
        let f = mutated(|d| d["nodes"][1]["name"] = json!("root"));
        assert_eq!(codes(&f), ["glb_node_name"]);
        assert!(matches!(f[0].severity, Severity::Error));
        assert!(f[0].message.contains("root"));
    }

    #[test]
    fn root_scale() {
        let f = mutated(|d| d["nodes"][0]["scale"] = json!([1.0, -1.0, 1.0]));
        assert_eq!(codes(&f), ["glb_root_scale"]);
        assert!(matches!(f[0].severity, Severity::Error));
        let f = mutated(|d| d["nodes"][0]["scale"] = json!([1.0, 2.0, 1.0]));
        assert_eq!(codes(&f), ["glb_root_scale"]);
        assert!(matches!(f[0].severity, Severity::Warn));
        assert!(mutated(|d| d["nodes"][0]["scale"] = json!([2.0, 2.0, 2.00001])).is_empty());
        assert!(mutated(|d| d["nodes"][1]["scale"] = json!([1.0, -1.0, 1.0])).is_empty());
    }
}
