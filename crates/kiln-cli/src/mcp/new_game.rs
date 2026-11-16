//! Tools: new_game.

use super::{Kiln, Reply, blocking, fail, ok, reply};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use std::collections::BTreeMap;
use std::sync::Arc;

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
    /// Muestra el paso de arranque: sin proyecto, los tipos de juego y las preguntas básicas con respuestas sugeridas; con proyecto, el paso en que está y qué falta. Úsala antes de `new_game`.
    #[tool(annotations(read_only_hint = true))]
    async fn new_game_options(&self) -> Reply {
        let project = self.project.clone();
        let ctx = self.ctx().ok();
        reply(blocking(move || crate::ops::new_game::options(&project, ctx.as_deref())).await?)
    }

    /// Crea un juego desde cero en esta carpeta: kiln.toml, una paleta vacía y el contexto con cada respuesta fijada. Pide antes a la persona el nombre y cada pregunta básica (ofrece las respuestas sugeridas de `new_game_options`); después ordena lo que explique con `context_propose` y pregunta con preguntas; usa `settle` cuando no falte nada. Falla si la carpeta ya tiene kiln.toml.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn new_game(&self, Parameters(a): Parameters<NewGameArgs>) -> Reply {
        if let Err(e) = crate::ops::check_len("el nombre", &a.name, 200) {
            return fail(e);
        }
        let project = self.project.clone();
        let game = kiln_core::new_game::NewGame { name: a.name, basics: a.basics, ..Default::default() };
        let (files, ctx) = match blocking(move || crate::ops::new_game::create(&project, game)).await? {
            Ok(r) => r,
            Err(e) => return fail(e),
        };
        *self.slot.write().unwrap() = Some(Arc::new(ctx));
        ok(serde_json::json!({ "created": files }))
    }
}
