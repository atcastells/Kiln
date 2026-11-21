//! The core: one Kiln process per person, with the page and every MCP
//! session as two doors into the same state. It keeps the jobs on disk (so a
//! job outlives the client that launched it), one write job at a time per
//! project, and the project the page shows, which follows the last one an
//! MCP session opened.
//!
//! The core listens on 127.0.0.1 for the page, as before, and on a Unix
//! socket in `$KILN_HOME` (only this user can open it) for `kiln mcp`, which
//! becomes a thin bridge over stdio: it starts the core when none is running
//! and forwards the protocol to it. Each connection opens with one JSON line
//! (`Hello`) and gets one JSON line back before anything else.
//!
//!   $KILN_HOME/core.lock   held while a core runs: only one per home
//!   $KILN_HOME/core.sock   the socket
//!   $KILN_HOME/core.json   pid, page port and version of the running core
//!   $KILN_HOME/core.log    what an auto-started core says
//!   $KILN_HOME/jobs.json   the jobs

use crate::agent::{Ctx, Jobs};
use crate::projects as registry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

pub const SOCKET: &str = "core.sock";
const LOCK: &str = "core.lock";
const INFO: &str = "core.json";
const LOG: &str = "core.log";
const JOBS: &str = "jobs.json";
/// An auto-started core leaves after this long with nobody using it.
const IDLE: Duration = Duration::from_secs(30 * 60);
const MAX_HELLO: u64 = 64 * 1024;
const MAX_LOG: u64 = 5 * 1024 * 1024;

/// What the page shows. `rev` changes when the folder changes, so an open
/// page knows it must load the new project.
#[derive(Default, Clone)]
pub struct Page {
    pub dir: Option<PathBuf>,
    /// `None` while the folder has no `kiln.toml` (the start wizard).
    pub ctx: Option<Arc<Ctx>>,
    pub rev: u64,
}

/// The state every door shares.
#[derive(Clone)]
pub struct Shared {
    pub jobs: Jobs,
    pub page: Arc<RwLock<Page>>,
    activity: Arc<Activity>,
}

struct Activity {
    sessions: AtomicUsize,
    last: Mutex<Instant>,
}

impl Default for Shared {
    fn default() -> Self {
        Shared::new(Jobs::default())
    }
}

impl Shared {
    pub fn new(jobs: Jobs) -> Shared {
        Shared {
            jobs,
            page: Default::default(),
            activity: Arc::new(Activity { sessions: AtomicUsize::new(0), last: Mutex::new(Instant::now()) }),
        }
    }

    /// The page shows `dir` from now on.
    pub fn show(&self, dir: &Path, ctx: Option<Arc<Ctx>>) {
        let mut p = self.page.write().unwrap();
        if p.dir.as_deref() != Some(dir) {
            p.rev += 1;
            p.dir = Some(dir.to_path_buf());
        }
        p.ctx = ctx;
    }

    /// Someone used the core (a request of the page, a session that ended).
    pub fn touch(&self) {
        *self.activity.last.lock().unwrap() = Instant::now();
    }

    fn idle_for(&self) -> Option<Duration> {
        let busy = self.activity.sessions.load(Ordering::SeqCst) > 0
            || self.jobs.lock().unwrap().iter().any(|j| j.status == "running");
        (!busy).then(|| self.activity.last.lock().unwrap().elapsed())
    }

    fn running(&self) -> usize {
        self.jobs.lock().unwrap().iter().filter(|j| j.status == "running").count()
    }
}

/// The first line of a connection.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Hello {
    /// An MCP session follows: what `kiln mcp` was started with.
    Mcp { project: Option<PathBuf>, config: Option<PathBuf>, cwd: Option<PathBuf> },
    /// `kiln serve --project` while a core runs: show that project.
    Open { project: PathBuf, config: Option<PathBuf> },
    /// `kiln stop`.
    Stop { force: bool },
}

/// The line the core answers a `Hello` with.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Answer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// What `core.json` says about the running core.
#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub pid: u32,
    pub port: Option<u16>,
    pub version: String,
}

pub fn info(home: &Path) -> Option<Info> {
    serde_json::from_slice(&std::fs::read(home.join(INFO)).ok()?).ok()
}

pub struct Opts {
    pub project: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub port: u16,
    pub ui_dir: Option<PathBuf>,
    /// Started by `kiln mcp`: leaves when idle and runs on without the page
    /// if its port is taken.
    pub auto: bool,
}

