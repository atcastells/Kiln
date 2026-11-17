//! WCAG 2.x contrast of the theme's text and shapes, measured without Godot.
use super::theme::{Rgba, Theme};
use crate::config::{ContrastRule, PairKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Outline,
    Pair,
}

#[derive(Debug, Clone)]
pub struct Measure {
    pub fg: String,
    pub bg: String,
    /// Full precision; round to one decimal only to display it.
    pub ratio: f64,
    pub min: f64,
    pub source: Source,
}

/// WCAG ratio. Alpha: the background is composited over opaque black and over opaque white, the
/// foreground over each result, and the lower ratio wins.
pub fn ratio(fg: Rgba, bg: Rgba) -> f64 {
    let over = |top: Rgba, under: Rgba| Rgba {
        r: top.r * top.a + under.r * (1.0 - top.a),
        g: top.g * top.a + under.g * (1.0 - top.a),
        b: top.b * top.a + under.b * (1.0 - top.a),
        a: 1.0,
    };
    // A translucent background shows whatever the scene has behind it: take the worst of black and white.
    [0.0, 1.0]
        .into_iter()
        .map(|v| {
            let bg = over(bg, Rgba { r: v, g: v, b: v, a: 1.0 });
            let fg = over(fg, bg);
            let (a, b) = (luminance(fg), luminance(bg));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        })
        .fold(f64::INFINITY, f64::min)
}

