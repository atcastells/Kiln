//! Prompts: the skills of Kiln, told to use the tools instead of writing files.
//! A bad argument is a protocol error (invalid params), not a message.

use super::{Kiln, Reply, fail, ok};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{PromptMessage, Role};
use rmcp::{ErrorData, prompt, prompt_router, schemars, tool, tool_router};

const ASSET_SKILL: &str = include_str!("../../../../skills/asset/SKILL.md");
const CONTEXT_SKILL: &str = include_str!("../../../../skills/context/SKILL.md");
const REVIEW_SKILL: &str = include_str!("../../../../skills/review/SKILL.md");
const START_SKILL: &str = include_str!("../../../../skills/start/SKILL.md");

const ASSET_HEADER: &str = "Eres el agente de la skill `kiln-asset` de Kiln. El texto de la skill \
habla de ficheros y rutas: aquí no leas ni escribas ficheros, usa las herramientas de Kiln.\n\
- Para leer: `status`, `catalog`, `context_get`, `tree` y `spec_get`.\n\
- La ficha se escribe SOLO con `spec_write`. Kiln la comprueba: si la rechaza, corrige lo que diga y vuelve a escribirla.\n\
- Las preguntas para el contexto van con `context_propose` (asked_by `asset:<id>`).\n\
- Para colocar las piezas (paso 4) usa `spec_layout`.\n\
- Al terminar, ofrece `build` y `review` y sigue su avance con `job`.\n\
- No descargues nada: las piezas de origen «buscar» esperan a la persona, que puede usar `search` e `import`.";

const CONTEXT_HEADER: &str = "Eres el agente de la skill `kiln-context` de Kiln. El texto de la skill \
habla de editar el fichero del contexto: aquí no lo edites, usa las herramientas.\n\
- Lee el contexto con `context_get`.\n\
- Tus cambios (afirmaciones, preguntas) van con `context_propose`.\n\
- Las palabras de la persona pasan por `context_edit`.\n\
- No cambies nunca lo que está fijado (pinned).";

