//! Tools: new_game.

use super::{Kiln, Reply, blocking, fail, ok, reply};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use std::collections::BTreeMap;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct NewGameArgs {
    /// Nombre del juego.
    name: String,
    /// La respuesta a cada pregunta básica, por su id (las de `new_game_options`).
    #[serde(default)]
    basics: BTreeMap<String, String>,
}

#[tool_router(router = new_game_router, vis = "pub(super)")]
impl Kiln {
    /// Muestra el paso de arranque: sin proyecto, los tipos de juego y las preguntas básicas con respuestas sugeridas; con proyecto, el paso en que está y qué falta. Úsala antes de `new_game` o `project_create`.
    #[tool(annotations(read_only_hint = true))]
    async fn new_game_options(&self) -> Reply {
        let ctx = self.ctx().ok();
        let Ok(project) = self.dir() else {
            return ok(serde_json::json!({
                "step": 1,
                "folder": null,
                "kinds": kiln_core::config::KINDS,
                "basics": kiln_core::new_game::BASICS,
                "next": "no hay carpeta abierta: crea el juego con `project_create`",
            }));
        };
        reply(blocking(move || crate::ops::new_game::options(&project, ctx.as_deref())).await?)
    }

    /// Crea un juego desde cero en la carpeta abierta: kiln.toml, una paleta vacía y el contexto con cada respuesta fijada. Pide antes a la persona el nombre y cada pregunta básica (ofrece las respuestas sugeridas de `new_game_options`); después ordena lo que explique con `context_propose` y pregunta con preguntas; usa `settle` cuando no falte nada. Falla si la carpeta ya tiene kiln.toml. Sin carpeta abierta, usa `project_create`.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn new_game(&self, Parameters(a): Parameters<NewGameArgs>) -> Reply {
        if let Err(e) = crate::ops::check_len("el nombre", &a.name, 200) {
            return fail(e);
        }
        // The folder and whether `--project` vouched for it, from one read.
        let (project, vouched) = {
            let a = self.active.read().unwrap();
            match &a.dir {
                Some(d) => (d.clone(), a.vouched),
                None => return fail(super::NO_PROJECT),
            }
        };
        let game = kiln_core::new_game::NewGame { name: a.name, basics: a.basics, ..Default::default() };
        let p = project.clone();
        let made = blocking(move || {
            let fresh = crate::projects::fresh(&p);
            crate::ops::new_game::create(&p, game).map(|r| (fresh, r))
        })
        .await?;
        let (fresh, (files, ctx)) = match made {
            Ok(r) => r,
            Err(e) => return fail(e),
        };
        // Only a fresh folder, or the one `--project` vouched for, is trusted.
        self.made(&project, ctx, fresh || vouched);
        ok(serde_json::json!({ "created": files }))
    }
}
