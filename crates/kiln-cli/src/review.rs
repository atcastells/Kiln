//! The visual review loop: build the asset, render its sheets in Blender,
//! have a reviewer agent (its own `claude -p`, the strongest model, read
//! only) judge them, apply its fixes to the spec and build again. Each round
//! after the first is weighed against the best so far, so the result never
//! gets worse; at most `[agents] review_rounds` rounds, and the best stays.
//!
//! Everything of a review lives in `<build>/<id>/review/`: one folder per
//! round (sheets, measures, the spec it was built from, the review) and
//! `review.json` for the page.
//!
//! Over MCP the agent of the client is the reviewer (D1): `render_round`
//! builds and renders one round with no reviewer, the client looks at the
//! sheets with the `kiln-review` prompt, and `submit` takes its review,
//! applies its fixes to the spec and keeps the summary.

use crate::agent;
use kiln_core::build;
use kiln_core::review::{self, Round, Summary};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;

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
fn render(
    cfg: &kiln_core::Config,
    blend: &Path,
    out: &Path,
    say: &dyn Fn(String),
) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let script = out.join("render.py");
    std::fs::write(&script, RENDER).map_err(|e| e.to_string())?;
    say("Blender saca las cuatro hojas y las medidas".into());
    let res = crate::run::blender(cfg, &script, &[blend, out], out, "render");
    let _ = std::fs::remove_file(&script);
    let res = res?;
    if !res.success() || !out.join("measures.json").exists() {
        return Err(res.failure("Blender no pudo sacar las hojas"));
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

/// The spec of `id` and where its build goes, refusing a generated asset.
fn target(project: &Path, config: Option<&Path>, id: &str) -> Result<(kiln_core::Config, PathBuf, PathBuf), String> {
    let cfg = crate::production::config(project, config)?;
    let cat = crate::load(project, config)?;
    crate::ops::refuse_generated(&cat.specs, id)?;
    let spec_rel = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .map(|s| s.file.clone())
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let build_dir = build::out_dir(project, &cfg, id);
    Ok((cfg, project.join(spec_rel), build_dir))
}

/// The rounds already in `base`, in order.
fn rounds_in(base: &Path) -> Vec<u32> {
    let mut v: Vec<u32> = std::fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_prefix("round-")?.parse().ok())
        .collect();
    v.sort();
    v
}

/// Builds `id` and renders the sheets of one round, without a reviewer: the
/// first round of a new review, or with `next` the round after the last one,
/// once that one has its review. Returns the round's folder.
pub fn render_round(
    project: &Path,
    config: Option<&Path>,
    id: &str,
    next: bool,
    say: &dyn Fn(String),
) -> Result<PathBuf, String> {
    let (cfg, spec_path, build_dir) = target(project, config, id)?;
    let base = build_dir.join("review");
    let max = cfg.agents.review_rounds.clamp(1, 5);
    let n = if next {
        let last = rounds_in(&base).last().copied().ok_or(format!("«{id}» no tiene revisión: lanza `review` sin next_round"))?;
        if !base.join(format!("round-{last}/review.toml")).is_file() {
            return Err(format!("la vuelta {last} aún no tiene revisión: mándala con `review_submit`"));
        }
        if last >= max {
            return Err(format!("ya van {last} vueltas, el máximo de `[agents] review_rounds`"));
        }
        last + 1
    } else {
        let _ = std::fs::remove_dir_all(&base);
        1
    };
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let _ = std::fs::write(base.join(".gdignore"), "");
    say(format!("— Vuelta {n} de {max} —"));
    let dir = base.join(format!("round-{n}"));
    let _ = std::fs::remove_dir_all(&dir);
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
    render(&cfg, &dir.join(format!("{id}.blend")), &dir, say)?;
    for l in measured(&dir) {
        say(format!("Medido: {l}"));
    }
    say("Hojas listas: míralas con `review_sheets` y manda la revisión con `review_submit`".into());
    Ok(dir)
}

/// What `submit` did with a review.
#[derive(Debug, serde::Serialize)]
pub struct Submitted {
    pub round: u32,
    pub verdict: String,
    pub applied: Vec<String>,
    pub left: Vec<String>,
    /// The best round so far.
    pub best: u32,
    pub next: String,
}

