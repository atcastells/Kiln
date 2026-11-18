//! Prompts: the skills of Kiln, told to use the tools instead of writing files.
//! A bad argument is a protocol error (invalid params), not a message.

use super::Kiln;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{PromptMessage, Role};
use rmcp::{ErrorData, prompt, prompt_router, schemars};

const ASSET_SKILL: &str = include_str!("../../../../skills/asset/SKILL.md");
const CONTEXT_SKILL: &str = include_str!("../../../../skills/context/SKILL.md");
const REVIEW_SKILL: &str = include_str!("../../../../skills/review/SKILL.md");

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

const REVIEW_HEADER: &str = "Kiln ejecuta este revisor por sí mismo con la herramienta `review` \
(decisión D1: un revisor independiente). Este prompt no es para juzgar tu propio montaje: \
úsalo para leer `review_sheets` y explicar a la persona los hallazgos, con lo concreto que \
hay que corregir en la ficha. Texto de la skill del revisor, como referencia:";

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

fn user(text: String) -> Vec<PromptMessage> {
    vec![PromptMessage::new_text(Role::User, text)]
}

#[prompt_router(router = "prompts_router", vis = "pub(super)")]
impl Kiln {
    /// Desglosar la petición de un asset en su ficha.
    #[prompt(
        name = "kiln-asset",
        description = "Desglosa la petición de un asset en su ficha usando las herramientas de Kiln."
    )]
    async fn kiln_asset(&self, Parameters(a): Parameters<AssetArgs>) -> Result<Vec<PromptMessage>, ErrorData> {
        let request = a.request.as_deref().map(str::trim).unwrap_or_default();
        crate::ops::check_len("la petición", request, 500).map_err(|e| ErrorData::invalid_params(e, None))?;
        let mut text = format!(
            "{ASSET_HEADER}\n\n{ASSET_SKILL}\n\nVocabulario de generación:\n{}\n\n",
            crate::agent::generator_vocabulary()
        );
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
        let mut text = format!("{CONTEXT_HEADER}\n\n{CONTEXT_SKILL}\n\nModo: {}\n", a.mode);
        if let Some(l) = &a.layer {
            crate::ops::check_id("la capa", l).map_err(|e| ErrorData::invalid_params(e, None))?;
            text.push_str(&format!("Capa: {l}\n"));
        }
        Ok(user(text))
    }

    /// Entender los hallazgos de la revisión.
    #[prompt(
        name = "kiln-review",
        description = "Explica a la persona los hallazgos de la revisión (la revisión la ejecuta Kiln con `review`)."
    )]
    async fn kiln_review(&self, Parameters(_): Parameters<ReviewArgs>) -> Vec<PromptMessage> {
        user(format!("{REVIEW_HEADER}\n\n{REVIEW_SKILL}"))
    }
}
