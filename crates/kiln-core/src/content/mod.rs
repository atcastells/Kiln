//! Generic content validation, driven by the schema each game declares in `kiln.toml`.

pub mod fields;
pub mod index;
pub mod manifest;
pub mod markers;
pub mod parse;
#[cfg(test)]
mod parity;
pub mod refs;
pub mod schema;
pub mod value;

use crate::catalog::{Catalog, ContentSetSummary, ElementRef, Finding, KindCount, Severity};
use crate::config::Config;
use std::collections::BTreeSet;
use std::path::Path;

/// Validates every content set the project declares; without `[content]`
/// nothing runs and the catalog is left as it was.
pub fn run(root: &Path, config: &Config, catalog: &mut Catalog) {
    let (sets, problems) = config.content_sets();
    for p in problems {
        catalog.findings.push(Finding {
            severity: Severity::Error,
            subject: p.set,
            code: "content_schema".into(),
            message: p.message,
        });
    }
    let keys: BTreeSet<String> = catalog.pieces.iter().map(|p| p.key.clone()).collect();
    let godot = config.tools.godot_project.as_deref().map(|g| root.join(g));
    for (name, set) in &sets {
        let mut out = Vec::new();
        let elements = index::load(root, name, set, &mut out);
        // With definitions missing, every manifest node would show up as a
        // mismatch; the read error already explains the cause.
        let complete = out.is_empty();
        refs::check(name, set, &elements, &mut out);
        refs::check_external(name, set, &elements, &keys, godot.as_deref(), &mut out);
        fields::check(name, set, &elements, &mut out);
        let loaded = manifest::load(root, name, set, &mut out);
        if let Some(m) = loaded.filter(|_| complete) {
            manifest::compare(name, set, &elements, &m, &mut out);
            markers::check(name, set, &elements, &m, &mut out);
        }
        let count = |s: Severity| out.iter().filter(|f| f.severity == s).count();
        catalog.content.push(ContentSetSummary {
            name: name.clone(),
            kinds: set
                .kinds
                .keys()
                .map(|k| KindCount {
                    name: k.clone(),
                    count: elements.iter().filter(|e| &e.kind == k).count(),
                })
                .collect(),
            elements: elements
                .iter()
                .map(|e| ElementRef {
                    kind: e.kind.clone(),
                    id: e.id.clone(),
                    file: e.file.clone(),
                    line: e.line,
                })
                .collect(),
            errors: count(Severity::Error),
            warnings: count(Severity::Warn),
        });
        catalog.findings.extend(out);
    }
}
