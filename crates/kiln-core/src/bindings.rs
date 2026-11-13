//! Which palette material each material slot of a piece takes. Kept in one
//! TOML file per project (`[materials] bindings`), a table per piece:
//!
//! ```toml
//! ["props/barrel"]
//! Wood = "oldwood"
//! ```

use crate::catalog::{Catalog, Finding, Severity};
use crate::config::Config;
use std::collections::BTreeMap;
use std::path::Path;

pub type Bindings = BTreeMap<String, BTreeMap<String, String>>;

pub fn load(root: &Path, config: &Config) -> Result<Bindings, String> {
    let path = root.join(&config.materials.bindings);
    match std::fs::read_to_string(&path) {
        Ok(t) => toml::from_str(&t).map_err(|e| format!("{}: {e}", path.display())),
        Err(_) => Ok(Bindings::new()),
    }
}

/// Binds (or with `material` empty, unbinds) one slot and writes the file.
pub fn set(
    root: &Path,
    config: &Config,
    piece: &str,
    slot: &str,
    material: &str,
) -> Result<(), String> {
    let mut b = load(root, config)?;
    if material.is_empty() {
        if let Some(t) = b.get_mut(piece) {
            t.remove(slot);
            if t.is_empty() {
                b.remove(piece);
            }
        }
    } else {
        b.entry(piece.into())
            .or_default()
            .insert(slot.into(), material.into());
    }
    let path = root.join(&config.materials.bindings);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string(&b).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Puts the bindings on the pieces and flags those that point nowhere.
pub fn apply(b: &Bindings, cat: &mut Catalog) {
    let mut out = Vec::new();
    for (key, slots) in b {
        let Some(p) = cat.pieces.iter_mut().find(|p| &p.key == key) else {
            out.push((
                key.clone(),
                "binding_piece",
                "la pieza ya no está en el catálogo".to_string(),
            ));
            continue;
        };
        for (slot, mat) in slots {
            if !p.material_slots.is_empty() && !p.material_slots.contains(slot) {
                out.push((
                    key.clone(),
                    "binding_slot",
                    format!("la pieza no tiene el hueco «{slot}»"),
                ));
            }
            if !cat.materials.iter().any(|m| &m.key == mat) {
                out.push((
                    key.clone(),
                    "binding_material",
                    format!("«{mat}» no está en la paleta"),
                ));
            }
            p.bindings.insert(slot.clone(), mat.clone());
        }
    }
    cat.findings
        .extend(out.into_iter().map(|(subject, code, message)| Finding {
            severity: Severity::Warn,
            subject,
            code: code.into(),
            message,
        }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_and_unbinds_a_slot() {
        let dir = std::env::temp_dir().join(format!("kiln-bind-{}", std::process::id()));
        let cfg = Config::parse("[project]\nname = \"t\"").unwrap();
        set(&dir, &cfg, "kit/barrel", "Wood", "oak").unwrap();
        set(&dir, &cfg, "kit/barrel", "Iron", "iron").unwrap();
        assert_eq!(load(&dir, &cfg).unwrap()["kit/barrel"]["Wood"], "oak");
        set(&dir, &cfg, "kit/barrel", "Wood", "").unwrap();
        set(&dir, &cfg, "kit/barrel", "Iron", "").unwrap();
        assert!(load(&dir, &cfg).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
