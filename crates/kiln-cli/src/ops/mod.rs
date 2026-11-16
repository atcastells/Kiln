//! What Kiln does for the page and for the MCP: each area takes Rust types
//! and returns plain values, with no HTTP and no MCP inside.

pub mod context;
pub mod materials;
pub mod new_game;
pub mod production;
pub mod read;
pub mod sources;
pub mod specs;

use crate::agent::Ctx;
use std::path::{Path, PathBuf};

/// The context of a project with a `kiln.toml` (or the given config).
pub fn ctx(project: &Path, config: Option<PathBuf>) -> Result<Ctx, String> {
    let cfg = match &config {
        Some(c) => kiln_core::Config::load_file(c)?,
        None => kiln_core::Config::load(project)?,
    };
    Ok(Ctx {
        project: project.to_path_buf(),
        config,
        specs_dir: cfg
            .assets
            .specs
            .clone()
            .unwrap_or_else(|| "kiln/specs".into()),
        roots: [
            project.to_path_buf(),
            project.join(&cfg.tools.build),
            project.join(&cfg.tools.cache),
        ]
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok().or(Some(p.clone())))
        .collect(),
        context_file: project.join(&cfg.context.file),
        layers: cfg.context.layers(),
    })
}

/// The spec file of an asset, absolute.
pub fn spec_file(project: &Path, cat: &kiln_core::Catalog, asset: &str) -> Result<PathBuf, String> {
    cat.specs
        .iter()
        .find(|s| s.asset.id == asset)
        .map(|s| project.join(&s.file))
        .ok_or(format!("no hay ficha con id «{asset}»"))
}

/// `p` relative to `root`, with forward slashes.
pub fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// An id that is safe in a file name: letters, digits, `-` and `_`.
pub fn check_id(what: &str, id: &str) -> Result<(), String> {
    if kiln_core::spec::plain_id(id) {
        Ok(())
    } else {
        Err(format!(
            "{what} «{}» no vale: solo letras, números, «-» y «_»",
            id.chars().take(40).collect::<String>()
        ))
    }
}

/// A text of at most `max` characters.
pub fn check_len(what: &str, text: &str, max: usize) -> Result<(), String> {
    let n = text.chars().count();
    if n > max {
        Err(format!("{what} pasa de {max} caracteres ({n})"))
    } else {
        Ok(())
    }
}

/// `path` (relative to the project, or absolute) resolved inside one of the
/// project's roots, following links. The file itself may not exist yet; its
/// folder must.
pub fn confine(ctx: &Ctx, path: &str) -> Result<PathBuf, String> {
    let outside = || format!("«{path}» queda fuera del proyecto");
    let full = ctx.project.join(path);
    let resolved = match std::fs::canonicalize(&full) {
        Ok(p) => p,
        Err(_) => {
            let name = full.file_name().filter(|n| *n != "..").ok_or_else(outside)?;
            let dir = full.parent().ok_or_else(outside)?;
            std::fs::canonicalize(dir).map_err(|_| outside())?.join(name)
        }
    };
    if ctx.roots.iter().any(|r| resolved.starts_with(r)) {
        Ok(resolved)
    } else {
        Err(outside())
    }
}

/// Test helpers shared by the areas.
#[cfg(test)]
pub mod testing {
    use std::path::{Path, PathBuf};

    /// A fresh copy of `examples/<sub>` (`""` is the examples root) in a
    /// unique temp folder, removed on drop.
    pub struct Temp(pub PathBuf);

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub fn examples(sub: &str) -> Temp {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(sub);
        let dst = std::env::temp_dir().join(format!(
            "kiln-ops-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        copy(&src, &dst);
        Temp(std::fs::canonicalize(&dst).unwrap())
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                copy(&p, &to.join(e.file_name()));
            } else {
                std::fs::copy(&p, to.join(e.file_name())).unwrap();
            }
        }
    }

    /// The context of such a copy.
    pub fn ctx(t: &Temp) -> crate::agent::Ctx {
        super::ctx(&t.0, None).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testing_copies_are_independent() {
        let a = testing::examples("");
        let b = testing::examples("");
        std::fs::write(a.0.join("kiln.toml"), "x").unwrap();
        assert_ne!(std::fs::read(a.0.join("kiln.toml")).unwrap(), std::fs::read(b.0.join("kiln.toml")).unwrap());
        let gone = a.0.clone();
        drop(a);
        assert!(!gone.exists());
    }

    fn examples() -> Ctx {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        ctx(&std::fs::canonicalize(dir).unwrap(), None).unwrap()
    }

    #[test]
    fn check_id_takes_plain_ids_only() {
        assert!(check_id("el asset", "old_hoist-2").is_ok());
        for bad in ["", "a/b", "../x", "a b", "a.toml", "x\\y"] {
            assert!(check_id("el asset", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn check_len_counts_characters_not_bytes() {
        assert!(check_len("la petición", &"ñ".repeat(500), 500).is_ok());
        let e = check_len("la petición", &"a".repeat(501), 500).unwrap_err();
        assert!(e.contains("500"), "{e}");
    }

    #[test]
    fn confine_keeps_paths_inside_the_project() {
        let c = examples();
        assert!(confine(&c, "kiln.toml").unwrap().starts_with(&c.project));
        assert!(confine(&c, "specs/new_one.toml").is_ok());
        for bad in ["../Cargo.toml", "specs/../../Cargo.toml", "/etc/passwd", ".."] {
            assert!(confine(&c, bad).is_err(), "{bad}");
        }
        assert!(confine(&c, "no_such_dir/x.toml").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn confine_follows_links_out_of_the_project() {
        let dir = std::env::temp_dir().join(format!("kiln-confine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("kiln.toml"), "[project]\nname = \"t\"\n").unwrap();
        let link = dir.join("out");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        let c = ctx(&std::fs::canonicalize(&dir).unwrap(), None).unwrap();
        assert!(confine(&c, "out/hosts").is_err());
        assert!(confine(&c, "out/new.toml").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
