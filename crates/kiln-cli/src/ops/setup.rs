//! `setup`: finds Blender, Godot and Claude Code where a beginner installs
//! them (apps, Steam, Homebrew) and saves them for every project of the
//! user, so nobody has to edit `kiln.toml` or the PATH.

use kiln_core::tools::{self, SETUP};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What each program is for, said to a beginner.
fn purpose(name: &str) -> &'static str {
    match name {
        "blender" => "monta los assets hechos de piezas o de formas",
        "godot" => "genera los assets con receta y verifica que el juego los abre",
        _ => "solo hace falta para el chat de la página de Kiln; en Claude Code no se usa",
    }
}

fn min(name: &str) -> Option<&'static str> {
    match name {
        "blender" => Some("5.0"),
        "godot" => Some("4.7"),
        _ => None,
    }
}

/// Looks for each program and saves in `file` the one found (unless it is already on
/// the PATH). `chosen` names one program per tool that the person picked;
/// it must be on the PATH or one of the places found, so an agent cannot
/// make Kiln run a program of its own choice. `root` is `/` outside tests.
pub fn setup(
    file: Option<&Path>,
    chosen: &BTreeMap<String, String>,
    root: &Path,
    home: Option<&Path>,
) -> Result<Value, String> {
    for name in chosen.keys() {
        if !SETUP.iter().any(|(n, _)| n == name) {
            return Err(format!("herramienta desconocida «{name}»: solo blender, godot o claude"));
        }
    }
    let saved = file.map(tools::saved_in).unwrap_or_default();
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for (name, link) in SETUP {
        let on_path = tools::locate(name);
        let found = tools::candidates(name, root, home);
        let kept = saved.get(name).map(PathBuf::from).filter(|p| tools::locate(&p.to_string_lossy()).is_some());
        let mut all: Vec<PathBuf> = on_path.iter().chain(kept.iter()).chain(found.iter()).cloned().collect();
        let mut seen = std::collections::BTreeSet::new();
        all.retain(|p| seen.insert(p.clone()));
        let (path, source) = match chosen.get(name) {
            Some(c) => {
                let c = PathBuf::from(c);
                if !all.contains(&c) {
                    return Err(format!(
                        "{} no es uno de los programas que Kiln encontró para {name}. Si la persona quiere usar \
                         ese, que lo diga ella en su terminal: `kiln setup --{name} <ruta>`. No lo ejecutes tú.",
                        c.display()
                    ));
                }
                (Some(c), "elegido")
            }
            None => match (&on_path, &kept, found.first()) {
                (Some(p), _, _) => (Some(p.clone()), "PATH"),
                (None, Some(p), _) => (Some(p.clone()), "guardado"),
                (None, None, Some(p)) => (Some(p.clone()), "encontrado"),
                _ => (None, ""),
            },
        };
        let Some(path) = path else {
            missing.push(name);
            out.push(json!({
                "name": name, "found": false, "for": purpose(name), "download": link,
                "note": format!("no está en el PATH ni en los sitios de siempre: se descarga en {link}"),
            }));
            continue;
        };
        let program = path.to_string_lossy().into_owned();
        let probe = tools::probe(name, &program, min(name));
        let mut saved_now = false;
        if let Some(f) = file
            && source != "PATH"
            && saved.get(name) != Some(&program)
        {
            tools::save_in(f, name, &path)?;
            saved_now = true;
        }
        out.push(json!({
            "name": name,
            "found": true,
            "for": purpose(name),
            "path": program,
            "source": source,
            "version": probe.version,
            "ok": probe.ok,
            "note": probe.note,
            "saved": saved_now,
            "others": all.iter().filter(|p| **p != path).collect::<Vec<_>>(),
            "download": (!probe.ok).then_some(link),
        }));
    }
    let next = if missing.contains(&"blender") {
        format!(
            "Falta Blender, que hace falta para montar el primer asset: da a la persona el enlace de descarga, espera a que \
             lo instale y vuelve a llamar a `setup`.{}",
            if missing.contains(&"godot") { " Godot puede instalarlo después: solo hace falta para verificar y publicar." } else { "" }
        )
    } else if missing.contains(&"godot") {
        "Blender está listo: se puede seguir con `status` (o `example_create`) y montar. Godot falta y hará falta para \
         verificar y publicar: da a la persona el enlace para que lo instale mientras tanto."
            .to_string()
    } else {
        "Todo listo: sigue con `status`, o con `example_create` si la persona aún no tiene juego.".to_string()
    };
    Ok(json!({ "tools": out, "next": next }))
}

