//! What a beginner needs to reach a first asset by talking: the essential
//! settings of `kiln.toml`, a preview to look at, the next step from where
//! the project is, and an example game made in one call.

use crate::agent::Ctx;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

// --- Settings -------------------------------------------------------------

/// The settings `config_set` may change. Never a program (`[tools]
/// blender`, `godot`): those decide what Kiln runs, and only the person
/// names them (`kiln setup`). The folders must stay inside the project.
pub const SETTINGS: [(&str, &str); 7] = [
    ("project.name", "nombre del juego (texto)"),
    ("rules.licenses", "licencias admitidas para lo que viene de fuera (lista de textos, p. ej. [\"CC0\", \"CC-BY\"])"),
    ("rules.max_tris", "triángulos como mucho por pieza (entero mayor que 0)"),
    ("rules.require_use", "si cada pieza debe decir para qué se usa (true o false)"),
    ("publish.dir", "carpeta, dentro del proyecto, donde se publica cada asset para el juego"),
    ("tools.godot_project", "carpeta, dentro del proyecto, que tiene project.godot"),
    ("tools.timeout_s", "segundos que puede tardar Blender o Godot (de 10 a 7200)"),
];

fn value_of(cfg: &kiln_core::Config, key: &str) -> Value {
    match key {
        "project.name" => json!(cfg.project.name),
        "rules.licenses" => json!(cfg.rules.licenses),
        "rules.max_tris" => json!(cfg.rules.max_tris),
        "rules.require_use" => json!(cfg.rules.require_use),
        "publish.dir" => json!(cfg.publish.dir),
        "tools.godot_project" => json!(cfg.tools.godot_project),
        "tools.timeout_s" => json!(cfg.tools.timeout_s),
        _ => Value::Null,
    }
}

fn kiln_toml(ctx: &Ctx) -> PathBuf {
    ctx.config.clone().unwrap_or_else(|| ctx.project.join(kiln_core::config::FILE))
}

pub fn config_get(ctx: &Ctx) -> Result<Value, String> {
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let settings: Vec<Value> = SETTINGS
        .iter()
        .map(|(k, what)| json!({ "key": k, "value": value_of(&cfg, k), "what": what }))
        .collect();
    Ok(json!({ "file": super::rel(&ctx.project, &kiln_toml(ctx)), "settings": settings }))
}

/// A folder of the project as written: relative, without `..`.
fn inside(v: &str) -> Result<(), String> {
    let p = Path::new(v);
    if v.trim().is_empty() || p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("«{v}» debe ser una carpeta dentro del proyecto, relativa y sin «..»"));
    }
    Ok(())
}

/// The TOML text of `value` for `key`, checked.
fn literal(key: &str, value: &Value) -> Result<String, String> {
    let bad = |what: &str| Err(format!("{key} debe ser {what}"));
    let text = |v: &Value| v.as_str().map(|s| toml::Value::String(s.to_string()).to_string());
    match key {
        "project.name" => match value.as_str().map(str::trim) {
            Some(s) if !s.is_empty() && s.chars().count() <= 200 => Ok(toml::Value::String(s.into()).to_string()),
            _ => bad("un texto de 1 a 200 caracteres"),
        },
        "rules.licenses" => {
            let Some(a) = value.as_array().filter(|a| !a.is_empty() && a.len() <= 20) else {
                return bad("una lista de 1 a 20 textos");
            };
            let items: Option<Vec<String>> = a
                .iter()
                .map(|v| v.as_str().filter(|s| !s.trim().is_empty() && s.len() <= 40).and(text(v)))
                .collect();
            match items {
                Some(i) => Ok(format!("[{}]", i.join(", "))),
                None => bad("una lista de textos cortos"),
            }
        }
        "rules.max_tris" => match value.as_u64() {
            Some(n) if n > 0 && n <= 10_000_000 => Ok(n.to_string()),
            _ => bad("un entero entre 1 y 10000000"),
        },
        "rules.require_use" => match value.as_bool() {
            Some(b) => Ok(b.to_string()),
            None => bad("true o false"),
        },
        "tools.timeout_s" => match value.as_u64() {
            Some(n) if (10..=7200).contains(&n) => Ok(n.to_string()),
            _ => bad("un entero entre 10 y 7200"),
        },
        "publish.dir" | "tools.godot_project" => {
            let Some(s) = value.as_str() else { return bad("un texto") };
            inside(s)?;
            Ok(toml::Value::String(s.trim().into()).to_string())
        }
        _ => Err(format!(
            "«{key}» no se cambia por aquí. Se pueden cambiar: {}. Los programas los elige la persona con `kiln setup`.",
            SETTINGS.map(|(k, _)| k).join(", ")
        )),
    }
}