fn idle_limit() -> Duration {
    std::env::var("KILN_CORE_IDLE_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(IDLE)
}

/// `kiln serve`: the core when there is a home for it, else the page alone.
pub fn run(o: Opts) -> Result<(), String> {
    #[cfg(unix)]
    if let Some(home) = registry::home() {
        return unix::run(o, &home);
    }
    let shared = Shared::default();
    if let Some(p) = &o.project {
        crate::serve::open_page(&shared, p, o.config.clone())?;
    }
    let listener = crate::serve::bind(o.port)?;
    crate::serve::listen(listener, shared, o.ui_dir);
    Ok(())
}

/// `kiln serve` while a core already runs: that core shows the project.
/// `Ok(None)`: no core is running.
pub fn open_in_running(project: &Path, config: Option<&Path>) -> Result<Option<String>, String> {
    #[cfg(unix)]
    if let Some(home) = registry::home()
        && let Ok(s) = std::os::unix::net::UnixStream::connect(home.join(SOCKET))
    {
        let hello = Hello::Open { project: project.to_path_buf(), config: config.map(Path::to_path_buf) };
        let a = unix::ask(&s, &hello)?;
        return match a.error {
            Some(e) => Err(e),
            None => Ok(Some(a.url.unwrap_or_else(|| "(el núcleo no sirve la página)".into()))),
        };
    }
    let _ = (project, config);
    Ok(None)
}

/// `kiln stop`: the running core saves its jobs and leaves. Refused while a
/// job runs unless `force`.
pub fn stop(force: bool) -> Result<bool, String> {
    #[cfg(unix)]
    if let Some(home) = registry::home()
        && let Ok(s) = std::os::unix::net::UnixStream::connect(home.join(SOCKET))
    {
        let a = unix::ask(&s, &Hello::Stop { force })?;
        return a.error.map_or(Ok(true), Err);
    }
    let _ = force;
    Ok(false)
}

/// `kiln mcp` through the core. `Ok(false)`: there is no core to use (no
/// home, not Unix, `KILN_CORE=off`, or it would not start); the caller then
/// serves MCP in this process, as before.
pub fn bridge(project: Option<PathBuf>, config: Option<PathBuf>) -> Result<bool, String> {
    if std::env::var("KILN_CORE").is_ok_and(|v| v == "off") {
        return Ok(false);
    }
    #[cfg(unix)]
    if let Some(home) = registry::home() {
        return unix::bridge(&home, project, config);
    }
    let _ = (project, config);
    Ok(false)
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    /// Sends `hello` and reads the answer line.
    pub fn ask(s: &UnixStream, hello: &Hello) -> Result<Answer, String> {
        let mut w = s;
        let mut line = serde_json::to_vec(hello).map_err(|e| e.to_string())?;
        line.push(b'\n');
        w.write_all(&line).map_err(|e| format!("no se pudo hablar con el núcleo de Kiln: {e}"))?;
        let mut answer = String::new();
        BufReader::new(s.take(MAX_HELLO))
            .read_line(&mut answer)
            .map_err(|e| format!("el núcleo de Kiln no contestó: {e}"))?;
        serde_json::from_str(&answer).map_err(|_| "el núcleo de Kiln contestó algo ilegible".to_string())
    }

    pub fn run(o: Opts, home: &Path) -> Result<(), String> {
        std::fs::create_dir_all(home).map_err(|e| format!("{}: {e}", home.display()))?;
        let lock = std::fs::File::create(home.join(LOCK)).map_err(|e| format!("{}: {e}", home.display()))?;
        if lock.try_lock().is_err() {
            // another core got there first (two bridges started one each)
            if o.auto {
                return Ok(());
            }
            return Err(format!("ya hay un núcleo de Kiln en marcha en {}", home.display()));
        }
        let shared = Shared::new(Jobs::persistent(home.join(JOBS)));
        if let Some(p) = &o.project {
            crate::serve::open_page(&shared, p, o.config.clone())?;
        }
        let port = match crate::serve::bind(o.port) {
            Ok(l) => {
                let port = l.local_addr().map(|a| a.port()).ok();
                let (sh, ui) = (shared.clone(), o.ui_dir.clone());
                std::thread::spawn(move || crate::serve::listen(l, sh, ui));
                port
            }
            Err(e) if o.auto => {
                eprintln!("{e}: el núcleo sigue sin la página");
                None
            }
            Err(e) => return Err(e),
        };
        let sock = home.join(SOCKET);
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock).map_err(|e| format!("{}: {e}", sock.display()))?;
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600));
        }
        let info = Info { pid: std::process::id(), port, version: env!("CARGO_PKG_VERSION").into() };
        let _ = std::fs::write(home.join(INFO), serde_json::to_vec_pretty(&info).unwrap_or_default());
        eprintln!("núcleo de Kiln en {} (pid {})", sock.display(), info.pid);
        {
            let (sh, home) = (shared.clone(), home.to_path_buf());
            std::thread::spawn(move || sessions(listener, sh, home, port));
        }
        let limit = idle_limit();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if !home.is_dir() {
                std::process::exit(0); // the home went away (a test's)
            }
            if o.auto && shared.idle_for().is_some_and(|d| d >= limit) {
                eprintln!("el núcleo se cierra: nadie lo usa");
                leave(&shared, home);
            }
        }
    }

    /// Saves the jobs, removes the socket and `core.json`, and exits.
    fn leave(shared: &Shared, home: &Path) -> ! {
        let list = shared.jobs.lock().unwrap().clone();
        if let Err(e) = crate::jobs::save(&home.join(JOBS), list) {
            eprintln!("no se pudieron guardar los trabajos: {e}");
        }
        let _ = std::fs::remove_file(home.join(SOCKET));
        let _ = std::fs::remove_file(home.join(INFO));
        std::process::exit(0)
    }

    fn sessions(listener: UnixListener, shared: Shared, home: PathBuf, port: Option<u16>) {
        let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("no se pudo arrancar el núcleo: {e}");
                return;
            }
        };
        let _ = listener.set_nonblocking(true);
        rt.block_on(async move {
            let l = match tokio::net::UnixListener::from_std(listener) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("no se pudo escuchar en el socket: {e}");
                    return;
                }
            };
            loop {
                let Ok((s, _)) = l.accept().await else { continue };
                let (sh, home) = (shared.clone(), home.clone());
                tokio::spawn(async move { session(s, sh, home, port).await });
            }
        });
    }

    async fn session(s: tokio::net::UnixStream, shared: Shared, home: PathBuf, port: Option<u16>) {
        use rmcp::ServiceExt;
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
        let (r, mut w) = s.into_split();
        let mut r = tokio::io::BufReader::new(r);
        let mut line = String::new();
        if (&mut r).take(MAX_HELLO).read_line(&mut line).await.is_err() {
            return;
        }
        let answer = |a: Answer| {
            let mut v = serde_json::to_vec(&a).unwrap_or_default();
            v.push(b'\n');
            v
        };
        let url = port.map(|p| format!("http://127.0.0.1:{p}"));
        let hello: Hello = match serde_json::from_str(&line) {
            Ok(h) => h,
            Err(e) => {
                let _ = w.write_all(&answer(Answer { error: Some(format!("saludo ilegible: {e}")), url: None })).await;
                return;
            }
        };
        match hello {
            Hello::Mcp { project, config, cwd } => {
                let here: Vec<PathBuf> = cwd.into_iter().collect();
                let sh = shared.clone();
                let made = tokio::task::spawn_blocking(move || crate::mcp::Kiln::new(project, config, here, Some(sh)))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r);
                let kiln = match made {
                    Ok(k) => k,
                    Err(e) => {
                        let _ = w.write_all(&answer(Answer { error: Some(e), url: None })).await;
                        return;
                    }
                };
                if w.write_all(&answer(Answer { error: None, url })).await.is_err() {
                    return;
                }
                shared.activity.sessions.fetch_add(1, Ordering::SeqCst);
                match kiln.serve((r, w)).await {
                    Ok(server) => {
                        let _ = server.waiting().await;
                    }
                    Err(e) => eprintln!("sesión MCP fallida: {e}"),
                }
                shared.activity.sessions.fetch_sub(1, Ordering::SeqCst);
                shared.touch();
            }
            Hello::Open { project, config } => {
                let sh = shared.clone();
                let r = tokio::task::spawn_blocking(move || crate::serve::open_page(&sh, &project, config))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r);
                let a = match r {
                    Ok(()) => Answer { error: None, url },
                    Err(e) => Answer { error: Some(e), url: None },
                };
                let _ = w.write_all(&answer(a)).await;
            }
            Hello::Stop { force } => {
                let n = shared.running();
                if n > 0 && !force {
                    let e = format!("hay {n} trabajos en marcha: espera a que terminen o usa `kiln stop --force`");
                    let _ = w.write_all(&answer(Answer { error: Some(e), url: None })).await;
                    return;
                }
                let _ = w.write_all(&answer(Answer::default())).await;
                let _ = w.flush().await;
                eprintln!("el núcleo se cierra: `kiln stop`");
                leave(&shared, &home);
            }
        }
    }

    fn connect(sock: &Path) -> Option<UnixStream> {
        UnixStream::connect(sock).ok()
    }

    /// Starts a core in the background, in its own process group so that
    /// closing the client does not take it down.
    fn start_core(home: &Path) -> Result<(), String> {
        use std::os::unix::process::CommandExt;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let port = std::env::var("KILN_PORT").ok().filter(|p| p.parse::<u16>().is_ok()).unwrap_or_else(|| "4330".into());
        std::fs::create_dir_all(home).map_err(|e| format!("{}: {e}", home.display()))?;
        let log = home.join(LOG);
        if std::fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG) {
            let _ = std::fs::remove_file(&log);
        }
        let err = match std::fs::OpenOptions::new().create(true).append(true).open(&log) {
            Ok(f) => std::process::Stdio::from(f),
            Err(_) => std::process::Stdio::null(),
        };
        std::process::Command::new(exe)
            .args(["serve", "--core", "--port", &port])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(err)
            .process_group(0)
            .spawn()
            .map(drop)
            .map_err(|e| format!("no se pudo arrancar el núcleo de Kiln: {e}"))
    }

    pub fn bridge(home: &Path, project: Option<PathBuf>, config: Option<PathBuf>) -> Result<bool, String> {
        let sock = home.join(SOCKET);
        let s = match connect(&sock) {
            Some(s) => s,
            None => {
                if let Err(e) = start_core(home) {
                    eprintln!("{e}; Kiln MCP sigue sin núcleo");
                    return Ok(false);
                }
                let until = Instant::now() + Duration::from_secs(15);
                loop {
                    if let Some(s) = connect(&sock) {
                        break s;
                    }
                    if Instant::now() > until {
                        eprintln!("el núcleo de Kiln no arrancó (mira {}); Kiln MCP sigue sin núcleo", home.join(LOG).display());
                        return Ok(false);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };
        if let Some(i) = info(home)
            && i.version != env!("CARGO_PKG_VERSION")
        {
            eprintln!(
                "el núcleo en marcha es de kiln {} y este es {}: ciérralo con `kiln stop` para usar el nuevo",
                i.version,
                env!("CARGO_PKG_VERSION")
            );
        }
        let cwd = std::env::current_dir().ok();
        let hello = Hello::Mcp { project, config, cwd };
        let mut w = &s;
        let mut line = serde_json::to_vec(&hello).map_err(|e| e.to_string())?;
        line.push(b'\n');
        w.write_all(&line).map_err(|e| e.to_string())?;
        let mut r = BufReader::new(s.try_clone().map_err(|e| e.to_string())?);
        let mut first = String::new();
        r.read_line(&mut first).map_err(|e| format!("el núcleo de Kiln no contestó: {e}"))?;
        let a: Answer = serde_json::from_str(&first).map_err(|_| "el núcleo de Kiln contestó algo ilegible".to_string())?;
        if let Some(e) = a.error {
            return Err(e);
        }
        match &a.url {
            Some(u) => eprintln!("Kiln MCP por stdio, núcleo en {} (página en {u})", sock.display()),
            None => eprintln!("Kiln MCP por stdio, núcleo en {}", sock.display()),
        }
        let up = s.try_clone().map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            let mut up = up;
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut up);
            let _ = up.shutdown(std::net::Shutdown::Write);
        });
        let mut out = std::io::stdout().lock();
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            if out.write_all(&buf[..n]).and_then(|_| out.flush()).is_err() {
                break;
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_rev_changes_only_with_the_folder() {
        let s = Shared::default();
        s.show(Path::new("/a"), None);
        let r1 = s.page.read().unwrap().rev;
        s.show(Path::new("/a"), None);
        assert_eq!(s.page.read().unwrap().rev, r1);
        s.show(Path::new("/b"), None);
        assert_eq!(s.page.read().unwrap().rev, r1 + 1);
    }

    #[test]
    fn hello_lines_round_trip() {
        let h = Hello::Mcp { project: Some("/p".into()), config: None, cwd: Some("/c".into()) };
        let t = serde_json::to_string(&h).unwrap();
        assert!(t.contains("\"mode\":\"mcp\""), "{t}");
        assert!(matches!(serde_json::from_str::<Hello>(&t).unwrap(), Hello::Mcp { .. }));
        let s: Hello = serde_json::from_str(r#"{"mode":"stop","force":false}"#).unwrap();
        assert!(matches!(s, Hello::Stop { force: false }));
    }
}