/// `kiln setup --blender PATH`: the person names a program herself.
pub fn set(file: &Path, name: &str, program: &Path) -> Result<(), String> {
    if !SETUP.iter().any(|(n, _)| *n == name) {
        return Err(format!("herramienta desconocida «{name}»"));
    }
    let program = std::fs::canonicalize(program).map_err(|e| format!("{}: {e}", program.display()))?;
    if tools::locate(&program.to_string_lossy()).is_none() {
        return Err(format!("{} no es un programa que se pueda ejecutar", program.display()));
    }
    tools::save_in(file, name, &program)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn exe(p: &Path, says: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, format!("#!/bin/sh\necho '{says}'\n")).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// One test, so the saved file (a process global) is not shared.
    #[test]
    fn finds_saves_and_refuses_a_program_it_did_not_find() {
        let root = std::env::temp_dir().join(format!("kiln-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("Users/ana");
        let a = root.join("Applications/Blender.app/Contents/MacOS/Blender");
        let b = root.join("Applications/Blender 4.2.app/Contents/MacOS/Blender");
        exe(&a, "Blender 5.0.1");
        exe(&b, "Blender 4.2.0");
        let file = root.join("kiln/tools.json");
        let f = Some(file.as_path());

        let v = setup(f, &BTreeMap::new(), &root, Some(&home)).unwrap();
        let blender = &v["tools"][0];
        if blender["source"] != "PATH" {
            // The first app in reverse name order: "Blender.app" > "Blender 4.2.app".
            assert_eq!(blender["path"], a.to_string_lossy().as_ref(), "{v}");
            assert_eq!(blender["version"], "5.0.1");
            assert_eq!(blender["ok"], true);
            assert_eq!(blender["saved"], true);
            assert_eq!(tools::saved_in(&file)["blender"], a.to_string_lossy());
            assert_eq!(blender["others"][0], b.to_string_lossy().as_ref());

            // The person picks the other one, found by Kiln: it is saved.
            let pick = BTreeMap::from([("blender".to_string(), b.to_string_lossy().into_owned())]);
            let v = setup(f, &pick, &root, Some(&home)).unwrap();
            assert_eq!(v["tools"][0]["source"], "elegido");
            assert_eq!(v["tools"][0]["ok"], false, "4.2 is older than 5.0");
            assert!(v["tools"][0]["download"].is_string());
            assert_eq!(tools::saved_in(&file)["blender"], b.to_string_lossy());
        }
        let godot = &v["tools"][1];
        if godot["source"] != "PATH" {
            assert_eq!(godot["found"], false);
            assert_eq!(godot["download"], "https://godotengine.org/download/");
            assert!(v["next"].as_str().unwrap().contains("Godot"), "{v}");
        }

        // A program Kiln did not find is refused: only the person can name it.
        let evil = root.join("project/evil");
        exe(&evil, "x");
        let pick = BTreeMap::from([("blender".to_string(), evil.to_string_lossy().into_owned())]);
        let e = setup(f, &pick, &root, Some(&home)).unwrap_err();
        assert!(e.contains("kiln setup --blender"), "{e}");
        assert!(setup(f, &BTreeMap::from([("rm".into(), "/bin/rm".into())]), &root, Some(&home)).is_err());
        // ... and the person can, from the terminal.
        set(&file, "blender", &evil).unwrap();
        assert_eq!(tools::saved_in(&file)["blender"], std::fs::canonicalize(&evil).unwrap().to_string_lossy());
        assert!(set(&file, "blender", &root.join("nope")).is_err());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
