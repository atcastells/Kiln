//! The conversation between the person and the agent in the start wizard,
//! kept in one TOML file of the project (`kiln/chat.toml`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Chat {
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// `person` or `agent`.
    pub who: String,
    /// The wizard step it belongs to: `1` to `5`, or `contexto`.
    pub step: String,
    pub text: String,
}

pub fn load(path: &Path) -> Result<Chat, String> {
    match std::fs::read_to_string(path) {
        Ok(t) => {
            toml::from_str(&t).map_err(|e| format!("no se puede leer {}: {e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Chat::default()),
        Err(e) => Err(format!("no se puede leer {}: {e}", path.display())),
    }
}

pub fn save(path: &Path, chat: &Chat) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("no se puede crear {}: {e}", dir.display()))?;
    }
    let text = toml::to_string_pretty(chat).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("no se puede escribir {}: {e}", path.display()))
}

impl Chat {
    /// The last `n` messages, oldest first (all of them if there are fewer).
    pub fn recent(&self, n: usize) -> &[Message] {
        &self.messages[self.messages.len().saturating_sub(n)..]
    }
}

/// Values the agent proposes for the wizard, read from its `kiln` block.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Offer {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub basics: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Offer {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.basics.is_empty() && self.text.is_none()
    }

    /// The offer in one line for the thread, numbered as the buttons show it,
    /// so a later turn can say «the second one».
    pub fn summary(&self) -> String {
        let list = |l: &[String]| l.iter().enumerate().map(|(i, v)| format!("{}) {v}", i + 1)).collect::<Vec<_>>().join(" ");
        let mut parts = Vec::new();
        if !self.names.is_empty() {
            parts.push(format!("nombres {}", list(&self.names)));
        }
        for (id, vs) in &self.basics {
            parts.push(format!("{id} {}", list(vs)));
        }
        if self.text.is_some() {
            parts.push("una versión de tu explicación".into());
        }
        if parts.is_empty() { String::new() } else { format!("Propuse: {}", parts.join(" · ")) }
    }
}

const MAX_ITEMS: usize = 6;
const MAX_VALUE_CHARS: usize = 200;
/// The longest message or text Kiln keeps, in characters.
pub const MAX_TEXT_CHARS: usize = 4000;
/// The most messages a thread sent by the page may carry.
pub const MAX_THREAD: usize = 40;

/// A thread that came from outside (the page): only `person` and `agent`
/// messages, each cut to `MAX_TEXT_CHARS`, and the last `MAX_THREAD`.
pub fn clean(messages: Vec<Message>) -> Vec<Message> {
    let mut out: Vec<Message> = messages
        .into_iter()
        .filter(|m| matches!(m.who.as_str(), "person" | "agent"))
        .map(|mut m| {
            m.text = m.text.chars().take(MAX_TEXT_CHARS).collect();
            m
        })
        .collect();
    out.drain(..out.len().saturating_sub(MAX_THREAD));
    out
}

/// Splits the agent reply into what the person reads and the offer in its
/// last `kiln` block. The block is always removed from the text.
pub fn split_offer(reply: &str) -> (String, Option<Offer>) {
    let lines: Vec<&str> = reply.split('\n').collect();
    let close_of = |i: usize| (i + 1..lines.len()).find(|&j| lines[j].trim() == "```");
    let found = (0..lines.len())
        .rev()
        .filter(|&i| lines[i].trim() == "```kiln")
        .find_map(|i| close_of(i).map(|j| (i, j)));
    let Some((i, j)) = found else {
        return (reply.trim().to_string(), None);
    };
    let text = [&lines[..i], &lines[j + 1..]].concat().join("\n");
    let json = lines[i + 1..j].join("\n");
    let offer = serde_json::from_str::<serde_json::Value>(&json)
        .ok()
        .map(|v| clean_offer(&v))
        .filter(|o| !o.is_empty());
    (text.trim().to_string(), offer)
}

fn clean_list(v: &serde_json::Value) -> Vec<String> {
    let Some(items) = v.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.chars().count() <= MAX_VALUE_CHARS)
        .take(MAX_ITEMS)
        .map(String::from)
        .collect()
}

