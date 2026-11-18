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
    let meshes = arr(&doc, "meshes");
    let mut rig: Vec<(&str, String)> = Vec::new();
    let mut skinned = std::collections::BTreeSet::new();
    for (i, n) in arr(&doc, "nodes").iter().enumerate() {
        let Some(skin) = n.get("skin") else { continue };
        broken(format!("/nodes/{i}/skin"), "skins", skin);
        let mesh = n.get("mesh").and_then(Value::as_u64);
        let Some(m) = mesh.and_then(|m| meshes.get(m as usize)) else {
            continue;
        };
        // once per mesh and attribute: a crafted file must not multiply findings
        if !skinned.insert(mesh.unwrap_or_default()) {
            continue;
        }
        for (attr, code) in [
            ("JOINTS_0", "skin_no_joints_attr"),
            ("WEIGHTS_0", "skin_no_weights_attr"),
        ] {
            let lacks = arr(m, "primitives")
                .iter()
                .position(|p| p.pointer(&format!("/attributes/{attr}")).is_none());
            if let Some(j) = lacks {
                rig.push((
                    code,
                    format!(
                        "/meshes/{}/primitives/{j} se usa con una skin (/nodes/{i}) pero no tiene {attr}",
                        mesh.unwrap_or_default()
                    ),
                ));
            }
        }
    }
    for (i, s) in arr(&doc, "skins").iter().enumerate() {
        for (j, n) in arr(s, "joints").iter().enumerate() {
            broken(format!("/skins/{i}/joints/{j}"), "nodes", n);
        }
    }
    for (i, a) in arr(&doc, "animations").iter().enumerate() {
        let channels = arr(a, "channels");
        if channels.is_empty() {
            rig.push((
                "animation_empty",
                format!("/animations/{i} no tiene ningún canal"),
            ));
        }
        for (j, c) in channels.iter().enumerate() {
            if let Some(v) = c.pointer("/target/node") {
                broken(
                    format!("/animations/{i}/channels/{j}/target/node"),
                    "nodes",
                    v,
                );
            }
        }
    }
    for m in overruns.into_iter().chain(external) {
        push(Severity::Error, "glb_broken_ref", m);
    }
    for (code, m) in rig {
        push(Severity::Error, code, m);
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

/// What the build of an asset should look like, to compare its GLB against.
pub struct Expected<'a> {
    pub spec: &'a crate::Spec,
    /// Triangle budget when the spec gives none (`[rules] max_tris`).
    pub max_tris: u64,
    /// Size Blender reported for the build (`report.json`), width, height, depth.
    pub size_m: Option<[f64; 3]>,
    /// GLB material name -> palette key it wears (`materials.json`).
    pub worn: &'a BTreeMap<String, Option<String>>,
    /// Keys of the project's palette.
    pub palette: &'a [String],
}

