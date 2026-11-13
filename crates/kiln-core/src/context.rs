//! The context of a project: what the agent must know of the game before it
//! breaks an asset down. The person writes it in their own words; the agent
//! sorts it into short claims, one layer each, and asks questions with
//! suggested answers to fill the gaps. All of it lives in one TOML file of
//! the project (`[context] file`, `kiln/context.toml`).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Context {
    /// The project as the person explains it, as written.
    #[serde(default)]
    pub text: String,
    /// What the game is, in one paragraph: the start every asset leans on.
    #[serde(default)]
    pub intent: Intent,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub questions: Vec<Question>,
}

/// The paragraph that says what the game is. Pinned by the person, the agent
/// only proposes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed: Option<String>,
    /// Only for a project made with the start wizard: false until the
    /// person settles its start, true after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<bool>,
}

/// One short statement about the game, in one layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Claim {
    pub id: String,
    pub layer: String,
    pub text: String,
    /// `text` (the person's explanation), `answer:<question id>`, `file:<path>`.
    #[serde(default)]
    pub source: String,
    /// Set by the person: the agent never changes it, it only proposes.
    #[serde(default)]
    pub pinned: bool,
    /// What the agent would write instead, for a pinned claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed: Option<String>,
    /// Other claims it bears on (ids).
    #[serde(default)]
    pub links: Vec<String>,
    /// Another claim it contradicts, when the agent sees one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<String>,
}

/// A question the agent asks, with the answers it suggests.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub id: String,
    pub layer: String,
    pub text: String,
    #[serde(default)]
    pub options: Vec<String>,
    /// open | answered | skipped | not_applicable
    #[serde(default = "open")]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    /// `refine`, or `asset:<id>` when a breakdown needed it.
    #[serde(default)]
    pub asked_by: String,
}

fn open() -> String {
    "open".into()
}

/// One layer of the context, as `kiln.toml` declares it (or the defaults).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: String,
    pub name: String,
    /// What goes in it, for the agent and the page.
    pub holds: String,
}

pub fn default_layers() -> Vec<Layer> {
    [
        (
            "world",
            "Mundo",
            "época, lugar, tecnología, qué existe y qué no",
        ),
        (
            "look",
            "Estética",
            "estilo visual, paleta, nivel de detalle, referencias",
        ),
        ("scale", "Escala", "unidades, tamaño del jugador, cámara"),
        (
            "materials",
            "Materiales",
            "de qué están hechas las cosas, desgaste",
        ),
        (
            "play",
            "Juego",
            "qué hace el jugador, qué sistemas usan los assets",
        ),
        (
            "tech",
            "Técnica",
            "motor, presupuestos, formatos, licencias",
        ),
    ]
    .map(|(id, name, holds)| Layer {
        id: id.into(),
        name: name.into(),
        holds: holds.into(),
    })
    .to_vec()
}

pub fn load(path: &Path) -> Result<Context, String> {
    match std::fs::read_to_string(path) {
        Ok(t) => toml::from_str(&t).map_err(|e| format!("{}: {e}", path.display())),
        Err(_) => Ok(Context::default()),
    }
}

pub fn save(path: &Path, c: &Context) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string_pretty(c).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

