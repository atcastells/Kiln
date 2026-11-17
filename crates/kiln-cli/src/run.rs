//! The single place where Kiln launches Blender and Godot.

use kiln_core::Config;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Run {
    pub program: String,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub log: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Output {
    pub code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub log: Option<PathBuf>,
    timeout_s: u64,
}

impl Output {
    pub fn success(&self) -> bool {
        self.code == Some(0) && !self.timed_out
    }

    pub fn error_tail(&self, lines: usize) -> String {
        if self.timed_out {
            return format!("superó el plazo de {} s", self.timeout_s);
        }
        let mut tail: Vec<&str> = self.stderr.lines().rev().take(lines).collect();
        tail.reverse();
        tail.join(" | ")
    }

    /// What went wrong and where the whole record of the run is.
    pub fn failure(&self, what: &str) -> String {
        let log = self
            .log
            .as_ref()
            .map_or(String::new(), |l| format!(" (registro: {})", l.display()));
        format!("{what}: {}{log}", self.error_tail(6))
    }
}

pub fn is_python(tool: &str) -> bool {
    Path::new(tool)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("python"))
}

fn tool(cfg: &Config, program: &str, args: Vec<OsString>, work: &Path, name: &str) -> Run {
    // the tool runs from `work`: a relative path is resolved here, as before
    let given = Path::new(program);
    let program = if given.components().count() > 1 && given.is_relative() {
        std::path::absolute(given).map_or(program.to_string(), |p| p.to_string_lossy().into_owned())
    } else {
        program.to_string()
    };
    Run {
        program,
        args,
        cwd: work.to_path_buf(),
        timeout: Duration::from_secs(cfg.tools.timeout_s),
        log: Some(work.join(".kiln/logs").join(format!("{name}.log"))),
    }
}

/// Runs a Kiln script in Blender (or in a Python with `bpy`) from `work`;
/// the record of the run stays in `<work>/.kiln/logs/<name>.log`.
pub fn blender(
    cfg: &Config,
    script: &Path,
    args: &[&Path],
    work: &Path,
    name: &str,
) -> Result<Output, String> {
    let mut all: Vec<OsString> = Vec::new();
    if is_python(&cfg.tools.blender) {
        all.push(script.into());
    } else {
        all.extend(
            [
                "--background",
                "--factory-startup",
                "--disable-autoexec",
                "--python",
            ]
            .map(OsString::from),
        );
        all.push(script.into());
        all.push("--".into());
    }
    all.extend(args.iter().map(|a| OsString::from(*a)));
    run(&tool(cfg, &cfg.tools.blender, all, work, name))
}

/// Runs another tool of `[tools]` (a validator) from `work`, with the same record.
pub fn program(
    cfg: &Config,
    program: &str,
    args: Vec<OsString>,
    work: &Path,
    name: &str,
) -> Result<Output, String> {
    run(&tool(cfg, program, args, work, name))
}

/// Runs Godot without a window from `work`, with the same record as `blender`.
pub fn godot(cfg: &Config, args: Vec<OsString>, work: &Path, name: &str) -> Result<Output, String> {
    let mut all: Vec<OsString> = vec!["--headless".into()];
    all.extend(args);
    run(&tool(cfg, &cfg.tools.godot, all, work, name))
}

/// What a pipe has given so far, filled by a thread that reads it to its end.
type Drained = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

fn drain<R: std::io::Read + Send + 'static>(mut src: R) -> (Drained, std::thread::JoinHandle<()>) {
    let buf: Drained = Default::default();
    let sink = buf.clone();
    let reader = std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = src.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(&chunk[..n]);
        }
    });
    (buf, reader)
}

