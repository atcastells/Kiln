//! Turns a generated spec and a seed into a GLB in the cache: Kiln's own
//! Godot script runs headless in an empty project inside the cache.

use std::path::{Path, PathBuf};

use kiln_core::generate::resolve::{resolve, Resolved};

const SCRIPT: &str = include_str!("../godot/generate.gd");
pub const BUILDER: &str = include_str!("../godot/kiln_generator.gd");

pub fn builder_sha256() -> String {
    kiln_core::asset::sha256_hex(BUILDER.as_bytes())
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Generated {
    pub glb: PathBuf,
    pub rel: String,
    pub seed: u32,
    pub resolved: Resolved,
    pub out: serde_json::Value,
    pub report: serde_json::Value,
}

pub const GENERATOR_VERSION: u32 = 1;

/// A folder of the cache that must lie inside the project, like the verify
/// scratch, but a sibling that never wipes the verification of the asset.
pub(crate) fn cache_dir(project: &Path, cache: &str, tail: &Path) -> Result<PathBuf, String> {
    let outside = || format!("[tools] cache «{cache}» debe quedar dentro del proyecto");
    let rel = Path::new(cache);
    if rel.is_absolute() || rel.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(outside());
    }
    let root = std::fs::canonicalize(project).map_err(|e| e.to_string())?;
    let dir = project.join(rel).join(tail);
    let standing = dir.ancestors().find(|a| a.exists()).unwrap_or(project);
    if !std::fs::canonicalize(standing).is_ok_and(|a| a.starts_with(&root)) {
        return Err(outside());
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
    if dir.starts_with(&root) { Ok(dir) } else { Err(outside()) }
}

pub fn generate_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    seed: Option<u32>,
    mut say: impl FnMut(String),
) -> Result<Generated, String> {
    crate::ops::check_id("el asset", id)?;
    let cfg = crate::production::config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let g = spec
        .generator
        .as_ref()
        .ok_or(format!("«{id}» no es un asset generado"))?;
    if !spec.parts.is_empty() {
        return Err(format!("la ficha «{id}» lleva receta y piezas: es una cosa o la otra"));
    }
    let palette: Vec<&str> = cat.materials.iter().map(|m| m.key.as_str()).collect();
    let found = kiln_core::generate::check::check(id, g, spec.asset.budget_tris, &palette);
    let (errors, warnings): (Vec<_>, Vec<_>) = found
        .iter()
        .partition(|f| f.severity == kiln_core::Severity::Error);
    if !errors.is_empty() {
        let all: Vec<&str> = errors.iter().map(|f| f.message.as_str()).collect();
        return Err(all.join("; "));
    }
    for w in warnings {
        say(format!("Aviso: {}", w.message));
    }
    let seed = seed.unwrap_or(g.seed);
    let resolved = resolve(g, seed);

    let _lock = lock(project, &cfg, id)?;
    let out_dir = cache_dir(project, &cfg.tools.cache, &Path::new("generated").join(id))?;
    let report_file = out_dir.join("generated.json");
    remove_entry(&report_file)?;
    let Run { glb, out } = run_seed(&cfg, project, id, &out_dir, &resolved, &mut say)?;
    let root = std::fs::canonicalize(project).map_err(|e| e.to_string())?;
    let rel = crate::ops::rel(&root, &glb);
    let segments = resolved
        .ops
        .iter()
        .find(|o| o.op == "sphere")
        .and_then(|o| o.params.get("segments"))
        .map_or(0, |s| *s as u64);
    let structure = kiln_core::glb::structure(&glb)?;
    let report = serde_json::json!({
        "asset": id,
        "seed": seed,
        "file": format!("{seed}.glb"),
        "glb_sha256": kiln_core::asset::sha256_file(&glb)?,
        "structure_sha256": kiln_core::glb::structure_hash(&structure),
        "recipe": g,
        "resolved": resolved,
        "size_m": out["size_m"],
        "min": out["min"],
        "tris": out["tris"],
        "tris_expected": kiln_core::generate::vocab::sphere_tris(segments),
        "godot": out["godot"],
        "generator_version": GENERATOR_VERSION,
    });
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&report_file, text).map_err(|e| e.to_string())?;
    say(format!("GLB generado en {rel}"));
    Ok(Generated { glb, rel, seed, resolved, out, report })
}

