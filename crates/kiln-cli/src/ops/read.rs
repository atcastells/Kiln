//! Reading the project: status, catalog, findings, tree, specs, context, content.

use super::rel;
use crate::agent::Ctx;
use kiln_core::{Catalog, Severity};
use serde_json::{Value, json};
use std::sync::Mutex;

pub(crate) fn load(ctx: &Ctx) -> Result<Catalog, String> {
    crate::load(&ctx.project, ctx.config.as_deref())
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

fn counts(cat: &Catalog) -> Value {
    let n = |s: Severity| cat.findings.iter().filter(|f| f.severity == s).count();
    json!({ "error": n(Severity::Error), "warn": n(Severity::Warn), "info": n(Severity::Info) })
}

/// One page of `rows`: total, offset and the items.
fn page(rows: Vec<Value>, offset: usize, limit: usize) -> Value {
    let limit = if limit == 0 { 50 } else { limit.min(200) };
    let items: Vec<&Value> = rows.iter().skip(offset).take(limit).collect();
    json!({ "total": rows.len(), "offset": offset, "items": items })
}

type ToolsCache = Mutex<Option<(String, Vec<kiln_core::tools::Tool>)>>;
static TOOLS: ToolsCache = Mutex::new(None);

/// External tools with their versions, probed once per process for the same
/// configured programs. It runs them: not for read-only operations.
pub fn tools(cfg: &kiln_core::Config) -> Vec<kiln_core::tools::Tool> {
    let t = &cfg.tools;
    let key = [&t.blender, &t.godot, &t.gltf_validator, &t.gltf_transform]
        .map(|s| s.as_str())
        .join("\u{1f}");
    let mut cache = TOOLS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, found)) = cache.as_ref()
        && *k == key
    {
        return found.clone();
    }
    let found = kiln_core::tools::detect(cfg);
    *cache = Some((key, found.clone()));
    found
}

/// The `tools` operation: every external tool with its version.
pub fn tools_report(ctx: &Ctx) -> Result<Value, String> {
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    Ok(json!({ "tools": tools(&cfg) }))
}

pub fn status(ctx: &Ctx) -> Result<Value, String> {
    let cat = load(ctx)?;
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let c = kiln_core::context::load(&ctx.context_file)?;
    let missing = c.missing(&ctx.layers);
    let findings = counts(&cat);
    let search = cat
        .specs
        .iter()
        .find(|s| s.parts.iter().any(|p| p.source.kind == "search"));
    let journey = super::start::journey(ctx, &cat, &cfg);
    let next = if findings["error"].as_u64().unwrap_or(0) > 0 {
        "Corrige primero los errores de «check».".to_string()
    } else if let Some(s) = search {
        format!("La ficha «{}» tiene piezas por buscar: resuélvelas.", s.asset.id)
    } else if let Some(n) = super::start::next(&cfg, &journey) {
        n
    } else if !missing.is_empty() {
        format!("Completa el contexto: {}.", missing[0])
    } else {
        "Nada pendiente.".to_string()
    };
    let mut out = json!({
        "project": cat.project,
        "kits": cat.kits.len(),
        "pieces": cat.pieces.len(),
        "materials": cat.materials.len(),
        "specs": cat.specs.len(),
        "content_sets": cat.content.len(),
        "findings": findings,
        "context": { "claims": c.claims.len(), "open_questions": c.questions.iter().filter(|q| q.status == "open").count(), "missing": missing },
        "next": next,
        "assets": journey,
        "tools": kiln_core::tools::present(&cfg),
    });
    if let Some(t) = ui_target(ctx, &cfg, &c, &cat) {
        out["ui_target"] = t;
    }
    Ok(out)
}

/// Progress of the UI target; `None` when the project has no `[ui] target`.
fn ui_target(ctx: &Ctx, cfg: &kiln_core::Config, c: &kiln_core::context::Context, cat: &Catalog) -> Option<Value> {
    let ui = cfg.ui.as_ref()?;
    let path = ui.target.as_ref()?;
    let result = kiln_core::ui::inside(&ctx.project, path)
        .and_then(|real| kiln_core::ui::target::load(&real))
        .map(|file| kiln_core::ui::target::evaluate(&c.claims, &file, &cat.findings, ui));
    Some(match result {
        Ok(p) => json!({ "met": p.met, "total": p.total, "percent": p.percent, "open": p.open, "problems": p.problems }),
        Err(e) => json!({ "error": e }),
    })
}