fn luminance(c: Rgba) -> f64 {
    let lin = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

fn text_min(theme: &Theme, rule: &ContrastRule, variation: &str) -> f64 {
    let size = theme
        .chain(variation)
        .into_iter()
        .find_map(|n| theme.variations[n].font_sizes.get("font_size").copied());
    if size.is_some_and(|s| s >= rule.large_from_px) { rule.large } else { rule.normal }
}

/// The `bg_color` of `<variation>/<style box>`, the style resolved through the variation chain.
fn style_bg(theme: &Theme, path: &str) -> Result<Rgba, String> {
    let (var, style) = path.split_once('/').ok_or_else(|| format!("`{path}` is not <variation>/<style box>"))?;
    let id = theme
        .chain(var)
        .into_iter()
        .find_map(|n| theme.variations[n].styles.get(style))
        .ok_or_else(|| format!("`{path}`: no such style box in the theme"))?;
    theme
        .styles
        .get(id)
        .and_then(|s| s.bg)
        .ok_or_else(|| format!("`{path}`: the style box has no bg_color"))
}

/// Measures every outlined text and every declared pair. The second list holds the variations with a
/// `font_color` that have no background, and the pairs that could not be resolved (with the reason).
pub fn measure(theme: &Theme, rule: &ContrastRule) -> (Vec<Measure>, Vec<String>) {
    let (mut out, mut unbacked) = (Vec::new(), Vec::new());
    for name in theme.variations.keys() {
        let Some(font) = theme.color(name, "font_color") else { continue };
        let outline = theme.color(name, "font_outline_color");
        if let (Some(o), true) = (outline, theme.constant(name, "outline_size").unwrap_or(0) > 0) {
            out.push(Measure {
                fg: name.clone(),
                bg: format!("{name}/outline"),
                ratio: ratio(font, o),
                min: text_min(theme, rule, name),
                source: Source::Outline,
            });
        } else if !rule.pairs.iter().any(|p| p.fg == *name) {
            unbacked.push(name.clone());
        }
    }
    for p in &rule.pairs {
        let fg = if p.fg.contains('/') {
            style_bg(theme, &p.fg)
        } else {
            theme.color(&p.fg, "font_color").ok_or_else(|| format!("`{}`: no font_color", p.fg))
        };
        let min = match p.kind {
            PairKind::NonText => rule.large,
            PairKind::Text => text_min(theme, rule, &p.fg),
        };
        match (fg, style_bg(theme, &p.bg)) {
            (Ok(f), Ok(b)) => out.push(Measure {
                fg: p.fg.clone(),
                bg: p.bg.clone(),
                ratio: ratio(f, b),
                min,
                source: Source::Pair,
            }),
            (Err(e), _) | (_, Err(e)) => unbacked.push(format!("pair {} / {}: {e}", p.fg, p.bg)),
        }
    }
    (out, unbacked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::parse;

    const FIXTURE: &str = include_str!("../../tests/fixtures/ui/theme.tres");

    fn hex(s: &str) -> Rgba {
        let n = |i: usize| f64::from(u8::from_str_radix(&s[i..i + 2], 16).unwrap()) / 255.0;
        Rgba { r: n(1), g: n(3), b: n(5), a: 1.0 }
    }

    /// sRGB channel to linear light, from the WCAG definition (a pure grey has the same value in each channel).
    fn lin(v: f64) -> f64 {
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    }

    fn grey(v: f64) -> Rgba {
        Rgba { r: v, g: v, b: v, a: 1.0 }
    }

    #[test]
    fn matches_ratios_worked_out_by_hand() {
        // Black on white: (1 + 0.05) / (0 + 0.05) = 21.
        assert!((ratio(hex("#FFFFFF"), hex("#000000")) - 21.0).abs() < 1e-9);
        assert!((ratio(hex("#000000"), hex("#FFFFFF")) - 21.0).abs() < 1e-9);
        // Same colour on itself: 1.
        assert!((ratio(hex("#808080"), hex("#808080")) - 1.0).abs() < 1e-9);
        // Mid grey #777777 (119/255) on white: L = 0.18447, so 1.05 / 0.23447 = 4.478, the 4.48 of the
        // usual "grey text on white" example.
        let l = lin(119.0 / 255.0);
        assert!((l - 0.18447).abs() < 1e-4, "{l}");
        assert!((ratio(hex("#777777"), hex("#FFFFFF")) - 1.05 / (l + 0.05)).abs() < 1e-9);
        assert!((ratio(hex("#777777"), hex("#FFFFFF")) - 4.48).abs() < 0.005);
        // Mid grey #808080 on black: L = 0.21586, so (0.21586 + 0.05) / 0.05 = 5.32.
        assert!((ratio(hex("#808080"), hex("#000000")) - 5.32).abs() < 0.005);
    }

    #[test]
    fn a_translucent_background_takes_the_worst_of_black_and_white_behind() {
        let at = |c: &str, a: f64| Rgba { a, ..hex(c) };
        // White text on a black panel at 90%: behind it, black gives 21:1 and white gives a background
        // of 0.1 grey (L = lin(0.1) = 0.01002), so 1.05 / (0.01002 + 0.05) = 17.5. The worst is white behind.
        let want = 1.05 / (lin(0.1) + 0.05);
        assert!((want - 17.5).abs() < 0.05, "{want}");
        assert!((ratio(hex("#FFFFFF"), at("#000000", 0.9)) - want).abs() < 1e-9);
        // Dark text on a light translucent panel: the worst case is black behind it.
        let dark_on_light = ratio(hex("#000000"), at("#FFFFFF", 0.5));
        assert!((dark_on_light - ratio(hex("#000000"), grey(0.5))).abs() < 1e-9);
        assert!(dark_on_light < ratio(hex("#000000"), hex("#FFFFFF")));
    }

    fn pair(fg: &str, bg: &str, kind: PairKind) -> crate::config::ContrastPair {
        crate::config::ContrastPair { fg: fg.into(), bg: bg.into(), kind }
    }

    #[test]
    fn outline_is_inherited_and_unbacked_text_is_listed() {
        let t = parse(FIXTURE).unwrap();
        let (m, none) = measure(&t, &ContrastRule::default());
        let callout = m.iter().find(|x| x.fg == "Callout").unwrap();
        assert_eq!(callout.source, Source::Outline);
        // White text on a half-transparent black outline: worst case is white behind it, a mid grey.
        let grey = Rgba { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };
        assert!((callout.ratio - ratio(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, grey)).abs() < 1e-9);
        assert_eq!(callout.min, 4.5, "inherited size 15 is below the large threshold");
        assert_eq!(none, ["Tag"]);
    }

    #[test]
    fn pairs_use_the_kind_threshold_and_report_unresolved() {
        let t = parse(FIXTURE).unwrap();
        let rule = ContrastRule {
            pairs: vec![
                pair("Gauge/fill", "Gauge/background", PairKind::NonText),
                pair("Tag", "Card/panel", PairKind::Text),
                pair("Tag", "Nope/panel", PairKind::Text),
            ],
            ..ContrastRule::default()
        };
        let (m, none) = measure(&t, &rule);
        let nt = m.iter().find(|x| x.fg == "Gauge/fill").unwrap();
        assert_eq!((nt.min, nt.source), (3.0, Source::Pair));
        assert!((nt.ratio - 8.3).abs() < 0.05);
        let text = m.iter().find(|x| x.fg == "Tag").unwrap();
        assert_eq!(text.min, 4.5);
        assert!(none.iter().any(|s| s.contains("Nope/panel")));
        assert!(!none.iter().any(|s| s == "Tag"));
    }
}