fn remove_entry(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(_) => Ok(()),
    }
    .map_err(|e| e.to_string())
}

/// Held while an asset is generated, verified or published: its folders in
/// the cache are shared, so one run at a time. Dropping it frees the asset.
#[derive(Debug)]
pub struct AssetLock(PathBuf);

impl Drop for AssetLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl AssetLock {
    /// Says the run is still alive: a lock is only taken over when it is
    /// older than any single tool run can be.
    pub fn touch(&self) {
        if let Ok(f) = std::fs::File::options().write(true).open(&self.0) {
            let _ = f.set_modified(std::time::SystemTime::now());
        }
    }
}

/// Takes the lock of `id` or says who must end first. A run that died
/// without freeing it is not obeyed for ever: past the time limit of a tool
/// plus a margin with no sign of life, the lock is taken over.
pub fn lock(project: &Path, cfg: &kiln_core::Config, id: &str) -> Result<AssetLock, String> {
    crate::ops::check_id("el asset", id)?;
    let dir = cache_dir(project, &cfg.tools.cache, &Path::new("generated").join(id))?;
    let file = dir.join(".lock");
    for attempt in 0..2 {
        // created only if nothing is there: a link left in its place is not followed
        match std::fs::File::options().write(true).create_new(true).open(&file) {
            Ok(mut f) => {
                use std::io::Write;
                let _ = write!(f, "{}", std::process::id());
                return Ok(AssetLock(file));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let idle = std::fs::symlink_metadata(&file)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok());
                if attempt == 0 && idle.is_some_and(|a| a.as_secs() > cfg.tools.timeout_s + 60) {
                    let _ = std::fs::remove_file(&file);
                    continue;
                }
                return Err(format!(
                    "«{id}» ya se está generando o verificando: espera a que termine (si no hay nada en marcha, borra {}/generated/{id}/.lock)",
                    cfg.tools.cache
                ));
            }
            Err(e) => return Err(format!("no se puede bloquear «{id}»: {e}")),
        }
    }
    Err(format!("no se puede bloquear «{id}»"))
}

