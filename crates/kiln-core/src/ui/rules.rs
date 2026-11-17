//! The UI rules of `kiln check`: contrast, glyphs, coherent theme, style kept in the theme.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{cmap, contrast, inside, names_outside, theme};
use crate::catalog::{Finding, Severity};
use crate::config::{Config, Level};

const BUILTIN: &[&str] = &[
    "Label", "Button", "PanelContainer", "ProgressBar", "LineEdit", "RichTextLabel", "CheckBox", "Panel",
    "VBoxContainer", "HBoxContainer", "ItemList", "TabContainer", "ScrollContainer", "MarginContainer",
    "TextureRect", "Control", "OptionButton", "Tree", "SpinBox", "Slider", "HSlider", "VSlider", "TextEdit",
    "CodeEdit", "Window", "AcceptDialog", "PopupMenu", "MenuButton", "LinkButton", "ColorRect",
    "NinePatchRect", "GridContainer", "CenterContainer", "FlowContainer", "SplitContainer",
    "HSplitContainer", "VSplitContainer", "TabBar", "GraphEdit",
];
const STYLE_MARKS: [&str; 6] = [
    "Color(", "Color.html(", "add_theme_color_override", "add_theme_font_size_override",
    "add_theme_stylebox_override", "StyleBoxFlat.new",
];

/// A string literal: its characters with the line (1-based) each one sits on, and where its
/// opening quote is in `Line::bare`.
struct Lit {
    name: bool,
    col: usize,
    chars: Vec<(usize, char)>,
}

impl Lit {
    fn text(&self) -> String {
        self.chars.iter().map(|c| c.1).collect()
    }
}

/// One source line. `code`: comment removed, string contents kept. `bare`: the same without the
/// contents of the strings (quotes kept). `strings`: the literals that START on this line, even
/// when they end on a later one.
struct Line {
    code: String,
    bare: String,
    strings: Vec<Lit>,
}

/// Splits source into lines with string state carried across them: `"""` strings, strings with
/// a newline inside and `\` continuations. `gd`: `#` comments and `'` strings exist (a scene has
/// neither). `\u` escapes are not handled.
fn scan(text: &str, gd: bool) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    // (quote, triple, start line index, literal being read)
    let mut open: Option<(char, bool, usize, Lit)> = None;
    let mut esc = false;
    for (n, l) in text.lines().enumerate() {
        out.push(Line { code: String::new(), bare: String::new(), strings: Vec::new() });
        let chars: Vec<char> = l.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let line = &mut out[n];
            if let Some((q, triple, _, lit)) = open.as_mut() {
                let (q, triple) = (*q, *triple);
                line.code.push(c);
                if esc {
                    esc = false;
                    if !"ntr".contains(c) {
                        lit.chars.push((n + 1, c));
                    }
                } else if c == '\\' {
                    esc = true;
                } else if c == q && (!triple || chars.get(i + 1..i + 3) == Some(&[q, q][..])) {
                    let width = if triple { 3 } else { 1 };
                    for _ in 0..width {
                        line.bare.push(q);
                    }
                    for _ in 1..width {
                        line.code.push(q);
                    }
                    i += width - 1;
                    if let Some((_, _, start, lit)) = open.take() {
                        out[start].strings.push(lit);
                    }
                } else {
                    lit.chars.push((n + 1, c));
                }
            } else if c == '"' || (gd && c == '\'') {
                let triple = chars.get(i + 1..i + 3) == Some(&[c, c][..]);
                let width = if triple { 3 } else { 1 };
                let lit = Lit { name: line.code.ends_with('&'), col: line.bare.len(), chars: Vec::new() };
                for _ in 0..width {
                    line.code.push(c);
                    line.bare.push(c);
                }
                i += width - 1;
                open = Some((c, triple, n, lit));
            } else if c == '#' && gd {
                break;
            } else {
                line.code.push(c);
                line.bare.push(c);
            }
            i += 1;
        }
        if let Some((_, _, _, lit)) = open.as_mut() {
            if esc {
                esc = false;
            } else {
                lit.chars.push((n + 1, '\n'));
            }
        }
    }
    if let Some((_, _, start, lit)) = open {
        out[start].strings.push(lit);
    }
    out
}

