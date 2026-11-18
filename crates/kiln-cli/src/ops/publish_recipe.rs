//! Publishing an asset made from a recipe: the gate that decides whether the
//! verification still holds, and the files a verified recipe publishes.

use super::publish::Published;
use serde_json::{Value, json};
use std::path::Path;

const SCRIPT_NAME: &str = "kiln_generator.gd";

/// What the gate lets through: the verified recipe, ready to be written.
pub struct Verified {
    /// The `recipe` block of `validation.json`.
    pub recipe: Value,
    /// Its `checks`.
    pub checks: Value,
}

/// Publishes the verified recipe of `id` as `recipe.json` and the script; no
/// GLB and no Godot. `asset.json` goes last: a run that fails before it leaves
/// the previous contract, and the next one still sees a change against it.
pub fn publish(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    say: &mut dyn FnMut(String),
) -> Result<Published, String> {
    super::check_id("el asset", id)?;
    let cfg = crate::production::config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or_else(|| format!("no hay ficha con id «{id}»"))?;
    let verified = gate(project, &cfg, spec)?;
    let (dest, dir) = super::publish::dest_dir(project, &cfg, id)?;
    let r = &verified.recipe;

    let seeds: Vec<Value> = r["seeds"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            json!({ "seed": s["seed"], "material": s["resolved"]["material"],
                "ops": s["resolved"]["ops"], "tris": s["tris"], "size_m": s["size_m"] })
        })
        .collect();
    let recipe = serde_json::to_string_pretty(&json!({
        "asset": id, "name": spec.asset.name,
        "generator_version": r["generator_version"], "script": SCRIPT_NAME,
        "script_sha256": r["script_sha256"], "recipe_sha256": r["recipe_sha256"],
        "recipe": spec.generator, "budget_tris": spec.asset.budget_tris,
        "seeds": seeds, "worst": r["worst"],
    }))
    .map_err(|e| e.to_string())?;

    super::publish::create_dest(project, &dest, &dir)?;
    let old = super::publish::read_json(&dest.join("asset.json"), "").unwrap_or(Value::Null);
    let old_version = old["version"].as_u64().unwrap_or(0);
    // against the contract that stands, never against the files on disk: a
    // run that wrote the payload and then failed must not look published
    let payload = kiln_core::asset::sha256_hex(recipe.as_bytes());
    let changed = old.is_null()
        || old["recipe_json_sha256"] != payload.as_str()
        || ["script_sha256", "generator_version"].iter().any(|k| old[k] != r[k]);
    let version = if changed { old_version + 1 } else { old_version };
    let files = vec!["recipe.json".to_string(), SCRIPT_NAME.to_string()];

    let contract = serde_json::to_string_pretty(&json!({
        "asset": id, "kind": "generated", "version": version, "files": files,
        "recipe_sha256": r["recipe_sha256"], "recipe_json_sha256": payload,
        "script_sha256": r["script_sha256"], "generator_version": r["generator_version"],
        "validation": { "valid": true, "checks": verified.checks },
    }))
    .map_err(|e| e.to_string())?;

    // what is there is removed first, so a link is replaced and never followed
    let put = |name: &str, content: &str| {
        let _ = std::fs::remove_file(dest.join(name));
        std::fs::write(dest.join(name), content)
            .map_err(|e| format!("no se puede escribir {name}: {e}"))
    };
    put("recipe.json", &recipe)?;
    put(SCRIPT_NAME, crate::generate::BUILDER)?;
    for e in std::fs::read_dir(&dest).map_err(|e| e.to_string())?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with(".glb") || n.ends_with(".glb.import") {
            let _ = std::fs::remove_file(e.path());
        }
    }
    say(format!("Receta y guion de «{id}» escritos en {}", super::rel(project, &dest)));
    say("Asset generado: no hay nada que Godot deba importar".into());
    put("asset.json", &contract)?;
    let done = Published {
        asset: id.to_string(),
        dir: super::rel(project, &dest),
        version,
        changed,
        files,
    };
    say(format!(
        "«{id}» publicado en {} (versión {}{})",
        done.dir,
        done.version,
        if changed { "" } else { ", sin cambios" }
    ));
    Ok(done)
}