/// Puts `key = literal` in `[section]` of `text`, keeping every other line
/// (and the comments) as it is.
pub fn set_line(text: &str, section: &str, key: &str, literal: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let header = |l: &str| -> Option<String> {
        let t = l.split('#').next().unwrap_or("").trim();
        (t.starts_with('[') && t.ends_with(']')).then(|| t.trim_matches(|c| c == '[' || c == ']').trim().to_string())
    };
    let mut current: Option<String> = None;
    let mut header_at: Option<usize> = None;
    let mut found: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(h) = header(l) {
            // `[[x]]` names an array of tables: never the section looked for.
            current = (!l.trim_start().starts_with("[[")).then_some(h);
            if current.as_deref() == Some(section) {
                header_at = Some(i);
            }
            i += 1;
            continue;
        }
        let is_key = l.split_once('=').is_some_and(|(k, _)| k.trim() == key);
        if current.as_deref() == Some(section) && is_key {
            // A value may run over several lines: an array until its brackets close.
            let mut end = i;
            let depth = |s: &str| s.chars().fold(0i32, |d, c| d + (c == '[') as i32 - (c == ']') as i32);
            let mut open = depth(l.split_once('=').map(|(_, v)| v).unwrap_or(""));
            while open > 0 && end + 1 < lines.len() {
                end += 1;
                open += depth(lines[end]);
            }
            found = Some((i, end));
            break;
        }
        i += 1;
    }
    let line = format!("{key} = {literal}");
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    match (found, header_at) {
        (Some((a, b)), _) => {
            out.splice(a..=b, [line]);
        }
        (None, Some(h)) => out.insert(h + 1, line),
        (None, None) => {
            if out.last().is_some_and(|l| !l.trim().is_empty()) {
                out.push(String::new());
            }
            out.push(format!("[{section}]"));
            out.push(line);
        }
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// Changes one setting of `kiln.toml`; the file must still load after it.
pub fn config_set(ctx: &Ctx, key: &str, value: &Value) -> Result<Value, String> {
    let lit = literal(key, value)?;
    let (section, name) = key.split_once('.').ok_or("clave sin sección")?;
    let file = kiln_toml(ctx);
    let before = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let after = set_line(&before, section, name, &lit);
    let cfg = kiln_core::Config::parse(&after).map_err(|e| format!("kiln.toml no quedaría válido: {e}"))?;
    let tmp = file.with_extension(format!("toml.{}", std::process::id()));
    std::fs::write(&tmp, &after).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(json!({ "key": key, "value": value_of(&cfg, key), "file": super::rel(&ctx.project, &file) }))
}

// --- Palette ------------------------------------------------------------

/// Adds a plain colour to the palette: a `[[materials]]` row at the end of
/// the first table of `[materials] tables`, which must lie in the project.
pub fn palette_add(ctx: &Ctx, id: &str, name: &str, color: [f64; 3]) -> Result<Value, String> {
    super::check_id("el material", id)?;
    super::check_len("el nombre", name, 100)?;
    if color.iter().any(|c| !c.is_finite() || !(0.0..=1.0).contains(c)) {
        return Err("color debe ser [r, g, b] con cada valor entre 0 y 1".into());
    }
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let cat = super::read::load(ctx)?;
    if cat.materials.iter().any(|m| m.key == id) {
        return Err(format!("la paleta ya tiene «{id}»"));
    }
    let table = cfg
        .materials
        .tables
        .iter()
        .find(|t| !t.contains('*'))
        .ok_or("kiln.toml no tiene ninguna tabla de materiales en [materials] tables")?;
    inside(table)?;
    let file = ctx.project.join(table);
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let round = |v: f64| (v * 1000.).round() / 1000.;
    let row = format!(
        "{}[[materials]]\nid = {}\nname = {}\ncolor = [{}, {}, {}]\n",
        if before.is_empty() || before.ends_with("\n\n") { "" } else if before.ends_with('\n') { "\n" } else { "\n\n" },
        toml::Value::String(id.into()),
        toml::Value::String(name.trim().into()),
        round(color[0]),
        round(color[1]),
        round(color[2]),
    );
    let after = format!("{before}{row}");
    toml::from_str::<toml::Table>(&after).map_err(|e| format!("{table} no quedaría válido: {e}"))?;
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, after).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(json!({ "added": id, "file": table, "palette": cat.materials.len() + 1 }))
}

