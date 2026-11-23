//! `kiln`: the command line and the local server of Kiln.
//!
//!   kiln index  --project DIR [--config FILE] [--out FILE]   catalog as JSON
//!   kiln check  --project DIR [--config FILE]                findings; exit 1 on errors
//!   kiln tools  --project DIR [--config FILE]                external tools found, with their versions
//!   kiln mcp    [--project DIR] [--config FILE]              MCP over stdio, for Claude Code; without --project it serves any project
//!   kiln setup  [--blender PATH] [--godot PATH] [--claude PATH]   find Blender, Godot and Claude Code outside the PATH and save them for every project
//!   kiln trust  --project DIR                                let `kiln mcp` run Blender, Godot and Claude in that project
//!   kiln serve  [--project DIR] [--config FILE] [--port 4330] [--ui-dir DIR]  the core and its page at http://127.0.0.1:PORT (--ui-dir: read the new UI from disk); with a core running, it shows DIR there
//!   kiln stop   [--force]                                    the core saves its jobs and leaves (--force: even with jobs running)
//!   kiln build  --project DIR [--config FILE] --asset ID     assemble an asset in Blender
//!   kiln generate --project DIR [--config FILE] --asset ID [--seed N]   generate a recipe asset in Godot, no Blender
//!   kiln paint  --project DIR [--config FILE] --asset ID [--variant ID]   wear its materials again, no assembling
//!   kiln verify --project DIR [--config FILE] --asset ID     check the built GLB: own checks, glTF Validator if installed, Godot import
//!   kiln publish --project DIR [--config FILE] --asset ID    copy a verified asset to `[publish] dir` for the game
//!   kiln godot  --project DIR [--config FILE]                Godot measures its own pieces
//!   kiln review --project DIR [--config FILE] --asset ID     build, render, review and fix, up to 3 rounds
//!   kiln content-manifest --project DIR [--config FILE] [--set NAME]   Godot lists the ids of the scenes of the content sets

mod agent;
mod content;
mod core;
mod fetch;
mod generate;
mod jobs;
mod mcp;
mod ops;
mod projects;
mod production;
mod review;
mod run;
mod serve;
mod verify;
mod verify_recipe;

use kiln_core::{Catalog, Config, Severity};
use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    cmd: String,
    project: PathBuf,
    config: Option<PathBuf>,
    out: Option<PathBuf>,
    port: u16,
    asset: Option<String>,
    variant: Option<String>,
    set: Option<String>,
    ui_dir: Option<PathBuf>,
    seed: Option<u32>,
    /// Whether `--project` was given (only `mcp` works without it).
    project_given: bool,
    /// `serve --core`: started by `kiln mcp`, leaves when nobody uses it.
    core: bool,
    force: bool,
    /// `setup --blender PATH` and the like: programs the person names.
    programs: Vec<(String, PathBuf)>,
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or(USAGE)?;
    let mut a = Args {
        cmd,
        project: PathBuf::from("."),
        config: None,
        out: None,
        port: 4330,
        asset: None,
        variant: None,
        set: None,
        ui_dir: None,
        seed: None,
        project_given: false,
        core: false,
        force: false,
        programs: Vec::new(),
    };
    while let Some(flag) = it.next() {
        if flag == "--core" || flag == "--force" {
            *(if flag == "--core" { &mut a.core } else { &mut a.force }) = true;
            continue;
        }
        let mut val = || it.next().ok_or(format!("falta el valor de {flag}"));
        match flag.as_str() {
            "--project" => {
                a.project = PathBuf::from(val()?);
                a.project_given = true;
            }
            "--config" => a.config = Some(PathBuf::from(val()?)),
            "--out" => a.out = Some(PathBuf::from(val()?)),
            "--asset" => a.asset = Some(val()?),
            "--variant" => a.variant = Some(val()?),
            "--set" => a.set = Some(val()?),
            "--ui-dir" => a.ui_dir = Some(PathBuf::from(val()?)),
            "--blender" | "--godot" | "--claude" => {
                let p = PathBuf::from(val()?);
                a.programs.push((flag.trim_start_matches('-').to_string(), p));
            }
            "--port" => a.port = val()?.parse().map_err(|_| "--port debe ser un número")?,
            "--seed" => a.seed = Some(val()?.parse().map_err(|_| "--seed debe ser un número entero sin signo (u32)")?),
            other => return Err(format!("opción desconocida {other}\n{USAGE}")),
        }
    }
    a.project =
        std::fs::canonicalize(&a.project).map_err(|e| format!("{}: {e}", a.project.display()))?;
    Ok(a)
}

