//! The list of jobs, shared by every door into Kiln (the page and each MCP
//! session) and, in the core, kept on disk so that a job outlives the client
//! that launched it and the page finds it after a reload.
//!
//! A job's id is its place in the list plus one, so jobs are never removed:
//! the file keeps them all, with the log of old finished ones cut short.

use crate::agent::Job;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LockResult, Mutex, MutexGuard, Weak};
use std::time::Duration;

/// Finished jobs whose whole log is kept; older ones keep `OLD_LOG` lines.
const FULL_LOGS: usize = 200;
const OLD_LOG: usize = 40;
const SAVE_EVERY: Duration = Duration::from_millis(400);

/// What a job left running when the core stopped says on the next start.
pub const INTERRUPTED: &str = "Kiln se cerró mientras corría este trabajo: no terminó. Lánzalo otra vez.";

#[derive(Clone, Default)]
pub struct Jobs(Arc<Mutex<Vec<Job>>>);

impl Jobs {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, Vec<Job>>> {
        self.0.lock()
    }

    /// The jobs of `file`, saved back to it while this list lives. A job the
    /// file has as running did not survive the process that ran it, so it is
    /// marked failed. A missing or broken file starts an empty list.
    pub fn persistent(file: PathBuf) -> Jobs {
        let mut list = load(&file);
        for j in list.iter_mut().filter(|j| j.status == "running") {
            j.status = "failed".into();
            j.log.push(INTERRUPTED.into());
        }
        // ids are places in the list: a hand-edited file must not break that
        for (n, j) in list.iter_mut().enumerate() {
            j.id = n as u64 + 1;
        }
        let jobs = Jobs(Arc::new(Mutex::new(list)));
        let weak = Arc::downgrade(&jobs.0);
        std::thread::spawn(move || saver(weak, file));
        jobs
    }
}

fn load(file: &Path) -> Vec<Job> {
    std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// A cheap mark of what changed: count, statuses and log lengths.
fn mark(list: &[Job]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for j in list {
        (j.id, &j.status, j.log.len(), &j.spec, j.reply.is_some(), &j.project).hash(&mut h);
    }
    h.finish()
}

/// What goes to disk: every job, the log of old finished ones cut short.
fn for_disk(mut list: Vec<Job>) -> Vec<Job> {
    let finished: Vec<usize> = (0..list.len()).filter(|&i| list[i].status != "running").collect();
    for &i in finished.iter().rev().skip(FULL_LOGS) {
        let log = &mut list[i].log;
        if log.len() > OLD_LOG {
            log.drain(..log.len() - OLD_LOG);
        }
    }
    list
}

pub fn save(file: &Path, list: Vec<Job>) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = serde_json::to_vec(&for_disk(list)).map_err(|e| e.to_string())?;
    let tmp = file.with_extension(format!("json.{}", std::process::id()));
    std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|e| format!("{}: {e}", file.display()))
}

fn saver(jobs: Weak<Mutex<Vec<Job>>>, file: PathBuf) {
    let mut saved = None;
    loop {
        std::thread::sleep(SAVE_EVERY);
        let Some(jobs) = jobs.upgrade() else { return };
        let (now, copy) = {
            let list = jobs.lock().unwrap();
            let m = mark(&list);
            (m, (Some(m) != saved).then(|| list.clone()))
        };
        drop(jobs);
        if let Some(list) = copy {
            match save(&file, list) {
                Ok(()) => saved = Some(now),
                Err(e) => eprintln!("no se pudieron guardar los trabajos: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-jobs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn wait_for(file: &Path, what: impl Fn(&[Job]) -> bool) -> Vec<Job> {
        for _ in 0..100 {
            let list = load(file);
            if what(&list) {
                return list;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("{} never got there", file.display());
    }

    #[test]
    fn a_job_is_saved_as_it_runs_and_found_again_by_a_new_list() {
        let d = scratch("saved");
        let file = d.join("jobs.json");
        let jobs = Jobs::persistent(file.clone());
        let id = crate::agent::job(&jobs, "montar a".into(), |say| {
            say("uno".into());
            Ok(Some("a.glb".into()))
        });
        let list = wait_for(&file, |l| l.first().is_some_and(|j| j.status == "done"));
        assert_eq!((list[0].id, list[0].spec.as_deref()), (id, Some("a.glb")));
        drop(jobs);
        let again = Jobs::persistent(file.clone());
        let next = crate::agent::job(&again, "montar b".into(), |_| Ok(None));
        assert_eq!(next, 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_job_left_running_comes_back_failed() {
        let d = scratch("interrupted");
        let file = d.join("jobs.json");
        let jobs = Jobs::persistent(file.clone());
        crate::agent::job(&jobs, "montar a".into(), |_| {
            std::thread::sleep(Duration::from_secs(3));
            Ok(None)
        });
        wait_for(&file, |l| l.len() == 1);
        let back = Jobs::persistent(file.clone());
        let j = back.lock().unwrap()[0].clone();
        assert_eq!(j.status, "failed");
        assert_eq!(j.log.last().map(String::as_str), Some(INTERRUPTED));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_broken_file_starts_empty_and_old_logs_are_cut() {
        let d = scratch("broken");
        let file = d.join("jobs.json");
        std::fs::write(&file, "{no").unwrap();
        assert!(Jobs::persistent(file.clone()).lock().unwrap().is_empty());
        let job = |id: u64| Job {
            id,
            asked: "x".into(),
            status: "done".into(),
            log: (0..100).map(|n| n.to_string()).collect(),
            spec: None,
            reply: None,
            offer: None,
            kind: "x".into(),
            started: 0,
            project: None,
            writes: false,
        };
        let list: Vec<Job> = (1..=FULL_LOGS as u64 + 1).map(job).collect();
        let disk = for_disk(list);
        assert_eq!((disk[0].log.len(), disk[0].log[0].as_str()), (OLD_LOG, "60"));
        assert_eq!(disk[1].log.len(), 100);
        let _ = std::fs::remove_dir_all(&d);
    }
}
