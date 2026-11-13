//! The visual review loop: build the asset, render its sheets in Blender,
//! have a reviewer agent (its own `claude -p`, the strongest model, read
//! only) judge them, apply its fixes to the spec and build again. Each round
//! after the first is weighed against the best so far, so the result never
//! gets worse; at most `[agents] review_rounds` rounds, and the best stays.
//!
//! Everything of a review lives in `<build>/<id>/review/`: one folder per
//! round (sheets, measures, the spec it was built from, the review) and
//! `review.json` for the page.

use crate::agent;
use kiln_core::build;
use kiln_core::review::{self, Round, Summary};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RENDER: &str = include_str!("../blender/render.py");
const SKILL: &str = include_str!("../../../skills/review/SKILL.md");
pub const SHEETS: [(&str, &str); 4] = [
    ("1-turn.png", "giro: seis vistas oblicuas"),
    (
        "2-elevations.png",
        "alzados con figura de 1,8 m, rejilla de 1 m y cotas",
    ),
    (
        "3-parts.png",
        "piezas: cada una de un color, con su id en la leyenda",
    ),
    (
        "4-silhouette.png",
        "silueta, y el asset al tamaño del juego",
    ),
];

fn rel(project: &Path, p: &Path) -> String {
    p.strip_prefix(project)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("{} → {}: {e}", from.display(), to.display()))
}

/// The files of the last build (glb, blend, report), from its plan.
fn built_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let plan: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("plan.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(["glb", "blend", "report"]
        .iter()
        .filter_map(|k| plan.get(*k).and_then(Value::as_str).map(PathBuf::from))
        .collect())
}

