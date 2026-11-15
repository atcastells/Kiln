//! ASTRA definitions: a provisional, non-canonical RON schema. Kiln validates
//! the content of the project; it is not ASTRA's engine.

pub mod manifest;
pub mod refs;

use crate::catalog::{Catalog, Finding, Severity};
use crate::config::{AstraSection, Config};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum Definition {
    Sector {
        id: String,
        scene: String,
    },
    Door {
        id: String,
        connects: (String, String),
    },
    Object {
        id: String,
        asset: String,
    },
    Installation {
        id: String,
        sector: String,
        objects: Vec<String>,
    },
}

impl Definition {
    pub fn id(&self) -> &str {
        match self {
            Definition::Sector { id, .. }
            | Definition::Door { id, .. }
            | Definition::Object { id, .. }
            | Definition::Installation { id, .. } => id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Definition::Sector { .. } => "sector",
            Definition::Door { .. } => "door",
            Definition::Object { .. } => "object",
            Definition::Installation { .. } => "installation",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parsed {
    /// Path relative to the project root.
    pub file: String,
    pub defs: Vec<Definition>,
}

pub fn parse_file(file: &str, text: &str) -> Result<Parsed, Finding> {
    match ron::from_str::<Vec<Definition>>(text) {
        Ok(defs) => Ok(Parsed {
            file: file.into(),
            defs,
        }),
        Err(e) => Err(Finding {
            severity: Severity::Error,
            subject: file.into(),
            code: "astra_ron_syntax".into(),
            message: format!(
                "línea {}, columna {}: {}",
                e.span.start.line, e.span.start.col, e.code
            ),
        }),
    }
}

/// Validates the ASTRA content of the project when `[integrations.astra]` is
/// enabled; without it, nothing runs.
pub fn run(root: &Path, config: &Config, cat: &mut Catalog) {
    let Some(astra) = config.astra() else {
        return;
    };
    let (files, mut out) = load_files(root, astra);
    // Con definiciones que faltan, cada nodo del manifiesto saldría como
    // discrepancia; el error de lectura ya explica la causa.
    let complete = out.is_empty();
    out.extend(refs::check(&files));
    let keys = cat.pieces.iter().map(|p| p.key.clone()).collect();
    let gp = config.tools.godot_project.as_deref();
    out.extend(refs::check_external(root, &files, &keys, gp, astra.greybox));
    if let Some(rel) = &astra.godot_manifest {
        match manifest::load(root, rel) {
            Ok(m) if complete => out.extend(manifest::compare(&files, &m, rel)),
            Ok(_) => {}
            Err(f) => out.push(f),
        }
    }
    cat.findings.extend(out);
}

/// Reads every RON file the section's globs match: the parsed ones, and the
/// findings of the ones that could not be read or parsed.
pub fn load_files(root: &Path, astra: &AstraSection) -> (Vec<Parsed>, Vec<Finding>) {
    let mut paths: Vec<_> = astra
        .definitions
        .iter()
        .filter_map(|g| glob::glob(&root.join(g).to_string_lossy()).ok())
        .flatten()
        .flatten()
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    paths.dedup();
    let mut out = Vec::new();
    if paths.is_empty() {
        out.push(Finding {
            severity: Severity::Warn,
            subject: "integrations.astra".into(),
            code: "astra_no_definitions".into(),
            message: format!("ningún fichero RON coincide con {:?}", astra.definitions),
        });
    }
    let mut files = Vec::new();
    for path in &paths {
        let file = crate::catalog::rel(root, path);
        let parsed = std::fs::read_to_string(path)
            .map_err(|e| Finding {
                severity: Severity::Error,
                subject: file.clone(),
                code: "astra_ron_syntax".into(),
                message: format!("no se puede leer: {e}"),
            })
            .and_then(|text| parse_file(&file, &text));
        match parsed {
            Ok(p) => files.push(p),
            Err(f) => out.push(f),
        }
    }
    (files, out)
}

/// The `res://` scenes of the sectors, sorted and without repeats: what
/// `kiln astra-manifest` asks Godot to read.
pub fn scenes(files: &[Parsed]) -> Vec<String> {
    let mut out: Vec<String> = files
        .iter()
        .flat_map(|p| &p.defs)
        .filter_map(|d| match d {
            Definition::Sector { scene, .. } => Some(scene.clone()),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-astra-run-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (path, text) in files {
            let f = d.join(path);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, text).unwrap();
        }
        d
    }

    fn codes(cat: &Catalog) -> Vec<(String, String)> {
        cat.findings
            .iter()
            .filter(|f| f.code.starts_with("astra_"))
            .map(|f| (f.code.clone(), f.subject.clone()))
            .collect()
    }

    const TOML: &str = "[project]\nname = \"t\"\n[integrations.astra]\nenabled = true\ndefinitions = [\"data/**/*.ron\"]\n[tools]\ngodot_project = \"godot\"\n";

    #[test]
    fn example_astra_fixture_has_no_errors() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/astra");
        let cat = crate::open(&root).unwrap();
        let errors: Vec<_> = cat
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(cat.pieces.iter().any(|p| p.key == "harbor/door"));
    }

    #[test]
    fn example_without_section_has_no_astra_findings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        assert!(codes(&crate::open(&root).unwrap()).is_empty());
    }

    #[test]
    fn no_definitions_warns() {
        let d = project("empty", &[("kiln.toml", TOML)]);
        let cat = crate::open(&d).unwrap();
        assert_eq!(
            codes(&cat),
            vec![("astra_no_definitions".into(), "integrations.astra".into())]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn broken_ron_does_not_cascade_into_manifest_mismatches() {
        let manifest = r#"{"version": 1, "scenes": [{"scene": "res://a.tscn", "nodes": [{"id": "sector:a"}]}]}"#;
        let toml = "[project]\nname = \"t\"\n[integrations.astra]\nenabled = true\ndefinitions = [\"data/**/*.ron\"]\ngodot_manifest = \"m.json\"\n";
        let d = project(
            "cascade",
            &[
                ("kiln.toml", toml),
                ("m.json", manifest),
                ("data/a.ron", "[Sector(id: \"sector:a\""),
            ],
        );
        let cat = crate::open(&d).unwrap();
        let got: Vec<_> = codes(&cat).into_iter().map(|(c, _)| c).collect();
        assert_eq!(got, vec!["astra_ron_syntax".to_string()]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn broken_and_duplicate_files_are_reported() {
        let d = project(
            "broken",
            &[
                ("kiln.toml", TOML),
                ("data/a.ron", "[Object(id: \"object:x\", asset: \"k/x\")]"),
                ("data/b.ron", "[Object(id: \"object:x\", asset: \"k/x\")]"),
                ("data/c.ron", "[Sector(id: \"s\""),
            ],
        );
        let cat = crate::open(&d).unwrap();
        let got = codes(&cat);
        assert!(
            got.contains(&("astra_ron_syntax".into(), "data/c.ron".into())),
            "{got:?}"
        );
        assert!(
            got.contains(&("astra_duplicate_id".into(), "data/b.ron#object:x".into())),
            "{got:?}"
        );
        assert!(
            got.contains(&("astra_asset_missing".into(), "data/a.ron#object:x".into())),
            "{got:?}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    const GOOD: &str = r#"[
    Sector(id: "sector:warehouse", scene: "res://astra/warehouse.tscn"),
    Sector(id: "sector:yard", scene: "res://astra/yard.tscn"),
    Door(id: "door:warehouse_yard", connects: ("sector:warehouse", "sector:yard")),
    Object(id: "object:crate", asset: "props/crate"),
    Installation(id: "installation:loading_bay", sector: "sector:warehouse", objects: ["object:crate"]),
]"#;

    #[test]
    fn scenes_are_sorted_and_unique() {
        let p = parse_file("a.ron", GOOD).unwrap();
        assert_eq!(
            scenes(&[p.clone(), p]),
            vec!["res://astra/warehouse.tscn", "res://astra/yard.tscn"]
        );
    }

    #[test]
    fn parses_five_definitions() {
        let p = parse_file("astra/a.ron", GOOD).unwrap();
        assert_eq!(p.file, "astra/a.ron");
        let got: Vec<_> = p.defs.iter().map(|d| (d.kind(), d.id())).collect();
        assert_eq!(
            got,
            vec![
                ("sector", "sector:warehouse"),
                ("sector", "sector:yard"),
                ("door", "door:warehouse_yard"),
                ("object", "object:crate"),
                ("installation", "installation:loading_bay"),
            ]
        );
    }

    #[test]
    fn syntax_error_reports_line() {
        let text = "[\n    Sector(id: \"a\", scene: \"b\"),\n    Object(id: \"c\", asset: \"d\"]";
        let f = parse_file("astra/a.ron", text).unwrap_err();
        assert_eq!(f.code, "astra_ron_syntax");
        assert_eq!(f.subject, "astra/a.ron");
        assert!(f.severity == Severity::Error);
        assert!(f.message.contains("línea 3"), "{}", f.message);
    }

    #[test]
    fn unknown_variant_is_syntax_error() {
        let f = parse_file("astra/a.ron", "[Window(id: \"x\")]").unwrap_err();
        assert_eq!(f.code, "astra_ron_syntax");
    }
}
