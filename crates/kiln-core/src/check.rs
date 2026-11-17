//! The checks of the catalog: what would break a game or its licence
//! without anyone seeing it. Visual judgement stays with the person.

use crate::catalog::{Catalog, Finding, Severity};
use crate::config::Config;

pub fn run(config: &Config, cat: &mut Catalog) {
    let rules = &config.rules;
    let mut out = Vec::new();
    let mut push = |severity, subject: &str, code: &str, message: String| {
        out.push(Finding {
            severity,
            subject: subject.into(),
            code: code.into(),
            message,
        })
    };

    if !crate::config::KINDS.contains(&config.project.kind.as_str()) {
        push(
            Severity::Warn,
            &config.project.name,
            "kind_unknown",
            format!(
                "tipo de proyecto «{}»; Kiln solo conoce: {}",
                config.project.kind,
                crate::config::KINDS.join(", ")
            ),
        );
    }
    for kit in &cat.kits {
        if !kit.has_license_file {
            push(
                Severity::Error,
                &kit.path,
                "kit_without_license",
                "el kit no tiene LICENSE.md".into(),
            );
        }
    }
    for p in &cat.pieces {
        if p.licenses.is_empty() {
            push(
                Severity::Error,
                &p.key,
                "no_license",
                "ni la pieza ni su kit nombran una licencia".into(),
            );
        }
        for l in &p.licenses {
            if !rules.licenses.iter().any(|ok| ok == l) {
                push(
                    Severity::Error,
                    &p.key,
                    "license_not_allowed",
                    format!(
                        "licencia {l}, que el proyecto no admite ({})",
                        rules.licenses.join(", ")
                    ),
                );
            }
        }
        let kit_credit = cat.kits.iter().any(|k| k.name == p.kit && k.has_credit);
        if p.licenses.iter().any(|l| l == "CC-BY") && p.credit.is_none() && !kit_credit {
            push(Severity::Warn, &p.key, "credit_missing", "CC-BY pide citar al autor y ni la pieza ni el LICENSE.md de su kit traen el crédito".into());
        }
        if p.file.is_none() {
            push(
                Severity::Warn,
                &p.key,
                "model_missing",
                "un manifiesto la nombra pero no hay fichero de modelo".into(),
            );
            continue;
        }
        match (p.format.as_str(), p.size_m) {
            (_, Some(size)) => {
                let longest = size.iter().cloned().fold(0.0, f64::max);
                if longest > rules.max_size_m || longest < rules.min_size_m {
                    push(
                        Severity::Warn,
                        &p.key,
                        "scale",
                        format!("lado mayor de {longest:.3} m: ¿escala equivocada?"),
                    );
                }
                if let Some(min) = p.min
                    && min[1].abs() > 0.05
                {
                    push(
                        Severity::Info,
                        &p.key,
                        "not_on_ground",
                        format!("la base está a {:.3} m del suelo (y = 0)", min[1]),
                    );
                }
                if let Some(t) = p.tris
                    && t > rules.max_tris
                {
                    push(
                        Severity::Warn,
                        &p.key,
                        "tris",
                        format!("{t} triángulos, por encima de {}", rules.max_tris),
                    );
                }
            }
            ("glb" | "gltf", None) => {}
            (f, None) => push(
                Severity::Info,
                &p.key,
                "not_measured",
                format!("formato {f}: sin medir; mídela con Godot (`kiln godot`)"),
            ),
        }
    }
    let mut seen = std::collections::BTreeMap::new();
    for m in &cat.materials {
        if let Some(prev) = seen.insert(m.key.clone(), m.file.clone()) {
            push(
                Severity::Info,
                &m.key,
                "material_twice",
                format!("el material está en {prev} y en {}", m.file),
            );
        }
    }
    if config.ui.is_some() {
        out.extend(crate::ui::rules::findings(std::path::Path::new(&cat.root), config));
    }
    cat.findings.extend(out);
    cat.findings
        .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.subject.cmp(&b.subject)));
}

