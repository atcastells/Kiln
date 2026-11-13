//! Production: Blender assembles an asset from its spec, Godot measures the
//! pieces only it can read. Both run without a window; each call returns the
//! lines it has to say, for the terminal or for a job of the page.

use kiln_core::{Config, build, godot};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    std::fs::write(&script, ASSEMBLE).map_err(|e| e.to_string())?;
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
    let mut cmd = Command::new(&cfg.tools.blender);
    if is_python(&cfg.tools.blender) {
        cmd.arg(&script).arg(&plan_file);
    } else {
        cmd.args(["--background", "--factory-startup", "--python"])
            .arg(&script)
            .arg("--")
            .arg(&plan_file);
    }
    say(format!("Montando en Blender ({})", cfg.tools.blender));
    let out = cmd
        .output()
        .map_err(|e| format!("no se puede lanzar {}: {e}", cfg.tools.blender))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !Path::new(&plan.glb).exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(6).collect();
        return Err(format!(
            "Blender falló: {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        ));
    }
    if let Some(l) = text.lines().find(|l| l.starts_with("kiln: built")) {
        say(l.trim_start_matches("kiln: ").to_string());
    }
    Ok(PathBuf::from(plan.glb))
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
