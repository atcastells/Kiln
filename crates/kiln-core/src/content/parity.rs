//! The K001 scenarios, run through the generic engine with the K001 schema:
//! each of its eleven codes must have its `content_*` equivalent.

use crate::catalog::Severity;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const KINDS: &str = r#"
[content.world.kinds.Sector]
id = "id"
scene = "scene"
[content.world.kinds.Door]
id = "id"
refs = { connects = "Sector" }
distinct = ["connects"]
node_meta = { connects = "astra_connects" }
[content.world.kinds.Object]
id = "id"
asset = "asset"
[content.world.kinds.Installation]
id = "id"
refs = { sector = "Sector", objects = "Object" }
"#;

const TOOLS: &str = "[tools]\ngodot_project = \"godot\"\n";

const GOOD: &str = r#"[
    Sector(id: "sector:warehouse", scene: "res://astra/warehouse.tscn"),
    Sector(id: "sector:yard", scene: "res://astra/yard.tscn"),
    Door(id: "door:warehouse_yard", connects: ("sector:warehouse", "sector:yard")),
    Object(id: "object:crate", asset: "harbor/crate"),
    Installation(id: "installation:loading_bay", sector: "sector:warehouse", objects: ["object:crate"]),
]"#;

const DOOR: &str = r#"{"id":"door:warehouse_yard","meta":{"astra_connects":["sector:warehouse","sector:yard"]}}"#;

struct Found {
    code: String,
    severity: Severity,
    subject: String,
    message: String,
}

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name)
}

/// Opens a temp project with the K001 schema and returns its `content_*` findings.
fn open(tools: &str, world: &str, files: &[(&str, &str)]) -> Vec<Found> {
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("kiln-parity-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    let toml = format!(
        "[project]\nname = \"t\"\n[assets]\nkits = [\"kits\"]\n[rules]\nlicenses = [\"CC0\"]\n{tools}\
         [content.world]\ndefinitions = [\"data/**/*.ron\"]\nid_meta = \"astra_id\"\n{world}\n{KINDS}"
    );
    let kit = example("content/kits/harbor");
    let mut all = vec![
        ("kiln.toml", toml),
        ("godot/astra/warehouse.tscn", String::new()),
        ("godot/astra/yard.tscn", String::new()),
    ];
    all.extend(files.iter().map(|(p, t)| (*p, t.to_string())));
    for (path, text) in &all {
        let f = d.join(path);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, text).unwrap();
    }
    for f in ["crate.glb", "LICENSE.md"] {
        std::fs::create_dir_all(d.join("kits/harbor")).unwrap();
        std::fs::copy(kit.join(f), d.join("kits/harbor").join(f)).unwrap();
    }
    let cat = crate::open(&d).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    cat.findings
        .into_iter()
        .filter(|f| f.code.starts_with("content_"))
        .map(|f| Found { code: f.code, severity: f.severity, subject: f.subject, message: f.message })
        .collect()
}

fn manifest(warehouse: &str, yard: &str) -> String {
    format!(
        r#"{{"version":2,"set":"world","scenes":[
          {{"scene":"res://astra/warehouse.tscn","nodes":[{warehouse}]}},
          {{"scene":"res://astra/yard.tscn","nodes":[{yard}]}}]}}"#
    )
}

