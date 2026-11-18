//! Reads a Godot 4 text theme (`.tres`) as text, without Godot.
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    /// `#RRGGBB`; the alpha is not part of it.
    pub fn to_hex(&self) -> String {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02X}{:02X}{:02X}", c(self.r), c(self.g), c(self.b))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleBox {
    pub bg: Option<Rgba>,
    pub border: Option<Rgba>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Variation {
    pub base: Option<String>,
    pub colors: BTreeMap<String, Rgba>,
    pub font_sizes: BTreeMap<String, u32>,
    pub constants: BTreeMap<String, i64>,
    /// Item name to `ext_resource` id.
    pub fonts: BTreeMap<String, String>,
    /// Item name to `sub_resource` id.
    pub styles: BTreeMap<String, String>,
}

impl Variation {
    /// Only an entry with `base_type` is a theme type variation; the others are styles of a
    /// built-in class (`Button/styles/normal`).
    pub fn is_variation(&self) -> bool {
        self.base.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Theme {
    /// `ext_resource` id to `res://` path.
    pub fonts: BTreeMap<String, String>,
    pub styles: BTreeMap<String, StyleBox>,
    pub variations: BTreeMap<String, Variation>,
}

pub fn load(path: &Path) -> Result<Theme, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let binary = path.extension().is_some_and(|e| e == "res");
    let text = String::from_utf8(bytes).ok().filter(|_| !binary).ok_or_else(|| {
        format!("{}: binary theme (.res) is not supported; save it as .tres", path.display())
    })?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn parse(text: &str) -> Result<Theme, String> {
    if !text.trim_start().starts_with("[gd_resource") {
        return Err("not a Godot text resource (missing [gd_resource header)".into());
    }
    let mut theme = Theme::default();
    let mut section = String::new();
    let mut style_id: Option<String> = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with('[') {
            style_id = None;
            section = line
                .trim_start_matches('[')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(']')
                .to_string();
            match section.as_str() {
                "ext_resource" => {
                    if let (Some(id), Some(path)) = (attr(line, "id"), attr(line, "path")) {
                        theme.fonts.insert(id, path);
                    }
                }
                "sub_resource" if attr(line, "type").as_deref() == Some("StyleBoxFlat") => {
                    if let Some(id) = attr(line, "id") {
                        theme.styles.insert(id.clone(), StyleBox::default());
                        style_id = Some(id);
                    }
                }
                _ => {}
            }
        } else if let Some((key, value)) = line.split_once(" = ") {
            if let Some(style) = style_id.as_ref().and_then(|id| theme.styles.get_mut(id)) {
                match key {
                    "bg_color" => style.bg = color(value),
                    "border_color" => style.border = color(value),
                    _ => {}
                }
            } else if section == "resource" {
                add_item(&mut theme, key, value.trim());
            }
        }
    }
    Ok(theme)
}

fn add_item(theme: &mut Theme, key: &str, value: &str) {
    let parts: Vec<&str> = key.split('/').collect();
    let (Some(name), true) = (parts.first(), parts.len() > 1) else { return };
    let var = theme.variations.entry(name.to_string()).or_default();
    match parts.as_slice() {
        [_, "base_type"] => {
            var.base = Some(value.trim_start_matches('&').trim_matches('"').to_string())
        }
        [_, "colors", k] => {
            if let Some(c) = color(value) {
                var.colors.insert(k.to_string(), c);
            }
        }
        [_, "font_sizes", k] => {
            if let Ok(n) = value.parse() {
                var.font_sizes.insert(k.to_string(), n);
            }
        }
        [_, "constants", k] => {
            if let Ok(n) = value.parse() {
                var.constants.insert(k.to_string(), n);
            }
        }
        [_, "fonts", k] => {
            if let Some(id) = quoted(value) {
                var.fonts.insert(k.to_string(), id);
            }
        }
        [_, "styles", k] => {
            if let Some(id) = quoted(value) {
                var.styles.insert(k.to_string(), id);
            }
        }
        _ => {}
    }
}

fn attr(line: &str, name: &str) -> Option<String> {
    let rest = line.split(&format!(" {name}=\"")).nth(1)?;
    Some(rest.split('"').next()?.to_string())
}

fn quoted(value: &str) -> Option<String> {
    Some(value.split('"').nth(1)?.to_string())
}

fn color(value: &str) -> Option<Rgba> {
    let inner = value.strip_prefix("Color(")?.strip_suffix(')')?;
    let v: Vec<f64> = inner.split(',').map(|n| n.trim().parse().ok()).collect::<Option<_>>()?;
    match v[..] {
        [r, g, b, a] => Some(Rgba { r, g, b, a }),
        _ => None,
    }
}

impl Theme {
    /// The variation and its ancestors, nearest first, while each `base` is
    /// another variation of the theme. A cycle ends the chain at the first
    /// repeated name (no error). An unknown name gives an empty chain.
    pub fn chain(&self, name: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut cur = self.variations.get_key_value(name);
        while let Some((n, v)) = cur {
            if out.contains(&n.as_str()) {
                break;
            }
            out.push(n.as_str());
            cur = v.base.as_deref().and_then(|b| self.variations.get_key_value(b));
        }
        out
    }

