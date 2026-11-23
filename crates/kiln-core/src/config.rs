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
    /// Content sets (`[content.<set>]`), kept raw so a broken set does not
    /// stop the config from loading; see `content_sets`.
    #[serde(default)]
    pub content: Option<toml::Value>,
    /// Classes of anchor (`[anchors.<class>]`) and which ones each fits.
    #[serde(default)]
    pub anchors: std::collections::BTreeMap<String, AnchorClass>,
    /// Where a verified asset is published for the game to use.
    #[serde(default)]
    pub publish: PublishSection,
    /// The UI of the game: theme, scripts and scenes to check. None when left out.
    #[serde(default)]
    pub ui: Option<UiSection>,
    /// How a recipe (`[generator]`) is verified.
    #[serde(default)]
    pub generate: GenerateSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateSection {
    /// Seeds a verification of a recipe generates (1..=64).
    #[serde(default = "default_verify_seeds")]
    pub verify_seeds: u32,
    /// Fraction of steep faces above which a seed gets a warning (0..=1).
    #[serde(default = "default_steep_max")]
    pub steep_max: f64,
    /// Canonical geometry verify computes per seed; None leaves verify as before.
    #[serde(default, deserialize_with = "canonical_class")]
    pub canonical: Option<CanonicalClass>,
}

impl Default for GenerateSection {
    fn default() -> Self {
        GenerateSection {
            verify_seeds: default_verify_seeds(),
            steep_max: default_steep_max(),
            canonical: None,
        }
    }
}

/// A version of the canonical geometry (`[generate] canonical`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalClass {
    RadialV0,
}

impl CanonicalClass {
    pub const ALL: [CanonicalClass; 1] = [CanonicalClass::RadialV0];

    pub fn name(self) -> &'static str {
        match self {
            CanonicalClass::RadialV0 => "radial_v0",
        }
    }
}