pub fn gate(
    project: &Path,
    cfg: &kiln_core::Config,
    spec: &kiln_core::Spec,
) -> Result<Verified, String> {
    let id = spec.asset.id.as_str();
    let g = spec.generator.as_ref().ok_or("no es un asset generado")?;
    let path = kiln_core::build::out_dir(project, cfg, id).join("validation.json");
    let bytes = std::fs::read(&path)
        .map_err(|_| format!("verifica antes el asset: kiln verify --asset {id}"))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{} no es JSON válido: {e}", path.display()))?;
    if v["valid"] != true {
        return Err(format!(
            "la verificación de «{id}» no pasó: corrige y vuelve a verificar"
        ));
    }
    let recipe = &v["recipe"];
    if !recipe.is_object() {
        return Err(format!(
            "la verificación de «{id}» no es de una receta: vuelve a verificar"
        ));
    }
    let now = kiln_core::generate::hash::recipe_sha256(g, spec.asset.budget_tris);
    if recipe["recipe_sha256"].as_str() != Some(now.as_str()) {
        return Err(format!(
            "la receta de «{id}» cambió después de verificarla: vuelve a verificar"
        ));
    }
    if recipe["generator_version"].as_u64() != Some(crate::generate::GENERATOR_VERSION.into())
        || recipe["script_sha256"].as_str() != Some(crate::generate::builder_sha256().as_str())
    {
        return Err(format!(
            "el generador de Kiln cambió desde que se verificó «{id}»: vuelve a verificar"
        ));
    }
    if recipe["seeds"].as_array().is_none_or(|s| s.is_empty()) {
        return Err(format!(
            "la verificación de «{id}» no generó ninguna semilla: vuelve a verificar"
        ));
    }
    Ok(Verified {
        recipe: recipe.clone(),
        checks: v["checks"].clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    const ROCK: &str = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 1\nmaterial = \"basalt\"\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = 10\nsegments = 8\n\n[[generator.ops]]\nop = \"noise\"\namplitude = 0.1\n";

    struct Proj(PathBuf);
    impl Drop for Proj {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn setup(tag: &str, spec: &str) -> Proj {
        let dir = std::env::temp_dir().join(format!("kiln-gate-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("specs")).unwrap();
        std::fs::write(
            dir.join("kiln.toml"),
            "[project]\nname = \"t\"\n\n[assets]\nspecs = \"specs\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("specs/rock.toml"), spec).unwrap();
        Proj(std::fs::canonicalize(&dir).unwrap())
    }

    fn state(p: &Proj) -> (kiln_core::Config, kiln_core::Spec) {
        let cfg = crate::production::config(&p.0, None).unwrap();
        let spec = crate::load(&p.0, None)
            .unwrap()
            .specs
            .into_iter()
            .find(|s| s.asset.id == "rock")
            .unwrap();
        (cfg, spec)
    }

    fn current_sha(spec: &kiln_core::Spec) -> String {
        kiln_core::generate::hash::recipe_sha256(
            spec.generator.as_ref().unwrap(),
            spec.asset.budget_tris,
        )
    }

    fn report(valid: bool, recipe_sha: &str, version: u64, script_sha: &str, seeds: Value) -> Value {
        json!({ "asset": "rock", "valid": valid, "checks": [{ "name": "tris", "ok": true }],
            "recipe": { "recipe_sha256": recipe_sha, "generator_version": version,
                "script_sha256": script_sha, "seeds": seeds } })
    }

    fn write(p: &Proj, cfg: &kiln_core::Config, v: &Value) {
        let dir = kiln_core::build::out_dir(&p.0, cfg, "rock");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("validation.json"), v.to_string()).unwrap();
    }

    fn files(dir: &Path) -> Vec<PathBuf> {
        let mut all = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            all.push(e.path());
            if e.path().is_dir() {
                all.extend(files(&e.path()));
            }
        }
        all.sort();
        all
    }

    fn version() -> u64 {
        crate::generate::GENERATOR_VERSION.into()
    }

    /// Writes `v` (when given), runs the gate and checks the message and that
    /// nothing was created.
    fn refused(tag: &str, v: impl Fn(&kiln_core::Spec) -> Option<Value>, want: &str) {
        let p = setup(tag, ROCK);
        let (cfg, spec) = state(&p);
        if let Some(v) = v(&spec) {
            write(&p, &cfg, &v);
        }
        let before = files(&p.0);
        let e = gate(&p.0, &cfg, &spec).err().expect("must refuse");
        assert_eq!(e, want);
        assert_eq!(files(&p.0), before);
    }

    #[test]
    fn refuses_without_a_validation() {
        refused("none", |_| None, "verifica antes el asset: kiln verify --asset rock");
    }

    #[test]
    fn refuses_a_validation_that_is_not_json() {
        let p = setup("junk", ROCK);
        let (cfg, spec) = state(&p);
        let dir = kiln_core::build::out_dir(&p.0, &cfg, "rock");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("validation.json"), "{ nope").unwrap();
        let before = files(&p.0);
        let e = gate(&p.0, &cfg, &spec).err().unwrap();
        assert!(e.contains("no es JSON válido"), "{e}");
        assert_eq!(files(&p.0), before);
    }

    #[test]
    fn refuses_a_validation_that_did_not_pass() {
        refused(
            "invalid",
            |s| Some(report(false, &current_sha(s), version(), &crate::generate::builder_sha256(), json!([{}]))),
            "la verificación de «rock» no pasó: corrige y vuelve a verificar",
        );
    }

    #[test]
    fn refuses_the_report_of_a_parts_build() {
        refused(
            "parts",
            |_| Some(json!({ "asset": "rock", "valid": true, "checks": [] })),
            "la verificación de «rock» no es de una receta: vuelve a verificar",
        );
    }

    #[test]
    fn refuses_a_recipe_that_differs_from_the_verified_one() {
        refused(
            "recipe",
            |_| Some(report(true, "other", version(), &crate::generate::builder_sha256(), json!([{}]))),
            "la receta de «rock» cambió después de verificarla: vuelve a verificar",
        );
    }

    #[test]
    fn refuses_another_generator_version_or_script() {
        let want = "el generador de Kiln cambió desde que se verificó «rock»: vuelve a verificar";
        refused(
            "version",
            |s| Some(report(true, &current_sha(s), version() + 1, &crate::generate::builder_sha256(), json!([{}]))),
            want,
        );
        refused(
            "script",
            |s| Some(report(true, &current_sha(s), version(), "other", json!([{}]))),
            want,
        );
    }

    #[test]
    fn refuses_a_verification_without_seeds() {
        let want = "la verificación de «rock» no generó ninguna semilla: vuelve a verificar";
        for (tag, seeds) in [("empty", json!([])), ("missing", Value::Null)] {
            refused(
                tag,
                |s| Some(report(true, &current_sha(s), version(), &crate::generate::builder_sha256(), seeds.clone())),
                want,
            );
        }
    }

    #[test]
    fn refuses_a_spec_without_a_generator() {
        let p = setup("parts-spec", ROCK);
        let (cfg, _) = state(&p);
        let parts = kiln_core::spec::parse("[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[[parts]]\nid = \"body\"\nwhat = \"a box\"\nsource = { kind = \"search\", query = \"box\" }\n").unwrap();
        assert_eq!(gate(&p.0, &cfg, &parts).err().unwrap(), "no es un asset generado");
    }

    fn two_seeds() -> Value {
        let one = |n: u64| {
            json!({ "seed": n, "tris": 80, "size_m": [1.0, 1.0, 1.0],
                "resolved": { "seed": n, "material": "basalt", "ops": [
                    { "op": "sphere", "params": { "radius_m": 10.0, "segments": 8.0 }, "noise_seed": 0 }
                ] } })
        };
        json!([one(1), one(2)])
    }

    /// A project with `[publish]` and a validation that matches its spec.
    fn ready(tag: &str, publish: bool) -> (Proj, kiln_core::Config) {
        let p = setup(tag, ROCK);
        if publish {
            let toml = p.0.join("kiln.toml");
            let mut t = std::fs::read_to_string(&toml).unwrap();
            t.push_str("\n[publish]\ndir = \"godot/assets\"\n");
            std::fs::write(&toml, t).unwrap();
        }
        let (cfg, spec) = state(&p);
        validate(&p, &cfg, &spec, true);
        (p, cfg)
    }

    fn validate(p: &Proj, cfg: &kiln_core::Config, spec: &kiln_core::Spec, valid: bool) {
        let sha = crate::generate::builder_sha256();
        write(p, cfg, &report(valid, &current_sha(spec), version(), &sha, two_seeds()));
    }

    fn publish_it(p: &Proj) -> Result<Published, String> {
        publish(&p.0, None, "rock", &mut |_| {})
    }

    fn json_at(p: &Proj, name: &str) -> Value {
        let f = p.0.join("godot/assets/rock").join(name);
        serde_json::from_slice(&std::fs::read(f).unwrap()).unwrap()
    }

    #[test]
    fn publishes_the_three_files_and_no_glb() {
        let (p, _) = ready("three", true);
        let r = publish_it(&p).unwrap();
        assert_eq!(r.dir, "godot/assets/rock");
        assert_eq!(r.files, ["recipe.json", "kiln_generator.gd"]);
        let d = p.0.join("godot/assets/rock");
        for f in ["recipe.json", "kiln_generator.gd", "asset.json"] {
            assert!(d.join(f).is_file(), "{f}");
        }
        assert!(files(&d).iter().all(|f| f.extension().is_none_or(|e| e != "glb")));
        let rec = json_at(&p, "recipe.json");
        assert_eq!(rec["seeds"].as_array().unwrap().len(), 2);
        assert_eq!(rec["seeds"][0]["ops"][0]["op"], "sphere");
        assert_eq!(rec["script"], "kiln_generator.gd");
        assert_eq!(std::fs::read_to_string(d.join("kiln_generator.gd")).unwrap(), crate::generate::BUILDER);
        let a = json_at(&p, "asset.json");
        assert_eq!((a["version"].as_u64(), a["kind"].as_str()), (Some(1), Some("generated")));
        assert_eq!(a["validation"]["valid"], true);
        let root = p.0.to_string_lossy().into_owned();
        for f in ["recipe.json", "asset.json"] {
            assert!(!std::fs::read_to_string(d.join(f)).unwrap().contains(&root), "{f}");
        }
    }

    #[test]
    fn publishing_again_without_changes_keeps_the_version() {
        let (p, _) = ready("again", true);
        publish_it(&p).unwrap();
        let r = publish_it(&p).unwrap();
        assert_eq!((r.version, r.changed), (1, false));
    }

    #[test]
    fn a_new_recipe_raises_the_version() {
        let (p, _) = ready("newer", true);
        publish_it(&p).unwrap();
        std::fs::write(p.0.join("specs/rock.toml"), ROCK.replace("amplitude = 0.1", "amplitude = 0.2")).unwrap();
        let (cfg, spec) = state(&p);
        validate(&p, &cfg, &spec, true);
        let r = publish_it(&p).unwrap();
        assert_eq!((r.version, r.changed), (2, true));
    }

    #[test]
    fn a_contract_that_could_not_be_written_still_raises_the_version_next_time() {
        let (p, _) = ready("halfway", true);
        publish_it(&p).unwrap();
        let contract = p.0.join("godot/assets/rock/asset.json");
        let v1 = std::fs::read(&contract).unwrap();
        // only the name changes: the recipe hash stays, the published recipe.json does not
        std::fs::write(p.0.join("specs/rock.toml"), ROCK.replace("\"Rock\"", "\"Boulder\"")).unwrap();
        // the contract cannot be written: the payload goes out, the version does not
        std::fs::remove_file(&contract).unwrap();
        std::fs::create_dir_all(contract.join("x")).unwrap();
        assert!(publish_it(&p).is_err());
        std::fs::remove_dir_all(&contract).unwrap();
        std::fs::write(&contract, v1).unwrap();
        let r = publish_it(&p).unwrap();
        assert_eq!((r.version, r.changed), (2, true));
    }

    #[test]
    fn nothing_is_written_when_the_gate_refuses() {
        let (p, cfg) = ready("gate", true);
        let (_, spec) = state(&p);
        validate(&p, &cfg, &spec, false);
        assert!(publish_it(&p).is_err());
        assert!(!p.0.join("godot").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_in_the_destination_is_replaced_not_followed() {
        let (p, _) = ready("link", true);
        publish_it(&p).unwrap();
        let out = p.0.join("outside.txt");
        std::fs::write(&out, "keep").unwrap();
        let f = p.0.join("godot/assets/rock/recipe.json");
        std::fs::remove_file(&f).unwrap();
        std::os::unix::fs::symlink(&out, &f).unwrap();
        publish_it(&p).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep");
        assert!(f.symlink_metadata().unwrap().file_type().is_file());
    }

    #[test]
    fn an_old_glb_of_the_same_id_is_removed() {
        let (p, _) = ready("glb", true);
        let d = p.0.join("godot/assets/rock");
        std::fs::create_dir_all(&d).unwrap();
        for f in ["rock.glb", "rock.glb.import"] {
            std::fs::write(d.join(f), "x").unwrap();
        }
        publish_it(&p).unwrap();
        assert!(!d.join("rock.glb").exists() && !d.join("rock.glb.import").exists());
    }

    #[test]
    fn without_publish_dir_it_says_so() {
        let (p, _) = ready("nodir", false);
        let e = publish_it(&p).unwrap_err();
        assert!(e.contains("falta [publish] dir en kiln.toml"), "{e}");
    }

    #[test]
    fn publish_asset_reaches_the_recipe() {
        let (p, _) = ready("branch", true);
        let mut said = Vec::new();
        let r = crate::ops::publish::publish_asset(&p.0, None, "rock", |s| said.push(s)).unwrap();
        assert_eq!((r.version, r.changed, r.dir.as_str()), (1, true, "godot/assets/rock"));
        assert!(said.iter().any(|s| s.contains("nada que Godot deba importar")));
        assert!(said.last().unwrap().contains("versión 1"));
    }

    #[test]
    fn a_matching_report_passes_and_an_edited_recipe_does_not() {
        let p = setup("ok", ROCK);
        let (cfg, spec) = state(&p);
        let v = report(true, &current_sha(&spec), version(), &crate::generate::builder_sha256(), json!([{ "seed": 1 }]));
        write(&p, &cfg, &v);
        let before = files(&p.0);
        let ok = gate(&p.0, &cfg, &spec).expect("must pass");
        assert_eq!(ok.recipe, v["recipe"]);
        assert_eq!(ok.checks, v["checks"]);
        assert_eq!(files(&p.0), before);

        std::fs::write(p.0.join("specs/rock.toml"), ROCK.replace("amplitude = 0.1", "amplitude = 0.2")).unwrap();
        let (cfg, edited) = state(&p);
        let e = gate(&p.0, &cfg, &edited).err().unwrap();
        assert_eq!(e, "la receta de «rock» cambió después de verificarla: vuelve a verificar");
    }
}
