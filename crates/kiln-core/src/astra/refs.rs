//! Cross-reference checks between ASTRA definitions.

use super::{Definition, Parsed};
use crate::catalog::{Finding, Severity};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn check(files: &[Parsed]) -> Vec<Finding> {
    // id -> (kind, file of the first occurrence); the first occurrence wins.
    let mut index: BTreeMap<&str, (&'static str, &str)> = BTreeMap::new();
    let mut dups: BTreeMap<(usize, usize), &str> = BTreeMap::new();
    for (fi, p) in files.iter().enumerate() {
        for (di, d) in p.defs.iter().enumerate() {
            match index.get(d.id()) {
                Some((_, first)) => {
                    dups.insert((fi, di), first);
                }
                None => {
                    index.insert(d.id(), (d.kind(), &p.file));
                }
            }
        }
    }

    let mut out = Vec::new();
    for (fi, p) in files.iter().enumerate() {
        for (di, d) in p.defs.iter().enumerate() {
            let mut refs: Vec<(&str, &str)> = Vec::new();
            match d {
                Definition::Door {
                    connects: (a, b), ..
                } => {
                    refs.push((a, "sector"));
                    refs.push((b, "sector"));
                }
                Definition::Installation {
                    sector, objects, ..
                } => {
                    refs.push((sector, "sector"));
                    refs.extend(objects.iter().map(|o| (o.as_str(), "object")));
                }
                _ => {}
            }
            let mut push = |code: &str, message: String| {
                out.push(Finding {
                    severity: Severity::Error,
                    subject: format!("{}#{}", p.file, d.id()),
                    code: code.into(),
                    message,
                })
            };
            if let Some(first) = dups.get(&(fi, di)) {
                push(
                    "astra_duplicate_id",
                    format!("el id ya está definido en {first}"),
                );
            }
            for (id, expected) in refs {
                match index.get(id) {
                    None => push(
                        "astra_unknown_ref",
                        format!("referencia a «{id}», que no existe"),
                    ),
                    Some((kind, _)) if *kind != expected => push(
                        "astra_wrong_kind",
                        format!("«{id}» debería ser {expected} y es {kind}"),
                    ),
                    _ => {}
                }
            }
            if let Definition::Door {
                connects: (a, b), ..
            } = d
                && a == b
            {
                push(
                    "astra_door_endpoint",
                    format!("la puerta conecta «{a}» consigo misma"),
                );
            }
        }
    }
    out
}

pub fn check_external(
    root: &Path,
    files: &[Parsed],
    piece_keys: &BTreeSet<String>,
    godot_project: Option<&str>,
    greybox: bool,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for p in files {
        for d in &p.defs {
            let subject = format!("{}#{}", p.file, d.id());
            let mut push = |severity, code: &str, message: String| {
                out.push(Finding {
                    severity,
                    subject: subject.clone(),
                    code: code.into(),
                    message,
                })
            };
            match d {
                Definition::Object { asset, .. } if !piece_keys.contains(asset) => {
                    let (severity, tail) = if greybox {
                        (Severity::Warn, " (se tolera como greybox)")
                    } else {
                        (Severity::Error, "")
                    };
                    push(
                        severity,
                        "astra_asset_missing",
                        format!("el asset «{asset}» no está en el catálogo{tail}"),
                    );
                }
                Definition::Sector { scene, .. } => {
                    let code = "astra_scene_missing";
                    let inside = |rest: &str| {
                        Path::new(rest)
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_)))
                    };
                    let Some(rest) = scene.strip_prefix("res://").filter(|r| inside(r)) else {
                        push(
                            Severity::Error,
                            code,
                            format!(
                                "la escena «{scene}» debe ser una ruta res:// relativa y sin «..»"
                            ),
                        );
                        continue;
                    };
                    match godot_project {
                        None => push(
                            Severity::Error,
                            code,
                            "no se puede comprobar la escena: falta [tools] godot_project".into(),
                        ),
                        Some(gp) if !root.join(gp).join(rest).exists() => push(
                            Severity::Error,
                            code,
                            format!("la escena «{scene}» no existe en el proyecto Godot"),
                        ),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sector(id: &str) -> Definition {
        Definition::Sector {
            id: id.into(),
            scene: "s".into(),
        }
    }
    fn object(id: &str) -> Definition {
        Definition::Object {
            id: id.into(),
            asset: "a".into(),
        }
    }
    fn door(id: &str, a: &str, b: &str) -> Definition {
        Definition::Door {
            id: id.into(),
            connects: (a.into(), b.into()),
        }
    }
    fn inst(id: &str, sector: &str, objects: &[&str]) -> Definition {
        Definition::Installation {
            id: id.into(),
            sector: sector.into(),
            objects: objects.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn file(name: &str, defs: Vec<Definition>) -> Parsed {
        Parsed {
            file: name.into(),
            defs,
        }
    }

    fn one(files: &[Parsed], code: &str, subject: &str) -> Finding {
        let found = check(files);
        assert_eq!(found.len(), 1, "{found:?}");
        let f = found.into_iter().next().unwrap();
        assert_eq!(f.code, code);
        assert_eq!(f.subject, subject);
        assert!(f.severity == Severity::Error);
        f
    }

    #[test]
    fn valid_set_has_no_findings() {
        let f = file(
            "a.ron",
            vec![
                sector("s1"),
                sector("s2"),
                door("d1", "s1", "s2"),
                object("o1"),
                inst("i1", "s1", &["o1"]),
            ],
        );
        assert!(check(&[f]).is_empty());
    }

    #[test]
    fn duplicate_id_across_files_names_first_file() {
        let a = file("a.ron", vec![sector("s1")]);
        let b = file("b.ron", vec![sector("s1")]);
        let f = one(&[a, b], "astra_duplicate_id", "b.ron#s1");
        assert!(f.message.contains("a.ron"), "{}", f.message);
    }

    #[test]
    fn unknown_ref_names_missing_id() {
        let f = file("a.ron", vec![sector("s1"), door("d1", "s1", "ghost")]);
        let f = one(&[f], "astra_unknown_ref", "a.ron#d1");
        assert!(f.message.contains("ghost"), "{}", f.message);
    }

    #[test]
    fn wrong_kind_reports_expected_and_actual() {
        let f = file("a.ron", vec![object("o1"), inst("i1", "o1", &[])]);
        let f = one(&[f], "astra_wrong_kind", "a.ron#i1");
        assert!(
            f.message.contains("sector") && f.message.contains("object"),
            "{}",
            f.message
        );
    }

    #[test]
    fn door_with_same_endpoints() {
        let f = file("a.ron", vec![sector("s1"), door("d1", "s1", "s1")]);
        one(&[f], "astra_door_endpoint", "a.ron#d1");
    }

    fn keys(ks: &[&str]) -> BTreeSet<String> {
        ks.iter().map(|s| s.to_string()).collect()
    }

    fn asset_file() -> Vec<Parsed> {
        vec![file("a.ron", vec![object("o1")])]
    }

    #[test]
    fn asset_present_has_no_findings() {
        let out = check_external(Path::new("."), &asset_file(), &keys(&["a"]), None, false);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn asset_missing_is_error() {
        let out = check_external(Path::new("."), &asset_file(), &keys(&["b"]), None, false);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_asset_missing");
        assert_eq!(out[0].subject, "a.ron#o1");
        assert!(out[0].severity == Severity::Error);
        assert!(out[0].message.contains('a'));
    }

    #[test]
    fn asset_missing_in_greybox_is_warn() {
        let out = check_external(Path::new("."), &asset_file(), &keys(&[]), None, true);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_asset_missing");
        assert!(out[0].severity == Severity::Warn);
        assert!(out[0].message.contains("greybox"), "{}", out[0].message);
    }

    fn scene_case(scene: &str, godot: Option<&str>) -> Vec<Finding> {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("kiln-astra-refs-{}-{n}", std::process::id()));
        std::fs::create_dir_all(root.join("godot/astra")).unwrap();
        std::fs::write(root.join("godot/astra/warehouse.tscn"), "").unwrap();
        let f = vec![file(
            "a.ron",
            vec![Definition::Sector {
                id: "s1".into(),
                scene: scene.into(),
            }],
        )];
        let out = check_external(&root, &f, &keys(&[]), godot, false);
        std::fs::remove_dir_all(&root).unwrap();
        out
    }

    #[test]
    fn scene_existing_has_no_findings() {
        let out = scene_case("res://astra/warehouse.tscn", Some("godot"));
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn scene_missing_is_error() {
        let out = scene_case("res://astra/yard.tscn", Some("godot"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_scene_missing");
        assert_eq!(out[0].subject, "a.ron#s1");
        assert!(out[0].severity == Severity::Error);
    }

    #[test]
    fn scene_outside_godot_project_is_error() {
        let out = scene_case(
            "res://astra/../../godot/astra/warehouse.tscn",
            Some("godot"),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_scene_missing");
    }

    #[test]
    fn scene_absolute_after_res_is_error() {
        let abs = std::env::current_dir().unwrap().join("Cargo.toml");
        let out = scene_case(&format!("res://{}", abs.display()), Some("godot"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_scene_missing");
    }

    #[test]
    fn scene_without_godot_project_is_error() {
        let out = scene_case("res://astra/warehouse.tscn", None);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].code, "astra_scene_missing");
        assert!(
            out[0].message.contains("godot_project"),
            "{}",
            out[0].message
        );
    }

    #[test]
    fn scene_not_res_path_is_error() {
        let out = scene_case("astra/warehouse.tscn", Some("godot"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].message.contains("res://"), "{}", out[0].message);
    }
}
