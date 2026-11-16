//! Starting a game in a folder without kiln.toml.

use crate::agent::Ctx;
use kiln_core::new_game::NewGame;
use std::path::Path;

/// What the wizard shows: with no context, the first step (folder, kinds and
/// basics); with one, the step the project is at and what is missing.
pub fn options(project: &Path, ctx: Option<&Ctx>) -> Result<serde_json::Value, String> {
    let Some(ctx) = ctx else {
        return Ok(serde_json::json!({
            "step": 1,
            "folder": project.display().to_string(),
            "kinds": kiln_core::config::KINDS,
            "basics": kiln_core::new_game::BASICS,
        }));
    };
    let c = kiln_core::context::load(&ctx.context_file)?;
    let step = match c.intent.settled {
        Some(false) if c.text.trim().is_empty() || c.intent.text.trim().is_empty() => 3,
        Some(false) if c.questions.iter().any(|q| q.status == "open") => 4,
        Some(false) => 5,
        _ => 0,
    };
    Ok(serde_json::json!({
        "step": step,
        "folder": ctx.project.display().to_string(),
        "missing": c.missing(&ctx.layers),
    }))
}

/// Writes the project in `project` and returns the files written and its
/// context. Never writes over a project.
pub fn create(project: &Path, game: NewGame) -> Result<(Vec<String>, Ctx), String> {
    if project.join(kiln_core::config::FILE).exists() {
        return Err("esta carpeta ya tiene kiln.toml".into());
    }
    let files = kiln_core::new_game::scaffold(project, &game)?;
    Ok((files, super::ctx(project, None)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(std::path::PathBuf);

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn empty() -> Dir {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "kiln-newgame-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }

    fn game() -> NewGame {
        NewGame {
            name: "Faro".into(),
            basics: [("camera", "primera persona"), ("licenses", "CC0"), ("tris", "hasta 5000")]
                .map(|(id, a)| (id.to_string(), a.to_string()))
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn an_empty_folder_offers_step_one() {
        let d = empty();
        let v = options(&d.0, None).unwrap();
        assert_eq!(v["step"], 1);
        assert_eq!(v["folder"], d.0.display().to_string());
        assert!(v["kinds"].is_array());
        assert_eq!(v["basics"].as_array().unwrap().len(), kiln_core::new_game::BASICS.len());
    }

    #[test]
    fn create_writes_the_project_once() {
        let d = empty();
        let (files, ctx) = create(&d.0, game()).unwrap();
        assert!(!files.is_empty());
        assert!(d.0.join("kiln.toml").exists());
        let v = options(&d.0, Some(&ctx)).unwrap();
        assert_eq!(v["step"], 3);
        assert!(v["missing"].is_array());
        let again = create(&d.0, game()).err().unwrap();
        assert_eq!(again, "esta carpeta ya tiene kiln.toml");
    }

    #[test]
    fn options_on_examples_give_its_step() {
        let dir = std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")).unwrap();
        let ctx = crate::ops::ctx(&dir, None).unwrap();
        let c = kiln_core::context::load(&ctx.context_file).unwrap();
        let v = options(&dir, Some(&ctx)).unwrap();
        assert_eq!(v["step"], if c.intent.settled == Some(false) { 3 } else { 0 });
        assert_eq!(v["folder"], dir.display().to_string());
        assert_eq!(v["missing"], serde_json::json!(c.missing(&ctx.layers)));
    }
}