/// Anchor classes: every one a spec uses or a `fits` names must be declared,
/// and a built asset must fit the opening its anchor class asks for.
pub fn anchors(config: &Config, cat: &mut Catalog) {
    if config.anchors.is_empty() && cat.specs.iter().all(|s| s.anchors.is_empty()) {
        return;
    }
    let mut out = Vec::new();
    let mut finding = |subject: String, code: &str, message: String| {
        out.push(Finding {
            severity: Severity::Error,
            subject,
            code: code.into(),
            message,
        })
    };
    for (class, def) in &config.anchors {
        for f in def.fits.iter().filter(|f| !config.anchors.contains_key(*f)) {
            finding(
                format!("anchors.{class}"),
                "anchor_class",
                format!("la clase «{class}» encaja en «{f}», que no está declarada en [anchors]"),
            );
        }
    }
    for s in &cat.specs {
        let id = &s.asset.id;
        for a in s
            .anchors
            .iter()
            .filter(|a| !config.anchors.contains_key(&a.class))
        {
            finding(
                format!("anchor:{id}/{}", a.id),
                "anchor_class",
                format!("la clase «{}» no está declarada en [anchors]", a.class),
            );
        }
        let Some([w, h, _]) = cat.builds.get(id).and_then(|r| r.size_m) else {
            continue;
        };
        let mut classes: Vec<&str> = s.anchors.iter().map(|a| a.class.as_str()).collect();
        classes.sort();
        classes.dedup();
        for class in classes {
            let Some([ow, oh]) = config.anchors.get(class).and_then(|c| c.opening_m) else {
                continue;
            };
            if w > ow + 0.001 || h > oh + 0.001 {
                finding(
                    format!("asset:{id}"),
                    "anchor_opening",
                    format!(
                        "mide {w:.3} x {h:.3} m y el hueco de «{class}» es de {ow:.3} x {oh:.3} m"
                    ),
                );
            }
        }
    }
    cat.findings.extend(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Catalog {
        Catalog {
            project: "x".into(),
            root: ".".into(),
            kits: vec![],
            pieces: vec![],
            materials: vec![],
            specs: vec![],
            tree: None,
            builds: Default::default(),
            content: vec![],
            findings: vec![],
        }
    }

    #[test]
    fn a_project_is_a_game_unless_it_says_otherwise() {
        let game = Config::parse("[project]\nname = \"x\"\n").unwrap();
        assert_eq!(game.project.kind, "game");
        let mut cat = empty();
        run(&game, &mut cat);
        assert!(cat.findings.is_empty());
        let film = Config::parse("[project]\nname = \"x\"\nkind = \"film\"\n").unwrap();
        let mut cat = empty();
        run(&film, &mut cat);
        assert_eq!(cat.findings[0].code, "kind_unknown");
    }

    #[test]
    fn ui_rules_run_only_with_a_ui_section() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ui/project");
        let base = "[project]\nname = \"x\"\n";
        let ui = format!("{base}[tools]\ngodot_project = \"..\"\n[ui]\ntheme = \"dirty.tres\"\nscripts = [\"dirty.gd\"]\n");
        for (toml, want_ui) in [(base.to_string(), false), (ui, true)] {
            let mut cat = empty();
            cat.root = dir.into();
            run(&Config::parse(&toml).unwrap(), &mut cat);
            assert_eq!(cat.findings.iter().any(|f| f.code.starts_with("ui-")), want_ui);
        }
    }

    const SPEC: &str = "[asset]\nid = \"door\"\nname = \"D\"\n[[parts]]\nid = \"p\"\nwhat = \"p\"\nmaterial = \"oak\"\n[parts.source]\nkind = \"search\"\nquery = \"q\"\n[[anchors]]\nid = \"in\"\nclass = \"doorway\"\n";
    const CLASSES: &str = "[project]\nname = \"x\"\n[anchors.doorway]\nopening_m = [1.4, 2.2]\n";

    fn anchor_codes(toml: &str, spec: Option<&str>, size: Option<[f64; 3]>) -> Vec<String> {
        let config = Config::parse(toml).unwrap();
        let mut cat = empty();
        if let Some(s) = spec {
            cat.specs.push(crate::spec::parse(s).unwrap());
        }
        if size.is_some() {
            cat.builds.insert(
                "door".into(),
                crate::build::Report {
                    size_m: size,
                    ..Default::default()
                },
            );
        }
        anchors(&config, &mut cat);
        cat.findings.into_iter().map(|f| f.code).collect()
    }

    #[test]
    fn an_anchor_of_an_undeclared_class_is_an_error() {
        let c = anchor_codes("[project]\nname = \"x\"\n", Some(SPEC), None);
        assert_eq!(c, ["anchor_class"]);
    }

    #[test]
    fn a_fits_entry_that_is_not_a_class_is_an_error() {
        let toml = "[project]\nname = \"x\"\n[anchors.doorway]\nfits = [\"nowhere\"]\n";
        assert_eq!(anchor_codes(toml, None, None), ["anchor_class"]);
    }

    #[test]
    fn a_built_asset_wider_than_the_opening_is_an_error() {
        let c = anchor_codes(CLASSES, Some(SPEC), Some([1.6, 2.0, 0.2]));
        assert_eq!(c, ["anchor_opening"]);
    }

    #[test]
    fn an_asset_exactly_the_opening_size_passes() {
        assert!(anchor_codes(CLASSES, Some(SPEC), Some([1.4, 2.2, 0.2])).is_empty());
    }

    #[test]
    fn an_asset_not_built_is_not_measured() {
        assert!(anchor_codes(CLASSES, Some(SPEC), None).is_empty());
    }

    #[test]
    fn no_anchors_anywhere_adds_nothing() {
        let c = anchor_codes("[project]\nname = \"x\"\n", None, Some([9.0, 9.0, 9.0]));
        assert!(c.is_empty());
    }
}
