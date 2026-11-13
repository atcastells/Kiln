//! A new game from an empty folder: what the start wizard writes.

use crate::context::{self, Context, Question};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// What the wizard asks before it writes anything.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewGame {
    pub name: String,
    /// The answer to each basic question, by its id.
    #[serde(default)]
    pub basics: BTreeMap<String, String>,
}

/// A question every game answers first, with the claim its answer makes.
#[derive(Debug, Clone, Serialize)]
pub struct Basic {
    pub id: &'static str,
    pub layer: &'static str,
    pub label: &'static str,
    pub text: &'static str,
    pub options: [&'static str; 3],
}

pub const BASICS: [Basic; 5] = [
    Basic {
        id: "camera",
        layer: "scale",
        label: "Cámara",
        text: "¿Desde dónde ve el jugador el mundo?",
        options: [
            "tercera persona, por encima del hombro",
            "primera persona",
            "desde arriba, cenital o isométrica",
        ],
    },
    Basic {
        id: "look",
        layer: "look",
        label: "Estilo",
        text: "¿Qué aspecto tienen los assets?",
        options: [
            "estilizado, de pocos polígonos y colores planos",
            "realista, con texturas de foto",
            "pixel art en 3D, texturas de baja resolución",
        ],
    },
    Basic {
        id: "height",
        layer: "scale",
        label: "Altura de una persona adulta",
        text: "¿Cuánto mide una persona adulta? Es la medida de puertas y muebles.",
        options: ["1,80 m", "1,75 m", "1,65 m"],
    },
    Basic {
        id: "licenses",
        layer: "tech",
        label: "Licencias de los assets",
        text: "¿Qué licencias pueden tener los assets de fuera?",
        options: ["CC0 y CC-BY", "solo CC0", "CC0, CC-BY y los míos propios"],
    },
    Basic {
        id: "tris",
        layer: "tech",
        label: "Triángulos por pieza",
        text: "¿Cuántos triángulos puede tener una pieza como mucho?",
        options: ["20000", "5000", "50000"],
    },
];

/// Writes `kiln.toml`, an empty palette and the context with the basics as
/// pinned claims. Never writes over a project. Returns the files written.
pub fn scaffold(dir: &Path, game: &NewGame) -> Result<Vec<String>, String> {
    let name = game.name.trim();
    if name.is_empty() {
        return Err("falta el nombre del juego".into());
    }
    if dir.join(crate::config::FILE).exists() {
        return Err("esta carpeta ya tiene kiln.toml".into());
    }
    let answer = |id: &str| game.basics.get(id).map(|a| a.trim()).unwrap_or("");
    let licenses = if answer("licenses").contains("CC-BY") {
        "[\"CC0\", \"CC-BY\"]"
    } else {
        "[\"CC0\"]"
    };
    let digits: String = answer("tris")
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let tris = digits
        .parse::<u64>()
        .ok()
        .filter(|t| *t > 0)
        .unwrap_or(20_000);
    let toml_text = format!(
        "[project]\nname = {}\nkind = \"game\"\nengine = \"godot\"\n\n[assets]\nkits = [\"assets/kits\"]\nspecs = \"kiln/specs\"\n\n[materials]\ntables = [\"kiln/materials.toml\"]\n\n[rules]\nlicenses = {licenses}\nmax_tris = {tris}\n\n[context]\nfile = \"kiln/context.toml\"\n",
        toml::Value::String(name.into())
    );
    let mut ctx = Context::default();
    ctx.intent.settled = Some(false);
    for (i, b) in BASICS.iter().enumerate() {
        let a = answer(b.id);
        if a.is_empty() {
            continue;
        }
        let id = format!("q{}", i + 1);
        ctx.questions.push(Question {
            id: id.clone(),
            layer: b.layer.into(),
            text: b.text.into(),
            options: b.options.map(String::from).to_vec(),
            asked_by: "start".into(),
            ..Default::default()
        });
        ctx.answer(&id, &format!("{}: {a}", b.label), "answered")?;
    }
    let write = |rel: &str, text: &str| -> Result<String, String> {
        let p = dir.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(rel.into())
    };
    let mut out = vec![
        write(
            "kiln/materials.toml",
            "# La paleta del juego: una tabla [[materials]] por material, con su `id`.\n",
        )?,
        write("assets/kits/.gitkeep", "")?,
        write("kiln/specs/.gitkeep", "")?,
    ];
    context::save(&dir.join("kiln/context.toml"), &ctx)?;
    out.push("kiln/context.toml".into());
    out.push(write(crate::config::FILE, &toml_text)?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-new-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn game() -> NewGame {
        NewGame {
            name: "Faro \"Gris\"".into(),
            basics: [
                ("camera", "primera persona"),
                ("licenses", "CC0 y CC-BY"),
                ("tris", "hasta 5000"),
            ]
            .map(|(id, a)| (id.to_string(), a.to_string()))
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn an_empty_folder_becomes_a_game_kiln_opens_clean() {
        let d = dir("clean");
        scaffold(&d, &game()).unwrap();
        let cat = crate::open(&d).unwrap();
        assert_eq!(cat.project, "Faro \"Gris\"");
        assert!(
            cat.findings
                .iter()
                .all(|f| f.severity != crate::Severity::Error),
            "{:?}",
            cat.findings
        );
        let cfg = crate::Config::load(&d).unwrap();
        assert_eq!(cfg.project.kind, "game");
        assert_eq!(cfg.rules.licenses, ["CC0", "CC-BY"]);
        assert_eq!(cfg.rules.max_tris, 5000);
        let ctx = crate::context::load(&d.join(&cfg.context.file)).unwrap();
        assert_eq!(ctx.intent.settled, Some(false));
        let camera = ctx.claims.iter().find(|c| c.layer == "scale").unwrap();
        assert!(camera.pinned && camera.text.contains("primera persona"));
        assert_eq!(
            ctx.questions
                .iter()
                .filter(|q| q.status == "answered")
                .count(),
            3
        );
    }

    #[test]
    fn it_never_writes_over_a_project() {
        let d = dir("twice");
        scaffold(&d, &game()).unwrap();
        assert!(scaffold(&d, &game()).is_err());
        assert!(scaffold(&dir("noname"), &NewGame::default()).is_err());
    }
}
