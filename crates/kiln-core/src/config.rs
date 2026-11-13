//! `kiln.toml`: what a project tells Kiln about itself.

use serde::Deserialize;
use std::path::Path;

pub const FILE: &str = "kiln.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub project: ProjectSection,
    #[serde(default)]
    pub assets: AssetsSection,
    #[serde(default)]
    pub materials: MaterialsSection,
    #[serde(default)]
    pub rules: Rules,
    /// The levels of the project tree between the game and its assets.
    #[serde(default)]
    pub levels: Vec<LevelSource>,
}

/// Where the nodes of one level are declared: each file matched by `files`
/// (globs) is a node, or each table of its array `table`.
#[derive(Debug, Clone, Deserialize)]
pub struct LevelSource {
    pub level: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub table: Option<String>,
    /// The key that holds the node's id.
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub children: Vec<ChildrenRef>,
}

/// The children of a node: under `key`, a string, or an array of strings or of tables (their `field`).
#[derive(Debug, Clone, Deserialize)]
pub struct ChildrenRef {
    pub key: String,
    #[serde(default = "default_field")]
    pub field: String,
    pub level: String,
}

fn default_field() -> String {
    "id".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssetsSection {
    /// Folders whose subfolders are kits of pieces (e.g. `godot/kit`).
    #[serde(default)]
    pub kits: Vec<String>,
    /// Folder of the asset specs Kiln writes (used from the second delivery).
    #[serde(default)]
    pub specs: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MaterialsSection {
    /// TOML files with arrays of tables that carry an `id` (one material each).
    #[serde(default)]
    pub tables: Vec<String>,
    /// Folders of textures named `<key>_albedo.<ext>`; each key is a material.
    #[serde(default)]
    pub textures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    /// Licence labels a piece may carry (as `license::normalize` writes them).
    #[serde(default = "default_licenses")]
    pub licenses: Vec<String>,
    /// Above this many triangles a piece is flagged.
    #[serde(default = "default_tris")]
    pub max_tris: u64,
    /// Longest side above this, in metres, is flagged as a scale error.
    #[serde(default = "default_max_size")]
    pub max_size_m: f64,
    /// Longest side below this, in metres, is flagged as a scale error.
    #[serde(default = "default_min_size")]
    pub min_size_m: f64,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            licenses: default_licenses(),
            max_tris: default_tris(),
            max_size_m: default_max_size(),
            min_size_m: default_min_size(),
        }
    }
}

fn default_licenses() -> Vec<String> {
    vec!["CC0".into(), "CC-BY".into()]
}
fn default_tris() -> u64 {
    20_000
}
fn default_max_size() -> f64 {
    30.0
}
fn default_min_size() -> f64 {
    0.02
}

impl Config {
    pub fn load(project: &Path) -> Result<Config, String> {
        let path = project.join(FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("no se puede leer {}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn load_file(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("no se puede leer {}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}
