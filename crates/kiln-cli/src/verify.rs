//! Verifying a built asset: Kiln's own checks on the GLB, the Khronos glTF
//! Validator when it is installed, and Godot importing and instancing it in a
//! scratch project. What comes out is `<build>/<id>/validation.json`.

use crate::production::config;
use kiln_core::validate::{self, Check, Expected, Report};
use kiln_core::{Config, Finding, Severity, build, glb, tools};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IMPORT_CHECK: &str = include_str!("../godot/import_check.gd");
const SCRATCH_PROJECT: &str = "config_version=5\n\n[application]\n\nconfig/name=\"kiln-verify\"\n";

/// Verifies the asset `id`; the report is written whatever the verdict, and
/// a verdict that is not valid is an error that says where the report is.
pub fn verify_asset(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    mut say: impl FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let dir = build::out_dir(project, &cfg, id);
    let file = dir.join(format!("{id}.glb"));
    let bytes = std::fs::read(&file).map_err(|_| "el asset no está montado: monta primero")?;
    let subject = format!("asset:{id}");
    let json = |name: &str| -> Option<Value> {
        serde_json::from_str(&std::fs::read_to_string(dir.join(name)).ok()?).ok()
    };

    say("Kiln comprueba el GLB".into());
    let mut findings = validate::glb_findings(&subject, &bytes);
    let mut structure_hash = String::new();
    let readable = !findings.iter().any(|f| f.code == "glb_invalid");
    if readable {
        let info = glb::read(&file)?;
        let structure = glb::structure(&file)?;
        structure_hash = glb::structure_hash(&structure);
        let worn: BTreeMap<String, Option<String>> = json("materials.json")
            .and_then(|m| serde_json::from_value(m).ok())
            .unwrap_or_default();
        let palette: Vec<String> = cat.materials.iter().map(|m| m.key.clone()).collect();
        let size_m = json("report.json").and_then(|r| {
            let v = r.get("size_m")?.as_array()?;
            Some([
                v.first()?.as_f64()?,
                v.get(1)?.as_f64()?,
                v.get(2)?.as_f64()?,
            ])
        });
        let expected = Expected {
            spec,
            max_tris: cfg.rules.max_tris,
            size_m,
            worn: &worn,
            palette: &palette,
        };
        findings.extend(validate::asset_findings(
            &subject, &info, &structure, &expected,
        ));
    }
    let mut outputs = BTreeMap::from([(format!("{id}.glb"), kiln_core::asset::sha256_hex(&bytes))]);
    // the colour variants are the same assembly: only the file itself is checked
    for v in spec
        .variants
        .iter()
        .filter(|v| kiln_core::spec::plain_id(&v.id))
    {
        let name = format!("{id}--{}.glb", v.id);
        if let Ok(bytes) = std::fs::read(dir.join(&name)) {
            outputs.insert(name, kiln_core::asset::sha256_hex(&bytes));
            findings.extend(validate::glb_findings(
                &format!("{subject}--{}", v.id),
                &bytes,
            ));
        }
    }
    let mut checks = vec![Check::own(&findings)];
    if readable {
        let (check, found) = khronos(&cfg, &dir, &file, &subject, &mut say)?;
        checks.push(check);
        findings.extend(found);
        let anchors: Vec<String> = spec
            .anchors
            .iter()
            .map(|a| format!("anchor_{}", a.id))
            .collect();
        let (check, found) = godot_import(project, &cfg, id, &file, &subject, &anchors, &mut say)?;
        checks.push(check);
        findings.extend(found);
    }

    let mut report = Report::new(
        id,
        kiln_core::asset::sha256_hex(&bytes),
        structure_hash,
        checks,
        findings,
    );
    report.outputs = outputs;
    let out = dir.join("validation.json");
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    kiln_core::asset::write_replacing(&out, text.as_bytes()).map_err(|e| e.to_string())?;
    for c in &report.checks {
        say(format!(
            "{}: {}{}",
            c.name,
            match c.status.as_str() {
                "passed" => "pasa",
                "absent" => "ausente",
                _ => "falla",
            },
            c.detail
                .as_ref()
                .map_or(String::new(), |d| format!(" ({d})"))
        ));
    }
    if report.valid {
        Ok(out)
    } else {
        let first = report
            .findings
            .iter()
            .find(|f| f.severity == Severity::Error)
            .map_or(String::new(), |f| format!(": {}", f.message));
        Err(format!(
            "la verificación de «{id}» no pasa{first} (informe: {})",
            out.display()
        ))
    }
}

fn khronos(
    cfg: &Config,
    dir: &Path,
    file: &Path,
    subject: &str,
    say: &mut dyn FnMut(String),
) -> Result<(Check, Vec<Finding>), String> {
    if tools::locate(&cfg.tools.gltf_validator).is_none() {
        return Ok((
            Check::absent(
                "gltf_validator",
                "no está instalado; valen las comprobaciones de Kiln",
            ),
            Vec::new(),
        ));
    }
    say(format!(
        "glTF Validator comprueba el GLB ({})",
        cfg.tools.gltf_validator
    ));
    let out = crate::run::program(
        cfg,
        &cfg.tools.gltf_validator,
        vec!["-o".into(), file.into()],
        dir,
        "gltf-validator",
    )?;
    let report: Value = serde_json::from_str(&out.stdout).unwrap_or(Value::Null);
    Ok(validate::khronos(subject, &report))
}

