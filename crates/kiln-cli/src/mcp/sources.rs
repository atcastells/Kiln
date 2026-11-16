//! Tools: sources.

use super::{Kiln, Reply, blocking, fail, reply};
use crate::ops::{self, check_id};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SearchArgs {
    /// Id de la ficha.
    asset: String,
    /// Id de la pieza marcada como «buscar».
    part: String,
}

#[tool_router(router = sources_router, vis = "pub(super)")]
impl Kiln {
    /// Busca candidatos para una pieza «buscar» en Poly Haven, Poly Pizza y Sketchfab; solo salen las licencias de `[rules] licenses`. No escribe nada: para traer uno, usa `import` (es un trabajo).
    #[tool(annotations(read_only_hint = true, open_world_hint = true))]
    async fn search(&self, Parameters(a): Parameters<SearchArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        if let Err(e) = check_id("asset", &a.asset).and_then(|_| check_id("part", &a.part)) {
            return fail(e);
        }
        reply(blocking(move || ops::sources::search(&ctx, &a.asset, &a.part)).await?)
    }
}