    pub fn color(&self, name: &str, key: &str) -> Option<Rgba> {
        self.chain(name).into_iter().find_map(|n| self.variations[n].colors.get(key).copied())
    }

    pub fn constant(&self, name: &str, key: &str) -> Option<i64> {
        self.chain(name).into_iter().find_map(|n| self.variations[n].constants.get(key).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/ui/theme.tres");

    #[test]
    fn reads_fonts_styles_and_variations() {
        let t = parse(FIXTURE).unwrap();
        assert_eq!(t.fonts.len(), 2);
        assert_eq!(t.fonts["2_bbbbb"], "res://fonts/Sans-Bold.ttf");
        assert_eq!(t.styles.len(), 3);
        let card = &t.styles["StyleBoxFlat_card"];
        assert_eq!(card.bg.unwrap().a, 0.9);
        assert_eq!(card.border.unwrap().to_hex(), "#8099B3");
        assert!(t.styles["StyleBoxFlat_bar"].border.is_none());
        let tag = &t.variations["Tag"];
        assert_eq!(tag.base.as_deref(), Some("Label"));
        assert_eq!(tag.font_sizes["font_size"], 15);
        assert_eq!(tag.fonts["font"], "1_aaaaa");
        assert_eq!(t.variations["Card"].styles["panel"], "StyleBoxFlat_card");
    }

    #[test]
    fn resolves_through_a_chain_of_three() {
        let t = parse(FIXTURE).unwrap();
        assert_eq!(t.chain("Callout"), ["Callout", "CalloutBase", "Tag"]);
        assert_eq!(t.color("Callout", "font_color").unwrap().to_hex(), "#FFFFFF");
        assert_eq!(t.color("CalloutBase", "font_color").unwrap().to_hex(), "#E6E6CC");
        assert_eq!(t.constant("Callout", "outline_size"), Some(4));
        assert_eq!(t.color("Card", "font_color"), None);
    }

    #[test]
    fn keys_without_a_slash_are_not_variations_and_type_level_entries_are_not_real_ones() {
        let t = parse("[gd_resource type=\"Theme\" format=3]\n\n[resource]\ndefault_font_size = 16\nButton/colors/font_color = Color(1, 1, 1, 1)\nTag/base_type = &\"Label\"\n").unwrap();
        assert!(!t.variations.contains_key("default_font_size"));
        assert!(!t.variations["Button"].is_variation());
        assert!(t.variations["Tag"].is_variation());
        assert_eq!(t.color("Button", "font_color").unwrap().to_hex(), "#FFFFFF");
    }

    #[test]
    fn a_cycle_stops_the_chain() {
        let t = parse("[gd_resource type=\"Theme\" format=3]\n\n[resource]\nA/base_type = &\"B\"\nB/base_type = &\"A\"\nA/constants/x = 1\n").unwrap();
        assert_eq!(t.chain("A"), ["A", "B"]);
        assert_eq!(t.constant("B", "x"), Some(1));
        assert_eq!(t.constant("B", "y"), None);
    }

    #[test]
    fn rejects_binary_and_foreign_files() {
        let dir = std::env::temp_dir().join(format!("kiln-theme-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let res = dir.join("t.res");
        std::fs::write(&res, b"RSRC\0\0").unwrap();
        assert!(load(&res).unwrap_err().contains("binary"));
        assert!(parse("hello").unwrap_err().contains("gd_resource"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Set `KILN_EXTERNAL_THEME` to a real game theme to also check it parses.
    #[test]
    fn parses_an_external_theme_when_present() {
        let Some(p) = std::env::var_os("KILN_EXTERNAL_THEME").map(std::path::PathBuf::from) else { return };
        let t = load(&p).unwrap();
        let inherited = t.variations.iter().any(|(name, v)| {
            v.base.as_ref().is_some_and(|b| t.variations.contains_key(b))
                && t.chain(name).iter().skip(1).any(|up| t.variations[*up].colors.contains_key("font_color"))
                && t.color(name, "font_color").is_some()
        });
        assert!(inherited);
    }
}