/// `catalog what=ui`: the variation rows (with where each one is used), the fonts and the colour tokens.
fn ui_rows(ctx: &Ctx) -> Result<(Vec<Value>, Value, Value), String> {
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let ui = cfg.ui.as_ref().ok_or("el proyecto no tiene [ui] en kiln.toml")?;
    let theme = kiln_core::ui::inside(&ctx.project, &ui.theme).and_then(|real| kiln_core::ui::theme::load(&real))?;
    let kept: std::collections::BTreeSet<String> = kiln_core::ui::rules::used_variations(&ctx.project, ui)
        .iter()
        .flat_map(|u| theme.chain(u).into_iter().map(String::from))
        .collect();
    let names: Vec<&str> = theme.variations.keys().map(String::as_str).collect();
    let uses = kiln_core::ui::target::variation_uses(&ctx.project, ui, &names);
    let fonts: Vec<Value> = theme.fonts.iter().map(|(id, path)| json!({ "id": id, "path": path })).collect();
    let colors: Vec<Value> = theme
        .variations
        .iter()
        .flat_map(|(name, v)| {
            v.colors.iter().map(move |(key, c)| json!({ "variation": name, "key": key, "hex": c.to_hex(), "alpha": c.a }))
        })
        .collect();
    let rows = theme
        .variations
        .iter()
        .map(|(name, v)| {
            json!({
                "name": name,
                "base": v.base,
                "chain": theme.chain(name),
                "font_color": theme.color(name, "font_color").map(|c| c.to_hex()),
                "font_size": theme.chain(name).into_iter().find_map(|n| theme.variations[n].font_sizes.get("font_size")),
                "used": kept.contains(name),
                "uses": uses[name],
            })
        })
        .collect();
    Ok((rows, json!(fonts), json!(colors)))
}

pub fn catalog(ctx: &Ctx, what: &str, filter: Option<&str>, offset: usize, limit: usize) -> Result<Value, String> {
    let cat = load(ctx)?;
    let f = filter.map(str::to_lowercase).filter(|f| !f.is_empty());
    // Rows whose key or name contains the filter.
    fn pick<T: serde::Serialize>(items: &[T], text: impl Fn(&T) -> String, f: &Option<String>) -> Result<Vec<Value>, String> {
        items
            .iter()
            .filter(|i| f.as_ref().is_none_or(|f| text(i).to_lowercase().contains(f)))
            .map(to_json)
            .collect()
    }
    let rows = match what {
        "kits" => pick(&cat.kits, |k| format!("{} {}", k.path, k.name), &f)?,
        "pieces" => pick(&cat.pieces, |p| format!("{} {}", p.key, p.name), &f)?,
        "materials" => pick(&cat.materials, |m| format!("{} {}", m.key, m.name), &f)?,
        "specs" => pick(&cat.specs, |s| format!("{} {}", s.asset.id, s.asset.name), &f)?,
        "ui" => {
            let (rows, fonts, colors) = ui_rows(ctx)?;
            let rows = rows
                .into_iter()
                .filter(|r| f.as_ref().is_none_or(|f| r["name"].as_str().unwrap_or("").to_lowercase().contains(f)))
                .collect();
            let mut out = page(rows, offset, limit);
            out["fonts"] = fonts;
            out["colors"] = colors;
            return Ok(out);
        }
        _ => return Err(format!("«{what}» no existe: usa kits, pieces, materials, specs o ui")),
    };
    Ok(page(rows, offset, limit))
}

pub fn check(ctx: &Ctx, severity: Option<&str>, offset: usize, limit: usize) -> Result<Value, String> {
    let cat = load(ctx)?;
    let rows: Vec<Value> = cat
        .findings
        .iter()
        .map(to_json)
        .collect::<Result<_, _>>()?;
    let rows = rows
        .into_iter()
        .filter(|f| severity.is_none_or(|s| f["severity"] == s))
        .collect();
    let mut out = page(rows, offset, limit);
    out["counts"] = counts(&cat);
    Ok(out)
}