/// Blender renders the sheets of `blend` into `out`.
fn render(blender: &str, blend: &Path, out: &Path, say: &dyn Fn(String)) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let script = out.join("render.py");
    std::fs::write(&script, RENDER).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(blender);
    let python = Path::new(blender)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("python"));
    if python {
        cmd.arg(&script).arg(blend).arg(out);
    } else {
        cmd.args(["--background", "--factory-startup", "--python"])
            .arg(&script)
            .arg("--")
            .arg(blend)
            .arg(out);
    }
    say("Blender saca las cuatro hojas y las medidas".into());
    let res = cmd
        .output()
        .map_err(|e| format!("no se puede lanzar {blender}: {e}"))?;
    let _ = std::fs::remove_file(&script);
    if !res.status.success() || !out.join("measures.json").exists() {
        let err = String::from_utf8_lossy(&res.stderr);
        let tail: Vec<&str> = err.lines().rev().take(6).collect();
        return Err(format!(
            "Blender no pudo sacar las hojas: {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        ));
    }
    Ok(())
}

/// Lines worth saying from what Blender measured.
fn measured(out: &Path) -> Vec<String> {
    let Ok(m) = std::fs::read_to_string(out.join("measures.json"))
        .map_err(|_| ())
        .and_then(|t| serde_json::from_str::<Value>(&t).map_err(|_| ()))
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for t in m["through"].as_array().into_iter().flatten() {
        lines.push(format!(
            "{} atraviesa {} ({} m)",
            t["a"].as_str().unwrap_or("?"),
            t["b"].as_str().unwrap_or("?"),
            t["depth_m"]
        ));
    }
    for f in m["floating"].as_array().into_iter().flatten() {
        lines.push(format!(
            "{} no se apoya en nada (a {} m)",
            f["name"].as_str().unwrap_or("?"),
            f["gap_m"]
        ));
    }
    lines
}

/// One call to the reviewer: one round, or two (compare mode, A and B).
fn ask(
    project: &Path,
    model: &str,
    rounds: &[(&str, &Path)],
    context_file: &Path,
    out: &Path,
    say: &dyn Fn(String),
) -> Result<review::Review, String> {
    let mut images = Vec::new();
    let mut n = 1;
    for (label, dir) in rounds {
        for (file, what) in SHEETS {
            let who = if rounds.len() > 1 {
                format!("montaje {label}, ")
            } else {
                String::new()
            };
            images.push(format!(
                "Imagen {n} ({who}{what}): {}",
                dir.join(file).display()
            ));
            n += 1;
        }
    }
    let data: Vec<String> = rounds
        .iter()
        .map(|(label, dir)| {
            let who = if rounds.len() > 1 {
                format!(" del montaje {label}")
            } else {
                String::new()
            };
            format!(
                "Medidas{who}: {} · triángulos{who}: {} · ficha{who}: {}",
                dir.join("measures.json").display(),
                dir.join("report.json").display(),
                dir.join("spec.toml").display()
            )
        })
        .collect();
    let mode = if rounds.len() > 1 {
        "Modo comparar: dos montajes del mismo asset, A y B, en orden aleatorio. Elige el mejor en `winner` y revisa solo ese."
    } else {
        "Modo normal: un montaje."
    };
    let has_context = context_file.exists();
    let prompt = format!(
        "Revisa el montaje del asset siguiendo la skill kiln-review de tu prompt de sistema. {mode}\n\n{}\n\n{}\n{}\nEscribe la revisión, en TOML, en {} y nada más.",
        images.join("\n"),
        data.join("\n"),
        if has_context {
            format!("Contexto del proyecto: {}", context_file.display())
        } else {
            "El proyecto aún no tiene contexto escrito.".into()
        },
        out.display()
    );
    let mut cmd = agent::claude();
    cmd.args(["-p", &prompt, "--model", model])
        .args(["--output-format", "stream-json", "--verbose"])
        .args(["--append-system-prompt", SKILL])
        .args([
            "--allowedTools",
            "Read",
            "Glob",
            "Grep",
            "Write",
            "--permission-mode",
            "acceptEdits",
        ])
        .args(["--disallowedTools", "Bash", "Edit"]);
    let mut dirs: Vec<PathBuf> = rounds.iter().map(|(_, d)| d.to_path_buf()).collect();
    dirs.extend(out.parent().map(Path::to_path_buf));
    dirs.extend(context_file.parent().map(Path::to_path_buf));
    for d in dirs {
        cmd.arg("--add-dir").arg(d);
    }
    let _ = std::fs::remove_file(out);
    let mut child = cmd
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se puede lanzar Claude Code (`claude`): {e}"))?;
    let stdout = child.stdout.take().unwrap();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(l) = agent::describe(&line, project) {
            say(format!("Revisor: {l}"));
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        return Err(format!("el revisor salió con {status}: {}", err.trim()));
    }
    let text = std::fs::read_to_string(out)
        .map_err(|_| "el revisor no escribió la revisión".to_string())?;
    review::parse(&text)
}

fn count(r: &review::Review) -> String {
    let n = |s: &str| r.findings.iter().filter(|f| f.severity == s).count();
    format!(
        "{} · {} graves, {} importantes, {} menores",
        if r.verdict == "pass" {
            "aprobado"
        } else {
            "a revisar"
        },
        n("blocker"),
        n("major"),
        n("minor")
    )
}

fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Builds, reviews and fixes the asset `id` until the reviewer approves it or
/// the rounds run out; leaves the best round built and in the spec. Returns
/// `review.json`.
pub fn run(
    project: &Path,
    config: Option<&Path>,
    id: &str,
    say: &dyn Fn(String),
) -> Result<PathBuf, String> {
    let cfg = crate::production::config(project, config)?;
    let cat = crate::load(project, config)?;
    let spec_rel = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .map(|s| s.file.clone())
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let spec_path = project.join(&spec_rel);
    let ctx_file = project.join(&cfg.context.file);
    let build_dir = build::out_dir(project, &cfg, id);
    let base = build_dir.join("review");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    // keeps Godot from importing the rounds when the build is in its project
    let _ = std::fs::write(base.join(".gdignore"), "");
    let model = cfg.agents.review_model.clone();
    let max = cfg.agents.review_rounds.clamp(1, 5);
    let mut rounds: Vec<Round> = Vec::new();
    let mut best: Option<u32> = None;
    let dir_of = |n: u32| base.join(format!("round-{n}"));

    for n in 1..=max {
        say(format!("— Vuelta {n} de {max} —"));
        let dir = dir_of(n);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        copy(&spec_path, &dir.join("spec.toml"))?;
        crate::production::build_asset(project, config, id, say)?;
        for f in built_files(&build_dir)? {
            let name = match f.extension().and_then(|e| e.to_str()) {
                Some("json") => "report.json".to_string(),
                Some(ext) => format!("{id}.{ext}"),
                None => continue,
            };
            copy(&f, &dir.join(name))?;
        }
        render(
            &cfg.tools.blender,
            &dir.join(format!("{id}.blend")),
            &dir,
            say,
        )?;
        for l in measured(&dir) {
            say(format!("Medido: {l}"));
        }
        let out = dir.join("review.toml");
        let (r, against, kept) = match best {
            None => {
                say(format!("El revisor ({model}) mira las hojas"));
                let r = ask(project, &model, &[("", &dir)], &ctx_file, &out, say)?;
                (r, None, n)
            }
            Some(b) => {
                // A and B in an order the reviewer cannot guess
                let flip = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos() % 2 == 1)
                    .unwrap_or(false);
                let (a, bb) = if flip { (n, b) } else { (b, n) };
                say(format!(
                    "El revisor ({model}) compara la vuelta {n} con la {b}, sin saber cuál es cuál"
                ));
                let r = ask(
                    project,
                    &model,
                    &[("A", &dir_of(a)), ("B", &dir_of(bb))],
                    &ctx_file,
                    &out,
                    say,
                )?;
                let kept = match r.winner.as_deref() {
                    Some("A") => a,
                    Some("B") => bb,
                    _ => b,
                };
                if let Some(w) = &r.why {
                    say(format!("Se queda la vuelta {kept}: {w}"));
                }
                (r, Some(b), kept)
            }
        };
        say(format!("Vuelta {kept}: {}", count(&r)));
        best = Some(kept);
        let tris = std::fs::read_to_string(dir.join("report.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v["tris"].as_u64())
            .unwrap_or(0);
        let mut round = Round {
            n,
            dir: rel(project, &dir),
            review: r.clone(),
            against,
            kept,
            applied: Vec::new(),
            left: Vec::new(),
            tris,
        };
        let pass = r.verdict == "pass";
        if !pass && n < max {
            // fixes go onto the spec of the round kept
            let from = std::fs::read_to_string(dir_of(kept).join("spec.toml"))
                .map_err(|e| e.to_string())?;
            let (text, applied, left) = review::apply(&from, &r.findings)?;
            for l in &applied {
                say(format!("Corrige {l}"));
            }
            round.applied = applied;
            round.left = left;
            if round.applied.is_empty() {
                say("Nada que corregir solo en la ficha: lo que queda es para ti".into());
                rounds.push(round);
                break;
            }
            std::fs::write(&spec_path, text).map_err(|e| e.to_string())?;
        } else {
            round.left = r
                .findings
                .iter()
                .map(|f| format!("{}: {}", f.part, f.problem))
                .collect();
        }
        rounds.push(round);
        if pass {
            break;
        }
    }

    // the best round stays: its spec and its build
    let best = best.unwrap_or(1);
    let bdir = dir_of(best);
    copy(&bdir.join("spec.toml"), &spec_path)?;
    for f in built_files(&build_dir)? {
        let name = match f.extension().and_then(|e| e.to_str()) {
            Some("json") => "report.json".to_string(),
            Some(ext) => format!("{id}.{ext}"),
            None => continue,
        };
        copy(&bdir.join(name), &f)?;
    }
    say(format!(
        "Se queda la vuelta {best}: su ficha y su montaje están en su sitio"
    ));
    let summary = Summary {
        asset: id.into(),
        rounds,
        best,
        model,
        done_at: now(),
    };
    let file = base.join("review.json");
    std::fs::write(
        &file,
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(file)
}