/// Which round is best: an approved one, else the fewest serious findings;
/// on a tie, the later one.
fn best_of(rounds: &[Round]) -> u32 {
    let key = |r: &Round| {
        let n = |s: &str| r.review.findings.iter().filter(|f| f.severity == s).count();
        (r.review.verdict != "pass", n("blocker"), n("major"), n("minor"), std::cmp::Reverse(r.n))
    };
    rounds.iter().min_by_key(|r| key(r)).map(|r| r.n).unwrap_or(1)
}

/// The review of the last round, in TOML as the `kiln-review` skill writes
/// it, from the agent of the client. Its fixes go onto the spec unless it
/// passes; `review.json` keeps every round for the page.
pub fn submit(project: &Path, config: Option<&Path>, id: &str, text: &str) -> Result<Submitted, String> {
    let (cfg, spec_path, build_dir) = target(project, config, id)?;
    let r = review::parse(text)?;
    if !["pass", "revise"].contains(&r.verdict.as_str()) {
        return Err("`verdict` debe ser pass o revise".into());
    }
    let base = build_dir.join("review");
    let n = rounds_in(&base).last().copied().ok_or(format!("«{id}» no tiene hojas: lanza `review` primero"))?;
    let dir = base.join(format!("round-{n}"));
    if !dir.join("measures.json").is_file() {
        return Err(format!("la vuelta {n} aún no tiene hojas: espera a que termine su trabajo"));
    }
    std::fs::write(dir.join("review.toml"), text).map_err(|e| e.to_string())?;
    let tris = std::fs::read_to_string(dir.join("report.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["tris"].as_u64())
        .unwrap_or(0);
    let pass = r.verdict == "pass";
    let (applied, left) = if pass {
        (Vec::new(), r.findings.iter().map(|f| format!("{}: {}", f.part, f.problem)).collect())
    } else {
        let from = std::fs::read_to_string(dir.join("spec.toml")).map_err(|e| e.to_string())?;
        let (text, applied, left) = review::apply(&from, &r.findings)?;
        if !applied.is_empty() {
            std::fs::write(&spec_path, text).map_err(|e| e.to_string())?;
        }
        (applied, left)
    };
    let file = base.join("review.json");
    let mut rounds: Vec<Round> = std::fs::read_to_string(&file)
        .ok()
        .and_then(|t| serde_json::from_str::<Summary>(&t).ok())
        .map(|s| s.rounds)
        .unwrap_or_default();
    rounds.retain(|x| x.n != n);
    rounds.push(Round {
        n,
        dir: rel(project, &dir),
        review: r.clone(),
        against: None,
        kept: n,
        applied: applied.clone(),
        left: left.clone(),
        tris,
    });
    rounds.sort_by_key(|x| x.n);
    let best = best_of(&rounds);
    let summary = Summary { asset: id.into(), rounds, best, model: "el agente del cliente MCP".into(), done_at: now() };
    std::fs::write(&file, serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let max = cfg.agents.review_rounds.clamp(1, 5);
    let next = if pass {
        "aprobado: sigue con `verify` y `publish`".to_string()
    } else if applied.is_empty() {
        "Kiln no puede corregir nada de esto solo en la ficha: cuéntaselo a la persona o cambia la ficha con `spec_write`".into()
    } else if n < max {
        format!("correcciones aplicadas a la ficha: lanza `review` con next_round = true para montar y revisar la vuelta {}", n + 1)
    } else {
        format!("correcciones aplicadas a la ficha, pero ya van {n} vueltas (el máximo): monta con `build` si quieres verlas")
    };
    Ok(Submitted { round: n, verdict: r.verdict, applied, left, best, next })
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
    let (cfg, spec_path, build_dir) = target(project, config, id)?;
    let ctx_file = project.join(&cfg.context.file);
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
            &cfg,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing;

    const REVISE: &str = "seen = \"un ascensor de madera\"\nverdict = \"revise\"\n\n[[findings]]\npart = \"crossbeam\"\ncriterion = \"construction\"\nseverity = \"major\"\nproblem = \"flota sobre los postes\"\nevidence = \"measures.json\"\nfix = { at = [[0, 4.4, 0]] }\n\n[[findings]]\npart = \"asset\"\ncriterion = \"interest\"\nseverity = \"minor\"\nproblem = \"genérico\"\nevidence = \"giro\"\n";
    const PASS: &str = "seen = \"un ascensor\"\nverdict = \"pass\"\n";

    /// A round on disk as `render_round` leaves it, without Blender.
    fn sheets(c: &crate::agent::Ctx, n: u32) -> PathBuf {
        let cfg = crate::production::config(&c.project, None).unwrap();
        let dir = build::out_dir(&c.project, &cfg, "hoist").join(format!("review/round-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(c.project.join("specs/hoist.toml"), dir.join("spec.toml")).unwrap();
        std::fs::write(dir.join("measures.json"), "{}").unwrap();
        std::fs::write(dir.join("report.json"), r#"{"tris":120}"#).unwrap();
        dir
    }

    #[test]
    fn a_client_review_fixes_the_spec_and_keeps_the_rounds_for_the_page() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        assert!(submit(&c.project, None, "hoist", PASS).unwrap_err().contains("lanza `review`"));
        let dir = sheets(&c, 1);
        let s = submit(&c.project, None, "hoist", REVISE).unwrap();
        assert_eq!((s.round, s.verdict.as_str(), s.best), (1, "revise", 1));
        assert_eq!(s.applied.len(), 1, "{s:?}");
        assert_eq!(s.left.len(), 1, "{s:?}");
        assert!(s.next.contains("next_round"), "{}", s.next);
        let spec = std::fs::read_to_string(c.project.join("specs/hoist.toml")).unwrap();
        assert!(spec.contains("4.4"), "{spec}");
        assert!(dir.join("review.toml").is_file());

        sheets(&c, 2);
        let s = submit(&c.project, None, "hoist", PASS).unwrap();
        assert_eq!((s.round, s.best), (2, 2));
        assert!(s.next.contains("publish"), "{}", s.next);
        let base = dir.parent().unwrap();
        let summary: Summary = serde_json::from_str(&std::fs::read_to_string(base.join("review.json")).unwrap()).unwrap();
        assert_eq!(summary.rounds.iter().map(|r| r.n).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!((summary.best, summary.rounds[0].tris), (2, 120));
    }

    #[test]
    fn a_bad_review_is_refused_and_writes_nothing() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        let dir = sheets(&c, 1);
        let before = std::fs::read(c.project.join("specs/hoist.toml")).unwrap();
        assert!(submit(&c.project, None, "hoist", "verdict = [").is_err());
        assert!(submit(&c.project, None, "hoist", "verdict = \"maybe\"").unwrap_err().contains("pass o revise"));
        assert!(submit(&c.project, None, "../x", PASS).is_err());
        assert!(!dir.join("review.toml").exists());
        assert_eq!(std::fs::read(c.project.join("specs/hoist.toml")).unwrap(), before);
    }

    #[test]
    fn another_round_needs_the_review_of_the_last_one() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        let none = render_round(&c.project, None, "hoist", true, &|_| {}).unwrap_err();
        assert!(none.contains("sin next_round"), "{none}");
        sheets(&c, 1);
        let unreviewed = render_round(&c.project, None, "hoist", true, &|_| {}).unwrap_err();
        assert!(unreviewed.contains("review_submit"), "{unreviewed}");
    }

    #[test]
    fn the_best_round_is_approved_or_has_the_fewest_serious_findings() {
        let round = |n: u32, verdict: &str, severities: &[&str]| Round {
            n,
            dir: String::new(),
            review: review::Review {
                verdict: verdict.into(),
                findings: severities
                    .iter()
                    .map(|s| review::Finding { severity: s.to_string(), ..Default::default() })
                    .collect(),
                ..Default::default()
            },
            against: None,
            kept: n,
            applied: vec![],
            left: vec![],
            tris: 0,
        };
        assert_eq!(best_of(&[round(1, "revise", &["major"]), round(2, "revise", &["blocker"])]), 1);
        assert_eq!(best_of(&[round(1, "revise", &["minor"]), round(2, "revise", &["minor"])]), 2);
        assert_eq!(best_of(&[round(1, "pass", &["minor", "minor"]), round(2, "revise", &[])]), 1);
    }
}