fn clean_offer(v: &serde_json::Value) -> Offer {
    let basics = v
        .get("basics")
        .and_then(|b| b.as_object())
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), clean_list(v)))
                .filter(|(_, l)| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.chars().count() <= MAX_TEXT_CHARS)
        .map(String::from);
    Offer {
        names: v.get("names").map(clean_list).unwrap_or_default(),
        basics,
        text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(who: &str, text: &str) -> Message {
        Message {
            who: who.into(),
            step: "1".into(),
            text: text.into(),
        }
    }

    #[test]
    fn clean_keeps_the_last_turns_of_known_speakers_and_cuts_long_texts() {
        let mut thread: Vec<Message> = (0..45).map(|i| msg("person", &format!("m{i}"))).collect();
        thread.push(msg("hacker", "x"));
        thread.push(msg("agent", &"a".repeat(5000)));
        let out = clean(thread);
        assert_eq!(out.len(), 40);
        assert!(out.iter().all(|m| m.who == "person" || m.who == "agent"));
        assert_eq!(out.last().unwrap().text.chars().count(), 4000);
        assert_eq!(out[0].text, "m6");
    }

    #[test]
    fn an_offer_summary_keeps_the_order_of_the_buttons() {
        let o = Offer {
            names: vec!["Alba".into(), "Brisa".into()],
            basics: BTreeMap::from([("camera".into(), vec!["fija".into(), "cenital".into()])]),
            text: Some("Un puerto.".into()),
        };
        assert_eq!(o.summary(), "Propuse: nombres 1) Alba 2) Brisa · camera 1) fija 2) cenital · una versión de tu explicación");
        assert_eq!(Offer::default().summary(), "");
    }

    fn chat_of(n: usize) -> Chat {
        Chat {
            messages: (1..=n).map(|i| msg("person", &format!("m{i}"))).collect(),
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_chat() {
        let p =
            std::env::temp_dir().join(format!("kiln-chat-none-{}/chat.toml", std::process::id()));
        assert_eq!(load(&p).unwrap(), Chat::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("kiln-chat-rt-{}", std::process::id()));
        let p = dir.join("kiln/chat.toml");
        let chat = Chat {
            messages: vec![msg("person", "hola"), msg("agent", "¿de qué va?")],
        };
        save(&p, &chat).unwrap();
        assert_eq!(load(&p).unwrap(), chat);
        std::fs::write(&p, "messages = 3").unwrap();
        assert!(load(&p).unwrap_err().contains("chat.toml"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_returns_the_last_ones_in_order() {
        let c = chat_of(5);
        let texts: Vec<_> = c.recent(2).iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["m4", "m5"]);
        assert_eq!(chat_of(3).recent(10).len(), 3);
    }

    fn block(json: &str) -> String {
        format!("```kiln\n{json}\n```")
    }

    #[test]
    fn no_block_returns_the_reply_trimmed() {
        assert_eq!(split_offer("  hola\n"), ("hola".to_string(), None));
    }

    #[test]
    fn a_good_block_is_parsed_and_removed() {
        let r = format!(
            "Te propongo esto.\n\n{}\n",
            block(r#"{"names":["Alba"," Brisa "],"basics":{"genre":["rpg"]},"text":"Un juego."}"#)
        );
        let (text, offer) = split_offer(&r);
        assert_eq!(text, "Te propongo esto.");
        let o = offer.unwrap();
        assert_eq!(o.names, ["Alba", "Brisa"]);
        assert_eq!(o.basics["genre"], ["rpg"]);
        assert_eq!(o.text.as_deref(), Some("Un juego."));
    }

    #[test]
    fn only_the_last_block_wins_and_is_removed() {
        let first = block(r#"{"names":["Uno"]}"#);
        let r = format!("{first}\nmedio\n{}", block(r#"{"names":["Dos"]}"#));
        let (text, offer) = split_offer(&r);
        assert_eq!(offer.unwrap().names, ["Dos"]);
        assert!(text.contains("Uno") && text.contains("medio"));
        assert!(!text.contains("Dos"));
    }

    #[test]
    fn broken_json_gives_no_offer_but_strips_the_block() {
        let (text, offer) = split_offer(&format!("Hola\n{}", block("{nope")));
        assert_eq!(text, "Hola");
        assert!(offer.is_none());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let (_, o) = split_offer(&block(r#"{"names":["A"],"extra":1}"#));
        assert_eq!(o.unwrap().names, ["A"]);
        let (_, o) = split_offer(&block(r#"{"extra":1}"#));
        assert!(o.is_none());
    }

    #[test]
    fn lists_keep_the_first_six() {
        let (_, o) = split_offer(&block(r#"{"names":["1","2","3","4","5","6","7"]}"#));
        assert_eq!(o.unwrap().names.len(), 6);
    }

    #[test]
    fn long_values_and_non_strings_are_dropped() {
        let long = "x".repeat(300);
        let ok = "ñ".repeat(200);
        let json = format!(
            r#"{{"names":["{long}","{ok}",3,null,"","  "],"text":"{}"}}"#,
            "y".repeat(4001)
        );
        let (_, o) = split_offer(&block(&json));
        let o = o.unwrap();
        assert_eq!(o.names, [ok]);
        assert!(o.text.is_none());
    }
}
