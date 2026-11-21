//! Long work as jobs: build, paint, Godot, review, import; waiting on a job.

use crate::agent::{self, Ctx, Job, Jobs};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_WAIT: Duration = Duration::from_secs(30);
const QUIET: Duration = Duration::from_secs(1);
const BURST: Duration = Duration::from_secs(10);

pub fn build(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("montar {id}"), move |say| {
        let glb = crate::production::build_asset(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(super::rel(&ctx.project, &glb)))
    })
}

pub fn paint(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str, variant: Option<String>) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("aplicar materiales a {id}"), move |say| {
        let v = variant.as_deref().filter(|v| !v.is_empty());
        let glb = crate::production::paint_asset(&ctx.project, ctx.config.as_deref(), &id, v, say)?;
        Ok(Some(super::rel(&ctx.project, &glb)))
    })
}

pub fn verify(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("verificar {id}"), move |say| {
        let report = crate::verify::verify_asset(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(super::rel(&ctx.project, &report)))
    })
}

pub fn publish(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("publicar {id}"), move |say| {
        let done = super::publish::publish_asset(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(format!("{}/asset.json", done.dir)))
    })
}

/// Generates the GLB of a generated asset. `recipe` is a spec as TOML text
/// whose `[generator]` is saved first: it replaces the recipe of the spec on
/// disk and leaves the rest of that file alone (with no spec yet, the text is
/// the new spec). An invalid one is refused and no job starts. The job's file
/// is the relative path of the GLB.
pub fn generate(
    jobs: &Jobs,
    ctx: Arc<Ctx>,
    asset: &str,
    seed: Option<u32>,
    recipe: Option<&str>,
) -> Result<u64, String> {
    super::check_id("el asset", asset)?;
    if let Some(text) = recipe {
        let text = super::specs::with_recipe(&ctx, asset, text)?;
        super::specs::spec_write(&ctx, asset, &text).map_err(|r| {
            let found: Vec<String> = r.findings.iter().map(|f| format!("{}: {}", f.code, f.message)).collect();
            if found.is_empty() { r.error } else { format!("{}: {}", r.error, found.join("; ")) }
        })?;
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("generar {id}"), move |say| {
        let got = crate::generate::generate_asset(&ctx.project, ctx.config.as_deref(), &id, seed, say)?;
        Ok(Some(got.rel))
    })
}

pub fn godot_measure(jobs: &Jobs, ctx: Arc<Ctx>) -> Result<u64, String> {
    agent::job_in(jobs, &ctx.project.clone(), "medir con Godot".into(), move |say| {
        crate::production::measure(&ctx.project, ctx.config.as_deref(), say).map(|_| None)
    })
}

/// `kiln content-manifest` as a job; an unknown set is refused before it starts.
pub fn content_manifest(jobs: &Jobs, ctx: Arc<Ctx>, set: Option<String>) -> Result<u64, String> {
    let set = set.filter(|s| !s.trim().is_empty());
    if let Some(name) = &set {
        let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
        if !cfg.content_sets().0.contains_key(name.as_str()) {
            return Err(format!("no hay conjunto de contenido «{name}» en kiln.toml"));
        }
    }
    let asked = match &set {
        Some(s) => format!("manifiesto {s}"),
        None => "manifiesto de contenido".into(),
    };
    agent::job_in(jobs, &ctx.project.clone(), asked, move |say| {
        crate::content::export(&ctx.project, ctx.config.as_deref(), set.as_deref(), say).map(|_| None)
    })
}

pub fn review(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    agent::job_in(jobs, &ctx.project.clone(), format!("revisar {id}"), move |say| {
        let file = crate::review::run(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(super::rel(&ctx.project, &file)))
    })
}

/// One round of the review with no reviewer of Kiln's own: build and render
/// the sheets; the agent of the client judges them (D1).
pub fn review_round(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str, next: bool) -> Result<u64, String> {
    super::check_id("el asset", asset)?;
    let id = asset.to_string();
    let asked = if next { format!("revisar {id}: otra vuelta") } else { format!("revisar {id}: hojas") };
    agent::job_in(jobs, &ctx.project.clone(), asked, move |say| {
        let dir = crate::review::render_round(&ctx.project, ctx.config.as_deref(), &id, next, say)?;
        Ok(Some(super::rel(&ctx.project, &dir)))
    })
}

/// The client's review of the last round: it may change the spec, so not
/// while a build or another write runs in the project.
pub fn review_submit(jobs: &Jobs, ctx: &Ctx, asset: &str, review: &str) -> Result<crate::review::Submitted, String> {
    super::check_id("el asset", asset)?;
    super::check_len("la revisión", review, 100_000)?;
    agent::busy(jobs, &ctx.project)?;
    crate::review::submit(&ctx.project, ctx.config.as_deref(), asset, review)
}

