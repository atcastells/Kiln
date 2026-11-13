//! Production: Blender assembles an asset from its spec, Godot measures the
//! pieces only it can read. Both run without a window; each call returns the
//! lines it has to say, for the terminal or for a job of the page.

use kiln_core::{Config, build, godot};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const PAINT: &str = include_str!("../blender/paint.py");
const SPLIT: &str = include_str!("../blender/split.py");
const REPAINT: &str = include_str!("../blender/repaint.py");
const ASSEMBLE: &str = include_str!("../blender/assemble.py");
const MEASURE: &str = include_str!("../godot/measure.gd");

pub fn config(project: &Path, config: Option<&Path>) -> Result<Config, String> {
    match config {
        Some(c) => Config::load_file(c),
        None => Config::load(project),
    }
}

/// Assembles the asset `id` in Blender; returns what happened, line by line.
pub fn build_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let plan = build::plan(project, &cfg, &cat, spec);
    let dir = build::out_dir(project, &cfg, id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let plan_file = dir.join("plan.json");
    let script = dir.join("assemble.py");
    std::fs::write(
        &plan_file,
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(&script, format!("{PAINT}\n{SPLIT}\n{ASSEMBLE}")).map_err(|e| e.to_string())?;
    say(format!(
        "Plan de montaje: {} copias de {} piezas",
        plan.items.len(),
        spec.parts.len()
    ));
    if !plan.auto_layout.is_empty() {
        say(format!(
            "Sin posición en la ficha, puestas en fila: {}",
            plan.auto_layout.join(", ")
        ));
    }
    for i in plan.items.iter().filter(|i| i.todo.is_some()) {
        say(format!("{}: {}", i.name, i.todo.as_deref().unwrap_or("")));
    }
    say(format!("Montando en Blender ({})", cfg.tools.blender));
    let text = blender(&cfg, &script, &plan_file, Path::new(&plan.glb))?;
    if let Some(l) = text.lines().find(|l| l.starts_with("kiln: built")) {
        say(l.trim_start_matches("kiln: ").to_string());
    }
    link_godot(project, &cfg, Path::new(&plan.glb), Path::new(&plan.materials), &mut say);
    // the variants are the same assembly: bring them up to date with it
    for v in spec.variants.iter().filter(|v| kiln_core::spec::plain_id(&v.id)) {
        paint_asset(project, config_file, id, Some(&v.id), &mut say)?;
    }
    Ok(PathBuf::from(plan.glb))
}

/// Runs a Kiln script in Blender on one JSON file; `made` must exist after.
fn blender(cfg: &Config, script: &Path, job: &Path, made: &Path) -> Result<String, String> {
    let mut cmd = Command::new(&cfg.tools.blender);
    if is_python(&cfg.tools.blender) {
        cmd.arg(script).arg(job);
    } else {
        cmd.args(["--background", "--factory-startup", "--python"])
            .arg(script)
            .arg("--")
            .arg(job);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("no se puede lanzar {}: {e}", cfg.tools.blender))?;
    if !out.status.success() || !made.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(6).collect();
        return Err(format!(
            "Blender falló: {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The file names of an asset or of one of its colour variants.
fn stem(id: &str, variant: Option<&str>) -> String {
    match variant {
        Some(v) => format!("{id}--{v}"),
        None => id.to_string(),
    }
}

/// Wears the materials of a built asset again from its spec, without
/// assembling: the asset itself, or one colour variant as its own GLB.
pub fn paint_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    variant: Option<&str>,
    mut say: impl FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    if let Some(v) = variant
        && !(kiln_core::spec::plain_id(v) && spec.variants.iter().any(|x| x.id == v))
    {
        return Err(format!("la ficha no tiene la variante «{v}»"));
    }
    let dir = build::out_dir(project, &cfg, id);
    let blend = build::blend_file(&dir, id);
    let names: Vec<String> = std::fs::read_to_string(dir.join("materials.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<BTreeMap<String, Option<String>>>(&t).ok())
        .filter(|_| blend.exists())
        .ok_or("el asset no está montado: monta primero")?
        .into_keys()
        .collect();
    let name = stem(id, variant);
    let glb = dir.join(format!("{name}.glb"));
    let materials = dir.join(match variant {
        Some(v) => format!("materials--{v}.json"),
        None => "materials.json".into(),
    });
    let job = serde_json::json!({
        "blend": blend, "glb": glb, "materials": materials, "save": variant.is_none(),
        "paints": build::paints(project, &cat, spec, &names, variant),
    });
    let job_file = dir.join("repaint.json");
    let script = dir.join("repaint.py");
    std::fs::write(&job_file, job.to_string()).map_err(|e| e.to_string())?;
    std::fs::write(&script, format!("{PAINT}\n{REPAINT}")).map_err(|e| e.to_string())?;
    say(format!("Aplicando materiales en Blender ({})", cfg.tools.blender));
    let _ = std::fs::remove_file(&materials);
    let text = blender(&cfg, &script, &job_file, &materials)?;
    if let Some(l) = text.lines().find(|l| l.starts_with("kiln: painted")) {
        say(l.trim_start_matches("kiln: ").to_string());
    }
    link_godot(project, &cfg, &glb, &materials, &mut say);
    Ok(glb)
}

/// Links every `part/zone` material of a built GLB to the Godot material of
/// its palette key, in the GLB's `.import`. Says why when it cannot.
fn link_godot(project: &Path, cfg: &Config, glb: &Path, materials: &Path, say: &mut dyn FnMut(String)) {
    let (Some(template), Some(gp)) = (&cfg.materials.godot, &cfg.tools.godot_project) else {
        return;
    };
    let mut run = || -> Result<usize, String> {
        let gp = std::fs::canonicalize(project.join(gp)).map_err(|e| e.to_string())?;
        let glb = std::fs::canonicalize(glb).map_err(|e| e.to_string())?;
        if !glb.starts_with(&gp) {
            return Err("el GLB montado está fuera del proyecto de Godot ([tools] build)".into());
        }
        let worn: BTreeMap<String, Option<String>> = std::fs::read_to_string(materials)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))?;
        let import = PathBuf::from(format!("{}.import", glb.display()));
        if !import.exists() {
            say(format!("Godot importa el GLB por primera vez ({})", cfg.tools.godot));
            let _ = Command::new(&cfg.tools.godot)
                .args(["--headless", "--path"])
                .arg(&gp)
                .arg("--import")
                .output()
                .map_err(|e| format!("no se puede lanzar {}: {e}", cfg.tools.godot))?;
        }
        let text = std::fs::read_to_string(&import)
            .map_err(|_| "Godot no ha escrito el .import del GLB".to_string())?;
        let links: BTreeMap<String, Option<String>> = worn
            .into_iter()
            .map(|(n, k)| (n, k.map(|k| godot::material_path(template, &k))))
            .collect();
        let n = links.values().flatten().count();
        std::fs::write(&import, godot::link_materials(&text, &links)?).map_err(|e| e.to_string())?;
        Ok(n)
    };
    match run() {
        Ok(n) => say(format!("Godot: {n} materiales vinculados en el .import")),
        Err(e) => say(format!("Godot: materiales sin vincular: {e}")),
    }
}

fn is_python(tool: &str) -> bool {
    Path::new(tool)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("python"))
}

/// Has Godot measure and export every piece only it can read.
pub fn measure(
    project: &Path,
    config_file: Option<&Path>,
    mut say: impl FnMut(String),
) -> Result<usize, String> {
    let cfg = config(project, config_file)?;
    let gp = cfg
        .tools
        .godot_project
        .as_ref()
        .ok_or("falta [tools] godot_project en kiln.toml (la carpeta con project.godot)")?;
    let cat = kiln_core::catalog::index(project, &cfg);
    let reqs = godot::requests(project, &cfg, &cat);
    if reqs.is_empty() {
        say("No hay piezas que solo lea Godot".into());
        return Ok(0);
    }
    let out = godot::cache_file(project, &cfg);
    let dir = out.parent().unwrap_or(project).to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let req_file = dir.join("requests.json");
    let script = dir.join("measure.gd");
    std::fs::write(
        &req_file,
        serde_json::to_string_pretty(&reqs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(&script, MEASURE).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&out);
    say(format!(
        "Godot mide {} piezas ({})",
        reqs.len(),
        cfg.tools.godot
    ));
    let abs = |p: &Path| std::fs::canonicalize(p).unwrap_or(p.to_path_buf());
    let status = Command::new(&cfg.tools.godot)
        .arg("--headless")
        .arg("--path")
        .arg(project.join(gp))
        .arg("--script")
        .arg(abs(&script))
        .arg("--")
        .arg(abs(&req_file))
        .arg(abs(&dir).join("pieces.json"))
        .output()
        .map_err(|e| format!("no se puede lanzar {}: {e}", cfg.tools.godot))?;
    if !out.exists() {
        let err = String::from_utf8_lossy(&status.stderr);
        return Err(format!(
            "Godot no escribió las medidas: {}",
            err.lines().last().unwrap_or("")
        ));
    }
    say(format!("Medidas en {}", out.display()));
    Ok(reqs.len())
}
