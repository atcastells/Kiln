//! Tools: materials.

use super::{Kiln, Reply, blocking, fail, ok};
use crate::ops::{self, check_id, check_len};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};

const MAX: usize = 200;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BindArgs {
    /// Clave de la pieza del catálogo (lleva `/`, por ejemplo `harbor/crate`).
    piece: String,
    /// Nombre de la ranura de material de la pieza.
    slot: String,
    /// Clave de la paleta a la que se vincula la ranura.
    material: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct MaterialSetArgs {
    /// Id de la ficha.
    asset: String,
    /// Id de la pieza dentro de la ficha.
    part: String,
    /// Zona (ranura de material) de la pieza, con el nombre que le da el modelo.
    zone: String,
    /// Clave de la paleta. Vacío quita la elección de esa zona.
    material: String,
    /// Variante de la ficha a la que se aplica; sin ella, el montaje base.
    #[serde(default)]
    variant: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct MaterialValidateArgs {
    /// Id de la ficha.
    asset: String,
    /// Id de la pieza (reutilizar o variante) cuyos materiales pasan al catálogo.
    part: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ZonesSplitArgs {
    /// Clave de la pieza del catálogo.
    piece: String,
    /// Cómo partir la pieza en zonas: `loose` (mallas sueltas) o `texture` (por textura). Vacío quita el reparto.
    mode: String,
}

fn done(what: &str) -> Reply {
    ok(serde_json::json!({ "ok": true, "what": what }))
}

#[tool_router(router = materials_router, vis = "pub(super)")]
impl Kiln {
    /// Vincula la ranura de material de una pieza del catálogo a una clave de la paleta (`kiln/bindings.toml`); Blender lo usa al montar. Úsalo para dejar el material por defecto de una pieza.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn bind(&self, Parameters(a): Parameters<BindArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        for (what, v) in [("piece", &a.piece), ("slot", &a.slot), ("material", &a.material)] {
            if let Err(e) = check_len(what, v, MAX) {
                return fail(e);
            }
        }
        if a.piece.is_empty() || a.slot.is_empty() {
            return fail("falta la pieza o el hueco");
        }
        let r = blocking(move || ops::materials::bind(&ctx, &a.piece, &a.slot, &a.material)).await?;
        match r {
            Ok(()) => done("bind"),
            Err(e) => fail(e),
        }
    }

    /// Elige un material de la paleta para una zona de una pieza de la ficha; las zonas que no nombra se quedan con el material del modelo. Material vacío quita la elección. Úsalo para vestir una pieza solo en este montaje.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn material_set(&self, Parameters(a): Parameters<MaterialSetArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        if let Err(e) = check_id("asset", &a.asset).and_then(|_| check_id("part", &a.part)) {
            return fail(e);
        }
        for (what, v) in [("zone", &a.zone), ("material", &a.material)] {
            if let Err(e) = check_len(what, v, MAX) {
                return fail(e);
            }
        }
        if let Some(v) = &a.variant
            && let Err(e) = check_len("variant", v, MAX) {
                return fail(e);
            }
        let r = blocking(move || {
            ops::materials::material_set(&ctx, &a.asset, &a.part, &a.zone, &a.material, a.variant.as_deref())
        })
        .await?;
        match r {
            Ok(()) => done("material_set"),
            Err(e) => fail(e),
        }
    }

    /// Pasa los materiales de la pieza de la ficha al catálogo, como el material por defecto de esa pieza, y los quita de la ficha. Úsalo cuando la elección ya está aprobada.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn material_validate(&self, Parameters(a): Parameters<MaterialValidateArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        if let Err(e) = check_id("asset", &a.asset).and_then(|_| check_id("part", &a.part)) {
            return fail(e);
        }
        let r = blocking(move || ops::materials::material_validate(&ctx, &a.asset, &a.part)).await?;
        match r {
            Ok(piece) => ok(serde_json::json!({ "ok": true, "piece": piece })),
            Err(e) => fail(e),
        }
    }

    /// Dice cómo se parte una pieza del catálogo en zonas de material: `loose` (mallas sueltas) o `texture` (por textura); vacío quita el reparto. Úsalo cuando una pieza trae varias zonas que hay que vestir por separado.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true))]
    async fn zones_split(&self, Parameters(a): Parameters<ZonesSplitArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        for (what, v) in [("piece", &a.piece), ("mode", &a.mode)] {
            if let Err(e) = check_len(what, v, MAX) {
                return fail(e);
            }
        }
        let r = blocking(move || ops::materials::zones_split(&ctx, &a.piece, &a.mode)).await?;
        match r {
            Ok(()) => done("zones_split"),
            Err(e) => fail(e),
        }
    }
}