/// A ratio shown cut (not rounded) to one decimal, so a 4.46 against a minimum of 4.5 reads 4.4.
fn shown(v: f64) -> String {
    format!("{:.1}", (v * 10.0).floor() / 10.0)
}

/// The variation a line assigns: only when the right-hand side starts with a string literal.
fn assigned_variation(l: &Line) -> Option<String> {
    const KEY: &str = "theme_type_variation";
    let rest = l.bare[l.bare.find(KEY)? + KEY.len()..].trim_start();
    let rest = rest.strip_prefix('=').filter(|r| !r.starts_with('=')).or_else(|| rest.strip_prefix('('))?;
    let lead = rest.trim_start();
    let at = l.bare.len() - lead.len();
    let quote = at + usize::from(lead.starts_with('&'));
    l.strings.iter().find(|s| s.col == quote).map(Lit::text)
}

fn push(out: &mut Vec<Finding>, level: Level, subject: &str, code: &str, message: String) {
    let severity = match level {
        Level::Off => return,
        Level::Warn => Severity::Warn,
        Level::Error => Severity::Error,
    };
    out.push(Finding { severity, subject: subject.into(), code: code.into(), message });
}

/// Files of the project that the globs match, each one as (name, real path). A pattern that
/// leaves the project, matches nothing or matches a link that leads out is a `ui-files` warning.
fn load(root: &Path, globs: &[String], gd: bool, out: &mut Vec<Finding>) -> Vec<(String, Vec<Line>)> {
    let base = glob::Pattern::escape(&root.to_string_lossy());
    let canonical_root = std::fs::canonicalize(root).ok();
    let mut files: Vec<(PathBuf, PathBuf)> = Vec::new();
    for g in globs {
        if names_outside(g) {
            push(out, Level::Warn, g, "ui-files", format!("`{g}`: el patrón sale del proyecto; se ignora"));
            continue;
        }
        let Ok(paths) = glob::glob(&format!("{base}/{g}")) else {
            push(out, Level::Warn, g, "ui-files", format!("`{g}`: patrón no válido"));
            continue;
        };
        let (mut matched, mut skipped) = (0, 0);
        for p in paths.flatten().filter(|p| p.is_file()) {
            matched += 1;
            match std::fs::canonicalize(&p) {
                Ok(real) if canonical_root.as_ref().is_some_and(|r| real.starts_with(r)) => files.push((p, real)),
                _ => skipped += 1,
            }
        }
        if matched == 0 {
            push(out, Level::Warn, g, "ui-files", format!("`{g}`: ningún fichero coincide"));
        } else if skipped > 0 {
            push(out, Level::Warn, g, "ui-files", format!("`{g}`: {skipped} fichero(s) fuera del proyecto, ignorados"));
        }
    }
    files.sort();
    files.dedup();
    files
        .into_iter()
        .filter_map(|(p, real)| {
            let name = p.strip_prefix(root).unwrap_or(&p).display().to_string();
            Some((name, scan(&std::fs::read_to_string(&real).ok()?, gd)))
        })
        .collect()
}

/// Variation names that scripts and scenes use, before following each `base` chain.
pub fn used_variations(root: &Path, ui: &crate::config::UiSection) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    let mut ignored = Vec::new();
    for (_, lines) in load(root, &ui.scripts, true, &mut ignored).iter().chain(&load(root, &ui.scenes, false, &mut ignored)) {
        for l in lines {
            used.extend(l.strings.iter().filter(|s| s.name).map(Lit::text));
            used.extend(assigned_variation(l));
        }
    }
    used
}

