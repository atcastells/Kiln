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

/// Publishes the asset `id`: copies what was verified, has Godot import it
/// and only then writes the contract with its version. If Godot does not
/// import it, the contract of the previous version stays.
pub fn publish_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<Published, String> {
    let cfg = crate::production::config(project, config_file)?;
    let staged = stage(project, &cfg, id)?;
    say(format!("«{id}» copiado a {}", staged.done.dir));
    import_in_godot(project, &cfg, &staged, &mut say)?;
    let done = commit(staged)?;
    say(format!(
        "«{id}» publicado en {} (versión {}{})",
        done.dir,
        done.version,
        if done.changed { "" } else { ", sin cambios" }
    ));
    Ok(done)
}

/// Godot imports what was copied when it lies inside its project. The
/// `.import` of each GLB is removed first, so finding it afterwards means
/// this run wrote it.
fn import_in_godot(
    project: &Path,
    cfg: &kiln_core::Config,
    staged: &Staged,
    say: &mut dyn FnMut(String),
) -> Result<(), String> {
    let done = &staged.done;
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
    for (f, _) in &staged.outputs {
        let _ = std::fs::remove_file(dest.join(format!("{f}.import")));
    }
    say(format!("Godot importa lo publicado ({})", cfg.tools.godot));
    let build = kiln_core::build::out_dir(project, cfg, &done.asset);
    let out = crate::run::godot(
        cfg,
        vec!["--path".into(), gp.into(), "--import".into()],
        &build,
        "godot-publish",
    )?;
    for (f, variant) in &staged.outputs {
        if out.timed_out || !dest.join(format!("{f}.import")).exists() {
            return Err(format!(
                "copiado a {}, pero Godot no importó {f}: {} (la versión publicada no cambia)",
                done.dir,
                out.error_tail(2)
            ));
        }
        let materials = match variant {
            Some(v) => format!("materials--{v}.json"),
            None => "materials.json".into(),
        };
        crate::production::link_godot(project, cfg, &dest.join(f), &build.join(materials), say);
    }
    Ok(())
}

/// What `stage` copied, waiting for its contract to be written.
struct Staged {
    done: Published,
    dest: std::path::PathBuf,
    /// The contract to write, with its version.
    contract: String,
    /// (file name, variant) of every GLB copied.
    outputs: Vec<(String, Option<String>)>,
}

/// Copies a verified asset to `[publish] dir`/<id>/ and writes its contract,
/// without Godot.
#[cfg(test)]
pub fn publish_files(
    project: &Path,
    cfg: &kiln_core::Config,
    id: &str,
) -> Result<Published, String> {
    commit(stage(project, cfg, id)?)
}

/// Writes the contract of what was staged: from here on it is published.
fn commit(staged: Staged) -> Result<Published, String> {
    let file = staged.dest.join("asset.json");
    let _ = std::fs::remove_file(&file);
    std::fs::write(&file, &staged.contract)
        .map_err(|e| format!("no se puede escribir asset.json: {e}"))?;
    Ok(staged.done)
}

/// Checks that the build is the verified one and copies its GLBs.
fn stage(project: &Path, cfg: &kiln_core::Config, id: &str) -> Result<Staged, String> {
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
    let reverify = format!("el GLB de «{id}» cambió después de verificarlo: vuelve a verificar");

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
    let listed = contract["outputs"]
        .as_array()
        .filter(|o| !o.is_empty())
        .ok_or(&rebuild)?;
    let mut outputs = Vec::new();
    for o in listed {
        let name = o["file"].as_str().ok_or(&rebuild)?;
        if Path::new(name).components().count() != 1 || !name.ends_with(".glb") {
            return Err(rebuild);
        }
        let sha = kiln_core::asset::sha256_file(&build.join(name)).map_err(|_| rebuild.clone())?;
        // every file must be the one the verification saw, the variants too
        if validation["outputs"][name].as_str() != Some(sha.as_str()) {
            return Err(reverify);
        }
        if o["sha256"].as_str() != Some(sha.as_str()) {
            return Err(rebuild);
        }
        outputs.push((name.to_string(), o["variant"].as_str().map(String::from)));
    }
    let names: Vec<String> = outputs.iter().map(|(n, _)| n.clone()).collect();

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
    for e in std::fs::read_dir(&dest)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with(".glb") && !names.contains(&n) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let mut published = contract.clone();
    published["version"] = json!(version);
    published["validation"] = json!({ "valid": true, "checks": validation["checks"] });

    Ok(Staged {
        done: Published {
            asset: id.to_string(),
            dir: super::rel(project, &dest),
            version,
            changed,
            files: names,
        },
        contract: serde_json::to_string_pretty(&published).map_err(|e| e.to_string())?,
        dest,
        outputs,
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
        let verified: serde_json::Map<String, Value> = outputs
            .iter()
            .map(|o| (o["file"].as_str().unwrap().to_string(), o["sha256"].clone()))
            .collect();
        let validation = json!({
            "asset": id, "glb_sha256": main_sha, "structure_sha256": "s", "outputs": verified,
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

    #[test]
    fn refuses_a_variant_painted_again_after_verifying() {
        let (p, c) = (project(), cfg(Some("game/assets")));
        make_build(&p, &c, "crate", &[(None, b"main"), (Some("red"), b"red")]);
        // what `paint --variant` does: a new file and a contract that follows it
        let dir = build_dir(&p, &c, "crate");
        std::fs::write(dir.join("crate--red.glb"), b"red, repainted").unwrap();
        let mut contract = read_json(&dir.join("asset.json"));
        contract["outputs"][1]["sha256"] =
            json!(kiln_core::asset::sha256_file(&dir.join("crate--red.glb")).unwrap());
        std::fs::write(dir.join("asset.json"), contract.to_string()).unwrap();
        let e = publish_files(&p.0, &c, "crate").unwrap_err();
        assert!(e.contains("cambió después de verificarlo"), "{e}");
        assert!(!dest(&p, "crate").join("crate--red.glb").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_godot_import_leaves_the_published_version_as_it_was() {
        use std::os::unix::fs::PermissionsExt;
        let p = project();
        let fake = p.0.join("fake-godot");
        std::fs::write(&fake, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let toml = format!(
            "[project]\nname = \"t\"\n[publish]\ndir = \"game/assets\"\n[tools]\ngodot = \"{}\"\ngodot_project = \"game\"\n",
            fake.display()
        );
        std::fs::write(p.0.join("kiln.toml"), &toml).unwrap();
        let c = kiln_core::Config::parse(&toml).unwrap();
        std::fs::create_dir_all(p.0.join("game")).unwrap();
        std::fs::write(p.0.join("game/project.godot"), "config_version=5\n").unwrap();
        make_build(&p, &c, "crate", &[(None, b"main")]);
        // an import left by an earlier publish must not count for this one
        std::fs::create_dir_all(dest(&p, "crate")).unwrap();
        std::fs::write(dest(&p, "crate").join("crate.glb.import"), b"old").unwrap();
        let e = publish_asset(&p.0, None, "crate", |_| {}).unwrap_err();
        assert!(e.contains("Godot no importó crate.glb"), "{e}");
        assert!(
            !dest(&p, "crate").join("asset.json").exists(),
            "no contract, no version"
        );
    }
}
