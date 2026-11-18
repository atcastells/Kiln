pub mod cmap;
pub mod contrast;
pub mod rules;
pub mod target;
pub mod theme;

use std::path::{Path, PathBuf};

/// True when the name is absolute or has a `..` component (either separator), so it can leave
/// the project whatever is on disk.
pub fn names_outside(rel: &str) -> bool {
    let path = Path::new(rel);
    path.is_absolute()
        || rel.starts_with(['/', '\\'])
        || rel.split(['/', '\\']).any(|part| part == "..")
}

/// The real path of a file named relative to the project root. Rejects absolute names, `..`,
/// names that do not exist and links that lead out: such names come from files of the project
/// that anyone may have written. Same convention as `asset::sha256_inside`.
pub fn inside(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let name = Path::new(rel);
    if names_outside(rel) {
        return Err(format!("`{rel}`: la ruta sale del proyecto"));
    }
    let real = std::fs::canonicalize(root.join(name)).map_err(|_| format!("`{rel}`: no se puede leer"))?;
    let base = std::fs::canonicalize(root).map_err(|_| "no se puede leer la raíz del proyecto".to_string())?;
    if !real.starts_with(base) {
        return Err(format!("`{rel}`: la ruta sale del proyecto"));
    }
    Ok(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-inside-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn inside_accepts_a_file_of_the_project_and_returns_its_real_path() {
        let d = tmp("ok");
        std::fs::write(d.join("a.txt"), "x").unwrap();
        let real = inside(&d, "a.txt").unwrap();
        assert!(real.starts_with(std::fs::canonicalize(&d).unwrap()));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn inside_rejects_absolute_parent_missing_and_links_out() {
        let d = tmp("bad");
        let out = tmp("bad-out");
        std::fs::write(out.join("secret.txt"), "x").unwrap();
        std::fs::write(d.join("a.txt"), "x").unwrap();
        assert!(inside(&d, out.join("secret.txt").to_str().unwrap()).is_err());
        assert!(inside(&d, "../x").is_err());
        assert!(inside(&d, "sub/../a.txt").is_err());
        assert!(inside(&d, "missing.txt").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(out.join("secret.txt"), d.join("link.txt")).unwrap();
            assert!(inside(&d, "link.txt").is_err());
        }
        std::fs::remove_dir_all(&d).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
    }
}