/// Findings of the UI rules, for a project whose `kiln.toml` has `[ui]`.
pub fn findings(root: &Path, config: &Config) -> Vec<Finding> {
    let Some(ui) = &config.ui else { return Vec::new() };
    let mut out = Vec::new();
    let theme_file = ui.theme.as_str();
    let loaded = inside(root, theme_file).and_then(|real| theme::load(&real));
    let theme = match loaded {
        Ok(t) => t,
        Err(e) => {
            push(&mut out, Level::Error, theme_file, "ui-theme", e);
            return out;
        }
    };
    let scripts = load(root, &ui.scripts, true, &mut out);
    let scenes = load(root, &ui.scenes, false, &mut out);

    if ui.contrast.level != Level::Off {
        let contrast_level = ui.contrast.level;
        let (measures, unbacked) = contrast::measure(&theme, &ui.contrast);
        for m in measures.iter().filter(|m| m.ratio < m.min) {
            let msg = format!("{} sobre {}: {}:1 < {:.1}:1", m.fg, m.bg, shown(m.ratio), m.min);
            push(&mut out, contrast_level, &m.fg, "ui-contrast", msg);
        }
        for u in unbacked {
            if u.starts_with("pair ") {
                push(&mut out, contrast_level, theme_file, "ui-contrast-unbacked", u);
            } else {
                let msg = format!("{u}: sin fondo declarado; no se mide");
                out.push(Finding { severity: Severity::Info, subject: u, code: "ui-contrast-unbacked".into(), message: msg });
            }
        }
    }

    if ui.glyphs.level != Level::Off {
        let mut fonts = Vec::new();
        let godot = config.tools.godot_project.as_deref().unwrap_or(".");
        let is_font = |p: &&String| {
            let p = p.to_ascii_lowercase();
            p.ends_with(".ttf") || p.ends_with(".otf")
        };
        for res in theme.fonts.values().filter(is_font) {
            let rel = format!("{godot}/{}", res.trim_start_matches("res://"));
            let real = match inside(root, &rel) {
                Ok(real) => real,
                Err(_) if names_outside(&rel) || root.join(&rel).exists() => {
                    let msg = format!("fuente `{res}`: la ruta sale del proyecto; no se lee");
                    push(&mut out, Level::Warn, res, "ui-files", msg);
                    continue;
                }
                Err(_) => {
                    push(&mut out, ui.glyphs.level, res, "ui-files", format!("fuente ilegible: `{res}` no existe"));
                    continue;
                }
            };
            match std::fs::read(&real).map_err(|e| e.kind().to_string()).and_then(|b| cmap::codepoints(&b)) {
                Ok(set) => fonts.push((res.as_str(), set)),
                Err(e) => push(&mut out, ui.glyphs.level, res, "ui-files", format!("fuente ilegible: {e}")),
            }
        }
        let texts = scripts.iter().map(|(f, ls)| {
            (f, ls.iter().flat_map(|l| l.strings.iter().flat_map(|s| s.chars.iter().copied())).collect::<Vec<_>>())
        });
        let scene_texts = scenes.iter().map(|(f, ls)| {
            let texts = ls.iter().filter(|l| l.bare.trim_start().starts_with("text = "));
            (f, texts.filter_map(|l| Some(l.strings.first()?.chars.clone())).flatten().collect::<Vec<_>>())
        });
        for (file, chars) in texts.chain(scene_texts) {
            let mut seen = BTreeSet::new();
            for (line, c) in chars.into_iter().filter(|(_, c)| !c.is_whitespace() && !c.is_control()) {
                let Some((font, _)) = fonts.iter().find(|(_, set)| !set.contains(&(c as u32))) else { continue };
                if seen.insert(c) {
                    let msg = format!("«{c}» (U+{:04X}) falta en {font}: {file}:{line}", c as u32);
                    push(&mut out, ui.glyphs.level, file, "ui-glyph", msg);
                }
            }
        }
    }

    if ui.theme_coherent.level != Level::Off {
        let (mut used, mut assigned) = (BTreeSet::new(), BTreeMap::new());
        for (file, lines) in scripts.iter().chain(&scenes) {
            for (i, l) in lines.iter().enumerate() {
                used.extend(l.strings.iter().filter(|s| s.name).map(Lit::text));
                if let Some(name) = assigned_variation(l) {
                    used.insert(name.clone());
                    assigned.entry(name).or_insert_with(|| format!("{file}:{}", i + 1));
                }
            }
        }
        for (name, at) in &assigned {
            if !theme.variations.contains_key(name) && !BUILTIN.contains(&name.as_str()) {
                let msg = format!("la variación «{name}» se usa en {at} y el tema no la define");
                push(&mut out, ui.theme_coherent.level, theme_file, "ui-variation-missing", msg);
            }
        }
        let kept: BTreeSet<&str> = used.iter().flat_map(|u| theme.chain(u)).collect();
        for (name, _) in theme.variations.iter().filter(|(n, v)| v.is_variation() && !kept.contains(n.as_str())) {
            let msg = format!("la variación «{name}» está en el tema y ningún script o escena la usa");
            push(&mut out, ui.theme_coherent.level, theme_file, "ui-variation-unused", msg);
        }
    }

    for (file, lines) in &scripts {
        for (i, l) in lines.iter().enumerate() {
            for mark in STYLE_MARKS {
                for _ in 0..l.bare.matches(mark).count() {
                    let msg = format!("`{mark}` fuera del tema: {file}:{}", i + 1);
                    push(&mut out, ui.theme_only.level, file, "ui-style-outside-theme", msg);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Severity;
    use theme::Rgba;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ui");
    const CODES: [&str; 6] = [
        "ui-contrast",
        "ui-contrast-unbacked",
        "ui-glyph",
        "ui-style-outside-theme",
        "ui-variation-missing",
        "ui-variation-unused",
    ];

    /// A copy of the fixture project (and its font) in a temp dir that goes away on drop.
    struct Tmp(PathBuf);

    impl Tmp {
        fn new() -> Tmp {
            static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("kiln-rules-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for f in ["clean.gd", "clean.tres", "clean.tscn", "dirty.gd", "dirty.tres", "dirty.tscn"] {
                std::fs::copy(Path::new(FIXTURES).join("project").join(f), dir.join(f)).unwrap();
            }
            std::fs::copy(Path::new(FIXTURES).join("IBMPlexMono-Regular.ttf"), dir.join("IBMPlexMono-Regular.ttf")).unwrap();
            Tmp(dir)
        }

        fn write(&self, name: &str, text: &str) {
            std::fs::write(self.0.join(name), text).unwrap();
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn config(set: &str, extra: &str) -> Config {
        raw_config(&format!("theme = \"{set}.tres\"\nscripts = [\"{set}.gd\"]\nscenes = [\"{set}.tscn\"]\n"), extra)
    }

    fn raw_config(ui: &str, extra: &str) -> Config {
        let toml = format!("[project]\nname = \"x\"\n[tools]\ngodot_project = \".\"\n[ui]\n{ui}{extra}");
        Config::parse(&toml).unwrap()
    }

    fn run(set: &str, extra: &str) -> Vec<Finding> {
        let t = Tmp::new();
        findings(&t.0, &config(set, extra))
    }

    fn of<'a>(f: &'a [Finding], code: &str) -> Vec<&'a Finding> {
        f.iter().filter(|x| x.code == code).collect()
    }

    const THEME: &str = "[gd_resource type=\"Theme\" format=3]\n\n[ext_resource type=\"FontFile\" path=\"res://IBMPlexMono-Regular.ttf\" id=\"1_font\"]\n\n[resource]\nPlain/base_type = &\"Label\"\nPlain/colors/font_color = Color(1, 1, 1, 1)\n";

    /// A project with `t.tres` = THEME plus `extra_theme`, one script and one scene.
    fn with_sources(extra_theme: &str, gd: &str, tscn: &str, extra: &str) -> Vec<Finding> {
        let t = Tmp::new();
        t.write("t.tres", &format!("{THEME}{extra_theme}"));
        t.write("t.gd", gd);
        t.write("t.tscn", tscn);
        findings(&t.0, &config("t", extra))
    }

    #[test]
    fn a_dirty_project_gives_one_finding_per_rule() {
        let f = run("dirty", "");
        for code in CODES {
            assert_eq!(of(&f, code).len(), 1, "{code}: {f:?}");
        }
        assert_eq!(f.len(), CODES.len(), "{f:?}");
        assert!(of(&f, "ui-contrast")[0].message.starts_with("Faint sobre Faint/outline: 1.1:1 < 4.5:1"));
        let glyph = &of(&f, "ui-glyph")[0].message;
        assert!(glyph.contains("U+26A0") && glyph.contains("dirty.gd:8"), "{glyph}");
        assert!(of(&f, "ui-style-outside-theme")[0].message.contains("dirty.gd:9"));
        assert!(of(&f, "ui-variation-missing")[0].message.contains("Ghost"));
        assert!(of(&f, "ui-variation-unused")[0].message.contains("Orphan"));
        assert_eq!(of(&f, "ui-contrast-unbacked")[0].severity, Severity::Info);
        assert!(f.iter().filter(|x| x.code != "ui-contrast-unbacked").all(|x| x.severity == Severity::Warn));
    }

    #[test]
    fn a_clean_project_gives_none() {
        assert!(run("clean", "").is_empty());
    }

    #[test]
    fn error_level_is_an_error_and_off_is_silent() {
        let all = |l: &str| {
            format!("[ui.contrast]\nlevel = \"{l}\"\n[ui.glyphs]\nlevel = \"{l}\"\n[ui.theme_coherent]\nlevel = \"{l}\"\n[ui.theme_only]\nlevel = \"{l}\"\n")
        };
        let f = run("dirty", &all("error"));
        assert_eq!(f.len(), CODES.len());
        for x in &f {
            let want = if x.code == "ui-contrast-unbacked" { Severity::Info } else { Severity::Error };
            assert_eq!(x.severity, want, "{}", x.code);
        }
        assert!(run("dirty", &all("off")).is_empty());
    }

    #[test]
    fn an_unresolved_pair_follows_the_rule_level() {
        let pair = "[[ui.contrast.pairs]]\nfg = \"Plain\"\nbg = \"Nope/panel\"\n[ui.contrast]\nlevel = \"error\"\n";
        let f = run("dirty", pair);
        let u = of(&f, "ui-contrast-unbacked");
        assert_eq!((u.len(), u[0].severity), (1, Severity::Error));
        assert!(u[0].message.contains("Nope/panel"));
    }

    #[test]
    fn an_unreadable_theme_is_the_only_finding() {
        let t = Tmp::new();
        let mut c = config("dirty", "");
        c.ui.as_mut().unwrap().theme = "missing.tres".into();
        let f = findings(&t.0, &c);
        assert_eq!((f.len(), f[0].code.as_str(), f[0].severity), (1, "ui-theme", Severity::Error));
    }

    #[test]
    fn a_theme_outside_the_project_is_a_theme_error_without_reading_it() {
        let t = Tmp::new();
        std::fs::create_dir(t.0.join("sub")).unwrap();
        let abs = t.0.join("clean.tres").display().to_string();
        for theme in ["../clean.tres", abs.as_str(), "x/../../clean.tres"] {
            let mut c = config("clean", "");
            c.ui.as_mut().unwrap().theme = theme.into();
            let f = findings(&t.0.join("sub"), &c);
            assert_eq!((f.len(), f[0].code.as_str(), f[0].severity), (1, "ui-theme", Severity::Error), "{theme}: {f:?}");
        }
    }

    #[test]
    fn a_glob_that_matches_nothing_is_a_files_warning_whatever_the_rule_levels() {
        let t = Tmp::new();
        let mut c = config("clean", "[ui.contrast]\nlevel = \"off\"\n[ui.glyphs]\nlevel = \"off\"\n[ui.theme_coherent]\nlevel = \"off\"\n[ui.theme_only]\nlevel = \"off\"\n");
        c.ui.as_mut().unwrap().scripts = vec!["nothing/*.gd".into()];
        let f = findings(&t.0, &c);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!((f[0].code.as_str(), f[0].severity), ("ui-files", Severity::Warn));
        assert!(f[0].message.contains("nothing/*.gd"));
    }

    #[test]
    fn globs_that_leave_the_project_are_rejected() {
        let t = Tmp::new();
        for g in ["../*.gd", "sub/../../*.gd", "/etc/*"] {
            let mut c = config("clean", "");
            c.ui.as_mut().unwrap().scripts = vec![g.into(), "clean.gd".into()];
            let f = findings(&t.0, &c);
            let files = of(&f, "ui-files");
            assert_eq!((files.len(), files[0].severity), (1, Severity::Warn), "{g}: {f:?}");
            assert!(files[0].message.contains(g), "{g}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_project_is_skipped_not_read() {
        let t = Tmp::new();
        let out = Tmp::new();
        out.write("evil.gd", "var c = Color(1, 0, 0)\n");
        std::os::unix::fs::symlink(out.0.join("evil.gd"), t.0.join("link.gd")).unwrap();
        let mut c = config("clean", "");
        c.ui.as_mut().unwrap().scripts = vec!["clean.gd".into(), "link.gd".into()];
        let f = findings(&t.0, &c);
        assert!(of(&f, "ui-style-outside-theme").is_empty(), "{f:?}");
        let files = of(&f, "ui-files");
        assert_eq!((files.len(), files[0].severity), (1, Severity::Warn), "{f:?}");
    }

    #[test]
    fn a_font_path_that_leaves_the_project_is_a_files_warning() {
        let t = Tmp::new();
        t.write("t.tres", &THEME.replace("res://IBMPlexMono-Regular.ttf", "res://../IBMPlexMono-Regular.ttf"));
        t.write("t.gd", "");
        t.write("t.tscn", "");
        let f = findings(&t.0, &config("t", ""));
        let files: Vec<_> = of(&f, "ui-files").into_iter().filter(|x| x.subject.contains("..")).collect();
        assert_eq!((files.len(), files[0].severity), (1, Severity::Warn), "{f:?}");
    }

    #[test]
    fn font_extensions_are_case_insensitive() {
        let t = Tmp::new();
        std::fs::copy(t.0.join("IBMPlexMono-Regular.ttf"), t.0.join("Upper.TTF")).unwrap();
        t.write("t.tres", &THEME.replace("IBMPlexMono-Regular.ttf", "Upper.TTF"));
        t.write("t.gd", "var a = \"\u{26A0}\"\n");
        t.write("t.tscn", "");
        let f = findings(&t.0, &config("t", ""));
        assert_eq!(of(&f, "ui-glyph").len(), 1, "{f:?}");
    }

    #[test]
    fn an_unreadable_font_follows_the_glyphs_level() {
        for (level, want) in [("warn", Severity::Warn), ("error", Severity::Error)] {
            let t = Tmp::new();
            std::fs::remove_file(t.0.join("IBMPlexMono-Regular.ttf")).unwrap();
            t.write("t.tres", THEME);
            t.write("t.gd", "");
            t.write("t.tscn", "");
            let f = findings(&t.0, &config("t", &format!("[ui.glyphs]\nlevel = \"{level}\"\n")));
            let files = of(&f, "ui-files");
            assert_eq!((files.len(), files[0].severity), (1, want), "{level}: {f:?}");
        }
    }

    #[test]
    fn type_level_theme_entries_are_not_unused_variations() {
        let f = with_sources("Button/colors/font_color = Color(1, 1, 1, 1)\nOrphan/base_type = &\"Label\"\n", "", "", "");
        let unused = of(&f, "ui-variation-unused");
        assert_eq!(unused.len(), 2, "{f:?}");
        assert!(unused.iter().all(|x| !x.message.contains("Button")));
    }

    #[test]
    fn more_builtin_controls_need_no_definition() {
        let gd = "$A.theme_type_variation = &\"OptionButton\"\n$B.theme_type_variation = &\"GraphEdit\"\n";
        let f = with_sources("", gd, "", "");
        assert!(of(&f, "ui-variation-missing").is_empty(), "{f:?}");
    }

    #[test]
    fn a_multi_line_scene_text_is_scanned_on_the_line_of_the_glyph() {
        let tscn = "[node name=\"L\" type=\"Label\"]\ntext = \"fine\nstill fine \u{26A0}\nend\"\n";
        let f = with_sources("", "", tscn, "");
        let g = of(&f, "ui-glyph");
        assert_eq!(g.len(), 1, "{f:?}");
        assert!(g[0].message.contains("t.tscn:3"), "{}", g[0].message);
    }

    #[test]
    fn multi_line_and_continued_gd_strings_are_scanned() {
        let gd = "var a = \"\"\"one\ntwo \u{26A0}\n\"\"\"\nvar b = \"x\\\n\u{2715}\"\n";
        let f = with_sources("", gd, "", "");
        let msgs: Vec<_> = of(&f, "ui-glyph").iter().map(|x| x.message.clone()).collect();
        assert_eq!(msgs.len(), 2, "{msgs:?}");
        assert!(msgs[0].contains("t.gd:2") && msgs[1].contains("t.gd:5"), "{msgs:?}");
    }

    #[test]
    fn a_variation_is_used_only_when_a_string_literal_is_assigned() {
        let gd = "$A.theme_type_variation = _pick(\"Ghost\")\n$B.theme_type_variation = &\"Plain\"\n$C.theme_type_variation = \"Plain\"\n";
        let f = with_sources("", gd, "", "");
        assert!(of(&f, "ui-variation-missing").is_empty(), "{f:?}");
        assert!(of(&f, "ui-variation-unused").is_empty(), "{f:?}");
        let f = with_sources("", "$A.theme_type_variation = _pick(\"Plain\")\n", "", "");
        assert_eq!(of(&f, "ui-variation-unused").len(), 1, "{f:?}");
    }

    #[test]
    fn style_marks_inside_string_literals_are_not_style_written_outside_the_theme() {
        let gd = "print(\"Color(1, 0, 0)\")\nvar x = \"\"\"\nadd_theme_color_override\n\"\"\"\n";
        let f = with_sources("", gd, "", "");
        assert!(of(&f, "ui-style-outside-theme").is_empty(), "{f:?}");
        let f = with_sources("", "var c = Color(1, 0, 0)\n", "", "");
        assert_eq!(of(&f, "ui-style-outside-theme").len(), 1);
    }

    #[test]
    fn ratios_are_shown_truncated_to_one_decimal() {
        assert_eq!(shown(4.46), "4.4");
        assert_eq!(shown(4.5), "4.5");
        assert_eq!(shown(4.4999999), "4.4");
        assert_eq!(shown(21.0), "21.0");
    }

    #[test]
    fn colour_channels_are_clamped_before_luminance() {
        let white = contrast::ratio(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
        let over = contrast::ratio(Rgba { r: 2.0, g: 2.0, b: 2.0, a: 1.0 }, Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
        assert!((white - 21.0).abs() < 1e-9 && (over - white).abs() < 1e-9, "{white} {over}");
    }

    #[test]
    fn the_scanner_skips_comments_and_keeps_quotes() {
        let s = scan("a = \"⚠⚠\"\nb = '⚠' # \"x\"\n", true);
        assert_eq!(s[1].strings.len(), 1);
        assert_eq!((s[1].strings[0].text(), s[1].strings[0].name), ("⚠".to_string(), false));
        assert_eq!(s[1].code, "b = '⚠' ");
        assert_eq!(s[1].bare, "b = '' ");
    }
}
