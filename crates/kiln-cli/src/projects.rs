//! The projects this user has opened, newest first, kept in one file for
//! every `kiln` process: `$KILN_HOME/projects.json`, or the platform's data
//! folder when `KILN_HOME` is not set.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE: &str = "projects.json";
const MAX: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    /// Unix seconds.
    pub last_opened: u64,
}

/// Where the registry lives.
pub fn home() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("KILN_HOME").filter(|h| !h.is_empty()) {
        return Some(PathBuf::from(h));
    }
    let user = std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return user.map(|h| h.join("Library/Application Support/Kiln"));
    }
    match std::env::var_os("XDG_DATA_HOME").filter(|h| !h.is_empty()) {
        Some(x) => Some(PathBuf::from(x).join("kiln")),
        None => user.map(|h| h.join(".local/share/kiln")),
    }
}

/// The entries in `dir`, newest first. A missing or broken file is empty.
pub fn list(dir: &Path) -> Vec<Entry> {
    let mut v: Vec<Entry> = std::fs::read(dir.join(FILE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    v.sort_by_key(|e| std::cmp::Reverse(e.last_opened));
    v
}

/// Puts `path` first with `name` and `now`, keeping at most `MAX` entries.
pub fn touch(dir: &Path, path: &Path, name: &str, now: u64) -> Result<(), String> {
    let mut v = list(dir);
    v.retain(|e| e.path != path);
    v.insert(0, Entry { name: name.to_string(), path: path.to_path_buf(), last_opened: now });
    v.truncate(MAX);
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let json = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{FILE}.{}", std::process::id()));
    std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, dir.join(FILE)).map_err(|e| format!("{}: {e}", dir.display()))
}

const TRUSTED: &str = "trusted.json";

/// The folders the person trusts to run Blender, Godot and Claude in: a
/// project's `kiln.toml` names those programs and its Godot project runs its
/// own editor plugins. Only `kiln trust` and the projects Kiln creates add to it.
pub fn trusted(dir: &Path) -> Vec<PathBuf> {
    std::fs::read(dir.join(TRUSTED))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn is_trusted(dir: &Path, path: &Path) -> bool {
    trusted(dir).iter().any(|p| p == path)
}

pub fn trust(dir: &Path, path: &Path) -> Result<(), String> {
    let mut v = trusted(dir);
    if v.iter().any(|p| p == path) {
        return Ok(());
    }
    v.push(path.to_path_buf());
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let json = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{TRUSTED}.{}", std::process::id()));
    std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, dir.join(TRUSTED)).map_err(|e| format!("{}: {e}", dir.display()))
}

/// What a folder holds, for choosing and listing projects.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Kind {
    pub exists: bool,
    pub kiln: bool,
    pub godot: bool,
}

pub fn kind(path: &Path) -> Kind {
    Kind {
        exists: path.is_dir(),
        kiln: path.join(kiln_core::config::FILE).is_file(),
        godot: path.join("project.godot").is_file(),
    }
}

/// A folder that does not exist yet or holds nothing: Kiln writes all of it.
pub fn fresh(path: &Path) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut d) => d.next().is_none(),
        Err(_) => !path.exists(),
    }
}

/// The first folder worth opening without asking: one with `kiln.toml` or
/// `project.godot` among `candidates`, else the newest registered project
/// that still has `kiln.toml`.
pub fn pick(candidates: &[PathBuf], recent: &[Entry]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|c| {
            let k = kind(c);
            k.kiln || k.godot
        })
        .cloned()
        .or_else(|| recent.iter().find(|e| kind(&e.path).kiln).map(|e| e.path.clone()))
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn dir() -> Dir {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "kiln-projects-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }

    #[test]
    fn a_missing_or_broken_file_is_an_empty_list() {
        let d = dir();
        assert!(list(&d.0).is_empty());
        std::fs::write(d.0.join(FILE), "{not json").unwrap();
        assert!(list(&d.0).is_empty());
    }

    #[test]
    fn touch_puts_the_project_first_once() {
        let d = dir();
        let home = d.0.join("home");
        touch(&home, Path::new("/a"), "A", 1).unwrap();
        touch(&home, Path::new("/b"), "B", 2).unwrap();
        touch(&home, Path::new("/a"), "A2", 3).unwrap();
        let v = list(&home);
        assert_eq!(v.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["A2", "B"]);
        assert_eq!(v[0].last_opened, 3);
    }

    #[test]
    fn trust_adds_a_folder_once_and_nothing_else_is_trusted() {
        let d = dir();
        let home = d.0.join("home");
        assert!(!is_trusted(&home, Path::new("/a")));
        trust(&home, Path::new("/a")).unwrap();
        trust(&home, Path::new("/a")).unwrap();
        assert!(is_trusted(&home, Path::new("/a")));
        assert!(!is_trusted(&home, Path::new("/a/b")));
        assert_eq!(trusted(&home).len(), 1);
        std::fs::write(home.join(TRUSTED), "broken").unwrap();
        assert!(!is_trusted(&home, Path::new("/a")));
    }

    #[test]
    fn only_a_missing_or_empty_folder_is_fresh() {
        let d = dir();
        assert!(fresh(&d.0));
        assert!(fresh(&d.0.join("missing")));
        std::fs::write(d.0.join("project.godot"), "").unwrap();
        assert!(!fresh(&d.0));
    }

    #[test]
    fn touch_keeps_at_most_max() {
        let d = dir();
        for i in 0..(MAX as u64 + 5) {
            touch(&d.0, &PathBuf::from(format!("/p{i}")), "p", i).unwrap();
        }
        let v = list(&d.0);
        assert_eq!(v.len(), MAX);
        assert_eq!(v[0].path, PathBuf::from(format!("/p{}", MAX + 4)));
    }

    #[test]
    fn pick_prefers_a_candidate_project_then_the_newest_kiln_project() {
        let d = dir();
        let plain = d.0.join("plain");
        let godot = d.0.join("godot");
        let kiln = d.0.join("kiln");
        let gone = d.0.join("gone");
        for p in [&plain, &godot, &kiln] {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(godot.join("project.godot"), "").unwrap();
        std::fs::write(kiln.join("kiln.toml"), "").unwrap();
        let recent = vec![
            Entry { name: "gone".into(), path: gone.clone(), last_opened: 3 },
            Entry { name: "kiln".into(), path: kiln.clone(), last_opened: 2 },
        ];
        assert_eq!(pick(&[plain.clone(), godot.clone()], &recent), Some(godot));
        assert_eq!(pick(std::slice::from_ref(&plain), &recent), Some(kiln));
        assert_eq!(pick(&[plain], &[]), None);
    }
}
