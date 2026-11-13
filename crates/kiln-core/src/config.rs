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
    /// The programs Kiln calls to produce (Blender, Godot) and where it writes.
    #[serde(default)]
    pub tools: Tools,
    /// What the agent must know of the game (`kiln/context.toml`).
    #[serde(default)]
    pub context: ContextSection,
    /// Asset sites to search for `search` parts, and where pieces brought in go.
    #[serde(default)]
    pub sources: SourcesSection,
    /// Which model each agent runs on, and how far the review loop goes.
    #[serde(default)]
    pub agents: AgentsSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentsSection {
    /// The reviewer looks at images: the strongest model (`opus` is the
    /// latest Opus for Claude Code).
    #[serde(default = "default_review_model")]
    pub review_model: String,
    /// Rounds of review, fix and build at most; the best one is kept.
    #[serde(default = "default_review_rounds")]
    pub review_rounds: u32,
}

impl Default for AgentsSection {
    fn default() -> Self {
        AgentsSection {
            review_model: default_review_model(),
            review_rounds: default_review_rounds(),
        }
    }
}

fn default_review_model() -> String {
    "opus".into()
}
fn default_review_rounds() -> u32 {
    3
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourcesSection {
    /// polyhaven | polypizza | sketchfab
    #[serde(default = "default_sites")]
    pub sites: Vec<String>,
    /// The folder of kits where each site gets its own (`<kits>/<site>/`);
    /// the first of `[assets] kits` when left out.
    #[serde(default)]
    pub kits: Option<String>,
    /// Texture size of Poly Haven models: 1k, 2k, 4k.
    #[serde(default = "default_resolution")]
    pub resolution: String,
    /// The credits file of the game.
    #[serde(default = "default_credits")]
    pub credits: String,
}

impl Default for SourcesSection {
    fn default() -> Self {
        SourcesSection {
            sites: default_sites(),
            kits: None,
            resolution: default_resolution(),
            credits: default_credits(),
        }
    }
}

fn default_sites() -> Vec<String> {
    crate::sources::SITES.map(String::from).to_vec()
}
fn default_resolution() -> String {
    "1k".into()
}
fn default_credits() -> String {
    "kiln/CREDITS.md".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContextSection {
    #[serde(default = "default_context_file")]
    pub file: String,
    /// Layers of the context; the six of `context::default_layers` when empty.
    #[serde(default)]
    pub layers: Vec<crate::context::Layer>,
}

impl Default for ContextSection {
    fn default() -> Self {
        ContextSection {
            file: default_context_file(),
            layers: Vec::new(),
        }
    }
}

impl ContextSection {
    pub fn layers(&self) -> Vec<crate::context::Layer> {
        if self.layers.is_empty() {
            crate::context::default_layers()
        } else {
            self.layers.clone()
        }
    }
}

fn default_context_file() -> String {
    "kiln/context.toml".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tools {
    /// Blender, or a Python with the `bpy` module (`.../python3`).
    #[serde(default = "default_blender")]
    pub blender: String,
    #[serde(default = "default_godot")]
    pub godot: String,
    /// The folder with `project.godot`, for pieces only Godot reads (`.res`).
    #[serde(default)]
    pub godot_project: Option<String>,
    /// Where an asset is assembled (`<build>/<id>/<id>.glb`).
    #[serde(default = "default_build")]
    pub build: String,
    /// What Kiln works out and keeps (Godot measures, exported GLBs).
    #[serde(default = "default_cache")]
    pub cache: String,
}

impl Default for Tools {
    fn default() -> Self {
        Tools {
            blender: default_blender(),
            godot: default_godot(),
            godot_project: None,
            build: default_build(),
            cache: default_cache(),
        }
    }
}

fn default_blender() -> String {
    "blender".into()
}
fn default_godot() -> String {
    "godot".into()
}
fn default_build() -> String {
    "kiln/build".into()
}
fn default_cache() -> String {
    "kiln/cache".into()
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

#[derive(Debug, Clone, Deserialize)]
pub struct MaterialsSection {
    /// TOML files with arrays of tables that carry an `id` (one material each).
    #[serde(default)]
    pub tables: Vec<String>,
    /// Folders of textures named `<key><albedo_suffix>.<ext>`; each key is a material.
    #[serde(default)]
    pub textures: Vec<String>,
    /// The end of the colour map's file name (`_albedo`, `_diff`, `_basecolor`...).
    #[serde(default = "default_albedo")]
    pub albedo_suffix: String,
    /// The ends of the other maps Kiln looks for beside it.
    #[serde(default = "default_maps")]
    pub map_suffixes: Vec<String>,
    /// Keys of a table row that hold its colour as RGB 0-1, first found wins.
    #[serde(default = "default_color_keys")]
    pub color_keys: Vec<String>,
    /// The file where each slot of a piece is bound to a palette material.
    #[serde(default = "default_bindings")]
    pub bindings: String,
}

impl Default for MaterialsSection {
    fn default() -> Self {
        MaterialsSection {
            tables: Vec::new(),
            textures: Vec::new(),
            albedo_suffix: default_albedo(),
            map_suffixes: default_maps(),
            color_keys: default_color_keys(),
            bindings: default_bindings(),
        }
    }
}

fn default_albedo() -> String {
    "_albedo".into()
}
fn default_maps() -> Vec<String> {
    vec!["_normal".into(), "_roughness".into()]
}
fn default_color_keys() -> Vec<String> {
    ["color", "colour", "albedo", "base_color"]
        .map(String::from)
        .to_vec()
}
fn default_bindings() -> String {
    "kiln/bindings.toml".into()
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
    /// Every asset must name a system of the game that uses it (`used_by`).
    #[serde(default)]
    pub require_use: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            licenses: default_licenses(),
            max_tris: default_tris(),
            max_size_m: default_max_size(),
            min_size_m: default_min_size(),
            require_use: false,
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
    /// Where pieces brought in from sites go: `[sources] kits`, else the
    /// first of `[assets] kits`, else `kiln/kits`.
    pub fn sources_root(&self) -> String {
        self.sources
            .kits
            .clone()
            .or(self.assets.kits.first().cloned())
            .unwrap_or("kiln/kits".into())
    }

    /// Every folder of kits the catalog reads, the sources one included.
    pub fn kit_roots(&self) -> Vec<String> {
        let mut out = self.assets.kits.clone();
        let s = self.sources_root();
        if !out.contains(&s) {
            out.push(s);
        }
        out
    }

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
