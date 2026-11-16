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
    Ok(agent::job(jobs, format!("montar {id}"), move |say| {
        let glb = crate::production::build_asset(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(super::rel(&ctx.project, &glb)))
    }))
}

pub fn paint(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str, variant: Option<String>) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    Ok(agent::job(jobs, format!("aplicar materiales a {id}"), move |say| {
        let v = variant.as_deref().filter(|v| !v.is_empty());
        let glb = crate::production::paint_asset(&ctx.project, ctx.config.as_deref(), &id, v, say)?;
        Ok(Some(super::rel(&ctx.project, &glb)))
    }))
}

pub fn godot_measure(jobs: &Jobs, ctx: Arc<Ctx>) -> u64 {
    agent::job(jobs, "medir con Godot".into(), move |say| {
        crate::production::measure(&ctx.project, ctx.config.as_deref(), say).map(|_| None)
    })
}

pub fn review(jobs: &Jobs, ctx: Arc<Ctx>, asset: &str) -> Result<u64, String> {
    if asset.is_empty() {
        return Err("falta el asset".into());
    }
    let id = asset.to_string();
    Ok(agent::job(jobs, format!("revisar {id}"), move |say| {
        let file = crate::review::run(&ctx.project, ctx.config.as_deref(), &id, say)?;
        Ok(Some(super::rel(&ctx.project, &file)))
    }))
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
    Ok(agent::job(jobs, format!("traer {site}/{id} para {part}"), move |say| {
        crate::fetch::import(&ctx.project, ctx.config.as_deref(), &asset, &part, &site, &id, say).map(Some)
    }))
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
}

pub fn jobs_list(jobs: &Jobs) -> Vec<JobSummary> {
    let j = jobs.lock().unwrap();
    j.iter()
        .map(|j| JobSummary { id: j.id, asked: j.asked.clone(), status: j.status.clone(), lines: j.log.len() })
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
    use crate::ops::testing;

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
}
