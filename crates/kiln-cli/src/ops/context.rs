//! The project context: the person's edits, the agent's proposals, settling.

use super::check_len;
use crate::agent::Ctx;
use kiln_core::context::{self as core, Claim, Question};
use serde::Deserialize;

/// A change the person makes from the page: what they write is pinned.
pub enum Edit {
    Text(String),
    Intent { action: String, text: String },
    Answer { id: String, answer: String, status: String },
    Claim { id: String, action: String, text: String, layer: String },
}

pub fn context_edit(ctx: &Ctx, edit: Edit) -> Result<(), String> {
    let mut c = core::load(&ctx.context_file)?;
    match edit {
        Edit::Text(t) => c.text = t,
        Edit::Intent { action, text } => c.edit_intent(&action, &text)?,
        Edit::Answer { id, answer, status } => c.answer(&id, &answer, &status)?,
        Edit::Claim { id, action, text, layer } => {
            c.edit_claim(&id, &action, &text, &layer).map(|_| ())?
        }
    }
    core::save(&ctx.context_file, &c)
}

/// Settles the start of the project, if nothing is missing.
pub fn settle(ctx: &Ctx) -> Result<(), String> {
    let mut c = core::load(&ctx.context_file)?;
    c.settle(&ctx.layers).map_err(|m| m.join("; "))?;
    core::save(&ctx.context_file, &c)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Proposal {
    pub claims: Vec<NewClaim>,
    pub rewrites: Vec<Rewrite>,
    pub questions: Vec<NewQuestion>,
    pub intent: Option<String>,
}

#[derive(Deserialize)]
pub struct NewClaim {
    pub layer: String,
    pub text: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub conflicts: Option<String>,
}

#[derive(Deserialize)]
pub struct Rewrite {
    pub id: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct NewQuestion {
    pub layer: String,
    pub text: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub asked_by: Option<String>,
}

pub fn context_propose(ctx: &Ctx, p: Proposal) -> Result<serde_json::Value, String> {
    let before = core::load(&ctx.context_file)?;
    let mut after = before.clone();
    let layer = |id: &str| match ctx.layers.iter().any(|l| l.id == id) {
        true => Ok(id.to_string()),
        false => Err(format!("la capa «{id}» no existe")),
    };
    let text = |t: &str| {
        check_len("el texto", t, 2000)?;
        match t.trim() {
            "" => Err("el texto va vacío".to_string()),
            t => Ok(t.to_string()),
        }
    };
    let next = |after: &core::Context, prefix: &str| {
        let n = after.claims.iter().map(|c| &c.id).chain(after.questions.iter().map(|q| &q.id))
            .filter_map(|id| id.strip_prefix(prefix)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("{prefix}{}", n + 1)
    };
    let (mut added_claims, mut added_questions, mut rewritten) = (vec![], vec![], vec![]);
    for n in p.claims {
        let conflicts = match n.conflicts {
            Some(c) if !after.claims.iter().any(|x| x.id == c) => return Err(format!("no hay afirmación «{c}» con la que chocar")),
            c => c,
        };
        let id = next(&after, "c");
        after.claims.push(Claim { id: id.clone(), layer: layer(&n.layer)?, text: text(&n.text)?, source: n.source, links: n.links, conflicts, ..Default::default() });
        added_claims.push(id);
    }
    for n in p.questions {
        if !(2..=5).contains(&n.options.len()) {
            return Err(format!("la pregunta «{}» necesita de 2 a 5 opciones", n.text.chars().take(40).collect::<String>()));
        }
        let options = n.options.iter().map(|o| text(o)).collect::<Result<_, _>>()?;
        let id = next(&after, "q");
        after.questions.push(Question {
            id: id.clone(), layer: layer(&n.layer)?, text: text(&n.text)?, options,
            status: "open".into(), asked_by: n.asked_by.unwrap_or_else(|| "refine".into()), ..Default::default()
        });
        added_questions.push(id);
    }
    for r in p.rewrites {
        let c = after.claims.iter_mut().find(|c| c.id == r.id).ok_or(format!("no hay afirmación «{}»", r.id))?;
        c.text = text(&r.text)?;
        rewritten.push(r.id);
    }
    let intent_text = p.intent.as_deref().map(text).transpose()?;
    if let Some(t) = &intent_text {
        after.intent.text = t.clone();
    }
    after.guard_pinned(&before);
    let intent = intent_text.as_ref().and_then(|t| match (before.intent.pinned, *t != before.intent.text) {
        (false, _) => Some("written"),
        (true, true) => Some("proposed"),
        (true, false) => None,
    });
    let proposed: Vec<&str> = after.claims.iter().filter(|c| c.pinned && c.proposed.is_some() && rewritten.contains(&c.id)).map(|c| c.id.as_str()).collect();
    core::save(&ctx.context_file, &after)?;
    Ok(serde_json::json!({ "added_claims": added_claims, "added_questions": added_questions, "proposed": proposed, "rewritten": rewritten, "intent": intent }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing::{ctx, examples};

    /// A project whose context has one pinned claim (c1) and one free (c2).
    fn seeded() -> (crate::ops::testing::Temp, Ctx) {
        let t = examples("");
        let c = ctx(&t);
        let mut x = core::Context::default();
        for (id, pinned) in [("c1", true), ("c2", false)] {
            x.claims.push(Claim { id: id.into(), layer: "world".into(), text: format!("texto {id}"), pinned, ..Default::default() });
        }
        core::save(&c.context_file, &x).unwrap();
        (t, c)
    }

    fn prop(json: serde_json::Value) -> Proposal {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn rewriting_a_pinned_claim_keeps_it_and_proposes() {
        let (_t, c) = seeded();
        let r = context_propose(&c, prop(serde_json::json!({ "rewrites": [{ "id": "c1", "text": "otro" }, { "id": "c2", "text": "libre" }] }))).unwrap();
        assert_eq!(r["proposed"], serde_json::json!(["c1"]));
        assert_eq!(r["rewritten"], serde_json::json!(["c1", "c2"]));
        let x = core::load(&c.context_file).unwrap();
        assert_eq!((x.claims[0].text.as_str(), x.claims[0].proposed.as_deref()), ("texto c1", Some("otro")));
        assert_eq!((x.claims[1].text.as_str(), x.claims[1].proposed.as_deref()), ("libre", None));
    }

    #[test]
    fn new_claims_are_free_and_questions_open() {
        let (_t, c) = seeded();
        let r = context_propose(&c, prop(serde_json::json!({
            "claims": [{ "layer": "look", "text": "paleta fría", "source": "text" }],
            "questions": [{ "layer": "look", "text": "¿Época?", "options": ["a", "b"], "asked_by": "asset:x" }, { "layer": "look", "text": "¿Clima?", "options": ["a", "b"] }]
        }))).unwrap();
        assert_eq!(r["added_claims"], serde_json::json!(["c3"]));
        assert_eq!(r["added_questions"], serde_json::json!(["q1", "q2"]));
        let x = core::load(&c.context_file).unwrap();
        assert!(!x.claims[2].pinned && x.claims[2].source == "text");
        assert_eq!((x.questions[0].status.as_str(), x.questions[0].asked_by.as_str()), ("open", "asset:x"));
        assert_eq!(x.questions[1].asked_by, "refine");
    }

    #[test]
    fn a_bad_proposal_changes_nothing() {
        let (_t, c) = seeded();
        let before = std::fs::read(&c.context_file).unwrap();
        for bad in [
            serde_json::json!({ "claims": [{ "layer": "nope", "text": "x" }] }),
            serde_json::json!({ "claims": [{ "layer": "world", "text": "  " }] }),
            serde_json::json!({ "claims": [{ "layer": "world", "text": "ok" }], "rewrites": [{ "id": "c9", "text": "x" }] }),
            serde_json::json!({ "questions": [{ "layer": "world", "text": "¿?", "options": ["solo"] }] }),
            serde_json::json!({ "claims": [{ "layer": "world", "text": "a".repeat(2001) }] }),
        ] {
            assert!(context_propose(&c, prop(bad.clone())).is_err(), "{bad}");
        }
        assert_eq!(std::fs::read(&c.context_file).unwrap(), before);
    }

    #[test]
    fn the_persons_claim_is_pinned_and_settle_fails_as_before() {
        let t = examples("");
        let c = ctx(&t);
        let edit = Edit::Claim { id: String::new(), action: "add".into(), text: "algo".into(), layer: "world".into() };
        context_edit(&c, edit).unwrap();
        assert!(core::load(&c.context_file).unwrap().claims.last().unwrap().pinned);
        let e = settle(&c).unwrap_err();
        assert!(e.contains("falta el párrafo de qué juego es; "), "{e}");
    }

    #[test]
    fn intent_is_written_when_free_and_proposed_when_pinned() {
        let (_t, c) = seeded();
        let r = context_propose(&c, prop(serde_json::json!({ "intent": "Un juego de niebla." }))).unwrap();
        assert_eq!(r["intent"], "written");
        let x = core::load(&c.context_file).unwrap();
        assert_eq!((x.intent.text.as_str(), x.intent.proposed.as_deref()), ("Un juego de niebla.", None));
        let mut x = x;
        x.intent.pinned = true;
        core::save(&c.context_file, &x).unwrap();
        let r = context_propose(&c, prop(serde_json::json!({ "intent": "Otro juego." }))).unwrap();
        assert_eq!(r["intent"], "proposed");
        let x = core::load(&c.context_file).unwrap();
        assert_eq!((x.intent.text.as_str(), x.intent.proposed.as_deref()), ("Un juego de niebla.", Some("Otro juego.")));
        let r = context_propose(&c, prop(serde_json::json!({}))).unwrap();
        assert!(r["intent"].is_null());
    }

    #[test]
    fn a_new_claim_can_conflict_with_an_existing_one() {
        let (_t, c) = seeded();
        let r = context_propose(&c, prop(serde_json::json!({ "claims": [{ "layer": "look", "text": "choca", "conflicts": "c1" }] }))).unwrap();
        assert_eq!(r["added_claims"], serde_json::json!(["c3"]));
        let x = core::load(&c.context_file).unwrap();
        assert_eq!(x.claims[2].conflicts.as_deref(), Some("c1"));
        let before = std::fs::read(&c.context_file).unwrap();
        let bad = serde_json::json!({ "claims": [{ "layer": "look", "text": "x", "conflicts": "c9" }] });
        assert!(context_propose(&c, prop(bad)).unwrap_err().contains("c9"));
        assert_eq!(std::fs::read(&c.context_file).unwrap(), before);
    }
}
