//! Tools: context.

use super::{Kiln, Reply, blocking, fail, reply};
use crate::ops::{self, context::{Edit, Proposal}};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ProposeArgs {
    /// Afirmaciones nuevas: { layer, text, source? ("text" | "file:<ruta>"), links? (ids de afirmaciones), conflicts? (id de una afirmación existente con la que choca) }.
    #[serde(default)]
    claims: Option<Vec<ClaimArg>>,
    /// Reescrituras de afirmaciones existentes: { id, text }. Si la afirmación está fijada, queda como propuesta.
    #[serde(default)]
    rewrites: Option<Vec<RewriteArg>>,
    /// Preguntas para la persona: { layer, text, options (de 2 a 5), asked_by? ("refine" | "asset:<id>") }.
    #[serde(default)]
    questions: Option<Vec<QuestionArg>>,
    /// El párrafo de qué juego es (3 a 5 frases). Si la persona lo fijó, queda como propuesta.
    #[serde(default)]
    intent: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ClaimArg {
    layer: String,
    text: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    links: Option<Vec<String>>,
    #[serde(default)]
    conflicts: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct RewriteArg {
    id: String,
    text: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct QuestionArg {
    layer: String,
    text: String,
    options: Vec<String>,
    #[serde(default)]
    asked_by: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct EditArgs {
    /// Qué se edita: "text" (la explicación de la persona), "intent" (el párrafo de qué juego es), "answer" (respuesta a una pregunta) o "claim" (una afirmación).
    kind: String,
    /// text: la explicación completa. intent: el texto nuevo (con action "edit"). claim: el texto (con "add" o "edit").
    #[serde(default)]
    text: Option<String>,
    /// intent: edit | pin | unpin | accept | reject. claim: add | edit | pin | unpin | accept | reject | delete. accept y reject resuelven la propuesta del agente.
    #[serde(default)]
    action: Option<String>,
    /// answer: id de la pregunta. claim: id de la afirmación (no hace falta con "add").
    #[serde(default)]
    id: Option<String>,
    /// answer: la respuesta elegida o escrita por la persona (obligatoria con status "answered").
    #[serde(default)]
    answer: Option<String>,
    /// answer: answered | skipped | not_applicable | open.
    #[serde(default)]
    status: Option<String>,
    /// claim con "add": el id de la capa.
    #[serde(default)]
    layer: Option<String>,
}

fn build_edit(a: EditArgs) -> Result<Edit, String> {
    let text = a.text.unwrap_or_default();
    let action = a.action.unwrap_or_default();
    let id = a.id.unwrap_or_default();
    let layer = a.layer.unwrap_or_default();
    let answer = a.answer.unwrap_or_default();
    for (what, t) in [("el texto", &text), ("la respuesta", &answer)] {
        ops::check_len(what, t, 2000)?;
    }
    if !id.is_empty() {
        ops::check_id("el id", &id)?;
    }
    if !layer.is_empty() {
        ops::check_id("la capa", &layer)?;
    }
    match a.kind.as_str() {
        "text" => Ok(Edit::Text(text)),
        "intent" => Ok(Edit::Intent { action, text }),
        "answer" => {
            ops::check_id("el id", &id)?;
            let status = a.status.unwrap_or_else(|| "answered".into());
            Ok(Edit::Answer { id, answer, status })
        }
        "claim" => Ok(Edit::Claim { id, action, text, layer }),
        k => Err(format!(
            "kind «{}» desconocido; los válidos son: text, intent, answer, claim",
            k.chars().take(40).collect::<String>()
        )),
    }
}

#[tool_router(router = context_router, vis = "pub(super)")]
impl Kiln {
    /// Lee el contexto del proyecto: las capas, el párrafo de qué juego es, las afirmaciones (con lo fijado y las propuestas pendientes), las preguntas y la explicación de la persona. Léelo antes de proponer nada.
    #[tool(annotations(read_only_hint = true))]
    async fn context_get(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::read::context_get(&ctx)).await?)
    }

    /// La voz del agente: úsala para todo lo que tú infieres (ordenar la explicación de la persona en afirmaciones, afinar, reescribir, preguntar, escribir el párrafo de qué juego es). Nunca pisa lo que la persona fijó: se guarda como propuesta (`proposed`) para que ella la acepte o la descarte. Devuelve los ids creados, lo propuesto y si el párrafo se escribió o quedó propuesto.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn context_propose(&self, Parameters(a): Parameters<ProposeArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let p = Proposal {
            claims: a.claims.unwrap_or_default().into_iter().map(|c| ops::context::NewClaim {
                layer: c.layer, text: c.text, source: c.source.unwrap_or_default(),
                links: c.links.unwrap_or_default(), conflicts: c.conflicts,
            }).collect(),
            rewrites: a.rewrites.unwrap_or_default().into_iter().map(|r| ops::context::Rewrite { id: r.id, text: r.text }).collect(),
            questions: a.questions.unwrap_or_default().into_iter().map(|q| ops::context::NewQuestion {
                layer: q.layer, text: q.text, options: q.options, asked_by: q.asked_by,
            }).collect(),
            intent: a.intent,
        };
        let checked = (|| {
            for c in &p.claims {
                ops::check_len("el texto", &c.text, 2000)?;
                ops::check_len("la fuente", &c.source, 2000)?;
                c.links.iter().chain(c.conflicts.iter()).try_for_each(|l| ops::check_id("el id", l))?;
            }
            p.rewrites.iter().try_for_each(|r| ops::check_id("el id", &r.id))?;
            if let Some(i) = &p.intent {
                ops::check_len("el párrafo", i, 2000)?;
            }
            Ok::<(), String>(())
        })();
        if let Err(e) = checked {
            return fail(e);
        }
        reply(blocking(move || ops::context::context_propose(&ctx, p)).await?)
    }

    /// La voz de la persona: úsala SOLO para registrar lo que la persona dice o elige con sus palabras (una respuesta que escogió, una afirmación que dictó, aceptar o descartar una propuesta). Lo que escribe queda fijado y el agente ya no puede pisarlo. Para lo que tú infieres usa `context_propose`.
    #[tool(annotations(read_only_hint = false, destructive_hint = true))]
    async fn context_edit(&self, Parameters(a): Parameters<EditArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let edit = match build_edit(a) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::context::context_edit(&ctx, edit)).await?.map(|_| serde_json::json!({ "ok": true })))
    }

    /// Da por asentado el comienzo de un proyecto nuevo cuando no falta nada. Si falta algo, devuelve qué falta (el párrafo de qué juego es, afirmaciones fijadas por capa).
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn settle(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::context::settle(&ctx)).await?.map(|_| serde_json::json!({ "settled": true })))
    }
}
