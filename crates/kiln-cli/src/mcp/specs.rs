//! Tools: specs.

use super::{Kiln, Reply, blocking, fail, ok};
use crate::ops::specs::{PartLayout, Rejected, Written};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SpecWriteArgs {
    /// Id of the asset; must match `[asset] id` in the TOML.
    id: String,
    /// The whole spec, in TOML, in the format of the `kiln-asset` prompt.
    toml: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct PartPlacement {
    /// Id of the part inside the spec.
    part: String,
    /// One `[x, y, z]` position per instance of the part.
    at: Vec<[f64; 3]>,
    /// Optional `[x, y, z]` rotation (degrees) per instance; omitted clears it.
    turn: Option<Vec<[f64; 3]>>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SpecLayoutArgs {
    /// Id of an existing asset spec.
    id: String,
    /// Parts to place.
    parts: Vec<PartPlacement>,
}

fn outcome(r: Result<Written, Rejected>) -> Reply {
    match r {
        Ok(w) => ok(w),
        Err(rejected) => Ok(CallToolResult::error(vec![ContentBlock::json(&rejected)?])),
    }
}

#[tool_router(router = specs_router, vis = "pub(super)")]
impl Kiln {
    /// Escribe la ficha de un asset (TOML en el formato del prompt `kiln-asset`). Kiln la comprueba y no escribe nada si tiene errores; sustituye la ficha existente de ese id. Si la rechaza, devuelve los hallazgos para corregirla.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn spec_write(&self, Parameters(a): Parameters<SpecWriteArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        outcome(blocking(move || crate::ops::specs::spec_write(&ctx, &a.id, &a.toml)).await?)
    }

    /// Solo coloca las piezas de una ficha existente (paso 4 de la skill): cambia `at` y `turn` de las piezas indicadas y deja el estado en `layout`. No toca nada más de la ficha.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn spec_layout(&self, Parameters(a): Parameters<SpecLayoutArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let parts = a.parts.into_iter().map(|p| PartLayout { part: p.part, at: p.at, turn: p.turn }).collect();
        outcome(blocking(move || crate::ops::specs::spec_layout(&ctx, &a.id, parts)).await?)
    }
}