const REVIEW_HEADER: &str = "Eres el revisor del montaje (decisión D1: revisa el agente del cliente, \
no un segundo Claude). El texto de la skill de abajo habla de ficheros y rutas: aquí no leas ni \
escribas ficheros, usa las herramientas de Kiln.\n\
- Si aún no hay hojas, lanza `review` con el asset y sigue su trabajo con `job` hasta que termine.\n\
- Mira las hojas y las medidas con `review_sheets`; la ficha, con `spec_get`; el contexto, con `context_get`.\n\
- Revisa como dice la skill: primero describe en `seen` lo que ves, después juzga. Si colocaste tú las \
piezas, sé más duro, no menos: busca lo que está mal.\n\
- Manda la revisión, en el TOML de la skill, con `review_submit`. Kiln aplica a la ficha las \
correcciones que pueda y te dice qué hacer después (otra vuelta con `review` y next_round = true, o \
`verify` y `publish`).\n\
- Explica a la persona lo que encontraste, con lo concreto que queda para ella.\n\
Texto de la skill del revisor:";

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AssetArgs {
    /// Lo que pide la persona, con sus palabras (máximo 500 caracteres). Si falta,
    /// el agente se lo pregunta: un slash command de Claude Code no admite varias palabras.
    #[serde(default)]
    request: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ContextArgs {
    /// Modo: `sort` (ordenar), `refine` (afinar) o `talk` (conversar).
    mode: String,
    /// Capa del contexto a la que se refiere, si hay una.
    #[serde(default)]
    layer: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ReviewArgs {}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct StartArgs {}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GuideArgs {
    /// Qué guía: `start` (de cero al primer asset publicado, para quien empieza), `asset` (desgranar un asset en su ficha, con los vocabularios de generación y de modelado), `review` (revisar un montaje) o `context` (ordenar el contexto).
    topic: String,
}

fn user(text: String) -> Vec<PromptMessage> {
    vec![PromptMessage::new_text(Role::User, text)]
}

/// The `kiln-asset` text without the request: header, skill and vocabularies.
fn asset_text() -> String {
    format!(
        "{ASSET_HEADER}\n\n{ASSET_SKILL}\n\nVocabulario de generación:\n{}\n\nVocabulario de modelado:\n{}\n\n",
        crate::agent::generator_vocabulary(),
        kiln_core::model::VOCABULARY
    )
}

fn review_text() -> String {
    format!("{REVIEW_HEADER}\n\n{REVIEW_SKILL}")
}

/// The guide without its front matter, which is for skill loaders.
fn start_text() -> String {
    let body = START_SKILL.strip_prefix("---").and_then(|r| r.split_once("\n---\n")).map_or(START_SKILL, |(_, b)| b);
    body.trim_start().to_string()
}

fn context_text() -> String {
    format!("{CONTEXT_HEADER}\n\n{CONTEXT_SKILL}")
}

#[tool_router(router = guide_router, vis = "pub(super)")]
impl Kiln {
    /// El texto de los prompts `kiln-start`, `kiln-asset`, `kiln-review` y `kiln-context` como herramienta, para los clientes que no pasan los prompts al modelo. Léelo al empezar con alguien nuevo (`start`), antes de escribir una ficha (`asset`: formato, fuentes y vocabularios) o de revisar un montaje (`review`).
    #[tool(annotations(read_only_hint = true))]
    async fn guide(&self, Parameters(a): Parameters<GuideArgs>) -> Reply {
        match a.topic.as_str() {
            "asset" => ok(asset_text()),
            "review" => ok(review_text()),
            "context" => ok(context_text()),
            "start" => ok(start_text()),
            _ => fail("topic debe ser start, asset, review o context"),
        }
    }
}

#[prompt_router(router = "prompts_router", vis = "pub(super)")]
impl Kiln {
    /// De cero al primer asset publicado, para quien empieza.
    #[prompt(
        name = "kiln-start",
        description = "Empieza con Kiln: comprueba las herramientas, abre o crea tu juego (o uno de ejemplo) y lleva tu primer asset hasta el juego, solo conversando."
    )]
    async fn kiln_start(&self, Parameters(_): Parameters<StartArgs>) -> Vec<PromptMessage> {
        user(format!("{}\n\nEmpieza ahora por el paso 1.", start_text()))
    }

    /// Desglosar la petición de un asset en su ficha.
    #[prompt(
        name = "kiln-asset",
        description = "Desglosa la petición de un asset en su ficha usando las herramientas de Kiln."
    )]
    async fn kiln_asset(&self, Parameters(a): Parameters<AssetArgs>) -> Result<Vec<PromptMessage>, ErrorData> {
        let request = a.request.as_deref().map(str::trim).unwrap_or_default();
        crate::ops::check_len("la petición", request, 500).map_err(|e| ErrorData::invalid_params(e, None))?;
        let mut text = asset_text();
        if request.is_empty() {
            text.push_str("Pregunta a la persona qué asset quiere y espera su respuesta antes de empezar.");
        } else {
            text.push_str(&format!("Petición: {request}"));
        }
        Ok(user(text))
    }

    /// Ordenar o afinar el contexto del proyecto.
    #[prompt(
        name = "kiln-context",
        description = "Ordena o afina el contexto del proyecto usando las herramientas de Kiln."
    )]
    async fn kiln_context(&self, Parameters(a): Parameters<ContextArgs>) -> Result<Vec<PromptMessage>, ErrorData> {
        if !["sort", "refine", "talk"].contains(&a.mode.as_str()) {
            return Err(ErrorData::invalid_params("mode debe ser sort, refine o talk", None));
        }
        let mut text = format!("{}\n\nModo: {}\n", context_text(), a.mode);
        if let Some(l) = &a.layer {
            crate::ops::check_id("la capa", l).map_err(|e| ErrorData::invalid_params(e, None))?;
            text.push_str(&format!("Capa: {l}\n"));
        }
        Ok(user(text))
    }

    /// Revisar el montaje de un asset.
    #[prompt(
        name = "kiln-review",
        description = "Revisa tú el montaje de un asset: Kiln saca las hojas con `review` y tú mandas la revisión con `review_submit`."
    )]
    async fn kiln_review(&self, Parameters(_): Parameters<ReviewArgs>) -> Vec<PromptMessage> {
        user(review_text())
    }
}
