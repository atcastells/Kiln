//! Kiln core: reads a game project described by its `kiln.toml` and builds
//! the catalog of what the project already has (pieces, materials) with the
//! findings of the checks. No engine is needed to read it.

pub mod bindings;
pub mod build;
pub mod catalog;
pub mod check;
pub mod config;
pub mod context;
pub mod glb;
pub mod godot;
pub mod license;
pub mod review;
pub mod sources;
pub mod spec;
pub mod tree;
pub mod zones;

pub use catalog::{Catalog, Finding, Kit, Material, Piece, Severity};
pub use config::Config;
pub use spec::Spec;

use std::path::Path;

/// Reads `kiln.toml` at the project root and indexes the project.
pub fn open(project: &Path) -> Result<Catalog, String> {
    open_with(project, Config::load(project)?)
}

/// Indexes the project with a config read elsewhere (`--config`).
pub fn open_with(project: &Path, config: Config) -> Result<Catalog, String> {
    let mut catalog = catalog::index(project, &config);
    godot::apply(project, &config, &mut catalog);
    let splits = zones::load(project)?;
    for p in &mut catalog.pieces {
        p.split = zones::mode(&splits, &p.key).map(str::to_string);
    }
    bindings::apply(&bindings::load(project, &config)?, &mut catalog);
    check::run(&config, &mut catalog);
    if let Some(dir) = &config.assets.specs {
        catalog.specs = spec::load_all(project, dir, &config.rules, &mut catalog);
        catalog
            .findings
            .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.subject.cmp(&b.subject)));
    }
    catalog.builds = build::reports(project, &config, &catalog.specs);
    catalog.tree = Some(tree::build(project, &config, &catalog));
    Ok(catalog)
}