fn good_manifest() -> String {
    manifest(&format!(r#"{{"id":"sector:warehouse"}},{DOOR}"#), r#"{"id":"sector:yard"}"#)
}

const WITH_MANIFEST: &str = "manifest = \"m.json\"";

/// Exactly one finding, with the given code, severity and subject.
fn one(found: &[Found], code: &str, severity: Severity, subject: &str) -> String {
    assert_eq!(found.len(), 1, "{:?}", found.iter().map(|f| (&f.code, &f.subject)).collect::<Vec<_>>());
    let f = &found[0];
    assert_eq!((f.code.as_str(), f.severity, f.subject.as_str()), (code, severity, subject));
    f.message.clone()
}

fn with_manifest(m: &str) -> Vec<Found> {
    open(TOOLS, WITH_MANIFEST, &[("data/a.ron", GOOD), ("m.json", m)])
}

#[test]
fn fixture_has_no_errors() {
    let cat = crate::open(&example("content")).unwrap();
    let errors: Vec<_> = cat.findings.iter().filter(|f| f.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(cat.pieces.iter().any(|p| p.key == "harbor/door"));
    let world = &cat.content[0];
    assert_eq!((world.name.as_str(), world.elements.len(), world.errors), ("world", 5, 0));
}

#[test]
fn project_without_content_has_no_content_output() {
    let cat = crate::open(&example("")).unwrap();
    assert!(cat.content.is_empty());
    assert!(cat.findings.iter().all(|f| !f.code.starts_with("content_")));
}

#[test]
fn valid_set_has_no_findings() {
    assert!(open(TOOLS, WITH_MANIFEST, &[("data/a.ron", GOOD), ("m.json", &good_manifest())]).is_empty());
}

#[test]
fn ron_syntax_is_content_syntax() {
    let f = open(TOOLS, "", &[("data/c.ron", "[Sector(id: \"s\"")]);
    let m = one(&f, "content_syntax", Severity::Error, "world:data/c.ron");
    assert!(m.contains("línea"), "{m}");
}

#[test]
fn unknown_variant_is_an_unknown_kind() {
    // K001 reported a syntax error; the generic engine names the kind.
    let f = open(TOOLS, "", &[("data/a.ron", "[Window(id: \"x\")]")]);
    one(&f, "content_unknown_kind", Severity::Error, "world:data/a.ron");
}

#[test]
fn no_definitions_warns() {
    one(&open(TOOLS, "", &[]), "content_no_definitions", Severity::Warn, "world");
}

#[test]
fn broken_and_duplicate_files_are_reported() {
    let object = "[Object(id: \"object:x\", asset: \"harbor/nope\")]";
    let f = open(TOOLS, "", &[("data/a.ron", object), ("data/b.ron", object), ("data/c.ron", "[Sector(id: \"s\"")]);
    let has = |code: &str, subject: &str| f.iter().any(|f| f.code == code && f.subject == subject);
    assert!(has("content_syntax", "world:data/c.ron"));
    assert!(has("content_duplicate_id", "world:data/b.ron#object:x"));
    assert!(has("content_asset_missing", "world:data/a.ron#object:x"));
    let dup = f.iter().find(|f| f.code == "content_duplicate_id").unwrap();
    assert!(dup.message.contains("data/a.ron"), "{}", dup.message);
}

#[test]
fn broken_ron_does_not_cascade_into_manifest_mismatches() {
    let m = manifest(r#"{"id":"sector:a"}"#, "");
    let f = open(TOOLS, WITH_MANIFEST, &[("data/a.ron", "[Sector(id: \"sector:a\""), ("m.json", &m)]);
    one(&f, "content_syntax", Severity::Error, "world:data/a.ron");
}

#[test]
fn unknown_ref_names_missing_id() {
    let defs = GOOD.replace("sector:yard\")),", "ghost\")),");
    let f = open(TOOLS, "", &[("data/a.ron", &defs)]);
    let m = one(&f, "content_unknown_ref", Severity::Error, "world:data/a.ron#door:warehouse_yard");
    assert!(m.contains("ghost"), "{m}");
}

#[test]
fn wrong_kind_reports_expected_and_actual() {
    let defs = GOOD.replace("sector: \"sector:warehouse\"", "sector: \"object:crate\"");
    let f = open(TOOLS, "", &[("data/a.ron", &defs)]);
    let m = one(&f, "content_wrong_kind", Severity::Error, "world:data/a.ron#installation:loading_bay");
    assert!(m.contains("Sector") && m.contains("Object"), "{m}");
}

#[test]
fn door_with_same_endpoints_is_not_distinct() {
    let defs = GOOD.replace("sector:yard\")),", "sector:warehouse\")),");
    let f = open(TOOLS, "", &[("data/a.ron", &defs)]);
    one(&f, "content_not_distinct", Severity::Error, "world:data/a.ron#door:warehouse_yard");
}

#[test]
fn asset_missing_is_error_or_greybox_warn() {
    let defs = GOOD.replace("harbor/crate", "harbor/ghost");
    let f = open(TOOLS, "", &[("data/a.ron", &defs)]);
    one(&f, "content_asset_missing", Severity::Error, "world:data/a.ron#object:crate");
    let f = open(TOOLS, "greybox = true", &[("data/a.ron", &defs)]);
    let m = one(&f, "content_asset_missing", Severity::Warn, "world:data/a.ron#object:crate");
    assert!(m.contains("greybox"), "{m}");
}

#[test]
fn scene_problems_are_scene_missing() {
    let subject = "world:data/a.ron#sector:yard";
    for (scene, tools) in [
        ("res://astra/ghost.tscn", TOOLS),
        ("res://astra/../../godot/astra/yard.tscn", TOOLS),
        ("astra/yard.tscn", TOOLS),
        ("res://astra/yard.tscn", ""),
    ] {
        let defs = GOOD.replace("res://astra/yard.tscn", scene);
        let f = open(tools, "", &[("data/a.ron", &defs)]);
        let f: Vec<_> = f.into_iter().filter(|f| f.subject == subject).collect();
        one(&f, "content_scene_missing", Severity::Error, subject);
    }
}

#[test]
fn manifest_missing_warns() {
    let f = open(TOOLS, WITH_MANIFEST, &[("data/a.ron", GOOD)]);
    let m = one(&f, "content_manifest_missing", Severity::Warn, "world:m.json");
    assert!(m.contains("content-manifest"), "{m}");
}

#[test]
fn manifest_syntax_is_an_error() {
    let m = one(&with_manifest("{\n  \"version\": }"), "content_manifest_syntax", Severity::Error, "world:m.json");
    assert!(m.contains("línea 2"), "{m}");
}

#[test]
fn manifest_node_missing() {
    let m = manifest(DOOR, r#"{"id":"sector:yard"}"#);
    one(&with_manifest(&m), "content_manifest_mismatch", Severity::Error, "world:data/a.ron#sector:warehouse");
}

#[test]
fn manifest_node_in_another_scene() {
    let m = manifest(DOOR, r#"{"id":"sector:yard"},{"id":"sector:warehouse"}"#);
    let msg = one(&with_manifest(&m), "content_manifest_mismatch", Severity::Error, "world:data/a.ron#sector:warehouse");
    assert!(msg.contains("astra/yard.tscn") && msg.contains("astra/warehouse.tscn"), "{msg}");
}

#[test]
fn manifest_door_connects_differ_but_order_does_not_matter() {
    let door = |ends: &str| format!(r#"{{"id":"door:warehouse_yard","meta":{{"astra_connects":{ends}}}}}"#);
    let m = manifest(&format!(r#"{{"id":"sector:warehouse"}},{}"#, door(r#"["sector:warehouse","sector:z"]"#)), r#"{"id":"sector:yard"}"#);
    let msg = one(&with_manifest(&m), "content_manifest_mismatch", Severity::Error, "world:data/a.ron#door:warehouse_yard");
    assert!(msg.contains("sector:z") && msg.contains("sector:yard"), "{msg}");
    let reversed = manifest(&format!(r#"{{"id":"sector:warehouse"}},{}"#, door(r#"["sector:yard","sector:warehouse"]"#)), r#"{"id":"sector:yard"}"#);
    assert!(with_manifest(&reversed).is_empty());
}

#[test]
fn manifest_door_without_meta_or_node() {
    let bare = manifest(r#"{"id":"sector:warehouse"},{"id":"door:warehouse_yard"}"#, r#"{"id":"sector:yard"}"#);
    one(&with_manifest(&bare), "content_manifest_mismatch", Severity::Error, "world:data/a.ron#door:warehouse_yard");
    let gone = manifest(r#"{"id":"sector:warehouse"}"#, r#"{"id":"sector:yard"}"#);
    let msg = one(&with_manifest(&gone), "content_manifest_mismatch", Severity::Error, "world:data/a.ron#door:warehouse_yard");
    assert!(msg.contains("no aparece"), "{msg}");
}

#[test]
fn manifest_extra_node() {
    let m = manifest(&format!(r#"{{"id":"sector:warehouse"}},{DOOR}"#), r#"{"id":"sector:yard"},{"id":"sector:ghost"}"#);
    one(&with_manifest(&m), "content_manifest_mismatch", Severity::Error, "world:m.json#sector:ghost");
}