/// An empty folder for the scratch project, `<cache>/verify/<id>`. The cache
/// is set by `kiln.toml`: it must lie inside the project before anything in
/// it is deleted.
fn scratch_dir(project: &Path, cfg: &Config, id: &str) -> Result<PathBuf, String> {
    crate::ops::check_id("el asset", id)?;
    let outside = || {
        format!(
            "[tools] cache «{}» debe quedar dentro del proyecto",
            cfg.tools.cache
        )
    };
    let cache = Path::new(&cfg.tools.cache);
    if cache.is_absolute()
        || cache
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(outside());
    }
    let root = std::fs::canonicalize(project).map_err(|e| e.to_string())?;
    let parent = project.join(cache).join("verify");
    // a link on the way that leads out is refused before anything is created
    let standing = parent.ancestors().find(|a| a.exists()).unwrap_or(project);
    if !std::fs::canonicalize(standing).is_ok_and(|a| a.starts_with(&root)) {
        return Err(outside());
    }
    std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let parent = std::fs::canonicalize(&parent).map_err(|e| e.to_string())?;
    if !parent.starts_with(&root) {
        return Err(outside());
    }
    let scratch = parent.join(id);
    if scratch.symlink_metadata().is_ok_and(|m| m.is_symlink()) {
        std::fs::remove_file(&scratch).map_err(|e| e.to_string())?;
    } else if scratch.exists() {
        std::fs::remove_dir_all(&scratch).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    Ok(scratch)
}

/// Godot imports the GLB into a scratch project of the cache and instances it.
fn godot_import(
    project: &Path,
    cfg: &Config,
    id: &str,
    file: &Path,
    subject: &str,
    anchors: &[String],
    say: &mut dyn FnMut(String),
) -> Result<(Check, Vec<Finding>), String> {
    if tools::locate(&cfg.tools.godot).is_none() {
        return Ok((
            Check::absent(
                "godot_import",
                "Godot no está instalado: importación sin comprobar",
            ),
            Vec::new(),
        ));
    }
    let scratch = scratch_dir(project, cfg, id)?;
    let (script, result) = (scratch.join("import_check.gd"), scratch.join("out.json"));
    std::fs::write(scratch.join("project.godot"), SCRATCH_PROJECT).map_err(|e| e.to_string())?;
    std::fs::write(&script, IMPORT_CHECK).map_err(|e| e.to_string())?;
    std::fs::copy(file, scratch.join("asset.glb")).map_err(|e| e.to_string())?;
    say(format!(
        "Godot importa e instancia el GLB ({})",
        cfg.tools.godot
    ));
    let path = || vec!["--path".into(), scratch.clone().into()];
    let mut import = path();
    import.push("--import".into());
    crate::run::godot(cfg, import, &scratch, "godot-import")?;
    let mut check = path();
    check.extend([
        "--script".into(),
        script.into(),
        "--".into(),
        "res://asset.glb".into(),
        (&result).into(),
    ]);
    let run = crate::run::godot(cfg, check, &scratch, "godot-check")?;
    let out: Value = std::fs::read_to_string(&result)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);

    let mut problems = Vec::new();
    if out["ok"] != true {
        let said = out["error"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(String::from);
        problems.push(
            said.unwrap_or_else(|| format!("Godot no importó el GLB: {}", run.error_tail(2))),
        );
    } else {
        let names: Vec<&str> = out["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if out["meshes"].as_u64().unwrap_or(0) == 0 {
            problems.push("la escena importada no tiene ninguna malla".into());
        }
        for a in anchors.iter().filter(|a| !names.contains(&a.as_str())) {
            problems.push(format!("la escena importada no tiene el nodo «{a}»"));
        }
    }
    let findings: Vec<Finding> = problems
        .iter()
        .map(|m| Finding {
            severity: Severity::Error,
            subject: subject.to_string(),
            code: "godot_import".into(),
            message: m.clone(),
        })
        .collect();
    let check = Check {
        name: "godot_import".into(),
        status: if findings.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .into(),
        errors: findings.len() as u64,
        warnings: 0,
        detail: Some(format!(
            "proyecto de prueba en {}",
            crate::ops::rel(project, &scratch)
        )),
    };
    Ok((check, findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scratch_project_stays_inside_the_project() {
        let base = std::env::temp_dir().join(format!("kiln-verify-scratch-{}", std::process::id()));
        let (project, outside) = (base.join("game"), base.join("outside/verify/door"));
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("keep.txt"), "x").unwrap();
        let project = std::fs::canonicalize(&project).unwrap();
        let cfg = |cache: &str| {
            Config::parse(&format!(
                "[project]\nname = \"t\"\n[tools]\ncache = \"{cache}\"\n"
            ))
            .unwrap()
        };
        assert!(scratch_dir(&project, &cfg("../outside"), "door").is_err());
        assert!(
            outside.join("keep.txt").exists(),
            "nothing outside is deleted"
        );
        let inside = scratch_dir(&project, &cfg("kiln/cache"), "door").unwrap();
        assert!(inside.starts_with(&project) && inside.is_dir());
        std::fs::write(inside.join("old.txt"), "x").unwrap();
        let again = scratch_dir(&project, &cfg("kiln/cache"), "door").unwrap();
        assert!(
            !again.join("old.txt").exists(),
            "the scratch project starts empty"
        );
        std::fs::remove_dir_all(base).unwrap();
    }
}