/// What differs between a built GLB and the spec it was built from.
pub fn asset_findings(
    subject: &str,
    info: &glb::GlbInfo,
    structure: &glb::Structure,
    expected: &Expected,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut push = |severity, code: &str, message: String| {
        out.push(Finding {
            severity,
            subject: subject.into(),
            code: code.into(),
            message,
        })
    };

    let budget = expected.spec.asset.budget_tris.unwrap_or(expected.max_tris);
    if info.tris > budget {
        push(
            Severity::Error,
            "glb_tris",
            format!(
                "el GLB tiene {} triángulos y el presupuesto es {budget}",
                info.tris
            ),
        );
    }

    if let Some(want) = expected.size_m {
        let got = info.size();
        if got.iter().zip(want).any(|(g, w)| (g - w).abs() > 0.01) {
            push(
                Severity::Error,
                "glb_size",
                format!(
                    "el GLB mide {:.3} x {:.3} x {:.3} m y Blender informó {:.3} x {:.3} x {:.3} m",
                    got[0], got[1], got[2], want[0], want[1], want[2]
                ),
            );
        }
    }

    let used: std::collections::BTreeSet<&str> = structure
        .nodes
        .iter()
        .filter_map(|n| n.mesh.as_ref())
        .flat_map(|m| m.materials.iter().map(String::as_str))
        .filter(|m| !m.is_empty())
        .collect();
    for name in used {
        match expected.worn.get(name) {
            Some(Some(key)) if expected.palette.contains(key) => {}
            Some(Some(key)) => push(
                Severity::Warn,
                "glb_material",
                format!("el material '{name}' lleva '{key}', que no está en la paleta"),
            ),
            _ => push(
                Severity::Warn,
                "glb_material",
                format!("el material '{name}' no lleva ninguna clave de la paleta"),
            ),
        }
    }

    for n in &structure.nodes {
        if let Some(todo) = n.extras.get("kiln_todo") {
            let text = todo.as_str().map_or_else(|| todo.to_string(), String::from);
            push(
                Severity::Warn,
                "glb_todo",
                format!("el nodo '{}' es un sustituto pendiente: \"{text}\"", n.name),
            );
        }
    }

    for a in &expected.spec.anchors {
        let name = format!("anchor_{}", a.id);
        match structure
            .nodes
            .iter()
            .find(|n| n.parent.is_none() && n.name == name)
        {
            None => push(
                Severity::Error,
                "glb_anchor",
                format!(
                    "el ancla '{}' no tiene el nodo raíz '{name}' en el GLB",
                    a.id
                ),
            ),
            Some(n)
                if n.translation
                    .iter()
                    .zip(a.at)
                    .any(|(g, w)| (g - w).abs() > 0.001) =>
            {
                push(
                    Severity::Error,
                    "glb_anchor",
                    format!(
                        "el nodo '{name}' está en {:?} y el ancla '{}' pide {:?}",
                        n.translation, a.id, a.at
                    ),
                )
            }
            Some(_) => {}
        }
    }

    let declared = &expected.spec.asset.animations;
    if !declared.is_empty() {
        if structure.skins.is_empty() {
            push(
                Severity::Error,
                "rig_no_skeleton",
                "el spec declara animaciones pero el GLB no tiene esqueleto".into(),
            );
        }
        for name in declared {
            if !structure.animations.iter().any(|a| &a.name == name) {
                push(
                    Severity::Error,
                    "rig_animation_missing",
                    format!("la animación '{name}' está declarada en el spec y no está en el GLB"),
                );
            }
        }
    }
    out
}

/// One of the checks `verify` runs on a built asset.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Check {
    /// "kiln", "gltf_validator" or "godot_import"
    pub name: String,
    /// "passed", "failed" or "absent" (the tool is not installed: not a failure)
    pub status: String,
    pub errors: u64,
    pub warnings: u64,
    pub detail: Option<String>,
}

/// What `verify` writes to `validation.json`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    pub asset: String,
    pub glb_sha256: String,
    pub structure_sha256: String,
    /// File name -> sha256 of every GLB the verdict stands for (base and variants).
    pub outputs: std::collections::BTreeMap<String, String>,
    pub valid: bool,
    pub checks: Vec<Check>,
    pub findings: Vec<Finding>,
    /// Only a generated asset: how its recipe was verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<Value>,
}

impl Check {
    pub fn absent(name: &str, detail: &str) -> Check {
        Check {
            name: name.into(),
            status: "absent".into(),
            errors: 0,
            warnings: 0,
            detail: Some(detail.into()),
        }
    }

    /// From Kiln's own findings: failed when any is an Error.
    pub fn own(findings: &[Finding]) -> Check {
        let count = |s| findings.iter().filter(|f| f.severity == s).count() as u64;
        let errors = count(Severity::Error);
        Check {
            name: "kiln".into(),
            status: if errors > 0 { "failed" } else { "passed" }.into(),
            errors,
            warnings: count(Severity::Warn),
            detail: None,
        }
    }
}

