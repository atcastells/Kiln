//! Tools: reading the project.

use super::{Kiln, Reply, blocking, fail, reply};
use crate::ops;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CatalogArgs {
    /// Qué listar: kits, pieces, materials, specs o ui.
    what: String,
    /// Texto que debe contener la clave o el nombre (sin distinguir mayúsculas).
    #[serde(default)]
    filter: Option<String>,
    /// Filas a saltar (por defecto 0).
    #[serde(default)]
    offset: Option<usize>,
    /// Filas por página (por defecto 50, máximo 200).
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CheckArgs {
    /// Solo esta gravedad: error, warn o info.
    #[serde(default)]
    severity: Option<String>,
    /// Hallazgos a saltar (por defecto 0).
    #[serde(default)]
    offset: Option<usize>,
    /// Hallazgos por página (por defecto 50, máximo 200).
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct TreeArgs {
    /// Clave del nodo (`key`, con la forma `nivel:id`). Sin él se devuelve la raíz.
    #[serde(default)]
    node: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SpecArgs {
    /// Id de la ficha (letras, números, «-» y «_»).
    id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ContentArgs {
    /// Nombre del conjunto de contenido; sin él se devuelven todos.
    #[serde(default)]
    set: Option<String>,
}

#[tool_router(router = read_router, vis = "pub(super)")]
impl Kiln {
    /// Resumen del proyecto abierto (`folder` es su carpeta): piezas, fichas, hallazgos por gravedad, contexto y el siguiente paso sugerido. Empieza por aquí.
    #[tool(annotations(read_only_hint = true))]
    async fn status(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let folder = ctx.project.display().to_string();
        reply(
            blocking(move || {
                ops::read::status(&ctx).map(|mut s| {
                    s["folder"] = folder.into();
                    s
                })
            })
            .await?,
        )
    }

    /// Lista kits, piezas, materiales, fichas o `ui` (variaciones del tema, con su cadena, color, tamaño y si se usan), con total y paginación (por defecto 50 filas, máximo 200). Usa `filter` para acotar antes de paginar.
    #[tool(annotations(read_only_hint = true))]
    async fn catalog(&self, Parameters(a): Parameters<CatalogArgs>) -> Reply {
        if let Some(f) = &a.filter
            && let Err(e) = ops::check_len("filter", f, 200) {
                return fail(e);
            }
        if let Err(e) = ops::check_len("what", &a.what, 200) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(
            blocking(move || {
                ops::read::catalog(&ctx, &a.what, a.filter.as_deref(), a.offset.unwrap_or(0), a.limit.unwrap_or(0))
            })
            .await?,
        )
    }

    /// Hallazgos de la comprobación del proyecto, con el recuento por gravedad. Filtra por `severity` y pagina con `offset` y `limit`.
    #[tool(annotations(read_only_hint = true))]
    async fn check(&self, Parameters(a): Parameters<CheckArgs>) -> Reply {
        if let Some(s) = &a.severity {
            if let Err(e) = ops::check_len("severity", s, 200) {
                return fail(e);
            }
            if !["error", "warn", "info"].contains(&s.as_str()) {
                return fail("severity no existe: usa error, warn o info");
            }
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(
            blocking(move || {
                ops::read::check(&ctx, a.severity.as_deref(), a.offset.unwrap_or(0), a.limit.unwrap_or(0))
            })
            .await?,
        )
    }

    /// Árbol del proyecto, un nivel cada vez. Sin `node` devuelve la raíz con sus hijos; para bajar, pasa como `node` el campo `key` de un hijo (con la forma `nivel:id`).
    #[tool(annotations(read_only_hint = true))]
    async fn tree(&self, Parameters(a): Parameters<TreeArgs>) -> Reply {
        if let Some(n) = &a.node
            && let Err(e) = ops::check_len("node", n, 200) {
                return fail(e);
            }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::read::tree(&ctx, a.node.as_deref())).await?)
    }

    /// Una ficha: su contenido, su fichero y los hallazgos que la mencionan.
    #[tool(annotations(read_only_hint = true))]
    async fn spec_get(&self, Parameters(a): Parameters<SpecArgs>) -> Reply {
        if let Err(e) = ops::check_id("id", &a.id) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::read::spec_get(&ctx, &a.id)).await?)
    }

    /// Conjuntos de contenido del juego (tipos, elementos y errores). Con `set` devuelve solo ese.
    #[tool(annotations(read_only_hint = true))]
    async fn content(&self, Parameters(a): Parameters<ContentArgs>) -> Reply {
        if let Some(s) = &a.set
            && let Err(e) = ops::check_len("set", s, 200) {
                return fail(e);
            }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::read::content(&ctx, a.set.as_deref())).await?)
    }
}
