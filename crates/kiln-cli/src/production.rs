//! Production: Blender assembles an asset from its spec, Godot measures the
//! pieces only it can read. Both run without a window; each call returns the
//! lines it has to say, for the terminal or for a job of the page.

use kiln_core::{Config, build, godot};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PAINT: &str = include_str!("../blender/paint.py");
const SPLIT: &str = include_str!("../blender/split.py");
const REPAINT: &str = include_str!("../blender/repaint.py");
const ASSEMBLE: &str = include_str!("../blender/assemble.py");
const MEASURE: &str = include_str!("../godot/measure.gd");
const EXPORT_PIECE: &str = include_str!("../blender/export_piece.py");

pub fn config(project: &Path, config: Option<&Path>) -> Result<Config, String> {
    match config {
        Some(c) => Config::load_file(c),
        None => Config::load(project),
    }
}

/// Assembles the asset `id` in Blender; returns what happened, line by line.
/// The assets it is made of go first, from the bottom up, when they have not
/// been assembled or are to be assembled again.
pub fn build_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    crate::ops::refuse_generated(&cat.specs, id)?;
    let order = kiln_core::spec::order(&cat.specs, id)?;
    for c in &order {
        crate::ops::refuse_generated(&cat.specs, c)?;
    }
    let mut rebuilt: Vec<&str> = Vec::new();
    for c in &order[..order.len() - 1] {
        let spec = cat.specs.iter().find(|s| &s.asset.id == c);
        let fresh = build::child_glb(project, &cfg, c, None).is_some()
            && cat
                .builds
                .get(c)
                .is_some_and(|r| r.stale.is_empty() && r.changed.is_empty())
            && !spec.is_some_and(|s| {
                kiln_core::spec::made_of(s)
                    .iter()
                    .any(|g| rebuilt.contains(g))
            });
        if !fresh {
            say(format!("Antes, «{c}», que forma parte de «{id}»"));
            build_one(project, config_file, c, &mut say)?;
            rebuilt.push(c);
        }
    }
    build_one(project, config_file, id, say)
}

fn build_one(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = config(project, config_file)?;
    let mut cat = crate::load(project, config_file)?;
    let used: Vec<String> = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?
        .parts
        .iter()
        .filter_map(|p| p.source.from.clone())
        .collect();
    if export_blends(project, &cfg, &cat, Some(&used), &mut say)? > 0 {
        cat = crate::load(project, config_file)?;
    }
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
    link_godot(
        project,
        &cfg,
        Path::new(&plan.glb),
        Path::new(&plan.materials),
        &mut say,
    );
    // the variants are the same assembly: bring them up to date with it
    for v in spec
        .variants
        .iter()
        .filter(|v| kiln_core::spec::plain_id(&v.id))
    {
        paint_asset(project, config_file, id, Some(&v.id), &mut say)?;
    }
    write_contract(project, config_file, id)?;
    Ok(PathBuf::from(plan.glb))
}

/// Writes `asset.json` beside the build: what the asset is, what it was made
/// from and what came out, with the hashes that tell one result from another.
fn write_contract(project: &Path, config_file: Option<&Path>, id: &str) -> Result<(), String> {
    let cfg = config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let dir = build::out_dir(project, &cfg, id);
    let json = |name: &str| -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(dir.join(name)).ok()?).ok()
    };
    let report = json("report.json");
    let size_m = report.as_ref().and_then(|r| {
        let v = r.get("size_m")?.as_array()?;
        Some([v.first()?.as_f64()?, v.get(1)?.as_f64()?, v.get(2)?.as_f64()?])
    });
    let mut outputs = vec![(None, dir.join(format!("{id}.glb")))];
    outputs.extend(
        spec.variants
            .iter()
            .filter(|v| kiln_core::spec::plain_id(&v.id))
            .map(|v| (Some(v.id.clone()), dir.join(format!("{}.glb", stem(id, Some(&v.id))))))
            .filter(|(_, f)| f.exists()),
    );
    let tools = crate::ops::read::tools(&cfg);
    let contract = kiln_core::asset::contract(&kiln_core::asset::Built {
        root: project,
        spec,
        catalog: &cat,
        size_m,
        tris: report.as_ref().and_then(|r| r.get("tris")?.as_u64()),
        materials: json("materials.json")
            .and_then(|m| serde_json::from_value(m).ok())
            .unwrap_or_default(),
        outputs,
        tools: &tools,
    })?;
    let text = serde_json::to_string_pretty(&contract).map_err(|e| e.to_string())?;
    kiln_core::asset::write_replacing(&dir.join("asset.json"), text.as_bytes())
        .map_err(|e| e.to_string())
}

