//! Kiln core: reads a game project described by its `kiln.toml` and builds
//! the catalog of what the project already has (pieces, materials) with the
//! findings of the checks. No engine is needed to read it.

pub mod catalog;
pub mod check;
pub mod config;
pub mod glb;
pub mod license;

pub use catalog::{Catalog, Finding, Kit, Material, Piece, Severity};
pub use config::Config;

use std::path::Path;

/// Reads `kiln.toml` at the project root and indexes the project.
pub fn open(project: &Path) -> Result<Catalog, String> {
    open_with(project, Config::load(project)?)
}

/// Indexes the project with a config read elsewhere (`--config`).
pub fn open_with(project: &Path, config: Config) -> Result<Catalog, String> {
    let mut catalog = catalog::index(project, &config);
    check::run(&config, &mut catalog);
    Ok(catalog)
}