/// Reads the JSON report the Khronos glTF Validator prints with `-o`.
pub fn khronos(subject: &str, report: &Value) -> (Check, Vec<Finding>) {
    let mut check = Check {
        name: "gltf_validator".into(),
        status: "failed".into(),
        errors: 0,
        warnings: 0,
        detail: Some("informe del validador ilegible".into()),
    };
    let Some(issues) = report.get("issues") else {
        return (check, Vec::new());
    };
    let num = |k: &str| issues.get(k).and_then(Value::as_u64).unwrap_or(0);
    check.errors = num("numErrors");
    check.warnings = num("numWarnings");
    check.status = if check.errors > 0 { "failed" } else { "passed" }.into();
    let version = report
        .get("validatorVersion")
        .and_then(Value::as_str)
        .unwrap_or("?");
    check.detail = Some(format!("glTF Validator {version}"));

    let messages = issues.get("messages").and_then(Value::as_array);
    let findings = messages
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let severity = match m.get("severity").and_then(Value::as_u64)? {
                0 => Severity::Error,
                1 => Severity::Warn,
                _ => return None,
            };
            let text = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or("");
            let mut message = format!("{}: {}", text("code"), text("message"));
            if let Some(p) = m.get("pointer").and_then(Value::as_str) {
                message.push_str(&format!(" ({p})"));
            }
            Some(Finding {
                severity,
                subject: subject.into(),
                code: "khronos".into(),
                message,
            })
        })
        .collect();
    (check, findings)
}

impl Report {
    /// `valid` is true when no check failed.
    pub fn new(
        asset: &str,
        glb_sha256: String,
        structure_sha256: String,
        checks: Vec<Check>,
        findings: Vec<Finding>,
    ) -> Report {
        Report {
            asset: asset.into(),
            glb_sha256,
            structure_sha256,
            outputs: Default::default(),
            valid: checks.iter().all(|c| c.status != "failed"),
            checks,
            findings,
            recipe: None,
        }
    }
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

    fn rigged() -> Value {
        let mut d = clean();
        d["nodes"] = json!([
            {"name": "root", "children": [1, 2, 3]},
            {"name": "body", "mesh": 0, "skin": 0},
            {"name": "hip"},
            {"name": "leg"}
        ]);
        d["meshes"][0]["primitives"][0]["attributes"] =
            json!({"POSITION": 0, "JOINTS_0": 0, "WEIGHTS_0": 0});
        d["skins"] = json!([{"joints": [2, 3]}]);
        d["animations"] = json!([{
            "name": "idle",
            "samplers": [{"input": 0, "output": 0}],
            "channels": [{"sampler": 0, "target": {"node": 2, "path": "translation"}}]
        }]);
        d
    }

    #[test]
    fn well_formed_rig_has_no_findings() {
        assert!(run(&rigged()).is_empty());
    }

    /// A hostile GLB cannot multiply findings: one per mesh and attribute,
    /// however many nodes use the mesh and however many primitives it has.
    #[test]
    fn a_skinned_mesh_is_reported_once() {
        let mut d = rigged();
        let bad = json!({"attributes": {"POSITION": 0}, "material": 0});
        d["meshes"][0]["primitives"] = json!([bad.clone(), bad.clone(), bad]);
        d["nodes"][2] = json!({"name": "hip", "mesh": 0, "skin": 0});
        d["nodes"][3] = json!({"name": "leg", "mesh": 0, "skin": 0});
        let codes: Vec<String> = run(&d).into_iter().map(|f| f.code).collect();
        assert_eq!(codes, ["skin_no_joints_attr", "skin_no_weights_attr"]);
    }

