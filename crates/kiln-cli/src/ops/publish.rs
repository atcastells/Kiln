//! Publishing a verified asset: copies the build to the game's folder.

use serde_json::{Value, json};
use std::path::{Component, Path};

#[derive(Debug, serde::Serialize)]
pub struct Published {
    pub asset: String,
    /// Destination, relative to the project.
    pub dir: String,
    pub version: u64,
    /// False when what was already published is the same.
    pub changed: bool,
    pub files: Vec<String>,
}

fn read_json(path: &Path, missing: &str) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|_| missing.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{} no es JSON válido: {e}", path.display()))
}

/// Publishes the asset `id`: copies what was verified and has Godot import it.
pub fn publish_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<Published, String> {
    let cfg = crate::production::config(project, config_file)?;
    let done = publish_files(project, &cfg, id)?;
    say(format!(
        "«{id}» publicado en {} (versión {}{})",
        done.dir,
        done.version,
        if done.changed { "" } else { ", sin cambios" }
    ));
    import_in_godot(project, &cfg, &done, &mut say)?;
    Ok(done)
}

/// Godot imports what was published when it lies inside its project; the
/// import is judged by the `.import` file of each GLB.
fn import_in_godot(
    project: &Path,
    cfg: &kiln_core::Config,
    done: &Published,
    say: &mut dyn FnMut(String),
) -> Result<(), String> {
    let Some(gp) = &cfg.tools.godot_project else {
        say("Sin [tools] godot_project: Godot no importa lo publicado".into());
        return Ok(());
    };
    let canon = |p: std::path::PathBuf| {
        std::fs::canonicalize(&p).map_err(|e| format!("{}: {e}", p.display()))
    };
    let (gp, dest) = (canon(project.join(gp))?, canon(project.join(&done.dir))?);
    if !dest.starts_with(&gp) {
        say("Lo publicado queda fuera del proyecto de Godot: no se importa".into());
        return Ok(());
    }
    // without a project file Godot opens its project manager and never ends
    if !gp.join("project.godot").is_file() {
        say(format!(
            "{} no tiene project.godot: Godot no importa lo publicado",
            gp.display()
        ));
        return Ok(());
    }
    if kiln_core::tools::locate(&cfg.tools.godot).is_none() {
        say(format!(
            "No se encuentra «{}»: publicado sin importar en Godot",
            cfg.tools.godot
        ));
        return Ok(());
    }
    say(format!("Godot importa lo publicado ({})", cfg.tools.godot));
    let build = kiln_core::build::out_dir(project, cfg, &done.asset);
    let out = crate::run::godot(
        cfg,
        vec!["--path".into(), gp.into(), "--import".into()],
        &build,
        "godot-publish",
    )?;
    for f in &done.files {
        if out.timed_out || !dest.join(format!("{f}.import")).exists() {
            return Err(format!(
                "publicado en {}, pero Godot no importó {f}: {}",
                done.dir,
                out.error_tail(2)
            ));
        }
        let materials = match f.strip_suffix(".glb").and_then(|n| n.split_once("--")) {
            Some((_, variant)) => format!("materials--{variant}.json"),
            None => "materials.json".into(),
        };
        crate::production::link_godot(project, cfg, &dest.join(f), &build.join(materials), say);
    }
    Ok(())
}