/// A node of the project tree with its direct children (the root when `node` is `None`).
pub fn tree(ctx: &Ctx, node: Option<&str>) -> Result<Value, String> {
    let cat = load(ctx)?;
    let config = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let t = kiln_core::tree::build(&ctx.project, &config, &cat);
    let key = node.unwrap_or(&t.root);
    let n = t.nodes.get(key).ok_or(format!("no hay nodo «{key}» en el árbol"))?;
    let children: Vec<&kiln_core::tree::TreeNode> = n.children.iter().filter_map(|c| t.nodes.get(c)).collect();
    Ok(json!({ "node": n, "children": children }))
}

pub fn spec_get(ctx: &Ctx, id: &str) -> Result<Value, String> {
    let cat = load(ctx)?;
    let file = super::spec_file(&ctx.project, &cat, id)?;
    let spec = cat.specs.iter().find(|s| s.asset.id == id).ok_or(format!("no hay ficha con id «{id}»"))?;
    let toml = std::fs::read_to_string(&file).map_err(|e| format!("no se pudo leer la ficha: {e}"))?;
    let findings: Vec<_> = cat.findings.iter().filter(|f| f.subject.contains(id)).collect();
    let mut out = json!({ "spec": spec, "file": rel(&ctx.project, &file), "toml": toml, "findings": findings });
    // what the last build and the last verification left, when there are any
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let dir = kiln_core::build::out_dir(&ctx.project, &cfg, id);
    for (key, name) in [("contract", "asset.json"), ("validation", "validation.json")] {
        let read = std::fs::read_to_string(dir.join(name)).ok();
        if let Some(v) = read.and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            out[key] = v;
        }
    }
    Ok(out)
}

/// What `GET /api/context` serves.
pub fn context_get(ctx: &Ctx) -> Result<Value, String> {
    let c = kiln_core::context::load(&ctx.context_file)?;
    Ok(json!({ "file": rel(&ctx.project, &ctx.context_file), "layers": ctx.layers, "context": c }))
}