/// The text read so far. After a timeout the reader is given a moment and
/// then left behind: a process the tool started may keep the pipe open long
/// after the tool itself was killed, and the timeout must not wait for it.
fn collected(drained: (Drained, std::thread::JoinHandle<()>), timed_out: bool) -> String {
    let (buf, reader) = drained;
    if timed_out {
        let limit = std::time::Instant::now() + Duration::from_millis(500);
        while !reader.is_finished() && std::time::Instant::now() < limit {
            std::thread::sleep(Duration::from_millis(20));
        }
    } else {
        let _ = reader.join();
    }
    String::from_utf8_lossy(&buf.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
}

pub fn run(r: &Run) -> Result<Output, String> {
    use std::process::{Command, Stdio};
    let _ = std::fs::create_dir_all(&r.cwd);
    let started = std::time::Instant::now();
    let mut child = Command::new(&r.program)
        .args(&r.args)
        .current_dir(&r.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se puede lanzar {}: {e}", r.program))?;
    let out_t = drain(child.stdout.take().expect("piped stdout"));
    let err_t = drain(child.stderr.take().expect("piped stderr"));
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s.code(),
            Ok(None) if started.elapsed() >= r.timeout => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };
    let out = Output {
        code: status,
        timed_out,
        duration_ms: started.elapsed().as_millis(),
        stdout: collected(out_t, timed_out),
        stderr: collected(err_t, timed_out),
        log: r.log.clone(),
        timeout_s: r.timeout.as_secs(),
    };
    if let Some(path) = &r.log {
        write_log(path, r, &out);
    }
    Ok(out)
}

fn write_log(path: &std::path::Path, r: &Run, o: &Output) {
    let args: Vec<String> = r
        .args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let code = o.code.map_or("timeout".to_string(), |c| c.to_string());
    let text = format!(
        "program: {} {}\ncwd: {}\nexit: {code}\nduration_ms: {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        r.program,
        args.join(" "),
        r.cwd.display(),
        o.duration_ms,
        o.stdout,
        o.stderr
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = kiln_core::asset::write_replacing(path, text.as_bytes());
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Instant;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sh(script: &str, cwd: PathBuf, timeout_ms: u64, log: Option<PathBuf>) -> Run {
        Run {
            program: "sh".into(),
            args: vec!["-c".into(), script.into()],
            cwd,
            timeout: Duration::from_millis(timeout_ms),
            log,
        }
    }

    #[test]
    fn timeout_kills_the_child() {
        let d = dir("timeout");
        let t = Instant::now();
        let out = run(&sh("sleep 5", d.clone(), 200, None)).unwrap();
        assert!(out.timed_out);
        assert!(!out.success());
        assert!(t.elapsed() < Duration::from_secs(3));
        assert!(out.error_tail(6).contains("superó el plazo"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn failure_is_captured_and_logged() {
        let d = dir("fail");
        let log = d.join("logs").join("run.log");
        let out = run(&sh(
            "echo out; echo err 1>&2; exit 3",
            d.clone(),
            5000,
            Some(log.clone()),
        ))
        .unwrap();
        assert_eq!(out.code, Some(3));
        assert!(!out.success());
        assert_eq!(out.stdout, "out\n");
        assert_eq!(out.error_tail(6), "err");
        assert!(std::fs::read_to_string(&log).unwrap().contains("err"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn runs_in_the_given_cwd() {
        let d = dir("cwd");
        let out = run(&sh("pwd", d.clone(), 5000, None)).unwrap();
        assert!(out.success());
        let got = std::fs::canonicalize(out.stdout.trim()).unwrap();
        assert_eq!(got, std::fs::canonicalize(&d).unwrap());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn missing_program_is_an_error() {
        let d = dir("missing");
        let mut r = sh("", d.clone(), 1000, None);
        r.program = "kiln-no-such-program".into();
        let e = run(&r).unwrap_err();
        assert!(e.starts_with("no se puede lanzar"), "{e}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_grandchild_holding_the_pipes_does_not_stretch_the_timeout() {
        let dir = std::env::temp_dir().join(format!("kiln-run-grandchild-{}", std::process::id()));
        let started = std::time::Instant::now();
        let out = run(&Run {
            program: "sh".into(),
            // `sh` forks `sleep`, which keeps stdout open after `sh` is killed
            args: vec!["-c".into(), "echo before; sleep 6; echo after".into()],
            cwd: dir.clone(),
            timeout: Duration::from_millis(300),
            log: None,
        })
        .unwrap();
        assert!(out.timed_out);
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "{:?}",
            started.elapsed()
        );
        assert!(out.stdout.contains("before"), "{:?}", out.stdout);
        let _ = std::fs::remove_dir_all(dir);
    }
}