fn canonical_class<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<CanonicalClass>, D::Error> {
    let known = || {
        CanonicalClass::ALL
            .iter()
            .map(|c| format!("«{}»", c.name()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let value = toml::Value::deserialize(d)?;
    let name = value.as_str().unwrap_or_default();
    CanonicalClass::ALL
        .into_iter()
        .find(|c| c.name() == name)
        .map(Some)
        .ok_or_else(|| {
            serde::de::Error::custom(format!(
                "[generate] canonical: valor desconocido {value}; se admite {}",
                known()
            ))
        })
}

fn default_verify_seeds() -> u32 {
    16
}
fn default_steep_max() -> f64 {
    0.10
}

/// How serious a finding of a rule is; `error` makes `kiln check` fail.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Warn,
    Error,
    Off,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiSection {
    /// The Godot theme (`.tres`), relative to the project.
    pub theme: String,
    /// Globs of the UI scripts.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Globs of the scenes with UI.
    #[serde(default)]
    pub scenes: Vec<String>,
    /// The file that says which check marks each criterion of the UI target.
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub contrast: ContrastRule,
    #[serde(default)]
    pub glyphs: RuleLevel,
    #[serde(default)]
    pub theme_coherent: RuleLevel,
    #[serde(default)]
    pub theme_only: RuleLevel,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RuleLevel {
    #[serde(default)]
    pub level: Level,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContrastRule {
    /// Smallest WCAG ratio of normal text.
    #[serde(default = "default_contrast_normal")]
    pub normal: f64,
    /// Smallest ratio of large text and of what is not text.
    #[serde(default = "default_contrast_large")]
    pub large: f64,
    /// Text from this size, in px, counts as large.
    #[serde(default = "default_large_from_px")]
    pub large_from_px: u32,
    #[serde(default)]
    pub level: Level,
    /// Foreground and background to measure where the outline does not give one.
    #[serde(default)]
    pub pairs: Vec<ContrastPair>,
}

impl Default for ContrastRule {
    fn default() -> Self {
        ContrastRule {
            normal: default_contrast_normal(),
            large: default_contrast_large(),
            large_from_px: default_large_from_px(),
            level: Level::default(),
            pairs: Vec::new(),
        }
    }
}

fn default_contrast_normal() -> f64 {
    4.5
}
fn default_contrast_large() -> f64 {
    3.0
}
fn default_large_from_px() -> u32 {
    18
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContrastPair {
    /// A theme variation (text) or `<variation>/<style box>` (non text).
    pub fg: String,
    /// `<variation>/<style box>`.
    pub bg: String,
    #[serde(default)]
    pub kind: PairKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairKind {
    #[default]
    Text,
    NonText,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnchorClass {
    /// The classes an anchor of this class may be mounted on.
    #[serde(default)]
    pub fits: Vec<String>,
    /// Width and height, in metres, of the opening an asset with this anchor must fit in.
    #[serde(default)]
    pub opening_m: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PublishSection {
    /// Folder inside the project; each asset gets `<dir>/<id>/`.
    #[serde(default)]
    pub dir: Option<String>,
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
    /// Longest a run of Blender or Godot may take, in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_s: u64,
    /// The Khronos glTF Validator; optional, used by `verify` when found.
    #[serde(default = "default_gltf_validator")]
    pub gltf_validator: String,
    /// glTF-Transform; `kiln tools` reports its version, nothing else uses it.
    #[serde(default = "default_gltf_transform")]
    pub gltf_transform: String,
}

impl Default for Tools {
    fn default() -> Self {
        Tools {
            blender: default_blender(),
            godot: default_godot(),
            godot_project: None,
            build: default_build(),
            cache: default_cache(),
            timeout_s: default_timeout(),
            gltf_validator: default_gltf_validator(),
            gltf_transform: default_gltf_transform(),
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
fn default_timeout() -> u64 {
    600
}
fn default_gltf_validator() -> String {
    "gltf_validator".into()
}
fn default_gltf_transform() -> String {
    "gltf-transform".into()
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
    /// What kind of project it is; only `game` exists for now.
    #[serde(default = "default_kind")]
    pub kind: String,
}

pub const KINDS: [&str; 1] = ["game"];

fn default_kind() -> String {
    "game".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssetsSection {
    /// Folders whose subfolders are kits of pieces (e.g. `godot/kit`).
    #[serde(default)]
    pub kits: Vec<String>,
    /// Folder of the asset specs Kiln writes (used from the second delivery).
    #[serde(default)]
    pub specs: Option<String>,
    /// Box-UV scale of modelled parts, in UV units per metre (1.0 when unset).
    #[serde(default)]
    pub uv_per_m: Option<f64>,
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
    /// Template of the Godot material of a palette key (`res://materials/{key}.tres`).
    #[serde(default)]
    pub godot: Option<String>,
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
            godot: None,
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

    /// Box-UV scale of modelled parts: `[assets] uv_per_m`, else 1.0.
    pub fn uv_per_m(&self) -> f64 {
        self.assets.uv_per_m.unwrap_or(1.0)
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
        let mut c: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        // Blender and Godot installed as apps: the programs saved by `kiln setup`.
        c.tools.blender = crate::tools::resolve("blender", &c.tools.blender);
        c.tools.godot = crate::tools::resolve("godot", &c.tools.godot);
        if !(1..=64).contains(&c.generate.verify_seeds) {
            return Err("[generate] verify_seeds debe estar entre 1 y 64".into());
        }
        if !(0.0..=1.0).contains(&c.generate.steep_max) {
            return Err("[generate] steep_max debe estar entre 0 y 1".into());
        }
        if let Some(v) = c.assets.uv_per_m
            && (!v.is_finite() || v <= 0.0)
        {
            return Err("[assets] uv_per_m debe ser un número finito mayor que 0".into());
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_without_the_new_sections_gets_the_defaults() {
        let c = Config::parse("[project]\nname = \"x\"\n").unwrap();
        assert_eq!(c.tools.timeout_s, 600);
        assert_eq!(c.tools.gltf_validator, "gltf_validator");
        assert_eq!(c.tools.gltf_transform, "gltf-transform");
        assert!(c.anchors.is_empty());
        assert!(c.publish.dir.is_none());
    }

    #[test]
    fn reads_anchor_classes_publish_and_timeout() {
        let c = Config::parse(
            "[project]\nname = \"x\"\n[tools]\ntimeout_s = 5\n\
             [anchors.doorway]\nfits = [\"wall_gap\"]\nopening_m = [1.4, 2.2]\n\
             [anchors.wall_gap]\n[publish]\ndir = \"godot/assets\"\n",
        )
        .unwrap();
        assert_eq!(c.tools.timeout_s, 5);
        assert_eq!(c.anchors["doorway"].fits, ["wall_gap"]);
        assert_eq!(c.anchors["doorway"].opening_m, Some([1.4, 2.2]));
        assert!(c.anchors["wall_gap"].fits.is_empty());
        assert_eq!(c.publish.dir.as_deref(), Some("godot/assets"));
    }

    #[test]
    fn uv_per_m_defaults_to_one_and_reads_back() {
        let c = Config::parse("[project]\nname = \"x\"\n[assets]\nkits = []\n").unwrap();
        assert_eq!(c.uv_per_m(), 1.0);
        let c = Config::parse("[project]\nname = \"x\"\n[assets]\nuv_per_m = 0.5\n").unwrap();
        assert_eq!(c.uv_per_m(), 0.5);
    }

    #[test]
    fn uv_per_m_rejects_non_positive_and_non_finite() {
        for v in ["0", "-1.5", "nan", "inf"] {
            let e = Config::parse(&format!("[project]\nname = \"x\"\n[assets]\nuv_per_m = {v}\n"))
                .unwrap_err();
            assert!(e.contains("[assets] uv_per_m"), "{v}: {e}");
        }
    }

    #[test]
    fn generate_defaults_when_the_section_is_absent() {
        let c = Config::parse("[project]\nname = \"x\"\n").unwrap();
        assert_eq!(c.generate.verify_seeds, 16);
        assert_eq!(c.generate.steep_max, 0.10);
    }

    #[test]
    fn reads_the_generate_section() {
        let c = Config::parse(
            "[project]\nname = \"x\"\n[generate]\nverify_seeds = 4\nsteep_max = 0.25\n",
        )
        .unwrap();
        assert_eq!(c.generate.verify_seeds, 4);
        assert_eq!(c.generate.steep_max, 0.25);
    }

    #[test]
    fn rejects_generate_values_out_of_range() {
        let parse = |body: &str| Config::parse(&format!("[project]\nname = \"x\"\n[generate]\n{body}\n"));
        for body in ["verify_seeds = 0", "verify_seeds = 65"] {
            let e = parse(body).unwrap_err();
            assert!(e.contains("[generate] verify_seeds debe estar entre 1 y 64"), "{e}");
        }
        for body in ["steep_max = 1.5", "steep_max = -0.1"] {
            let e = parse(body).unwrap_err();
            assert!(e.contains("[generate] steep_max debe estar entre 0 y 1"), "{e}");
        }
        assert!(parse("verify_seeds = 64\nsteep_max = 1.0").is_ok());
        assert!(parse("verify_seeds = 1\nsteep_max = 0.0").is_ok());
    }

    #[test]
    fn canonical_geometry_is_off_without_the_key() {
        let c = Config::parse("[project]\nname = \"x\"\n[generate]\nverify_seeds = 4\n").unwrap();
        assert_eq!(c.generate.canonical, None);
        assert_eq!(Config::parse("[project]\nname = \"x\"\n").unwrap().generate.canonical, None);
    }

    #[test]
    fn reads_the_canonical_class() {
        let c = Config::parse("[project]\nname = \"x\"\n[generate]\ncanonical = \"radial_v0\"\n").unwrap();
        assert_eq!(c.generate.canonical, Some(CanonicalClass::RadialV0));
    }

    #[test]
    fn rejects_an_unknown_canonical_class() {
        for value in ["\"radial_v1\"", "\"\"", "1"] {
            let e = Config::parse(&format!("[project]\nname = \"x\"\n[generate]\ncanonical = {value}\n"))
                .unwrap_err();
            assert!(e.contains("[generate] canonical"), "{e}");
            assert!(e.contains("radial_v0"), "{e}");
        }
    }

    #[test]
    fn a_config_without_ui_loads_with_none() {
        let c = Config::parse("[project]\nname = \"x\"\n").unwrap();
        assert!(c.ui.is_none());
    }

    #[test]
    fn reads_the_ui_section_and_defaults_the_levels_to_warn() {
        let c = Config::parse(
            "[project]\nname = \"x\"\n\
             [ui]\ntheme = \"ui/theme.tres\"\nscripts = [\"ui/hud.gd\", \"ui/*_panel.gd\"]\n\
             scenes = [\"ui/main.tscn\"]\ntarget = \"kiln/ui_target.toml\"\n\
             [ui.contrast]\nnormal = 5.0\n\
             [[ui.contrast.pairs]]\nfg = \"Body\"\nbg = \"Panel/panel\"\n\
             [[ui.contrast.pairs]]\nfg = \"Bar/fill\"\nbg = \"Bar/back\"\nkind = \"non_text\"\n\
             [ui.glyphs]\nlevel = \"error\"\n[ui.theme_only]\nlevel = \"off\"\n",
        )
        .unwrap();
        let ui = c.ui.unwrap();
        assert_eq!(ui.theme, "ui/theme.tres");
        assert_eq!(ui.scripts, ["ui/hud.gd", "ui/*_panel.gd"]);
        assert_eq!(ui.scenes, ["ui/main.tscn"]);
        assert_eq!(ui.target.as_deref(), Some("kiln/ui_target.toml"));
        assert_eq!(ui.contrast.normal, 5.0);
        assert_eq!(ui.contrast.large, 3.0);
        assert_eq!(ui.contrast.large_from_px, 18);
        assert_eq!(ui.contrast.level, Level::Warn);
        assert_eq!(ui.contrast.pairs.len(), 2);
        assert_eq!(ui.contrast.pairs[0].kind, PairKind::Text);
        assert_eq!(ui.contrast.pairs[1].kind, PairKind::NonText);
        assert_eq!(ui.contrast.pairs[1].bg, "Bar/back");
        assert_eq!(ui.glyphs.level, Level::Error);
        assert_eq!(ui.theme_coherent.level, Level::Warn);
        assert_eq!(ui.theme_only.level, Level::Off);
    }
}
