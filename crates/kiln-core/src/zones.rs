//! How a catalog piece with a single material slot is split into zones, kept
//! per project in `kiln/zones.toml`, a table per piece:
//!
//! ```toml
//! ["harbor/barrel"]
//! split = "loose"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const FILE: &str = "kiln/zones.toml";
pub const MODES: [&str; 2] = ["loose", "texture"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Split {
    pub split: String,
}

pub type Zones = BTreeMap<String, Split>;

fn check(piece: &str, mode: &str) -> Result<(), String> {
    if MODES.contains(&mode) {
        return Ok(());
    }
    Err(format!(
        "{piece}: «{mode}» no es una manera de partir ({})",
        MODES.join(", ")
    ))
}

pub fn load(root: &Path) -> Result<Zones, String> {
    let path = root.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Zones::new());
    };
    let z: Zones = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for (piece, s) in &z {
        check(piece, &s.split)?;
    }
    Ok(z)
}

/// Sets (or with `mode` empty, removes) the split of one piece and writes the file.
pub fn set(root: &Path, piece: &str, mode: &str) -> Result<(), String> {
    let mut z = load(root)?;
    if mode.is_empty() {
        z.remove(piece);
    } else {
        check(piece, mode)?;
        z.insert(piece.into(), Split { split: mode.into() });
    }
    let path = root.join(FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string(&z).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn mode<'a>(z: &'a Zones, piece: &str) -> Option<&'a str> {
    z.get(piece).map(|s| s.split.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("kiln-zones-{name}-{}", std::process::id()))
    }

    #[test]
    fn sets_and_loads_a_split() {
        let dir = tmp("rt");
        assert!(load(&dir).unwrap().is_empty());
        set(&dir, "harbor/barrel", "loose").unwrap();
        let z = load(&dir).unwrap();
        assert_eq!(mode(&z, "harbor/barrel"), Some("loose"));
        assert_eq!(mode(&z, "harbor/crate"), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_mode_removes_the_piece() {
        let dir = tmp("rm");
        set(&dir, "harbor/barrel", "texture").unwrap();
        set(&dir, "harbor/barrel", "").unwrap();
        assert!(load(&dir).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_an_unknown_mode() {
        let dir = tmp("bad");
        let e = set(&dir, "harbor/barrel", "melt").unwrap_err();
        assert!(e.contains("melt"));
        std::fs::create_dir_all(dir.join("kiln")).unwrap();
        std::fs::write(dir.join(FILE), "[\"harbor/barrel\"]\nsplit = \"melt\"\n").unwrap();
        let e = load(&dir).unwrap_err();
        assert!(e.contains("harbor/barrel") && e.contains("melt"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