// --- Preview --------------------------------------------------------------

/// The GLB to look at: the build of an asset of parts, or the generated
/// mesh of a recipe (its seed, or the one of the spec).
fn model(ctx: &Ctx, id: &str, seed: Option<u32>) -> Result<(PathBuf, PathBuf), String> {
    let cat = super::read::load(ctx)?;
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let spec = cat.specs.iter().find(|s| s.asset.id == id).ok_or(format!("no hay ficha con id «{id}»"))?;
    if let Some(g) = &spec.generator {
        let seed = seed.unwrap_or(g.seed);
        let dir = ctx.project.join(&cfg.tools.cache).join("generated").join(id);
        let glb = dir.join(format!("{seed}.glb"));
        if !glb.is_file() {
            return Err(format!("«{id}» aún no está generado con la semilla {seed}: lánzalo con `generate` y espera a que termine el trabajo"));
        }
        return Ok((glb, dir.join(format!("{seed}.preview.png"))));
    }
    let dir = kiln_core::build::out_dir(&ctx.project, &cfg, id);
    let glb = dir.join(format!("{id}.glb"));
    if !glb.is_file() {
        return Err(format!("«{id}» aún no está montado: lánzalo con `build` y espera a que termine el trabajo"));
    }
    Ok((glb, dir.join("preview.png")))
}

/// Draws the preview sheet of an asset, writes it next to the model and
/// returns what it shows with the PNG.
pub fn preview(ctx: &Ctx, id: &str, seed: Option<u32>) -> Result<(Value, Vec<u8>), String> {
    super::check_id("el asset", id)?;
    let (glb, png) = model(ctx, id, seed)?;
    let bytes = std::fs::read(&glb).map_err(|e| format!("{}: {e}", glb.display()))?;
    let tris = kiln_core::preview::triangles(&bytes)?;
    if tris.is_empty() {
        return Err(format!("{} no tiene triángulos que dibujar", super::rel(&ctx.project, &glb)));
    }
    let sheet = kiln_core::preview::sheet(&tris, 384);
    std::fs::write(&png, &sheet.png).map_err(|e| format!("{}: {e}", png.display()))?;
    let info = json!({
        "asset": id,
        "model": super::rel(&ctx.project, &glb),
        "image": png.display().to_string(),
        "views": kiln_core::preview::VIEWS,
        "layout": "2x2: arriba frente y perfil, abajo planta y tres cuartos",
        "triangles": sheet.triangles,
        "size_m": sheet.size_m.map(|v| (v * 1000.).round() / 1000.),
        "note": "dibujo plano de Kiln con el color de cada material, sin texturas ni luces del juego; para girarlo, `open_viewer`",
    });
    Ok((info, sheet.png))
}

