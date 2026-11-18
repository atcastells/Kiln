//! Tools: production (build, paint, measure, review, import) and jobs.

use super::{Kiln, Reply, blocking, fail, ok, reply};
use crate::ops;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router};
use serde_json::json;
use std::time::Duration;

const MAX_IMAGE: u64 = 5 * 1024 * 1024;
const SITES: [&str; 3] = ["polyhaven", "polypizza", "sketchfab"];

fn started(id: u64) -> Reply {
    ok(json!({ "job": id, "next": "llama a `job` con este id" }))
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AssetArgs {
    /// Id del asset.
    asset: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct PaintArgs {
    /// Id del asset.
    asset: String,
    /// Variante a pintar (opcional).
    #[serde(default)]
    variant: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GenerateArgs {
    /// Id del asset generado.
    asset: String,
    /// Semilla con la que resolver los rangos de la receta; por defecto la de la ficha.
    #[serde(default)]
    seed: Option<u32>,
    /// La ficha entera del asset en TOML, con su `[generator]`. Si se da, se guarda antes de generar; si no es válida, se rechaza y no se lanza nada.
    #[serde(default)]
    recipe: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ImportArgs {
    /// Id del asset.
    asset: String,
    /// Id de la pieza que usará el modelo.
    part: String,
    /// Sitio de origen: polyhaven, polypizza o sketchfab.
    site: String,
    /// Id del modelo en ese sitio.
    id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct JobArgs {
    /// Id del trabajo, tal como lo devolvió la herramienta que lo lanzó.
    id: u64,
    /// Línea desde la que leer (usa el `next` de la respuesta anterior). Por defecto 0.
    #[serde(default)]
    since: Option<usize>,
    /// Segundos a esperar novedades, máximo 30. Por defecto 20.
    #[serde(default)]
    wait_s: Option<u64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SheetsArgs {
    /// Id del asset.
    asset: String,
    /// Vuelta de la revisión; por defecto la última.
    #[serde(default)]
    round: Option<u32>,
}

#[tool_router(router = production_router, vis = "pub(super)")]
impl Kiln {
    /// Monta el asset con Blender a partir de su ficha. Tarda minutos: devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`. Sobrescribe el montaje anterior.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn build(&self, Parameters(a): Parameters<AssetArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::build(&jobs, ctx, &a.asset)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Aplica los materiales al montaje del asset con Blender (opcionalmente una variante). Tarda minutos: devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`. Sobrescribe el montaje.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn paint(&self, Parameters(a): Parameters<PaintArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        if let Some(v) = a.variant.as_deref().filter(|v| !v.is_empty())
            && let Err(e) = ops::check_id("la variante", v)
        {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::paint(&jobs, ctx, &a.asset, a.variant)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Verifica un asset montado: comprobaciones propias del GLB (estructura, referencias, presupuesto, medidas, anclajes), el glTF Validator de Khronos si está instalado, y Godot importándolo e instanciándolo en un proyecto de prueba. Escribe `validation.json` junto al montaje; `spec_get` lo devuelve. Devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`.
    #[tool(annotations(read_only_hint = false))]
    async fn verify(&self, Parameters(a): Parameters<AssetArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::verify(&jobs, ctx, &a.asset)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Publica un asset verificado en `[publish] dir`/<asset>/ para que el juego lo use: el GLB, sus variantes y el contrato `asset.json` con su versión. Solo publica si la última verificación pasó y corresponde al GLB actual. Sustituye lo publicado antes. Si la carpeta está dentro del proyecto de Godot, Godot la importa abriendo ese proyecto, con los plugins de editor que el proyecto tenga activados. Devuelve un trabajo; sigue su avance llamando a `job`.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn publish(&self, Parameters(a): Parameters<AssetArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::publish(&jobs, ctx, &a.asset)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Genera el GLB de un asset cuya ficha tiene una receta `[generator]` (no `[[parts]]`): Godot sin ventana construye la malla a partir de la receta y la semilla, y la deja en la caché. Con `recipe` guarda antes la ficha. Devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`; su `file` es la ruta del GLB relativa al proyecto.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn generate(&self, Parameters(a): Parameters<GenerateArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::generate(&jobs, ctx, &a.asset, a.seed, a.recipe.as_deref())).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Qué herramientas externas hay (Blender, Godot, validador de glTF, glTF-Transform, claude, curl) y en qué versión. Las ejecuta con `--version`; `status` dice solo si están, sin ejecutarlas.
    #[tool(annotations(read_only_hint = false))]
    async fn tools(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::read::tools_report(&ctx)).await?)
    }

    /// Godot mide las piezas que solo él abre (`.res`, `.tres`, `.tscn`) y las exporta a GLB en la caché, para que Blender y el montaje las usen. Tarda minutos: devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`.
    #[tool(annotations(read_only_hint = false))]
    async fn godot_measure(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        let id = blocking(move || ops::production::godot_measure(&jobs, ctx)).await?;
        started(id)
    }

    /// Revisa el montaje con el revisor propio de Kiln, aparte: Claude Opus por la sesión de `claude` (gasta uso de la suscripción), de modo que quien colocó las piezas nunca juzga su propio trabajo. Hace hasta `[agents] review_rounds` vueltas. Tarda minutos: devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`. Después, mira las hojas con `review_sheets`.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn review(&self, Parameters(a): Parameters<AssetArgs>) -> Reply {
        if let Err(e) = ops::check_id("el asset", &a.asset) {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::review(&jobs, ctx, &a.asset)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Trae un modelo de una fuente externa (polyhaven, polypizza o sketchfab) para una pieza del asset: descarga y reescribe LICENSE y CREDITS. Tarda minutos: devuelve un trabajo; sigue su avance llamando a `job` hasta que su estado deje de ser `running`.
    #[tool(annotations(read_only_hint = false, destructive_hint = true, open_world_hint = true))]
    async fn import(&self, Parameters(a): Parameters<ImportArgs>) -> Reply {
        let checks = ops::check_id("el asset", &a.asset)
            .and_then(|_| ops::check_id("la pieza", &a.part))
            .and_then(|_| {
                if SITES.contains(&a.site.as_str()) {
                    Ok(())
                } else {
                    Err(format!("el sitio debe ser uno de: {}", SITES.join(", ")))
                }
            })
            .and_then(|_| ops::check_len("el id del modelo", &a.id, 200));
        if let Err(e) = checks {
            return fail(e);
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let jobs = self.jobs.clone();
        match blocking(move || ops::production::import(&jobs, ctx, &a.asset, &a.part, &a.site, &a.id)).await? {
            Ok(id) => started(id),
            Err(e) => fail(e),
        }
    }

    /// Estado y novedades de un trabajo. Llámala otra vez con `since` = `next` hasta que el estado deje de ser `running`.
    #[tool(annotations(read_only_hint = true))]
    async fn job(&self, Parameters(a): Parameters<JobArgs>) -> Reply {
        let jobs = self.jobs.clone();
        let wait = Duration::from_secs(a.wait_s.unwrap_or(20).min(30));
        let since = a.since.unwrap_or(0);
        reply(blocking(move || ops::production::job_wait(&jobs, a.id, since, wait)).await?)
    }

    /// Lista los trabajos de esta sesión con su estado.
    #[tool(annotations(read_only_hint = true))]
    async fn jobs(&self) -> Reply {
        let jobs = self.jobs.clone();
        ok(blocking(move || ops::production::jobs_list(&jobs)).await?)
    }

    /// Las hojas de una vuelta de la revisión del asset como imágenes, con sus medidas y el veredicto del revisor.
    #[tool(annotations(read_only_hint = true))]
    async fn review_sheets(&self, Parameters(a): Parameters<SheetsArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let r = blocking(move || {
            let s = ops::production::review_sheets(&ctx, &a.asset, a.round)?;
            let mut notes = Vec::new();
            let mut labels = Vec::new();
            let mut images = Vec::new();
            for sh in &s.sheets {
                let path = ops::rel(&ctx.project, &sh.path);
                let big = std::fs::metadata(&sh.path).map(|m| m.len() > MAX_IMAGE).unwrap_or(false);
                if big {
                    notes.push(format!("{path}: omitida, pesa más de 5 MB"));
                    continue;
                }
                match std::fs::read(&sh.path) {
                    Ok(bytes) => {
                        labels.push(json!({ "label": sh.label, "path": path }));
                        images.push(base64(&bytes));
                    }
                    Err(e) => notes.push(format!("{path}: no se pudo leer ({e})")),
                }
            }
            let info = json!({
                "round": s.round,
                "sheets": labels,
                "measures": s.measures,
                "review": s.review,
                "notes": notes,
            });
            Ok::<_, String>((info, images))
        })
        .await?;
        match r {
            Ok((info, images)) => {
                let mut blocks = vec![ContentBlock::text(info.to_string())];
                blocks.extend(images.into_iter().map(|b| ContentBlock::image(b, "image/png")));
                Ok(CallToolResult::success(blocks))
            }
            Err(e) => fail(e),
        }
    }
}

/// Standard base64 (RFC 4648) with padding.
fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        for (i, o) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(i.as_bytes()), o);
        }
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }
}