const USAGE: &str = "uso: kiln <index|check|tools|setup|mcp|trust|serve|stop|build|generate|paint|verify|publish|review|godot|content-manifest> --project DIR [--config FILE] [--out FILE] [--port N] [--ui-dir DIR] [--asset ID] [--variant ID] [--set NAME] [--seed N] [--force] [--blender PATH] [--godot PATH] [--claude PATH]";

pub fn load(
    project: &std::path::Path,
    config: Option<&std::path::Path>,
) -> Result<Catalog, String> {
    match config {
        Some(c) => kiln_core::open_with(project, Config::load_file(c)?),
        None => kiln_core::open(project),
    }
}

fn main() -> ExitCode {
    kiln_core::tools::set_saved_file(projects::tools_file());
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let run = || -> Result<ExitCode, String> {
        match args.cmd.as_str() {
            "index" => {
                let cat = load(&args.project, args.config.as_deref())?;
                let json = serde_json::to_string_pretty(&cat).map_err(|e| e.to_string())?;
                match &args.out {
                    Some(o) => std::fs::write(o, json).map_err(|e| e.to_string())?,
                    None => println!("{json}"),
                }
                eprintln!(
                    "{}: {} kits, {} piezas, {} materiales, {} avisos",
                    cat.project,
                    cat.kits.len(),
                    cat.pieces.len(),
                    cat.materials.len(),
                    cat.findings.len()
                );
                Ok(ExitCode::SUCCESS)
            }
            "check" => {
                let cat = load(&args.project, args.config.as_deref())?;
                for f in &cat.findings {
                    let s = match f.severity {
                        Severity::Error => "ERROR",
                        Severity::Warn => "aviso",
                        Severity::Info => "info ",
                    };
                    println!("{s}  {:<34} {:<20} {}", f.subject, f.code, f.message);
                }
                let errors = cat
                    .findings
                    .iter()
                    .filter(|f| f.severity == Severity::Error)
                    .count();
                eprintln!(
                    "{} piezas, {} errores, {} avisos",
                    cat.pieces.len(),
                    errors,
                    cat.findings
                        .iter()
                        .filter(|f| f.severity == Severity::Warn)
                        .count()
                );
                Ok(if errors > 0 {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                })
            }
            "tools" => {
                let cfg = production::config(&args.project, args.config.as_deref())?;
                let found = ops::read::tools(&cfg);
                let w = found.iter().map(|t| t.name.len()).max().unwrap_or(0);
                for t in &found {
                    let state = match (t.found, t.ok) {
                        (false, _) => "ausente",
                        (true, true) => "ok",
                        (true, false) => "no compatible",
                    };
                    let version = t.version.as_deref().unwrap_or("-");
                    let line = format!("{:<w$}  {:<10}  {:<13}  {}", t.name, version, state, t.note.as_deref().unwrap_or(""));
                    println!("{}", line.trim_end());
                }
                Ok(ExitCode::SUCCESS)
            }
            "setup" => {
                let file = projects::tools_file().ok_or("no se sabe dónde guardar las herramientas: define KILN_HOME")?;
                for (name, p) in &args.programs {
                    ops::setup::set(&file, name, p)?;
                    eprintln!("{name}: {}", p.display());
                }
                let home = std::env::var_os("HOME").map(PathBuf::from);
                let v = ops::setup::setup(Some(&file), &Default::default(), std::path::Path::new("/"), home.as_deref())?;
                for t in v["tools"].as_array().into_iter().flatten() {
                    let name = t["name"].as_str().unwrap_or_default();
                    let line = match t["path"].as_str() {
                        Some(p) => format!(
                            "{name:<8} {:<10} {p}{}",
                            t["version"].as_str().unwrap_or("-"),
                            t["note"].as_str().map(|n| format!("  ({n})")).unwrap_or_default()
                        ),
                        None => format!("{name:<8} ausente    {}", t["download"].as_str().unwrap_or_default()),
                    };
                    println!("{line}");
                }
                eprintln!("{}", v["next"].as_str().unwrap_or_default());
                Ok(ExitCode::SUCCESS)
            }
            "trust" => {
                if !args.project_given {
                    return Err("falta --project DIR: la carpeta del juego en la que confías".into());
                }
                let k = projects::kind(&args.project);
                if !(k.kiln || k.godot) {
                    return Err(format!("{} no tiene kiln.toml ni project.godot", args.project.display()));
                }
                let home = projects::home().ok_or("no se sabe dónde guardar la confianza: define KILN_HOME")?;
                projects::trust(&home, &args.project)?;
                eprintln!("Kiln ya puede lanzar Blender, Godot y Claude en {}", args.project.display());
                Ok(ExitCode::SUCCESS)
            }
            "mcp" => {
                let project = (args.project_given || args.config.is_some()).then(|| args.project.clone());
                mcp::run(project, args.config.clone()).map(|_| ExitCode::SUCCESS)
            }
            "serve" => {
                let project = (!args.core).then(|| args.project.clone());
                // Only a folder the person named is vouched for, as with `kiln mcp`.
                let vouched = args.project_given || args.config.is_some();
                if let Some(p) = &project
                    && let Some(url) = core::open_in_running(p, args.config.as_deref(), vouched)?
                {
                    eprintln!("Kiln ya está en marcha: {url} muestra ahora {}", p.display());
                    return Ok(ExitCode::SUCCESS);
                }
                core::run(core::Opts {
                    project,
                    config: args.config.clone(),
                    port: args.port,
                    ui_dir: args.ui_dir.clone(),
                    auto: args.core,
                    vouched,
                })
                .map(|_| ExitCode::SUCCESS)
            }
            "stop" => {
                if core::stop(args.force)? {
                    eprintln!("el núcleo de Kiln se ha cerrado");
                } else {
                    eprintln!("no hay ningún núcleo de Kiln en marcha");
                }
                Ok(ExitCode::SUCCESS)
            }
            "build" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let glb =
                    production::build_asset(&args.project, args.config.as_deref(), id, |l| {
                        eprintln!("{l}")
                    })?;
                println!("{}", glb.display());
                Ok(ExitCode::SUCCESS)
            }
            "generate" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let made = generate::generate_asset(
                    &args.project,
                    args.config.as_deref(),
                    id,
                    args.seed,
                    |l| eprintln!("{l}"),
                )?;
                println!("{}", made.glb.display());
                Ok(ExitCode::SUCCESS)
            }
            "verify" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                ops::check_id("el asset", id)?;
                let report =
                    verify::verify_asset(&args.project, args.config.as_deref(), id, |l| {
                        eprintln!("{l}")
                    })?;
                println!("{}", report.display());
                Ok(ExitCode::SUCCESS)
            }
            "publish" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let done = ops::publish::publish_asset(
                    &args.project,
                    args.config.as_deref(),
                    id,
                    |l| eprintln!("{l}"),
                )?;
                println!("{}", args.project.join(&done.dir).display());
                Ok(ExitCode::SUCCESS)
            }
            "paint" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let glb = production::paint_asset(
                    &args.project,
                    args.config.as_deref(),
                    id,
                    args.variant.as_deref(),
                    |l| eprintln!("{l}"),
                )?;
                println!("{}", glb.display());
                Ok(ExitCode::SUCCESS)
            }
            "review" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let file = review::run(&args.project, args.config.as_deref(), id, &|l| {
                    eprintln!("{l}")
                })?;
                println!("{}", file.display());
                Ok(ExitCode::SUCCESS)
            }
            "godot" => {
                production::measure(&args.project, args.config.as_deref(), |l| eprintln!("{l}"))
                    .map(|_| ExitCode::SUCCESS)
            }
            "content-manifest" => content::export(
                &args.project,
                args.config.as_deref(),
                args.set.as_deref(),
                |l| eprintln!("{l}"),
            )
            .map(|_| ExitCode::SUCCESS),
            _ => Err(USAGE.into()),
        }
    };
    match run() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