pub fn import(
    jobs: &Jobs,
    ctx: Arc<Ctx>,
    asset: &str,
    part: &str,
    site: &str,
    id: &str,
) -> Result<u64, String> {
    if [asset, part, site, id].iter().any(|s| s.is_empty()) {
        return Err("falta el asset, la pieza, el sitio o el modelo".into());
    }
    let (asset, part, site, id) = (asset.to_string(), part.to_string(), site.to_string(), id.to_string());
    agent::job_in(jobs, &ctx.project.clone(), format!("traer {site}/{id} para {part}"), move |say| {
        crate::fetch::import(&ctx.project, ctx.config.as_deref(), &asset, &part, &site, &id, say).map(Some)
    })
}

/// The lookup the page makes (`id.max(1)`: id 0 reads the first job).
pub fn job_get(jobs: &Jobs, id: u64) -> Option<Job> {
    jobs.lock().unwrap().get((id.max(1) - 1) as usize).cloned()
}

#[derive(Debug, serde::Serialize)]
pub struct JobView {
    pub id: u64,
    pub asked: String,
    pub status: String,
    pub lines: Vec<String>,
    pub next: usize,
    pub file: Option<String>,
    pub reply: Option<String>,
}

/// The job from line `since` on; waits up to `max` (30 s at most) for news.
pub fn job_wait(jobs: &Jobs, id: u64, since: usize, max: Duration) -> Result<JobView, String> {
    let gone = || "no existe ese trabajo".to_string();
    if id == 0 {
        return Err(gone());
    }
    let end = Instant::now() + max.min(MAX_WAIT);
    // Once there is news, keep gathering lines until the job is quiet for
    // QUIET (or BURST has passed), so a chatty job costs one call, not one per line.
    let (mut first, mut seen, mut last) = (None::<Instant>, since, Instant::now());
    loop {
        let j = jobs.lock().unwrap().get((id - 1) as usize).cloned().ok_or_else(gone)?;
        let now = Instant::now();
        if j.log.len() > seen {
            first.get_or_insert(now);
            (seen, last) = (j.log.len(), now);
        }
        let gathered = first.is_some_and(|f| now - last >= QUIET || now - f >= BURST);
        if gathered || j.status != "running" || now >= end {
            return Ok(JobView {
                id,
                asked: j.asked,
                status: j.status,
                lines: j.log.iter().skip(since).cloned().collect(),
                next: j.log.len(),
                file: j.spec,
                reply: j.reply,
            });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[derive(Debug, serde::Serialize)]
pub struct JobSummary {
    pub id: u64,
    pub asked: String,
    pub status: String,
    pub lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

/// The newest `JOBS_LISTED` jobs and every one still running, oldest first.
pub fn jobs_list(jobs: &Jobs) -> Vec<JobSummary> {
    const JOBS_LISTED: usize = 30;
    let j = jobs.lock().unwrap();
    let from = j.len().saturating_sub(JOBS_LISTED);
    j.iter()
        .enumerate()
        .filter(|(i, j)| *i >= from || j.status == "running")
        .map(|(_, j)| JobSummary {
            id: j.id,
            asked: j.asked.clone(),
            status: j.status.clone(),
            lines: j.log.len(),
            project: j.project.clone(),
        })
        .collect()
}

#[derive(Debug, serde::Serialize)]
pub struct Sheet {
    pub path: PathBuf,
    pub label: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Sheets {
    pub round: u32,
    pub sheets: Vec<Sheet>,
    pub measures: Option<Value>,
    pub review: Option<Value>,
}

/// The sheets of a round of the review of `asset` (the last round if none).
pub fn review_sheets(ctx: &Ctx, asset: &str, round: Option<u32>) -> Result<Sheets, String> {
    super::check_id("el asset", asset)?;
    let cfg = crate::production::config(&ctx.project, ctx.config.as_deref())?;
    let base = kiln_core::build::out_dir(&ctx.project, &cfg, asset).join("review");
    let none = || format!("«{asset}» no tiene revisión: lanza `review` primero");
    let n = match round {
        Some(n) => n,
        None => std::fs::read_dir(&base)
            .map_err(|_| none())?
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.strip_prefix("round-")?.parse().ok())
            .max()
            .ok_or_else(none)?,
    };
    let dir = base.join(format!("round-{n}"));
    if !dir.is_dir() {
        return Err(format!("«{asset}» no tiene la vuelta {n} de la revisión"));
    }
    let json = |p: PathBuf| -> Option<Value> { serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok() };
    let sheets = crate::review::SHEETS
        .iter()
        .filter(|(f, _)| dir.join(f).is_file())
        .map(|(f, l)| Sheet { path: dir.join(f), label: l.to_string() })
        .collect();
    let review = json(base.join("review.json"))
        .and_then(|s| s["rounds"].as_array()?.iter().find(|r| r["n"] == n).map(|r| r["review"].clone()));
    Ok(Sheets { round: n, sheets, measures: json(dir.join("measures.json")), review })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::tests::{project, ROCK};
    use crate::ops::testing;

    fn finish(jobs: &Jobs, id: u64) -> Job {
        for _ in 0..200 {
            let j = job_get(jobs, id).unwrap();
            if j.status != "running" {
                return j;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the job did not finish");
    }

    #[test]
    fn generate_refuses_an_invalid_recipe_without_writing_or_starting() {
        let p = project("op-bad");
        let ctx = Arc::new(crate::ops::ctx(&p, None).unwrap());
        let jobs: Jobs = Default::default();
        let bad = format!("{ROCK}\n[[generator.ops]]\nop = \"crater\"\n");
        let err = generate(&jobs, ctx, "rock", None, Some(&bad)).unwrap_err();
        let spec = std::fs::read_to_string(p.join("specs/rock.toml")).unwrap();
        let (started, ran) = (jobs.lock().unwrap().len(), p.join("ran").exists());
        std::fs::remove_dir_all(&p).unwrap();
        assert!(err.contains("generator_op"), "{err}");
        assert_eq!((spec.as_str(), started, ran), (ROCK, 0, false));
    }

    #[test]
    fn generate_saves_a_valid_recipe_then_runs_with_the_seed() {
        let p = project("op-ok");
        let ctx = Arc::new(crate::ops::ctx(&p, None).unwrap());
        let jobs: Jobs = Default::default();
        let edited = ROCK.replace("seed = 7", "seed = 9");
        let id = generate(&jobs, ctx.clone(), "rock", Some(3), Some(&edited)).unwrap();
        let first = finish(&jobs, id);
        let id = generate(&jobs, ctx, "rock", None, Some(&edited)).unwrap();
        let second = finish(&jobs, id);
        let spec = std::fs::read_to_string(p.join("specs/rock.toml")).unwrap();
        let glbs = ["3", "9"].map(|s| p.join(format!("kiln/cache/generated/rock/{s}.glb")).is_file());
        std::fs::remove_dir_all(&p).unwrap();
        assert!(spec.contains("seed = 9"));
        assert_eq!(glbs, [true, true]);
        assert_eq!(first.spec.as_deref(), Some("kiln/cache/generated/rock/3.glb"));
        assert_eq!(second.spec.as_deref(), Some("kiln/cache/generated/rock/9.glb"));
    }

    #[test]
    fn generate_without_recipe_keeps_the_spec_bytes() {
        let p = project("op-keep");
        let ctx = Arc::new(crate::ops::ctx(&p, None).unwrap());
        let jobs: Jobs = Default::default();
        let id = generate(&jobs, ctx, "rock", None, None).unwrap();
        let done = finish(&jobs, id);
        let spec = std::fs::read_to_string(p.join("specs/rock.toml")).unwrap();
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!((done.status.as_str(), spec.as_str()), ("done", ROCK));
        assert_eq!(done.spec.as_deref(), Some("kiln/cache/generated/rock/7.glb"));
    }

    fn slow_job(jobs: &Jobs) -> u64 {
        agent::job(jobs, "x".into(), |say| {
            say("uno".into());
            std::thread::sleep(Duration::from_millis(300));
            say("dos".into());
            Ok(Some("x.glb".into()))
        })
    }

    #[test]
    fn job_wait_returns_news_then_the_file() {
        let jobs: Jobs = Default::default();
        let id = slow_job(&jobs);
        let t = Instant::now();
        let a = job_wait(&jobs, id, 0, Duration::from_secs(60)).unwrap();
        assert!(t.elapsed() < Duration::from_secs(5));
        assert!(!a.lines.is_empty() && a.next == a.lines.len());
        let b = job_wait(&jobs, id, a.next, Duration::from_secs(60)).unwrap();
        let c = if b.status == "running" { job_wait(&jobs, id, b.next, Duration::from_secs(60)).unwrap() } else { b };
        assert_eq!((c.status.as_str(), c.file.as_deref()), ("done", Some("x.glb")));
        assert!(!c.lines.contains(&"uno".to_string()));
        assert_eq!(jobs_list(&jobs)[0].lines, 2);
        assert!(job_get(&jobs, 0).is_some());
    }

    #[test]
    fn job_wait_gathers_a_burst_of_lines_in_one_call() {
        let jobs: Jobs = Default::default();
        let id = agent::job(&jobs, "burst".into(), |say| {
            for n in 0..6 {
                say(format!("línea {n}"));
                std::thread::sleep(Duration::from_millis(150));
            }
            std::thread::sleep(Duration::from_secs(3));
            Ok(None)
        });
        let t = Instant::now();
        let v = job_wait(&jobs, id, 0, Duration::from_secs(30)).unwrap();
        assert_eq!(v.lines.len(), 6, "{:?}", v.lines);
        assert_eq!(v.status, "running");
        assert!(t.elapsed() < Duration::from_millis(2900), "{:?}", t.elapsed());
    }

    #[test]
    fn job_wait_unknown_and_timeout() {
        let jobs: Jobs = Default::default();
        for id in [0, 7] {
            assert_eq!(job_wait(&jobs, id, 0, Duration::ZERO).unwrap_err(), "no existe ese trabajo");
        }
        let id = agent::job(&jobs, "y".into(), |_| {
            std::thread::sleep(Duration::from_millis(600));
            Ok(None)
        });
        let v = job_wait(&jobs, id, 0, Duration::from_millis(100)).unwrap();
        assert_eq!(v.status, "running");
        assert!(v.lines.is_empty());
    }

    #[test]
    fn review_sheets_reads_the_last_round() {
        let t = testing::examples("");
        let c = testing::ctx(&t);
        let cfg = crate::production::config(&c.project, None).unwrap();
        let base = kiln_core::build::out_dir(&c.project, &cfg, "old_hoist").join("review");
        assert!(review_sheets(&c, "old_hoist", None).unwrap_err().contains("no tiene revisión"));
        assert!(review_sheets(&c, "../x", None).is_err());
        for n in [1, 2] {
            std::fs::create_dir_all(base.join(format!("round-{n}"))).unwrap();
        }
        let d = base.join("round-2");
        std::fs::write(d.join("1-turn.png"), "").unwrap();
        std::fs::write(d.join("measures.json"), r#"{"h":1}"#).unwrap();
        std::fs::write(base.join("review.json"), r#"{"rounds":[{"n":2,"review":{"verdict":"pass"}}]}"#).unwrap();
        let s = review_sheets(&c, "old_hoist", None).unwrap();
        assert_eq!((s.round, s.sheets.len()), (2, 1));
        assert!(s.sheets[0].path.ends_with("round-2/1-turn.png"));
        assert_eq!(s.measures.unwrap()["h"], 1);
        assert_eq!(s.review.unwrap()["verdict"], "pass");
        assert!(review_sheets(&c, "old_hoist", Some(1)).unwrap().sheets.is_empty());
        assert!(review_sheets(&c, "old_hoist", Some(9)).is_err());
    }

    #[test]
    fn generate_replaces_only_the_recipe_of_an_existing_spec() {
        use crate::generate::tests::{ROCK, project};
        let p = project("keep");
        let file = p.join("specs/rock.toml");
        std::fs::write(&file, format!("{ROCK}\n[[variants]]\nid = \"dark\"\n")).unwrap();
        std::fs::write(
            p.join("specs/crate.toml"),
            "[asset]\nid = \"crate\"\nname = \"Crate\"\n\n[[parts]]\nid = \"b\"\nwhat = \"a box\"\nsource = { kind = \"search\", query = \"box\" }\n",
        )
        .unwrap();
        let c = Arc::new(crate::ops::ctx(&p, None).unwrap());
        let jobs: Jobs = Default::default();
        let sent = ROCK.replace("seed = 7", "seed = 9");
        let ok = generate(&jobs, c.clone(), "rock", None, Some(&sent));
        let saved = std::fs::read_to_string(&file).unwrap();
        let before = std::fs::read(p.join("specs/crate.toml")).unwrap();
        let onto_parts = generate(&jobs, c, "crate", None, Some(&sent.replace("\"rock\"", "\"crate\"")));
        let after = std::fs::read(p.join("specs/crate.toml")).unwrap();
        if let Ok(id) = ok {
            let _ = job_wait(&jobs, id, 0, Duration::from_secs(30));
        }
        std::fs::remove_dir_all(&p).unwrap();
        assert!(ok.is_ok(), "{ok:?}");
        assert!(saved.contains("seed = 9") && saved.contains("dark"), "{saved}");
        assert!(onto_parts.unwrap_err().contains("generator_and_parts"));
        assert_eq!(before, after);
    }
}