/// Runs a Kiln script in Blender on one JSON file; `made` must exist after.
fn blender(cfg: &Config, script: &Path, job: &Path, made: &Path) -> Result<String, String> {
    let work = job.parent().unwrap_or(Path::new("."));
    let name = script
        .file_stem()
        .map_or("blender".into(), |n| n.to_string_lossy().into_owned());
    let out = crate::run::blender(cfg, script, &[job], work, &name)?;
    if !out.success() || !made.exists() {
        return Err(out.failure("Blender falló"));
    }
    Ok(out.stdout)
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
    crate::ops::refuse_generated(&cat.specs, id)?;
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
    write_contract(project, config_file, id)?;
    Ok(glb)
}

/// Links every `part/zone` material of a built GLB to the Godot material of
/// its palette key, in the GLB's `.import`. Says why when it cannot.
pub(crate) fn link_godot(project: &Path, cfg: &Config, glb: &Path, materials: &Path, say: &mut dyn FnMut(String)) {
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
            if !gp.join("project.godot").is_file() {
                return Err(format!("{} no tiene project.godot", gp.display()));
            }
            say(format!("Godot importa el GLB por primera vez ({})", cfg.tools.godot));
            let work = glb.parent().unwrap_or(&gp).to_path_buf();
            let _ = crate::run::godot(
                cfg,
                vec!["--path".into(), gp.clone().into(), "--import".into()],
                &work,
                "godot-import",
            )?;
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

/// Has Godot measure and export every piece only it can read.
pub fn measure(
    project: &Path,
    config_file: Option<&Path>,
    mut say: impl FnMut(String),
) -> Result<usize, String> {
    let cfg = config(project, config_file)?;
    let cat = kiln_core::catalog::index(project, &cfg);
    let exported = export_blends(project, &cfg, &cat, None, &mut say)?;
    let gp = match cfg.tools.godot_project.as_ref() {
        Some(gp) => gp,
        None if cat.pieces.iter().any(|p| p.format == "blend") => return Ok(exported),
        None => {
            return Err(
                "falta [tools] godot_project en kiln.toml (la carpeta con project.godot)".into(),
            );
        }
    };
    let reqs = godot::requests(project, &cfg, &cat);
    if reqs.is_empty() {
        say("No hay piezas que solo lea Godot".into());
        return Ok(exported);
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
    let status = crate::run::godot(
        &cfg,
        vec![
            "--path".into(),
            project.join(gp).into(),
            "--script".into(),
            abs(&script).into(),
            "--".into(),
            abs(&req_file).into(),
            abs(&dir).join("pieces.json").into(),
        ],
        &abs(&dir),
        "measure",
    )?;
    if !out.exists() {
        return Err(format!(
            "Godot no escribió las medidas: {}",
            status.stderr.lines().last().unwrap_or("")
        ));
    }
    say(format!("Medidas en {}", out.display()));
    Ok(reqs.len() + exported)
}

/// Has Blender export the `.blend` pieces that are not exported or changed
/// since; with `only`, just those keys. Returns how many it exported.
fn export_blends(
    project: &Path,
    cfg: &Config,
    cat: &kiln_core::Catalog,
    only: Option<&[String]>,
    say: &mut dyn FnMut(String),
) -> Result<usize, String> {
    let reqs: Vec<_> = kiln_core::blend::requests(project, cfg, cat)
        .into_iter()
        .filter(|r| only.is_none_or(|keys| keys.contains(&r.key)))
        .collect();
    if reqs.is_empty() {
        return Ok(0);
    }
    let cache = kiln_core::blend::cache_file(project, cfg);
    let dir = cache.parent().unwrap_or(project).to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let script = dir.join("export_piece.py");
    std::fs::write(&script, EXPORT_PIECE).map_err(|e| e.to_string())?;
    for r in &reqs {
        say(format!("Blender exporta {} ({})", r.key, cfg.tools.blender));
        let glb = Path::new(&r.glb);
        let _ = std::fs::remove_file(glb);
        let name = format!("export-{}", r.key.replace('/', "__"));
        let out = crate::run::blender(cfg, &script, &[Path::new(&r.blend), glb], &dir, &name)?;
        if !out.success() || !glb.exists() {
            return Err(out.failure(&format!("Blender no pudo exportar {}", r.key)));
        }
        kiln_core::blend::record(project, cfg, r)?;
    }
    Ok(reqs.len())
}