/// `<cache>/generated/<id>/verify`, created empty.
pub fn verify_dir(project: &Path, cfg: &kiln_core::Config, id: &str) -> Result<PathBuf, String> {
    // the parent is resolved, the entry itself is not: a link left there is
    // removed as a link, never followed to what it points to
    let dir = cache_dir(project, &cfg.tools.cache, &Path::new("generated").join(id))?.join("verify");
    remove_entry(&dir)?;
    std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[derive(Debug)]
pub struct Run {
    pub glb: PathBuf,
    pub out: serde_json::Value,
}

/// Runs Godot for one resolved recipe inside `out_dir`; touches only
/// `.project` and `<seed>.glb` there.
pub fn run_seed(
    cfg: &kiln_core::Config,
    _project: &Path,
    id: &str,
    out_dir: &Path,
    resolved: &Resolved,
    say: &mut dyn FnMut(String),
) -> Result<Run, String> {
    let seed = resolved.seed;
    // Godot runs whatever a project holds: the work folder starts empty every
    // time, and nothing is written through a link left where an output goes.
    let work = out_dir.join(".project");
    let glb = out_dir.join(format!("{seed}.glb"));
    for old in [&work, &glb] {
        remove_entry(old)?;
    }
    std::fs::create_dir(&work).map_err(|e| e.to_string())?;
    let (script, job, result) = (work.join("generate.gd"), work.join("job.json"), work.join("out.json"));
    std::fs::write(work.join("project.godot"), crate::verify::SCRATCH_PROJECT).map_err(|e| e.to_string())?;
    std::fs::write(&script, SCRIPT).map_err(|e| e.to_string())?;
    std::fs::write(work.join("kiln_generator.gd"), BUILDER).map_err(|e| e.to_string())?;
    let spec_job = serde_json::json!({
        "id": id, "glb": glb, "material": resolved.material, "ops": resolved.ops,
    });
    std::fs::write(&job, spec_job.to_string()).map_err(|e| e.to_string())?;
    say(format!("Se resuelve la receta de «{id}» con la semilla {seed}"));

    let args = vec![
        "--path".into(), work.clone().into(), "--script".into(), script.into(),
        "--".into(), job.into(), (&result).into(),
    ];
    let run = crate::run::godot(cfg, args, &work, "godot-generate")?;
    let out: serde_json::Value = std::fs::read_to_string(&result)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null);
    if let Some(e) = out["error"].as_str() {
        return Err(e.to_string());
    }
    if !run.success() || !glb.is_file() || !out.is_object() {
        return Err(run.failure("Godot no generó el GLB"));
    }
    Ok(Run { glb, out })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    pub(crate) const FAKE:&str = "#!/bin/sh\nfor a; do job=$out; out=$a; done\nd=$(dirname \"$0\")\ncp \"$job\" \"$d/seen.json\"\ntouch \"$d/ran\"\nglb=$(sed -n 's/.*\"glb\":\"\\([^\"]*\\)\".*/\\1/p' \"$job\")\ncp \"$d/fixture.glb\" \"$glb\"\necho '{\"size_m\":[1,1,1],\"min\":[0,0,0],\"tris\":12,\"godot\":\"fake\"}' > \"$out\"\n";
    pub(crate) const ROCK:&str = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = [1, 5]\nsegments = 16\n\n[[generator.ops]]\nop = \"noise\"\namplitude = [0.1, 0.5]\nsize = 1\n";
    const BAD: &str = "[asset]\nid = \"bad\"\nname = \"Bad\"\n\n[generator]\nseed = 1\n\n[[generator.ops]]\nop = \"warp\"\n";
    const PARTS: &str = "[asset]\nid = \"crate\"\nname = \"Crate\"\n\n[[parts]]\nid = \"body\"\nwhat = \"a box\"\nsource = { kind = \"search\", query = \"box\" }\n";

    fn tiny_glb() -> Vec<u8> {
        let mut j = br#"{"scenes":[{"nodes":[0]}],"nodes":[{"name":"rock","mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,1]}]}"#.to_vec();
        while !j.len().is_multiple_of(4) {
            j.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((20 + j.len() as u32).to_le_bytes());
        out.extend((j.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(j);
        out
    }

    pub(crate) fn project(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kiln-generate-asset-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("specs")).unwrap();
        let tool = dir.join("fake-godot");
        std::fs::write(&tool, FAKE).unwrap();
        std::fs::write(dir.join("fixture.glb"), tiny_glb()).unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let toml = format!(
            "[project]\nname = \"t\"\n\n[assets]\nspecs = \"specs\"\n\n[tools]\ngodot = \"{}\"\n",
            tool.display()
        );
        std::fs::write(dir.join("kiln.toml"), toml).unwrap();
        for (n, t) in [("rock", ROCK), ("bad", BAD), ("crate", PARTS)] {
            std::fs::write(dir.join(format!("specs/{n}.toml")), t).unwrap();
        }
        std::fs::canonicalize(dir).unwrap()
    }

    #[test]
    fn generate_asset_fails_when_godot_leaves_nothing_even_with_an_old_glb() {
        let p = project("stale");
        generate_asset(&p, None, "rock", None, |_| {}).unwrap();
        std::fs::write(p.join("fake-godot"), "#!/bin/sh\nexit 0\n").unwrap();
        let again = generate_asset(&p, None, "rock", None, |_| {});
        let left = p.join("kiln/cache/generated/rock/7.glb").exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(again.is_err(), "an old GLB was reported as fresh");
        assert!(!left, "the old GLB survived a failed run");
    }

    #[test]
    fn generate_asset_starts_from_an_empty_work_folder_and_does_not_follow_links() {
        let p = project("wipe");
        generate_asset(&p, None, "rock", None, |_| {}).unwrap();
        let out = p.join("kiln/cache/generated/rock");
        std::fs::write(out.join(".project/override.cfg"), "[autoload]\n").unwrap();
        let outside = p.join("outside.glb");
        std::fs::write(&outside, b"keep").unwrap();
        std::fs::remove_file(out.join("7.glb")).unwrap();
        std::os::unix::fs::symlink(&outside, out.join("7.glb")).unwrap();
        generate_asset(&p, None, "rock", None, |_| {}).unwrap();
        let planted = out.join(".project/override.cfg").exists();
        let kept = std::fs::read(&outside).unwrap();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(!planted, "a file planted in the work folder survived");
        assert_eq!(kept, b"keep", "the GLB was written through a link");
    }

    #[test]
    fn generate_asset_refuses_a_spec_with_recipe_and_parts() {
        let p = project("both");
        let both = format!("{ROCK}\n[[parts]]\nid = \"b\"\nwhat = \"b\"\nsource = {{ kind = \"search\", query = \"box\" }}\n");
        std::fs::write(p.join("specs/rock.toml"), both).unwrap();
        let got = generate_asset(&p, None, "rock", None, |_| {});
        let ran = p.join("ran").exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(got.is_err() && !ran);
    }

    #[test]
    fn generate_asset_warns_about_a_material_outside_the_palette_and_goes_on() {
        let p = project("material");
        let spec = ROCK.replace("seed = 7\n", "seed = 7\nmaterial = \"basalt\"\n");
        std::fs::write(p.join("specs/rock.toml"), spec).unwrap();
        let mut said = Vec::new();
        let got = generate_asset(&p, None, "rock", None, |l| said.push(l));
        let made = got.as_ref().is_ok_and(|g| g.glb.is_file());
        std::fs::remove_dir_all(&p).unwrap();
        assert!(made, "{:?}", got.err());
        assert!(said.iter().any(|l| l.contains("«basalt» no está en la paleta")), "{said:?}");
    }

    #[test]
    fn generate_asset_sends_resolved_numbers_and_returns_the_glb() {
        let p = project("ok");
        let mut said = Vec::new();
        let got = generate_asset(&p, None, "rock", None, |l| said.push(l)).unwrap();
        let job: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p.join("seen.json")).unwrap()).unwrap();
        let builder =
            std::fs::read_to_string(p.join("kiln/cache/generated/rock/.project/kiln_generator.gd")).unwrap();
        let (rel, exists, seed) = (got.rel.clone(), got.glb.is_file(), got.seed);
        let written: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(p.join("kiln/cache/generated/rock/generated.json")).unwrap(),
        )
        .unwrap();
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(written["glb_sha256"], got.report["glb_sha256"]);
        assert_eq!((written["seed"].as_u64(), written["file"].as_str()), (Some(7), Some("7.glb")));
        assert_eq!(written["resolved"]["ops"][0]["op"], "sphere");
        assert_eq!((written["tris"].as_u64(), written["tris_expected"].as_u64()), (Some(12), Some(288)));
        assert_eq!(written["generator_version"], 1);
        assert_eq!(written["glb_sha256"].as_str().map(str::len), Some(64));
        assert_eq!(written["structure_sha256"].as_str().map(str::len), Some(64));
        assert_eq!(rel, "kiln/cache/generated/rock/7.glb");
        assert!(exists && seed == 7 && !said.is_empty());
        assert_eq!(got.out["godot"], "fake");
        assert_eq!(job["ops"][0]["op"], "sphere");
        assert_eq!(builder, BUILDER);
        let r = job["ops"][0]["params"]["radius_m"].as_f64().unwrap();
        assert!((1.0..=5.0).contains(&r));
        assert!(job["ops"][0]["params"].as_object().unwrap().values().all(|v| v.is_number()));
        assert!(job["ops"][1]["noise_seed"].is_number());
    }

    #[test]
    fn run_seed_into_the_verify_folder_leaves_the_preview_alone() {
        let p = project("verify-seed");
        let first = generate_asset(&p, None, "rock", Some(7), |_| {}).unwrap();
        let out = p.join("kiln/cache/generated/rock");
        let (glb_before, json_before) =
            (std::fs::read(out.join("7.glb")).unwrap(), std::fs::read(out.join("generated.json")).unwrap());
        let cfg = crate::production::config(&p, None).unwrap();
        let dir = verify_dir(&p, &cfg, "rock").unwrap();
        let mut runs = Vec::new();
        let recipe: kiln_core::generate::Generator = serde_json::from_value(first.report["recipe"].clone()).unwrap();
        for seed in [7u32, 8] {
            let g = resolve(&recipe, seed);
            runs.push(run_seed(&cfg, &p, "rock", &dir, &g, &mut |_| {}));
        }
        let (a, b) = (out.join("verify/7.glb").is_file(), out.join("verify/8.glb").is_file());
        let same = std::fs::read(out.join("7.glb")).unwrap() == glb_before
            && std::fs::read(out.join("generated.json")).unwrap() == json_before;
        let report = out.join("verify/generated.json").exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(runs.iter().all(|r| r.is_ok()), "{runs:?}");
        assert!(a && b, "the verify GLBs are missing");
        assert!(same, "the preview changed");
        assert!(!report, "verify/ got a generated.json");
    }

    #[test]
    fn verify_dir_starts_empty() {
        let p = project("verify-empty");
        let cfg = crate::production::config(&p, None).unwrap();
        let dir = verify_dir(&p, &cfg, "rock").unwrap();
        std::fs::write(dir.join("old.glb"), b"x").unwrap();
        let again = verify_dir(&p, &cfg, "rock").unwrap();
        let (left, same) = (again.join("old.glb").exists(), again == dir);
        std::fs::remove_dir_all(&p).unwrap();
        assert!(!left, "a file of the previous verification survived");
        assert!(same && again.ends_with("generated/rock/verify"));
    }

    #[test]
    fn verify_dir_removes_a_link_and_never_what_it_points_to() {
        let p = project("verify-link");
        let cfg = crate::production::config(&p, None).unwrap();
        let precious = p.join("precious");
        std::fs::create_dir_all(&precious).unwrap();
        std::fs::write(precious.join("important.txt"), b"keep").unwrap();
        let parent = p.join("kiln/cache/generated/rock");
        std::fs::create_dir_all(&parent).unwrap();
        std::os::unix::fs::symlink(&precious, parent.join("verify")).unwrap();
        let dir = verify_dir(&p, &cfg, "rock");
        let kept = std::fs::read(precious.join("important.txt")).ok();
        let real = dir.as_ref().is_ok_and(|d| !std::fs::symlink_metadata(d).unwrap().is_symlink());
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(kept.as_deref(), Some(&b"keep"[..]), "the folder behind the link was wiped");
        assert!(real, "{dir:?}");
    }

    #[test]
    fn generate_asset_refuses_parts_and_invalid_recipes_before_godot() {
        let p = project("refuse");
        let parts = generate_asset(&p, None, "crate", None, |_| {}).unwrap_err();
        let bad = generate_asset(&p, None, "bad", None, |_| {}).unwrap_err();
        let ran = p.join("ran").exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(parts, "«crate» no es un asset generado");
        assert!(!bad.is_empty() && !ran);
    }

    #[test]
    fn an_asset_is_locked_by_one_run_at_a_time() {
        let p = project("lock");
        let cfg = crate::production::config(&p, None).unwrap();
        let first = lock(&p, &cfg, "rock").unwrap();
        let second = lock(&p, &cfg, "rock").map(|_| ());
        let other = lock(&p, &cfg, "stone").is_ok();
        drop(first);
        let third = lock(&p, &cfg, "rock").is_ok();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(second.is_err_and(|e| e.contains("ya se está generando o verificando")));
        assert!(other, "another asset was locked too");
        assert!(third, "the lock outlived its run");
    }

    #[test]
    fn a_lock_left_by_a_dead_run_is_taken_over() {
        let p = project("lock-stale");
        let cfg = crate::production::config(&p, None).unwrap();
        std::mem::forget(lock(&p, &cfg, "rock").unwrap());
        let file = p.join("kiln/cache/generated/rock/.lock");
        let fresh = lock(&p, &cfg, "rock").is_err();
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(cfg.tools.timeout_s + 3600);
        std::fs::File::options().write(true).open(&file).unwrap().set_modified(long_ago).unwrap();
        let taken = lock(&p, &cfg, "rock").is_ok();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(fresh, "a lock just taken was ignored");
        assert!(taken, "a lock older than any run was respected");
    }

    #[test]
    fn generate_asset_is_refused_while_the_asset_is_locked() {
        let p = project("lock-generate");
        let cfg = crate::production::config(&p, None).unwrap();
        let held = lock(&p, &cfg, "rock").unwrap();
        let got = generate_asset(&p, None, "rock", None, |_| {}).map(|_| ());
        let ran = p.join("ran").exists();
        drop(held);
        let after = generate_asset(&p, None, "rock", None, |_| {}).is_ok();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(got.is_err_and(|e| e.contains("ya se está generando o verificando")));
        assert!(!ran, "Godot ran under somebody else's lock");
        assert!(after);
    }
}