// --- The next step --------------------------------------------------------

/// Where the first asset of the project is on the way to the game, and
/// what to do next, in the words of a beginner's guide.
pub fn journey(ctx: &Ctx, cat: &kiln_core::Catalog, cfg: &kiln_core::Config) -> Value {
    let publish = cfg.publish.dir.clone();
    let steps: Vec<Value> = cat
        .specs
        .iter()
        .map(|s| {
            let id = &s.asset.id;
            let generated = s.generator.is_some();
            let built = if generated {
                s.generator.as_ref().is_some_and(|g| {
                    ctx.project.join(&cfg.tools.cache).join("generated").join(id).join(format!("{}.glb", g.seed)).is_file()
                })
            } else {
                kiln_core::build::out_dir(&ctx.project, cfg, id).join(format!("{id}.glb")).is_file()
            };
            let validation: Option<Value> = std::fs::read(kiln_core::build::out_dir(&ctx.project, cfg, id).join("validation.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
            let verified = validation.as_ref().map(|v| v["status"] == "passed");
            let published = publish.as_ref().is_some_and(|d| ctx.project.join(d).join(id).join("asset.json").is_file());
            json!({ "asset": id, "generated": generated, "built": built, "verified": verified, "published": published })
        })
        .collect();
    json!(steps)
}

/// The next step for a beginner, or `None` when another rule comes first.
pub fn next(cfg: &kiln_core::Config, steps: &Value) -> Option<String> {
    let tools = kiln_core::tools::present(cfg);
    let missing = |n: &str| tools.iter().any(|t| t.name == n && !t.found);
    let steps = steps.as_array()?;
    if steps.is_empty() {
        return Some("Aún no hay ningún asset: pregunta a la persona qué quiere crear y sigue el prompt `kiln-asset` (o `guide` con `asset`).".into());
    }
    for s in steps {
        let id = s["asset"].as_str().unwrap_or_default();
        let generated = s["generated"] == true;
        if s["built"] != true {
            let (tool, step) = if generated { ("godot", "generate") } else { ("blender", "build") };
            if missing(tool) {
                return Some(format!("Para «{id}» hace falta {tool}: llama a `setup` para encontrarlo o para darle a la persona el enlace de descarga."));
            }
            return Some(format!("«{id}» está listo para {}: lanza `{step}`, sigue el trabajo con `job` y enséñaselo con `preview`.", if generated { "generarse" } else { "montarse" }));
        }
        if s["verified"].is_null() {
            if missing("godot") {
                return Some(format!("«{id}» ya tiene modelo: enséñaselo con `preview`. Para verificarlo hace falta godot: llama a `setup`."));
            }
            return Some(format!("«{id}» ya tiene modelo: enséñaselo con `preview` y, si le gusta, verifícalo con `verify`."));
        }
        if s["verified"] == false {
            return Some(format!("La verificación de «{id}» falló: mira `spec_get` (validation), corrige la ficha y vuelve a montar."));
        }
        if s["published"] != true {
            if cfg.publish.dir.is_none() {
                return Some(format!("«{id}» está verificado. Para publicarlo falta la carpeta del juego: pregúntasela a la persona y guárdala con `config_set` (publish.dir)."));
            }
            return Some(format!("«{id}» está verificado: publícalo con `publish` para que el juego lo use."));
        }
    }
    None
}

// --- An example game ------------------------------------------------------

const EXAMPLE_MATERIALS: &str = "# La paleta del juego: una tabla [[materials]] por material, con su `id` y su color (RGB de 0 a 1).\n\
[[materials]]\nid = \"madera\"\nname = \"Madera clara\"\ncolor = [0.78, 0.6, 0.38]\n\n\
[[materials]]\nid = \"madera_oscura\"\nname = \"Madera oscura\"\ncolor = [0.42, 0.28, 0.16]\n\n\
[[materials]]\nid = \"hierro\"\nname = \"Hierro\"\ncolor = [0.22, 0.23, 0.26]\n\n\
[[materials]]\nid = \"piedra\"\nname = \"Piedra\"\ncolor = [0.56, 0.55, 0.52]\n\n\
[[materials]]\nid = \"cristal\"\nname = \"Cristal con luz\"\ncolor = [1.0, 0.86, 0.45]\n\n\
[[materials]]\nid = \"tela_roja\"\nname = \"Tela roja\"\ncolor = [0.7, 0.16, 0.14]\n\n\
[[materials]]\nid = \"hoja\"\nname = \"Hoja verde\"\ncolor = [0.3, 0.55, 0.25]\n";

/// A street lamp made of shapes: no kit, no download, only Blender.
const EXAMPLE_SPEC: &str = "# Ficha de ejemplo: un farol hecho solo de formas (`kind = \"model\"`).\n\
# Pídele a Claude que lo monte (`build`), que te lo enseñe (`preview`) y que lo cambie a tu gusto.\n\
[asset]\nid = \"farol\"\nname = \"Farol de calle\"\nasked = \"un farol de calle\"\n\
where = \"en las calles del pueblo, junto a los caminos\"\nbudget_tris = 2000\nstatus = \"layout\"\n\n\
[[parts]]\nid = \"farol\"\nwhat = \"base de piedra, poste de hierro, farol con cristal y tejadillo\"\n\
source = { kind = \"model\" }\nmaterials = { Base = \"piedra\", Hierro = \"hierro\", Luz = \"cristal\" }\nat = [[0, 0, 0]]\n\n\
[[parts.shapes]]\nzone = \"Base\"\nbox = [[-0.25, 0, -0.25], [0.25, 0.18, 0.25]]\n\n\
[[parts.shapes]]\nzone = \"Hierro\"\ntube = { from = [0, 0.18, 0], to = [0, 2.3, 0], radius = 0.05, segments = 12 }\n\n\
[[parts.shapes]]\nzone = \"Hierro\"\nbox = [[-0.17, 2.3, -0.17], [0.17, 2.36, 0.17]]\n\n\
[[parts.shapes]]\nzone = \"Luz\"\nbox = [[-0.13, 2.36, -0.13], [0.13, 2.7, 0.13]]\n\n\
[[parts.shapes]]\nzone = \"Hierro\"\ntube = { from = [0, 2.7, 0], to = [0, 2.92, 0], radius = 0.24, radius_to = 0.03, segments = 4 }\n";

/// The answers of the example to the basic questions of a new game.
pub fn example_game(name: &str) -> kiln_core::new_game::NewGame {
    kiln_core::new_game::NewGame {
        name: name.to_string(),
        basics: [
            ("camera", "tercera persona, por encima del hombro"),
            ("look", "estilizado, de pocos polígonos y colores planos"),
            ("height", "1,80 m"),
            ("licenses", "solo CC0"),
            ("tris", "20000"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into_iter()
        .collect(),
        ..Default::default()
    }
}

/// Writes the example game in an empty folder: a new game with its answers,
/// a palette of seven materials and the spec of a street lamp ready to build.
pub fn example(dir: &Path, name: &str) -> Result<(Vec<String>, Ctx), String> {
    let (mut files, _) = super::new_game::create(dir, example_game(name))?;
    std::fs::write(dir.join("kiln/materials.toml"), EXAMPLE_MATERIALS).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("kiln/specs/farol.toml"), EXAMPLE_SPEC).map_err(|e| e.to_string())?;
    files.push("kiln/specs/farol.toml".into());
    Ok((files, super::ctx(dir, None)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_line_keeps_comments_and_other_lines() {
        let t = "# top\n[project]\nname = \"a\" # the name\n\n[rules]\nlicenses = [\n  \"CC0\",\n]\nmax_tris = 5\n\n[[levels]]\nmax_tris = 9\n";
        let s = set_line(t, "rules", "licenses", "[\"CC0\", \"CC-BY\"]");
        assert_eq!(s, "# top\n[project]\nname = \"a\" # the name\n\n[rules]\nlicenses = [\"CC0\", \"CC-BY\"]\nmax_tris = 5\n\n[[levels]]\nmax_tris = 9\n");
        let s = set_line(t, "rules", "max_tris", "7");
        assert!(s.contains("max_tris = 7\n\n[[levels]]\nmax_tris = 9"), "{s}");
        let s = set_line(t, "rules", "require_use", "true");
        assert!(s.contains("[rules]\nrequire_use = true\nlicenses"), "{s}");
        let s = set_line(t, "publish", "dir", "\"juego/assets\"");
        assert!(s.ends_with("max_tris = 9\n\n[publish]\ndir = \"juego/assets\"\n"), "{s}");
    }

    #[test]
    fn literal_refuses_programs_and_folders_outside() {
        assert!(literal("tools.blender", &json!("/tmp/evil")).unwrap_err().contains("kiln setup"));
        assert!(literal("publish.dir", &json!("../fuera")).is_err());
        assert!(literal("publish.dir", &json!("/abs")).is_err());
        assert_eq!(literal("publish.dir", &json!("godot/assets")).unwrap(), "\"godot/assets\"");
        assert!(literal("rules.max_tris", &json!(0)).is_err());
        assert_eq!(literal("rules.licenses", &json!(["CC0"])).unwrap(), "[\"CC0\"]");
        assert!(literal("rules.licenses", &json!("CC0")).is_err());
        assert_eq!(literal("project.name", &json!("  Faro ")).unwrap(), "\"Faro\"");
    }

    fn temp() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!("kiln-start-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_example_loads_and_its_lamp_is_ready_to_build() {
        let d = temp();
        let (files, ctx) = example(&d, "Mi primer juego").unwrap();
        assert!(files.contains(&"kiln/specs/farol.toml".to_string()));
        let cat = super::super::read::load(&ctx).unwrap();
        assert_eq!(cat.materials.len(), 7);
        let lamp = cat.specs.iter().find(|s| s.asset.id == "farol").unwrap();
        assert_eq!(lamp.parts[0].source.kind, "model");
        let errors: Vec<_> = cat.findings.iter().filter(|f| f.severity == kiln_core::Severity::Error).collect();
        assert!(errors.is_empty(), "{errors:?}");
        let cfg = crate::production::config(&d, None).unwrap();
        assert!(cfg.publish.dir.is_some(), "a new game can publish");
        let steps = journey(&ctx, &cat, &cfg);
        assert_eq!(steps[0]["built"], false);
        let n = next(&cfg, &steps).unwrap();
        assert!(n.contains("farol") && (n.contains("`build`") || n.contains("`setup`")), "{n}");

        // Settings: read, change, and the file still has its comments.
        let v = config_get(&ctx).unwrap();
        assert_eq!(v["settings"][0]["value"], "Mi primer juego");
        config_set(&ctx, "rules.max_tris", &json!(5000)).unwrap();
        config_set(&ctx, "publish.dir", &json!("juego/assets")).unwrap();
        let cfg = crate::production::config(&d, None).unwrap();
        assert_eq!((cfg.rules.max_tris, cfg.publish.dir.as_deref()), (5000, Some("juego/assets")));
        assert!(config_set(&ctx, "tools.godot", &json!("/bin/sh")).is_err());

        // A colour for the palette.
        palette_add(&ctx, "latón", "x", [0.5; 3]).unwrap_err();
        palette_add(&ctx, "laton", "Latón", [0.8, 0.62, 0.25]).unwrap();
        assert!(palette_add(&ctx, "laton", "otra vez", [0.1; 3]).is_err());
        assert!(palette_add(&ctx, "neon", "Neón", [2.0, 0.0, 0.0]).is_err());
        let cat = super::super::read::load(&ctx).unwrap();
        assert_eq!(cat.materials.len(), 8);
        let _ = std::fs::remove_dir_all(&d);
    }
}