    #[test]
    fn malformed_rig_is_reported() {
        let cases: [(&str, &str, Mutation); 6] = [
            (
                "skin_no_joints_attr",
                "JOINTS_0",
                Box::new(|d| {
                    let a = &mut d["meshes"][0]["primitives"][0]["attributes"];
                    a.as_object_mut().unwrap().remove("JOINTS_0");
                }),
            ),
            (
                "skin_no_weights_attr",
                "WEIGHTS_0",
                Box::new(|d| {
                    let a = &mut d["meshes"][0]["primitives"][0]["attributes"];
                    a.as_object_mut().unwrap().remove("WEIGHTS_0");
                }),
            ),
            (
                "glb_broken_ref",
                "/skins/0/joints/1",
                Box::new(|d| d["skins"][0]["joints"][1] = json!(9)),
            ),
            (
                "glb_broken_ref",
                "/nodes/1/skin",
                Box::new(|d| d["nodes"][1]["skin"] = json!(4)),
            ),
            (
                "animation_empty",
                "/animations/0",
                Box::new(|d| d["animations"][0]["channels"] = json!([])),
            ),
            (
                "glb_broken_ref",
                "/animations/0/channels/0/target/node",
                Box::new(|d| d["animations"][0]["channels"][0]["target"]["node"] = json!(9)),
            ),
        ];
        for (code, ptr, mutate) in cases {
            let mut d = rigged();
            mutate(&mut d);
            let f = run(&d);
            assert_eq!(codes(&f), [code], "{ptr}");
            assert!(f[0].message.contains(ptr), "{ptr}: {}", f[0].message);
            assert!(matches!(f[0].severity, Severity::Error));
        }
    }

    const SPEC: &str = r#"
[asset]
id = "crate"
name = "Crate"
budget_tris = 100

[[anchors]]
id = "top"
class = "stack"
at = [0.0, 1.0, 0.0]
"#;

    struct Case {
        spec: crate::Spec,
        info: glb::GlbInfo,
        structure: glb::Structure,
        worn: BTreeMap<String, Option<String>>,
        palette: Vec<String>,
        max_tris: u64,
        size_m: Option<[f64; 3]>,
    }

    fn node(name: &str, parent: Option<&str>, at: [f64; 3]) -> glb::Node {
        glb::Node {
            name: name.into(),
            parent: parent.map(String::from),
            translation: at,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            extras: BTreeMap::new(),
            mesh: None,
        }
    }

    fn case() -> Case {
        let mut body = node("body", Some("root"), [0.0; 3]);
        body.mesh = Some(glb::MeshInfo {
            tris: 12,
            materials: vec!["body/wood".into(), "".into(), "body/wood".into()],
        });
        Case {
            spec: crate::spec::parse(SPEC).unwrap(),
            info: glb::GlbInfo {
                min: [-0.5, 0.0, -0.5],
                max: [0.5, 1.0, 0.5],
                tris: 12,
                materials: vec!["body/wood".into()],
                images: 0,
            },
            structure: glb::Structure {
                nodes: vec![
                    node("root", None, [0.0; 3]),
                    body,
                    node("anchor_top", None, [0.0, 1.0, 0.0]),
                ],
                skins: vec![],
                animations: vec![],
            },
            worn: BTreeMap::from([("body/wood".into(), Some("wood".into()))]),
            palette: vec!["wood".into()],
            max_tris: 5000,
            size_m: Some([1.0, 1.0, 1.0]),
        }
    }

    fn compare(c: &Case) -> Vec<Finding> {
        let expected = Expected {
            spec: &c.spec,
            max_tris: c.max_tris,
            size_m: c.size_m,
            worn: &c.worn,
            palette: &c.palette,
        };
        asset_findings("a", &c.info, &c.structure, &expected)
    }

    #[test]
    fn matching_asset_has_no_findings() {
        assert!(compare(&case()).is_empty());
    }

    fn rig_case(declared: &[&str], skin: bool, present: &[&str]) -> Case {
        let mut c = case();
        c.spec.asset.animations = declared.iter().map(|s| s.to_string()).collect();
        if skin {
            c.structure.skins = vec![glb::Skin {
                name: "armature".into(),
                joints: vec!["root".into()],
            }];
        }
        c.structure.animations = present
            .iter()
            .map(|n| glb::Animation {
                name: n.to_string(),
                channels: 1,
                duration: 1.0,
            })
            .collect();
        c
    }

    #[test]
    fn declared_animation_missing_from_the_glb() {
        let f = compare(&rig_case(&["idle", "walk"], true, &["idle"]));
        let missing: Vec<_> = f
            .iter()
            .filter(|f| f.code == "rig_animation_missing")
            .collect();
        assert_eq!(missing.len(), 1);
        assert!(missing[0].message.contains("walk"));
        assert_eq!(missing[0].severity, Severity::Error);
        assert!(!codes(&f).contains(&"rig_no_skeleton"));
    }