impl Context {
    fn next_id(&self, prefix: &str) -> String {
        let n = self
            .claims
            .iter()
            .map(|c| &c.id)
            .chain(self.questions.iter().map(|q| &q.id))
            .filter_map(|id| id.strip_prefix(prefix)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("{prefix}{}", n + 1)
    }

    /// Answers a question; a real answer becomes a claim of its layer.
    pub fn answer(&mut self, id: &str, answer: &str, status: &str) -> Result<(), String> {
        let claim_id = self.next_id("c");
        let q = self
            .questions
            .iter_mut()
            .find(|q| q.id == id)
            .ok_or(format!("no hay pregunta «{id}»"))?;
        q.status = status.into();
        if status != "answered" {
            q.answer = None;
            return Ok(());
        }
        let answer = answer.trim();
        if answer.is_empty() {
            return Err("la respuesta va vacía".into());
        }
        q.answer = Some(answer.into());
        let (layer, source) = (q.layer.clone(), format!("answer:{}", q.id));
        // A question answered again replaces the claim its first answer made.
        self.claims.retain(|c| c.source != source);
        self.claims.push(Claim {
            id: claim_id,
            layer,
            text: answer.into(),
            source,
            pinned: true,
            ..Default::default()
        });
        Ok(())
    }

    /// A change the person makes to a claim from the page: `add` (in
    /// `layer`), `edit`, `pin`, `unpin`, `delete`, `accept` or `reject` the
    /// agent's proposal. What the person writes or accepts is pinned.
    pub fn edit_claim(
        &mut self,
        id: &str,
        action: &str,
        text: &str,
        layer: &str,
    ) -> Result<String, String> {
        if action == "add" {
            if text.trim().is_empty() || layer.is_empty() {
                return Err("falta el texto o la capa".into());
            }
            let id = self.next_id("c");
            self.claims.push(Claim {
                id: id.clone(),
                layer: layer.into(),
                text: text.trim().into(),
                source: "text".into(),
                pinned: true,
                ..Default::default()
            });
            return Ok(id);
        }
        let i = self
            .claims
            .iter()
            .position(|c| c.id == id)
            .ok_or(format!("no hay afirmación «{id}»"))?;
        let c = &mut self.claims[i];
        match action {
            "edit" if !text.trim().is_empty() => {
                c.text = text.trim().into();
                c.pinned = true;
                c.proposed = None;
            }
            "pin" => c.pinned = true,
            "unpin" => c.pinned = false,
            "accept" => {
                if let Some(p) = c.proposed.take() {
                    c.text = p;
                }
                c.pinned = true;
            }
            "reject" => c.proposed = None,
            "delete" => {
                self.claims.remove(i);
                for c in self.claims.iter_mut() {
                    c.links.retain(|l| l != id);
                    if c.conflicts.as_deref() == Some(id) {
                        c.conflicts = None;
                    }
                }
            }
            _ => return Err(format!("acción «{action}» desconocida o sin texto")),
        }
        Ok(id.into())
    }

    /// Keeps the person's pinned claims as they were after the agent wrote:
    /// a pinned claim the agent changed gets its version as a proposal, and
    /// one it dropped comes back.
    pub fn guard_pinned(&mut self, before: &Context) {
        for old in before.claims.iter().filter(|c| c.pinned) {
            match self.claims.iter_mut().find(|c| c.id == old.id) {
                Some(new) => {
                    let proposed = (new.text != old.text).then(|| new.text.clone());
                    *new = Claim {
                        proposed: proposed.or(new.proposed.take()),
                        conflicts: new.conflicts.take(),
                        ..old.clone()
                    };
                }
                None => self.claims.push(old.clone()),
            }
        }
        // The person's own words are theirs too.
        self.text = before.text.clone();
        if before.intent.pinned {
            let changed = (!self.intent.text.is_empty() && self.intent.text != before.intent.text)
                .then(|| self.intent.text.clone());
            self.intent = Intent {
                proposed: changed.or(self.intent.proposed.take()),
                ..before.intent.clone()
            };
        } else {
            self.intent.settled = before.intent.settled;
        }
    }

    /// A change the person makes to the intent: `edit`, `pin`, `unpin`,
    /// `accept` or `reject` the agent's proposal.
    pub fn edit_intent(&mut self, action: &str, text: &str) -> Result<(), String> {
        let i = &mut self.intent;
        match action {
            "edit" if !text.trim().is_empty() => {
                i.text = text.trim().into();
                i.pinned = true;
                i.proposed = None;
            }
            "pin" if !i.text.is_empty() => i.pinned = true,
            "unpin" => i.pinned = false,
            "accept" => {
                if let Some(p) = i.proposed.take() {
                    i.text = p;
                }
                i.pinned = !i.text.is_empty();
            }
            "reject" => i.proposed = None,
            _ => return Err(format!("acción «{action}» desconocida o sin texto")),
        }
        Ok(())
    }

    /// Settles the start of the project, if nothing is missing.
    pub fn settle(&mut self, layers: &[Layer]) -> Result<(), Vec<String>> {
        let missing = self.missing(layers);
        if !missing.is_empty() {
            return Err(missing);
        }
        self.intent.settled = Some(true);
        Ok(())
    }

    /// What is still missing for the start of the project to be settled:
    /// a pinned intent, one pinned claim per layer and no open conflict.
    /// Empty means settled.
    pub fn missing(&self, layers: &[Layer]) -> Vec<String> {
        let mut out = Vec::new();
        if self.intent.text.trim().is_empty() {
            out.push("falta el párrafo de qué juego es".to_string());
        } else if !self.intent.pinned {
            out.push("fija el párrafo de qué juego es".to_string());
        }
        for l in layers {
            if !self.claims.iter().any(|c| c.pinned && c.layer == l.id) {
                out.push(format!("«{}» no tiene nada fijado", l.name));
            }
        }
        for c in &self.claims {
            if let Some(other) = &c.conflicts {
                out.push(format!("«{}» choca con {other}", c.text));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(id: &str, text: &str, pinned: bool) -> Claim {
        Claim {
            id: id.into(),
            layer: "world".into(),
            text: text.into(),
            pinned,
            ..Default::default()
        }
    }

    #[test]
    fn the_agent_only_proposes_on_pinned_claims() {
        let before = Context {
            text: "un pueblo de pescadores".into(),
            claims: vec![
                claim("c1", "costa fría", true),
                claim("c2", "sin motores", true),
            ],
            ..Default::default()
        };
        let mut after = Context {
            text: "otro".into(),
            claims: vec![
                claim("c1", "costa templada", false),
                claim("c3", "sin pólvora", false),
            ],
            ..Default::default()
        };
        after.guard_pinned(&before);
        assert_eq!(after.text, "un pueblo de pescadores");
        let c1 = after.claims.iter().find(|c| c.id == "c1").unwrap();
        assert_eq!((c1.text.as_str(), c1.pinned), ("costa fría", true));
        assert_eq!(c1.proposed.as_deref(), Some("costa templada"));
        assert!(after.claims.iter().any(|c| c.id == "c2"));
        assert!(after.claims.iter().any(|c| c.id == "c3"));
    }

    #[test]
    fn an_answer_becomes_a_claim_once() {
        let mut c = Context {
            questions: vec![Question {
                id: "q1".into(),
                layer: "materials".into(),
                text: "¿De qué son las puertas?".into(),
                status: "open".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        c.answer("q1", "roble", "answered").unwrap();
        c.answer("q1", "roble con herrajes", "answered").unwrap();
        assert_eq!(c.claims.len(), 1);
        assert_eq!(c.claims[0].text, "roble con herrajes");
        assert_eq!(c.claims[0].layer, "materials");
        c.answer("q1", "", "skipped").unwrap();
        assert_eq!(c.questions[0].status, "skipped");
    }

    #[test]
    fn a_context_without_intent_still_reads() {
        let c: Context = toml::from_str("text = \"x\"\n").unwrap();
        assert_eq!(c.intent, Intent::default());
    }

    #[test]
    fn the_agent_only_proposes_on_a_pinned_intent() {
        let mut before = Context::default();
        before.edit_intent("edit", "un juego de pesca").unwrap();
        let mut after = before.clone();
        after.intent = Intent {
            text: "un juego de barcos".into(),
            ..Default::default()
        };
        after.guard_pinned(&before);
        assert_eq!(after.intent.text, "un juego de pesca");
        assert!(after.intent.pinned);
        assert_eq!(after.intent.proposed.as_deref(), Some("un juego de barcos"));
        after.edit_intent("accept", "").unwrap();
        assert_eq!(after.intent.text, "un juego de barcos");
    }

    #[test]
    fn settled_needs_a_pinned_intent_one_pinned_claim_per_layer_and_no_conflict() {
        let layers = default_layers();
        let mut c = Context::default();
        assert!(c.missing(&layers).len() > layers.len());
        c.edit_intent("edit", "un juego de pesca").unwrap();
        for l in &layers {
            c.edit_claim("", "add", &format!("algo de {}", l.id), &l.id)
                .unwrap();
        }
        assert!(c.missing(&layers).is_empty());
        let mut done = c.clone();
        done.settle(&layers).unwrap();
        assert_eq!(done.intent.settled, Some(true));
        c.claims[0].conflicts = Some("c2".into());
        assert_eq!(c.missing(&layers).len(), 1);
        c.claims[0].conflicts = None;
        c.claims.retain(|x| x.layer != "scale");
        let m = c.missing(&layers);
        assert_eq!(m.len(), 1);
        assert!(m[0].contains("Escala"));
        assert!(c.settle(&layers).is_err());
    }
}
