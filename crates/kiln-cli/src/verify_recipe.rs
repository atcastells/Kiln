//! Verifies a generated asset: runs the batch of seeds and judges every mesh.

use std::path::{Path, PathBuf};

use kiln_core::generate::measure::{judge, measure, segments_of, total_stretch, Limits, Measures};
use kiln_core::generate::resolve::{batch, resolve, Resolved};
use kiln_core::validate::{Check, Report};
use kiln_core::{Finding, Severity};

#[derive(Debug)]
pub struct SeedResult {
    pub seed: u32,
    pub glb: PathBuf,
    pub glb_sha256: String,
    pub resolved: Resolved,
    pub measures: Measures,
}

#[derive(Debug)]
pub struct Outcome {
    pub seeds: Vec<SeedResult>,
    pub findings: Vec<Finding>,
}

/// Verifies the recipe of `id` and writes `validation.json` whatever the verdict.
pub fn verify(
    project: &Path,
    config_file: Option<&Path>,
    id: &str,
    say: &mut dyn FnMut(String),
) -> Result<PathBuf, String> {
    let cfg = crate::production::config(project, config_file)?;
    let cat = crate::load(project, config_file)?;
    let spec = cat
        .specs
        .iter()
        .find(|s| s.asset.id == id)
        .ok_or(format!("no hay ficha con id «{id}»"))?;
    let palette: Vec<&str> = cat.materials.iter().map(|m| m.key.as_str()).collect();
    let lock = crate::generate::lock(project, &cfg, id)?;
    // the earlier verdict goes first: a run that a tool cuts short must leave
    // no report standing, least of all a valid one
    let report_file = kiln_core::build::out_dir(project, &cfg, id).join("validation.json");
    if std::fs::symlink_metadata(&report_file).is_ok() {
        std::fs::remove_file(&report_file).map_err(|e| e.to_string())?;
    }
    let say = &mut |line: String| {
        lock.touch();
        say(line)
    };
    let mut outcome = run(project, &cfg, spec, &palette, say)?;
    let subject = format!("asset:{id}");
    let mut outside = Vec::new();
    if let Some(first) = outcome.seeds.first() {
        for s in &outcome.seeds {
            let bytes = std::fs::read(&s.glb).map_err(|e| e.to_string())?;
            for f in kiln_core::validate::glb_findings(&subject, &bytes) {
                let seen = outcome.findings.iter().any(|g| g.code == f.code && g.message == f.message);
                if !seen {
                    outcome.findings.push(f);
                }
            }
        }
        // the spec's own seed stands for the batch in the tools outside Kiln
        let dir = kiln_core::build::out_dir(project, &cfg, id);
        let anchors: Vec<String> = spec.anchors.iter().map(|a| format!("anchor_{}", a.id)).collect();
        let wanted = crate::verify::Wanted { anchors: &anchors, animations: &spec.asset.animations };
        outside.push(crate::verify::khronos(&cfg, &dir, &first.glb, &subject, say)?);
        outside.push(crate::verify::godot_import(project, &cfg, id, &first.glb, &subject, &wanted, say)?);
    }
    let g =spec.generator.as_ref().ok_or(format!("«{id}» no es un asset generado"))?;

    let structure_hash = match outcome.seeds.first() {
        Some(s) => kiln_core::glb::structure_hash(&kiln_core::glb::structure(&s.glb)?),
        None => String::new(),
    };
    let glb_sha256 = outcome.seeds.first().map_or(String::new(), |s| s.glb_sha256.clone());
    let worst = (!outcome.seeds.is_empty()).then(|| {
        let max = |f: fn(&Measures) -> f64| outcome.seeds.iter().map(|s| f(&s.measures)).fold(0.0, f64::max);
        serde_json::json!({
            "tris": outcome.seeds.iter().map(|s| s.measures.tris).max(),
            "steep": max(|m| m.steep),
            "flipped": outcome.seeds.iter().map(|s| s.measures.flipped).max(),
            "size_m": [max(|m| m.size_m[0]), max(|m| m.size_m[1]), max(|m| m.size_m[2])],
        })
    });
    let seeds: Vec<serde_json::Value> = outcome
        .seeds
        .iter()
        .map(|s| {
            serde_json::json!({
                "seed": s.seed,
                "glb_sha256": s.glb_sha256,
                "resolved": s.resolved,
                "tris": s.measures.tris,
                "degenerate": s.measures.degenerate,
                "flipped": s.measures.flipped,
                "steep": s.measures.steep,
                "size_m": s.measures.size_m,
            })
        })
        .collect();
    let mut checks = vec![Check::own(&outcome.findings)];
    for (check, found) in outside {
        checks.push(check);
        outcome.findings.extend(found);
    }
    let mut report = Report::new(id, glb_sha256, structure_hash, checks, outcome.findings);
    report.recipe = Some(serde_json::json!({
        "recipe_sha256": kiln_core::generate::hash::recipe_sha256(g, spec.asset.budget_tris),
        "generator_version": crate::generate::GENERATOR_VERSION,
        "script_sha256": crate::generate::builder_sha256(),
        "verify_seeds": cfg.generate.verify_seeds,
        "max_tris": cfg.rules.max_tris,
        "seeds": seeds,
        "worst": worst,
    }));

    let dir = kiln_core::build::out_dir(project, &cfg, id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join("validation.json");
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    kiln_core::asset::write_replacing(&out, text.as_bytes()).map_err(|e| e.to_string())?;
    for c in &report.checks {
        let verdict = match c.status.as_str() {
            "passed" => "pasa",
            "absent" => "ausente",
            _ => "falla",
        };
        say(format!("{}: {verdict}{}", c.name, c.detail.as_ref().map_or(String::new(), |d| format!(" ({d})"))));
    }
    if let Some(w) = report.recipe.as_ref().and_then(|r| r.get("worst")).filter(|w| !w.is_null()) {
        say(format!(
            "{} semillas verificadas; peor caso: {} triángulos, {:.1}% de caras empinadas",
            outcome.seeds.len(),
            w["tris"],
            w["steep"].as_f64().unwrap_or(0.0) * 100.0
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
        Err(format!("la verificación de «{id}» no pasa{first} (informe: {})", out.display()))
    }
}

/// An `Err` means the tool could not run; a bad mesh is a finding.
pub fn run(
    project: &Path,
    cfg: &kiln_core::Config,
    spec: &kiln_core::Spec,
    palette: &[&str],
    say: &mut dyn FnMut(String),
) -> Result<Outcome, String> {
    let id = spec.asset.id.as_str();
    let g = spec.generator.as_ref().ok_or(format!("«{id}» no es un asset generado"))?;
    let mut findings = kiln_core::generate::check::check(id, g, spec.asset.budget_tris, palette);
    if findings.iter().any(|f| f.severity == Severity::Error) {
        return Ok(Outcome { seeds: vec![], findings });
    }
    let dir = crate::generate::verify_dir(project, cfg, id)?;
    let batch = batch(g.seed, cfg.generate.verify_seeds);
    let mut seeds = Vec::new();
    for (k, seed) in batch.iter().copied().enumerate() {
        say(format!("Semilla {} de {}: {seed}", k + 1, batch.len()));
        let resolved = resolve(g, seed);
        let run = crate::generate::run_seed(cfg, project, id, &dir, &resolved, say)?;
        let meshes = match kiln_core::mesh::read(&run.glb) {
            Ok(m) => m,
            Err(e) => {
                findings.push(Finding {
                    severity: Severity::Error,
                    subject: format!("asset:{id}"),
                    code: "generated_unreadable".into(),
                    message: format!("semilla {seed}: Kiln no lee la malla: {e}"),
                });
                continue;
            }
        };
        let measures = measure(&meshes, total_stretch(&resolved));
        let limits = Limits {
            segments: segments_of(&resolved),
            budget_tris: spec.asset.budget_tris,
            max_tris: Some(cfg.rules.max_tris),
            steep_max: cfg.generate.steep_max,
        };
        findings.extend(judge(id, seed, &measures, &limits));
        let glb_sha256 = kiln_core::asset::sha256_file(&run.glb)?;
        seeds.push(SeedResult { seed, glb: run.glb, glb_sha256, resolved, measures });
    }
    Ok(Outcome { seeds, findings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::tests::project;
    use kiln_core::generate::measure::measure as measure_meshes;

    pub(crate) fn sphere_glb(segments: u32, flip_one: bool) -> Vec<u8> {
        let rings = segments / 2;
        let mut pos: Vec<f32> = Vec::new();
        for r in 0..=rings + 1 {
            let theta = std::f64::consts::PI * r as f64 / (rings + 1) as f64;
            let ring = if r == 0 || r == rings + 1 { 0.0 } else { theta.sin() };
            for s in 0..=segments {
                let phi = std::f64::consts::TAU * s as f64 / segments as f64;
                pos.extend([(ring * phi.cos()) as f32, theta.cos() as f32, (ring * phi.sin()) as f32]);
            }
        }
        let mut idx: Vec<u16> = Vec::new();
        for r in 0..=rings {
            for s in 0..segments {
                let a = (r * (segments + 1) + s) as u16;
                let b = a + 1;
                let c = a + (segments + 1) as u16;
                let d = c + 1;
                idx.extend([a, b, c, b, d, c]);
            }
        }
        if flip_one {
            let at = (segments * 6) as usize;
            idx.swap(at + 1, at + 2);
        }
        let mut bin: Vec<u8> = pos.iter().flat_map(|f| f.to_le_bytes()).collect();
        let pos_len = bin.len();
        bin.extend(idx.iter().flat_map(|i| i.to_le_bytes()));
        let idx_len = bin.len() - pos_len;
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let col = |k: usize| -> Vec<f32> { pos.chunks(3).map(|v| v[k]).collect() };
        let lo: Vec<f32> = (0..3).map(|k| col(k).into_iter().fold(f32::MAX, f32::min)).collect();
        let hi: Vec<f32> = (0..3).map(|k| col(k).into_iter().fold(f32::MIN, f32::max)).collect();
        let doc = serde_json::json!({
            "asset": {"version": "2.0"},
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": pos.len() / 3, "type": "VEC3", "min": lo, "max": hi},
                {"bufferView": 1, "componentType": 5123, "count": idx.len(), "type": "SCALAR"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": pos_len},
                {"buffer": 0, "byteOffset": pos_len, "byteLength": idx_len}
            ],
            "buffers": [{"byteLength": bin.len()}]
        });
        let mut json = doc.to_string().into_bytes();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((12 + 8 + json.len() as u32 + 8 + bin.len() as u32).to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
        out
    }

    fn spec_text(extra_op: &str, budget: u32) -> String {
        format!(
            "[asset]\nid = \"rock\"\nname = \"Rock\"\nbudget_tris = {budget}\n\n[generator]\nseed = 7\n\n[[generator.ops]]\nop = \"sphere\"\nradius_m = 1\nsegments = 8\n{extra_op}\n[[generator.ops]]\nop = \"noise\"\namplitude = 0.1\nsize = 0.5\n"
        )
    }

    fn setup(tag: &str, spec: &str, glb: &[u8], extra_cfg: &str) -> PathBuf {
        let p = project(tag);
        std::fs::write(p.join("fixture.glb"), glb).unwrap();
        std::fs::write(p.join("specs/rock.toml"), spec).unwrap();
        // the generate fake plus the two Godot calls of the import check: `--import`
        // does nothing and `--script` writes the JSON of a scene with one mesh
        let fake = format!(
            "#!/bin/sh\ncase \" $* \" in\n*\" --import \"*) exit 0;;\n*import_check.gd*) for a; do o=$a; done\necho '{{\"ok\":true,\"error\":\"\",\"nodes\":[\"rock\"],\"meshes\":1,\"size_m\":[1,1,1],\"skeletons\":[],\"animations\":[]}}' > \"$o\"; exit 0;;\nesac\n{}",
            crate::generate::tests::FAKE.trim_start_matches("#!/bin/sh\n")
        );
        std::fs::write(p.join("fake-godot"), fake).unwrap();
        // a validator that does not exist, so the result does not depend on the machine
        let mut t = std::fs::read_to_string(p.join("kiln.toml")).unwrap();
        t.push_str("gltf_validator = \"kiln-no-such-validator\"\n");
        t.push_str(extra_cfg);
        std::fs::write(p.join("kiln.toml"), t).unwrap();
        p
    }

    fn go(p: &Path) -> Result<Outcome, String> {
        let cfg = crate::production::config(p, None).unwrap();
        let cat = crate::load(p, None).unwrap();
        let spec = cat.specs.iter().find(|s| s.asset.id == "rock").unwrap();
        run(p, &cfg, spec, &[], &mut |_| {})
    }

    fn codes(o: &Outcome, code: &str) -> usize {
        o.findings.iter().filter(|f| f.code == code).count()
    }

    #[test]
    fn the_sphere_helper_measures_like_godots_sphere() {
        let m = measure_meshes(&kiln_core::mesh::read_bytes(&sphere_glb(8, false)).unwrap(), [1.0; 3]);
        assert_eq!((m.tris, m.degenerate, m.flipped), (80, 16, 0));
    }

    #[test]
    fn a_good_recipe_generates_the_whole_batch() {
        let p = setup("vr-good", &spec_text("", 200), &sphere_glb(8, false), "");
        let got = go(&p);
        let out = got.as_ref().unwrap();
        let seeds: Vec<u32> = out.seeds.iter().map(|s| s.seed).collect();
        let files = out.seeds.iter().all(|s| s.glb.is_file() && s.glb.starts_with(p.join("kiln/cache/generated/rock/verify")));
        let (errors, tris) = (
            out.findings.iter().any(|f| f.severity == Severity::Error),
            out.seeds.iter().all(|s| s.measures.tris == 80),
        );
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(seeds.len(), 16);
        assert_eq!(seeds[0], 7);
        assert_eq!(seeds, batch(7, 16));
        assert!(files && tris && !errors, "{:?}", out.findings);
    }

    #[test]
    fn verify_seeds_comes_from_the_config() {
        let p = setup("vr-n", &spec_text("", 200), &sphere_glb(8, false), "\n[generate]\nverify_seeds = 3\n");
        let got = go(&p);
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(got.unwrap().seeds.len(), 3);
    }

    #[test]
    fn a_static_error_never_launches_godot() {
        let stretch = "\n[[generator.ops]]\nop = \"stretch\"\ny = 0.2\n";
        let spec = spec_text(stretch, 200).replace("amplitude = 0.1", "amplitude = 0.9");
        let p = setup("vr-static", &spec, &sphere_glb(8, false), "");
        let got = go(&p);
        let ran = p.join("ran").exists();
        std::fs::remove_dir_all(&p).unwrap();
        let out = got.unwrap();
        assert!(codes(&out, "generator_radius") > 0, "{:?}", out.findings);
        assert!(out.seeds.is_empty() && !ran);
    }

    #[test]
    fn a_flipped_face_fails_every_seed() {
        let p = setup("vr-flip", &spec_text("", 200), &sphere_glb(8, true), "");
        let got = go(&p);
        std::fs::remove_dir_all(&p).unwrap();
        let out = got.unwrap();
        let n = out.findings.iter().filter(|f| f.code == "generated_flipped" && f.severity == Severity::Error).count();
        assert_eq!(n, 16, "{:?}", out.findings);
    }

    #[test]
    fn an_unreadable_glb_is_a_finding_not_a_crash() {
        let p = setup("vr-junk", &spec_text("", 200), &[7u8; 64], "");
        let got = go(&p);
        std::fs::remove_dir_all(&p).unwrap();
        let out = got.unwrap();
        assert_eq!(codes(&out, "generated_unreadable"), 16, "{:?}", out.findings);
        assert!(out.seeds.is_empty());
    }

    fn report_of(p: &Path) -> (PathBuf, serde_json::Value) {
        let cfg = crate::production::config(p, None).unwrap();
        let f = kiln_core::build::out_dir(p, &cfg, "rock").join("validation.json");
        let v = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        (f, v)
    }

    #[test]
    fn verify_writes_the_report_of_a_good_recipe() {
        let p = setup("vr-rep", &spec_text("", 200), &sphere_glb(8, false), "");
        let mut lines = Vec::new();
        let got = verify(&p, None, "rock", &mut |l| lines.push(l));
        let (f, v) = report_of(&p);
        let cat = crate::load(&p, None).unwrap();
        let spec = cat.specs.iter().find(|s| s.asset.id == "rock").unwrap();
        let want = kiln_core::generate::hash::recipe_sha256(spec.generator.as_ref().unwrap(), spec.asset.budget_tris);
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(got.unwrap(), f);
        assert_eq!(v["valid"], true);
        assert_eq!(v["checks"][0]["name"], "kiln");
        assert_eq!(v["checks"][0]["status"], "passed");
        let r = &v["recipe"];
        assert_eq!(r["seeds"].as_array().unwrap().len(), 16);
        assert_eq!(r["seeds"][0]["seed"], 7);
        assert!(r["seeds"].as_array().unwrap().iter().all(|s| s["resolved"]["ops"][0]["op"] == "sphere"));
        assert_eq!(r["recipe_sha256"], want);
        assert_eq!(r["script_sha256"], crate::generate::builder_sha256());
        assert_eq!(r["generator_version"], 1);
        assert_eq!(r["worst"]["tris"], 80);
        assert!(lines.iter().any(|l| l == "kiln: pasa"), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("16 semillas verificadas; peor caso: 80 triángulos")), "{lines:?}");
    }

    #[test]
    fn verify_runs_the_three_checks() {
        let p = setup("vr-three", &spec_text("", 200), &sphere_glb(8, false), "");
        let mut lines = Vec::new();
        let got = verify(&p, None, "rock", &mut |l| lines.push(l));
        let (_, v) = report_of(&p);
        std::fs::remove_dir_all(&p).unwrap();
        got.unwrap();
        let names: Vec<&str> = v["checks"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["kiln", "gltf_validator", "godot_import"]);
        assert_eq!(v["checks"][1]["status"], "absent");
        assert_eq!(v["checks"][2]["status"], "passed");
        assert_eq!(v["valid"], true);
        for n in ["kiln", "gltf_validator", "godot_import"] {
            assert!(lines.iter().any(|l| l.starts_with(&format!("{n}: "))), "{n}: {lines:?}");
        }
    }

    #[test]
    fn a_broken_glb_is_reported_once() {
        let p = setup("vr-once", &spec_text("", 200), &[7u8; 64], "");
        let got = verify(&p, None, "rock", &mut |_| {});
        let (_, v) = report_of(&p);
        std::fs::remove_dir_all(&p).unwrap();
        assert!(got.is_err());
        let found = v["findings"].as_array().unwrap();
        for f in found {
            let same = found.iter().filter(|g| g["code"] == f["code"] && g["message"] == f["message"]).count();
            assert_eq!(same, 1, "{f}");
        }
    }

    #[test]
    fn the_checks_of_the_glb_are_not_repeated_per_seed() {
        // a GLB with a finding of Kiln's own: the 16 seeds give it once
        let p = setup("vr-dedup", &spec_text("", 200), &sphere_glb(8, false), "");
        verify(&p, None, "rock", &mut |_| {}).unwrap();
        let (_, v) = report_of(&p);
        std::fs::remove_dir_all(&p).unwrap();
        let found = v["findings"].as_array().unwrap();
        for f in found {
            let same = found.iter().filter(|g| g["code"] == f["code"] && g["message"] == f["message"]).count();
            assert_eq!(same, 1, "{f}");
        }
    }

    #[test]
    fn verify_fails_and_still_writes_the_report() {
        let p = setup("vr-repbad", &spec_text("", 200), &sphere_glb(8, true), "");
        let got = verify(&p, None, "rock", &mut |_| {});
        let (f, v) = report_of(&p);
        std::fs::remove_dir_all(&p).unwrap();
        let e = got.unwrap_err();
        assert!(e.contains("no pasa") && e.contains(&f.display().to_string()), "{e}");
        assert_eq!(v["valid"], false);
        assert!(v["findings"].as_array().unwrap().iter().any(|x| x["code"] == "generated_flipped"));
    }

    #[test]
    fn verify_of_a_static_error_writes_an_invalid_report_without_godot() {
        let stretch = "\n[[generator.ops]]\nop = \"stretch\"\ny = 0.2\n";
        let spec = spec_text(stretch, 200).replace("amplitude = 0.1", "amplitude = 0.9");
        let p = setup("vr-repstatic", &spec, &sphere_glb(8, false), "");
        let got = verify(&p, None, "rock", &mut |_| {});
        let (_, v) = report_of(&p);
        let ran = p.join("ran").exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert!(got.is_err() && !ran);
        assert_eq!(v["valid"], false);
        assert!(v["recipe"]["seeds"].as_array().unwrap().is_empty());
        assert!(v["recipe"]["worst"].is_null());
    }

    #[test]
    fn verify_asset_reaches_the_recipe_verification() {
        let p = setup("vr-branch", &spec_text("", 200), &sphere_glb(8, false), "");
        let got = crate::verify::verify_asset(&p, None, "rock", |_| {});
        let (f, _) = report_of(&p);
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(got.unwrap(), f);
    }

    #[test]
    fn a_budget_below_the_sphere_fails() {
        let p = setup("vr-budget", &spec_text("", 50), &sphere_glb(8, false), "");
        let got = go(&p);
        let ran = p.join("ran").exists();
        std::fs::remove_dir_all(&p).unwrap();
        let out = got.unwrap();
        assert!(codes(&out, "generator_budget") > 0, "{:?}", out.findings);
        assert!(out.seeds.is_empty() && !ran);
    }

    #[test]
    fn a_verification_that_cannot_run_leaves_no_report_behind() {
        let p = setup("vr-stale", &spec_text("", 200), &sphere_glb(8, false), "");
        verify(&p, None, "rock", &mut |_| {}).unwrap();
        let (f, before) = report_of(&p);
        std::fs::write(p.join("fake-godot"), "#!/bin/sh\nexit 3\n").unwrap();
        let again = verify(&p, None, "rock", &mut |_| {});
        let left = f.exists();
        std::fs::remove_dir_all(&p).unwrap();
        assert_eq!(before["valid"], true);
        assert!(again.is_err());
        assert!(!left, "the report of the earlier run still stands as valid");
    }

    #[test]
    fn verify_is_refused_while_the_asset_is_locked() {
        let p = setup("vr-lock", &spec_text("", 200), &sphere_glb(8, false), "");
        let cfg = crate::production::config(&p, None).unwrap();
        let held = crate::generate::lock(&p, &cfg, "rock").unwrap();
        let got = verify(&p, None, "rock", &mut |_| {});
        let ran = p.join("ran").exists();
        drop(held);
        std::fs::remove_dir_all(&p).unwrap();
        assert!(got.is_err_and(|e| e.contains("ya se está generando o verificando")));
        assert!(!ran);
    }
}