    #[test]
    fn declared_animations_without_a_skeleton() {
        let f = compare(&rig_case(&["idle"], false, &["idle"]));
        assert!(codes(&f).contains(&"rig_no_skeleton"));
    }

    #[test]
    fn declared_animations_all_present() {
        let f = compare(&rig_case(&["idle", "walk"], true, &["idle", "walk"]));
        assert!(f.is_empty());
    }

    #[test]
    fn nothing_declared_ignores_the_rig() {
        assert!(compare(&rig_case(&[], false, &[])).is_empty());
        assert!(compare(&rig_case(&[], true, &["idle"])).is_empty());
    }

    #[test]
    fn triangles_over_the_budget() {
        let mut c = case();
        c.info.tris = 101;
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_tris"]);
        assert!(matches!(f[0].severity, Severity::Error));
        assert!(f[0].message.contains("101") && f[0].message.contains("100"));
        c.info.tris = 100;
        assert!(compare(&c).is_empty());
        c.spec.asset.budget_tris = None;
        c.max_tris = 50;
        c.info.tris = 51;
        assert!(compare(&c)[0].message.contains("50"));
    }

    #[test]
    fn size_differs_from_the_reported_one() {
        let mut c = case();
        c.size_m = Some([1.0, 1.02, 1.0]);
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_size"]);
        assert!(matches!(f[0].severity, Severity::Error));
        c.size_m = Some([1.0, 1.005, 1.0]);
        assert!(compare(&c).is_empty());
        c.size_m = None;
        c.info.max[0] = 9.0;
        assert!(compare(&c).is_empty());
    }

    #[test]
    fn material_without_a_palette_key() {
        let mut c = case();
        c.worn.insert("body/wood".into(), None);
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_material"]);
        assert!(matches!(f[0].severity, Severity::Warn));
        assert!(f[0].message.contains("body/wood"));
        c.worn.clear();
        assert_eq!(codes(&compare(&c)), ["glb_material"]);
        c.worn.insert("body/wood".into(), Some("steel".into()));
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_material"]);
        assert!(f[0].message.contains("steel"));
    }

    #[test]
    fn stand_in_nodes_are_reported() {
        let mut c = case();
        c.structure.nodes[1]
            .extras
            .insert("kiln_todo".into(), json!("model the lid"));
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_todo"]);
        assert!(matches!(f[0].severity, Severity::Warn));
        assert!(f[0].message.contains("body") && f[0].message.contains("model the lid"));
    }

    #[test]
    fn anchors_missing_or_moved() {
        let mut c = case();
        c.structure.nodes[2].translation = [0.0, 1.0005, 0.0];
        assert!(compare(&c).is_empty());
        c.structure.nodes[2].translation = [0.0, 1.5, 0.0];
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_anchor"]);
        assert!(matches!(f[0].severity, Severity::Error));
        assert!(f[0].message.contains("anchor_top"));
        c.structure.nodes[2].parent = Some("root".into());
        assert!(compare(&c)[0].message.contains("no tiene el nodo raíz"));
        c.structure.nodes.pop();
        let f = compare(&c);
        assert_eq!(codes(&f), ["glb_anchor"]);
        assert!(f[0].message.contains("no tiene el nodo raíz"));
    }

    fn validator_output() -> Value {
        json!({
            "uri": "door.glb", "mimeType": "model/gltf-binary", "validatorVersion": "2.0.0-dev.3.10",
            "issues": {
                "numErrors": 0, "numWarnings": 0, "numInfos": 1, "numHints": 0,
                "messages": [{
                    "code": "UNUSED_OBJECT", "message": "This object may be unused.",
                    "severity": 2, "pointer": "/meshes/0/primitives/0/attributes/TEXCOORD_0"
                }],
                "truncated": false
            },
            "info": {"version": "2.0", "generator": "Khronos glTF Blender I/O v5.2.41"}
        })
    }