/// Copies a verified asset to `[publish] dir`/<id>/ for the game to use.
pub fn publish_files(
    project: &Path,
    cfg: &kiln_core::Config,
    id: &str,
) -> Result<Published, String> {
    super::check_id("el asset", id)?;
    let dir = cfg
        .publish
        .dir
        .as_deref()
        .ok_or("falta [publish] dir en kiln.toml")?;
    let dir_path = Path::new(dir);
    if dir.is_empty()
        || dir_path.is_absolute()
        || dir_path.components().any(|c| c == Component::ParentDir)
    {
        return Err(format!(
            "[publish] dir «{dir}» debe quedar dentro del proyecto"
        ));
    }

    let build = kiln_core::build::out_dir(project, cfg, id);
    let validation = read_json(
        &build.join("validation.json"),
        &format!("verifica antes el asset: kiln verify --asset {id}"),
    )?;
    if validation["valid"] != true {
        return Err(format!(
            "la verificación de «{id}» no pasó: corrige y vuelve a verificar"
        ));
    }
    let main_sha =
        kiln_core::asset::sha256_file(&build.join(format!("{id}.glb"))).unwrap_or_default();
    if validation["glb_sha256"].as_str() != Some(main_sha.as_str()) {
        return Err(format!(
            "el GLB de «{id}» cambió después de verificarlo: vuelve a verificar"
        ));
    }

    let rebuild = format!(
        "el contrato de «{id}» no cuadra con la carpeta de build: monta y verifica de nuevo"
    );
    let contract = read_json(&build.join("asset.json"), &rebuild)?;
    let changed = kiln_core::asset::changed_pieces(project, &contract);
    if !changed.is_empty() {
        return Err(format!(
            "«{id}» se montó con otra versión de {}: monta y verifica de nuevo",
            changed.join(", ")
        ));
    }
    let outputs = contract["outputs"]
        .as_array()
        .filter(|o| !o.is_empty())
        .ok_or(&rebuild)?;
    let mut names = Vec::new();
    for o in outputs {
        let name = o["file"].as_str().ok_or(&rebuild)?;
        if Path::new(name).components().count() != 1 || !name.ends_with(".glb") {
            return Err(rebuild);
        }
        let sha = kiln_core::asset::sha256_file(&build.join(name)).map_err(|_| rebuild.clone())?;
        if o["sha256"].as_str() != Some(sha.as_str()) {
            return Err(rebuild);
        }
        names.push(name.to_string());
    }

    let dest = project.join(dir_path);
    let root = std::fs::canonicalize(project).map_err(|e| format!("proyecto: {e}"))?;
    let dest = dest.join(id);
    // a link on the way that leads out is refused before anything is created
    let standing = dest.ancestors().find(|a| a.exists()).unwrap_or(project);
    if !std::fs::canonicalize(standing).is_ok_and(|a| a.starts_with(&root)) {
        return Err(format!(
            "[publish] dir «{dir}» debe quedar dentro del proyecto"
        ));
    }
    std::fs::create_dir_all(&dest)
        .map_err(|e| format!("no se puede crear {}: {e}", dest.display()))?;
    let real = std::fs::canonicalize(&dest).map_err(|e| e.to_string())?;
    if !real.starts_with(&root) {
        return Err(format!(
            "[publish] dir «{dir}» debe quedar dentro del proyecto"
        ));
    }

    let old = read_json(&dest.join("asset.json"), "").unwrap_or(Value::Null);
    let old_version = old["version"].as_u64().unwrap_or(0);
    let changed = old.get("outputs") != contract.get("outputs");
    let version = if changed {
        old_version + 1
    } else {
        old_version
    };

    // what is there is removed first, so a link is replaced and never followed
    for name in &names {
        let _ = std::fs::remove_file(dest.join(name));
        std::fs::copy(build.join(name), dest.join(name))
            .map_err(|e| format!("no se puede copiar {name}: {e}"))?;
    }
    let mut published = contract.clone();
    published["version"] = json!(version);
    published["validation"] = json!({ "valid": true, "checks": validation["checks"] });
    let text = serde_json::to_string_pretty(&published).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(dest.join("asset.json"));
    std::fs::write(dest.join("asset.json"), text)
        .map_err(|e| format!("no se puede escribir asset.json: {e}"))?;
    for e in std::fs::read_dir(&dest)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with(".glb") && !names.contains(&n) {
            let _ = std::fs::remove_file(e.path());
        }
    }

    Ok(Published {
        asset: id.to_string(),
        dir: super::rel(project, &dest),
        version,
        changed,
        files: names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Proj(PathBuf);
    impl Drop for Proj {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn cfg(dir: Option<&str>) -> kiln_core::Config {
        let publish = dir
            .map(|d| format!("[publish]\ndir = \"{d}\"\n"))
            .unwrap_or_default();
        kiln_core::Config::parse(&format!("[project]\nname = \"t\"\n{publish}")).unwrap()
    }

    fn project() -> Proj {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "kiln-publish-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Proj(std::fs::canonicalize(&p).unwrap())
    }

    fn build_dir(p: &Proj, c: &kiln_core::Config, id: &str) -> PathBuf {
        kiln_core::build::out_dir(&p.0, c, id)
    }

    /// Writes a consistent build: one GLB per `(variant, bytes)`, its contract
    /// and a passing validation.
    fn make_build(p: &Proj, c: &kiln_core::Config, id: &str, files: &[(Option<&str>, &[u8])]) {
        let dir = build_dir(p, c, id);
        std::fs::create_dir_all(&dir).unwrap();
        let mut outputs = Vec::new();
        let mut main_sha = String::new();
        for (variant, bytes) in files {
            let name = match variant {
                Some(v) => format!("{id}--{v}.glb"),
                None => format!("{id}.glb"),
            };
            std::fs::write(dir.join(&name), bytes).unwrap();
            let sha = kiln_core::asset::sha256_file(&dir.join(&name)).unwrap();
            if variant.is_none() {
                main_sha = sha.clone();
            }
            outputs.push(
                json!({ "variant": variant, "file": name, "sha256": sha, "structure_sha256": "s" }),
            );
        }
        let asset = json!({ "id": id, "outputs": outputs });
        std::fs::write(dir.join("asset.json"), asset.to_string()).unwrap();
        let validation = json!({
            "asset": id, "glb_sha256": main_sha, "structure_sha256": "s",
            "valid": true, "checks": [{ "name": "tris", "ok": true }], "findings": []
        });
        std::fs::write(dir.join("validation.json"), validation.to_string()).unwrap();
    }

    fn dest(p: &Proj, id: &str) -> PathBuf {
        p.0.join("game/assets").join(id)
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn publishes_a_verified_asset() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main"), (Some("red"), b"red")]);
        let r = publish_files(&p.0, &c, "crate").unwrap();
        assert_eq!((r.version, r.changed), (1, true));
        assert_eq!(r.dir, "game/assets/crate");
        assert_eq!(
            std::fs::read(dest(&p, "crate").join("crate.glb")).unwrap(),
            b"main"
        );
        assert_eq!(
            std::fs::read(dest(&p, "crate").join("crate--red.glb")).unwrap(),
            b"red"
        );
        let a = read_json(&dest(&p, "crate").join("asset.json"));
        assert_eq!(a["version"], 1);
        assert_eq!(a["validation"]["valid"], true);
        assert_eq!(a["validation"]["checks"][0]["name"], "tris");
    }

    #[test]
    fn publishing_the_same_build_keeps_the_version() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        publish_files(&p.0, &c, "crate").unwrap();
        std::fs::remove_file(dest(&p, "crate").join("crate.glb")).unwrap();
        let r = publish_files(&p.0, &c, "crate").unwrap();
        assert_eq!((r.version, r.changed), (1, false));
        assert!(dest(&p, "crate").join("crate.glb").exists());
    }

    #[test]
    fn a_changed_build_bumps_the_version() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        publish_files(&p.0, &c, "crate").unwrap();
        make_build(&p, &c, "crate", &[(None, b"main v2")]);
        let r = publish_files(&p.0, &c, "crate").unwrap();
        assert_eq!((r.version, r.changed), (2, true));
        assert_eq!(
            read_json(&dest(&p, "crate").join("asset.json"))["version"],
            2
        );
    }

    #[test]
    fn refuses_without_a_passing_validation() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        let v = build_dir(&p, &c, "crate").join("validation.json");
        let good = std::fs::read_to_string(&v).unwrap();
        std::fs::write(&v, good.replace("\"valid\":true", "\"valid\":false")).unwrap();
        let e = publish_files(&p.0, &c, "crate").unwrap_err();
        assert!(e.contains("no pasó"), "{e}");
        assert!(!dest(&p, "crate").join("crate.glb").exists());
        std::fs::remove_file(&v).unwrap();
        let e = publish_files(&p.0, &c, "crate").unwrap_err();
        assert!(e.contains("kiln verify --asset crate"), "{e}");
    }

    #[test]
    fn refuses_a_glb_changed_after_verifying() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        std::fs::write(build_dir(&p, &c, "crate").join("crate.glb"), b"other").unwrap();
        let e = publish_files(&p.0, &c, "crate").unwrap_err();
        assert!(e.contains("cambió después de verificarlo"), "{e}");
        assert!(!dest(&p, "crate").exists());
    }

    #[test]
    fn refuses_a_bad_publish_dir_or_id() {
        let p = project();
        let c = cfg(Some("game/assets"));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        let e = publish_files(&p.0, &cfg(None), "crate").unwrap_err();
        assert!(e.contains("falta [publish] dir"), "{e}");
        for bad in ["../out", "/tmp/out", "a/../../b"] {
            let e = publish_files(&p.0, &cfg(Some(bad)), "crate").unwrap_err();
            assert!(e.contains("debe quedar dentro del proyecto"), "{bad}: {e}");
        }
        assert!(publish_files(&p.0, &c, "../x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_link_that_leads_out() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        let out = project();
        make_build(&p, &c, "crate", &[(None, b"main")]);
        std::fs::create_dir_all(p.0.join("game")).unwrap();
        std::os::unix::fs::symlink(&out.0, p.0.join("game/assets")).unwrap();
        assert!(publish_files(&p.0, &c, "crate").is_err());
        assert!(!out.0.join("crate").join("crate.glb").exists());
    }

    #[test]
    fn removes_a_dropped_variant() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main"), (Some("red"), b"red")]);
        publish_files(&p.0, &c, "crate").unwrap();
        std::fs::write(dest(&p, "crate").join("notes.txt"), b"keep").unwrap();
        make_build(&p, &c, "crate", &[(None, b"main")]);
        let r = publish_files(&p.0, &c, "crate").unwrap();
        assert!(r.changed);
        assert!(!dest(&p, "crate").join("crate--red.glb").exists());
        assert!(dest(&p, "crate").join("crate.glb").exists());
        assert!(dest(&p, "crate").join("notes.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_file_link_in_the_destination_is_replaced_not_followed() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        let out = project();
        make_build(&p, &c, "crate", &[(None, b"main")]);
        let d = dest(&p, "crate");
        std::fs::create_dir_all(&d).unwrap();
        let (victim, fresh) = (out.0.join("victim.txt"), out.0.join("fresh.txt"));
        std::fs::write(&victim, b"untouched").unwrap();
        std::os::unix::fs::symlink(&victim, d.join("crate.glb")).unwrap();
        std::os::unix::fs::symlink(&fresh, d.join("asset.json")).unwrap();
        publish_files(&p.0, &c, "crate").unwrap();
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
        assert!(!fresh.exists());
        assert_eq!(std::fs::read(d.join("crate.glb")).unwrap(), b"main");
        assert!(
            !d.join("asset.json")
                .symlink_metadata()
                .unwrap()
                .is_symlink()
        );
    }

    #[test]
    fn refuses_a_build_whose_piece_changed_since() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main")]);
        std::fs::create_dir_all(p.0.join("kits/k")).unwrap();
        std::fs::write(p.0.join("kits/k/side.blend"), b"first").unwrap();
        let file = build_dir(&p, &c, "crate").join("asset.json");
        let mut contract: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        contract["source"] = json!({ "pieces": [{
            "key": "k/side", "file": "kits/k/side.blend",
            "sha256": kiln_core::asset::sha256_hex(b"first"),
        }] });
        std::fs::write(&file, contract.to_string()).unwrap();
        assert!(publish_files(&p.0, &c, "crate").is_ok());
        std::fs::write(p.0.join("kits/k/side.blend"), b"retouched").unwrap();
        let e = publish_files(&p.0, &c, "crate").unwrap_err();
        assert!(
            e.contains("k/side") && e.contains("monta y verifica de nuevo"),
            "{e}"
        );
    }
}