pub fn content(ctx: &Ctx, set: Option<&str>) -> Result<Value, String> {
    let cat = load(ctx)?;
    let sets: Vec<_> = cat.content.iter().filter(|s| set.is_none_or(|n| s.name == n)).collect();
    if let (Some(n), true) = (set, sets.is_empty()) {
        return Err(format!("no hay conjunto de contenido «{n}»"));
    }
    to_json(&sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn at(sub: &str) -> Ctx {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(sub);
        super::super::ctx(&std::fs::canonicalize(dir).unwrap(), None).unwrap()
    }

    #[test]
    fn paging_reports_total_and_caps_the_limit() {
        let c = at("");
        let all = catalog(&c, "pieces", None, 0, 0).unwrap();
        let total = all["total"].as_u64().unwrap();
        assert!(total > 0);
        assert_eq!(all["items"].as_array().unwrap().len() as u64, total.min(50));
        let one = catalog(&c, "pieces", None, 1, 1).unwrap();
        assert_eq!((one["total"].as_u64().unwrap(), one["offset"].as_u64().unwrap()), (total, 1));
        assert_eq!(one["items"].as_array().unwrap().len(), 1.min(total as usize - 1));
        let big = catalog(&c, "pieces", None, 0, 100000).unwrap();
        assert!(big["items"].as_array().unwrap().len() <= 200);
    }

    #[test]
    fn filter_is_case_insensitive_and_unknown_what_errors() {
        let c = at("");
        let key = crate::load(&c.project, None).unwrap().specs[0].asset.id.to_uppercase();
        let r = catalog(&c, "specs", Some(&key), 0, 0).unwrap();
        assert!(r["total"].as_u64().unwrap() >= 1);
        assert_eq!(catalog(&c, "specs", Some("zzz-no-such"), 0, 0).unwrap()["total"], 0);
        assert!(catalog(&c, "bogus", None, 0, 0).unwrap_err().contains("kits"));
    }

    #[test]
    fn status_counts_match_the_catalog() {
        let c = at("");
        let cat = crate::load(&c.project, None).unwrap();
        let s = status(&c).unwrap();
        assert_eq!(s["specs"], cat.specs.len());
        assert_eq!(s["pieces"], cat.pieces.len());
        let f = &s["findings"];
        let sum = f["error"].as_u64().unwrap() + f["warn"].as_u64().unwrap() + f["info"].as_u64().unwrap();
        assert_eq!(sum as usize, cat.findings.len());
        assert!(!s["next"].as_str().unwrap().is_empty());
        assert_eq!(check(&c, None, 0, 0).unwrap()["total"], cat.findings.len());
    }

    #[test]
    fn tree_gives_a_node_and_its_children() {
        let c = at("");
        let r = tree(&c, None).unwrap();
        assert!(!r["children"].as_array().unwrap().is_empty());
        assert!(tree(&c, Some("nope:nope")).is_err());
    }

    #[test]
    fn spec_get_and_context_and_content() {
        let c = at("");
        let s = spec_get(&c, "hoist").unwrap();
        assert_eq!(s["spec"]["asset"]["id"], "hoist");
        assert!(s["toml"].as_str().unwrap().contains("hoist"));
        assert!(spec_get(&c, "no_such").is_err());
        assert!(context_get(&c).unwrap()["layers"].is_array());
        let w = content(&at("content"), Some("world")).unwrap();
        assert_eq!(w.as_array().unwrap().len(), 1);
        assert!(content(&at("content"), Some("zz")).is_err());
    }

    /// The text of a `kiln.toml` without its `[ui]`, `[ui.*]` and `[[ui.*]]` tables.
    fn without_ui(toml: &str) -> String {
        let mut skip = false;
        let mut out = String::new();
        for line in toml.lines() {
            let t = line.trim_start();
            if t.starts_with('[') {
                let name = t.trim_start_matches('[');
                skip = name.starts_with("ui]") || name.starts_with("ui.");
            }
            if !skip {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    /// A copy of the examples without `[ui]`.
    fn no_ui() -> (super::super::testing::Temp, Ctx) {
        let t = super::super::testing::examples("");
        let toml = std::fs::read_to_string(t.0.join("kiln.toml")).unwrap();
        std::fs::write(t.0.join("kiln.toml"), without_ui(&toml)).unwrap();
        let c = super::super::testing::ctx(&t);
        (t, c)
    }

    /// A copy of the examples with `[ui]` over the core fixture, a target file and two pinned criteria.
    fn with_ui(target: Option<&str>) -> (super::super::testing::Temp, Ctx) {
        let t = super::super::testing::examples("");
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kiln-core/tests/fixtures/ui/project");
        for f in ["dirty.tres", "dirty.gd", "dirty.tscn"] {
            std::fs::copy(fixtures.join(f), t.0.join(f)).unwrap();
        }
        let mut toml = without_ui(&std::fs::read_to_string(t.0.join("kiln.toml")).unwrap());
        toml.push_str("\n[ui]\ntheme = \"dirty.tres\"\nscripts = [\"dirty.gd\"]\nscenes = [\"dirty.tscn\"]\n");
        if let Some(path) = target {
            toml.push_str(&format!("target = \"{path}\"\n"));
        }
        std::fs::write(t.0.join("kiln.toml"), toml).unwrap();
        // the example already pins its own criteria; start from its context without them
        let ctx = std::fs::read_to_string(t.0.join("context.toml")).unwrap();
        let mut parts = ctx.split("\n[[claims]]");
        let mut ctx = parts.next().unwrap().to_string();
        for part in parts.filter(|p| !p.contains("text = \"C")) {
            ctx.push_str("\n[[claims]]");
            ctx.push_str(part);
        }
        for (id, n) in [("u1", "C1"), ("u2", "C2")] {
            ctx.push_str(&format!("\n[[claims]]\nid = \"{id}\"\nlayer = \"look\"\ntext = \"{n} · x\"\npinned = true\n"));
        }
        std::fs::write(t.0.join("context.toml"), ctx).unwrap();
        let rows = "[[criteria]]\nid = \"C1\"\nmarker = \"kiln:glyphs\"\n[[criteria]]\nid = \"C2\"\nmarker = \"po\"\nstatus = \"met\"\n";
        std::fs::write(t.0.join("ui_target.toml"), rows).unwrap();
        let c = super::super::testing::ctx(&t);
        (t, c)
    }

    #[test]
    fn status_omits_ui_target_without_ui_or_target() {
        let (_n, plain) = no_ui();
        assert!(status(&plain).unwrap().get("ui_target").is_none());
        let (_t, c) = with_ui(None);
        assert!(status(&c).unwrap().get("ui_target").is_none());
    }

    #[test]
    fn status_reports_the_ui_target_and_survives_an_unreadable_file() {
        let (_t, c) = with_ui(Some("ui_target.toml"));
        let u = &status(&c).unwrap()["ui_target"];
        // C1 is `kiln:glyphs`: the theme's font is not in the test project, so the rule could not
        // measure (`ui-files`) and the criterion is unmet. Not reading a file is not a pass.
        assert_eq!((&u["met"], &u["total"], &u["percent"]), (&json!(1), &json!(2), &json!(50)));
        assert_eq!(u["open"][0]["id"], "C1");
        assert_eq!(u["open"][0]["status"], "unmet");
        let (_t, c) = with_ui(Some("nope.toml"));
        assert!(status(&c).unwrap()["ui_target"]["error"].as_str().unwrap().contains("nope.toml"));
    }

    #[test]
    fn a_target_or_theme_outside_the_project_is_an_error_and_is_not_read() {
        let (t, c) = with_ui(Some("../outside.toml"));
        let secret = t.0.parent().unwrap().join("outside.toml");
        std::fs::write(&secret, "[[criteria]]\nid = \"C1\"\nmarker = \"SECRETMARK\"\n").unwrap();
        let u = status(&c).unwrap()["ui_target"].to_string();
        std::fs::remove_file(&secret).unwrap();
        assert!(u.contains("error") && u.contains("sale del proyecto"), "{u}");
        assert!(!u.contains("SECRETMARK") && !u.contains("met\":"), "{u}");
        let (t, c) = with_ui(None);
        let toml = std::fs::read_to_string(t.0.join("kiln.toml")).unwrap().replace("dirty.tres", "../dirty.tres");
        std::fs::write(t.0.join("kiln.toml"), toml).unwrap();
        let e = catalog(&c, "ui", None, 0, 0).unwrap_err();
        assert!(e.contains("sale del proyecto"), "{e}");
    }

    #[test]
    fn catalog_ui_lists_the_variations_and_whether_they_are_used() {
        let (_t, c) = with_ui(None);
        let r = catalog(&c, "ui", None, 0, 0).unwrap();
        let rows = r["items"].as_array().unwrap();
        let row = |n: &str| rows.iter().find(|r| r["name"] == n).unwrap();
        assert_eq!(row("Faint")["font_color"], "#808080");
        assert_eq!(row("Orphan")["used"], false);
        assert_eq!(row("Plain")["used"], true);
        assert_eq!(row("Faint")["uses"], json!(["dirty.gd:5"]));
        assert_eq!(row("Plain")["uses"], json!(["dirty.gd:6", "dirty.tscn:6"]));
        assert_eq!(row("Orphan")["uses"], json!([]));
        assert_eq!(r["fonts"], json!([{ "id": "1_font", "path": "res://IBMPlexMono-Regular.ttf" }]));
        let colors = r["colors"].as_array().unwrap();
        let faint = colors.iter().find(|c| c["variation"] == "Faint" && c["key"] == "font_color").unwrap();
        assert_eq!((&faint["hex"], &faint["alpha"]), (&json!("#808080"), &json!(1.0)));
        assert_eq!(colors.len(), 3, "{colors:?}");
        let (_n, plain) = no_ui();
        assert!(catalog(&plain, "ui", None, 0, 0).unwrap_err().contains("[ui]"));
    }
}