    #[test]
    fn khronos_clean_report_passes() {
        let (c, f) = khronos("door", &validator_output());
        assert_eq!(c.name, "gltf_validator");
        assert_eq!(c.status, "passed");
        assert_eq!((c.errors, c.warnings), (0, 0));
        assert_eq!(c.detail.as_deref(), Some("glTF Validator 2.0.0-dev.3.10"));
        assert!(f.is_empty());
    }

    #[test]
    fn khronos_error_fails_and_warning_does_not() {
        let mut r = validator_output();
        r["issues"]["numErrors"] = json!(1);
        r["issues"]["messages"] = json!([
            {"code": "ACCESSOR_MIN_MISMATCH", "message": "bad min", "severity": 0, "pointer": "/accessors/0"},
            {"code": "INFO", "message": "x", "severity": 3}
        ]);
        let (c, f) = khronos("door", &r);
        assert_eq!((c.status.as_str(), c.errors), ("failed", 1));
        assert_eq!(codes(&f), ["khronos"]);
        assert!(matches!(f[0].severity, Severity::Error));
        assert_eq!(f[0].subject, "door");
        assert_eq!(
            f[0].message,
            "ACCESSOR_MIN_MISMATCH: bad min (/accessors/0)"
        );

        let mut r = validator_output();
        r["issues"]["numWarnings"] = json!(1);
        r["issues"]["messages"] = json!([{"code": "W", "message": "careful", "severity": 1}]);
        let (c, f) = khronos("door", &r);
        assert_eq!((c.status.as_str(), c.warnings), ("passed", 1));
        assert_eq!(codes(&f), ["khronos"]);
        assert!(matches!(f[0].severity, Severity::Warn));
        assert_eq!(f[0].message, "W: careful");
    }

    #[test]
    fn khronos_garbage_fails() {
        let (c, f) = khronos("door", &json!({"hello": 1}));
        assert_eq!(c.status, "failed");
        assert_eq!(c.detail.as_deref(), Some("informe del validador ilegible"));
        assert!(f.is_empty());
    }

    fn finding(severity: Severity) -> Finding {
        Finding {
            severity,
            subject: "a".into(),
            code: "c".into(),
            message: "m".into(),
        }
    }

    #[test]
    fn own_check_counts_findings() {
        let c = Check::own(&[finding(Severity::Error), finding(Severity::Warn)]);
        assert_eq!((c.name.as_str(), c.status.as_str()), ("kiln", "failed"));
        assert_eq!((c.errors, c.warnings), (1, 1));
        let c = Check::own(&[finding(Severity::Warn), finding(Severity::Warn)]);
        assert_eq!((c.status.as_str(), c.errors, c.warnings), ("passed", 0, 2));
    }

    #[test]
    fn report_is_valid_unless_a_check_failed() {
        let passed = Check::own(&[]);
        let absent = Check::absent("godot_import", "Godot no está instalado");
        assert_eq!(absent.status, "absent");
        assert_eq!(absent.detail.as_deref(), Some("Godot no está instalado"));
        let ok = Report::new(
            "a",
            "h".into(),
            "s".into(),
            vec![passed.clone(), absent],
            vec![],
        );
        assert!(ok.valid);
        let failed = Check::own(&[finding(Severity::Error)]);
        let bad = Report::new("a", "h".into(), "s".into(), vec![passed, failed], vec![]);
        assert!(!bad.valid);
        let v = serde_json::to_value(&bad).unwrap();
        for k in [
            "asset",
            "glb_sha256",
            "structure_sha256",
            "valid",
            "checks",
            "findings",
        ] {
            assert!(v.get(k).is_some(), "{k}");
        }
    }

    #[test]
    fn a_report_without_recipe_has_no_recipe_key() {
        let r = Report::new("a", "h".into(), "s".into(), vec![], vec![]);
        let v = serde_json::to_value(&r).unwrap();
        assert!(v.get("recipe").is_none());
    }
}
