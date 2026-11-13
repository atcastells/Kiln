//! `kiln`: the command line and the local server of Kiln.
//!
//!   kiln index  --project DIR [--config FILE] [--out FILE]   catalog as JSON
//!   kiln check  --project DIR [--config FILE]                findings; exit 1 on errors
//!   kiln serve  --project DIR [--config FILE] [--port 4330]  the graph at http://127.0.0.1:PORT
//!   kiln build  --project DIR [--config FILE] --asset ID     assemble an asset in Blender
//!   kiln godot  --project DIR [--config FILE]                Godot measures its own pieces

mod agent;
mod production;
mod serve;

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
    };
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("falta el valor de {flag}"));
        match flag.as_str() {
            "--project" => a.project = PathBuf::from(val()?),
            "--config" => a.config = Some(PathBuf::from(val()?)),
            "--out" => a.out = Some(PathBuf::from(val()?)),
            "--asset" => a.asset = Some(val()?),
            "--port" => a.port = val()?.parse().map_err(|_| "--port debe ser un número")?,
            other => return Err(format!("opción desconocida {other}\n{USAGE}")),
        }
    }
    a.project =
        std::fs::canonicalize(&a.project).map_err(|e| format!("{}: {e}", a.project.display()))?;
    Ok(a)
}

const USAGE: &str = "uso: kiln <index|check|serve|build|godot> --project DIR [--config FILE] [--out FILE] [--port N] [--asset ID]";

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
            "serve" => serve::run(args.project.clone(), args.config.clone(), args.port)
                .map(|_| ExitCode::SUCCESS),
            "build" => {
                let id = args.asset.as_deref().ok_or("falta --asset ID")?;
                let glb =
                    production::build_asset(&args.project, args.config.as_deref(), id, |l| {
                        eprintln!("{l}")
                    })?;
                println!("{}", glb.display());
                Ok(ExitCode::SUCCESS)
            }
            "godot" => {
                production::measure(&args.project, args.config.as_deref(), |l| eprintln!("{l}"))
                    .map(|_| ExitCode::SUCCESS)
            }
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
