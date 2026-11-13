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
            ("glb" | "gltf", Some(size)) => {
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
            (f, _) => push(
                Severity::Info,
                &p.key,
                "not_measured",
                format!("formato {f}: lo medirá Godot en la próxima entrega"),
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
    cat.findings.extend(out);
    cat.findings
        .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.subject.cmp(&b.subject)));
}
