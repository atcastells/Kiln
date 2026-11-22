//! R14: every `/api/*` route of the page has an MCP tool, a prompt, or an
//! explicit exclusion. Routes are read from the text of `src/serve.rs`.
mod common;

use common::{Mcp, temp_project};
use std::collections::BTreeSet;

const SERVE: &str = include_str!("../src/serve.rs");

enum Cover {
    Tool(&'static [&'static str]),
    Prompt(&'static str, &'static [&'static str]),
    Excluded(&'static str),
}
use Cover::*;

/// Route key is "METHOD /api/path"; prefix arms end in "*".
const TABLE: &[(&str, Cover)] = &[
    ("GET /api/new", Tool(&["new_game_options"])),
    ("POST /api/new", Tool(&["new_game"])),
    ("GET /api/jobs/*", Tool(&["job", "jobs"])),
    ("GET /api/jobs", Tool(&["jobs"])),
    ("GET /api/session", Tool(&["status"])),
    ("GET /api/catalog", Tool(&["catalog"])),
    ("GET /api/context", Tool(&["context_get"])),
    ("POST /api/ask", Prompt("kiln-asset", &["spec_write", "spec_layout"])),
    ("POST /api/build", Tool(&["build"])),
    ("POST /api/generate", Tool(&["generate"])),
    ("POST /api/review", Tool(&["review", "review_sheets"])),
    ("POST /api/godot", Tool(&["godot_measure"])),
    ("POST /api/search", Tool(&["search"])),
    ("POST /api/import", Tool(&["import"])),
    ("POST /api/bind", Tool(&["bind"])),
    ("POST /api/material", Tool(&["material_set"])),
    ("POST /api/validate", Tool(&["material_validate"])),
    ("POST /api/split", Tool(&["zones_split"])),
    ("POST /api/paint", Tool(&["paint"])),
    ("POST /api/settle", Tool(&["settle"])),
    ("POST /api/context/sort", Prompt("kiln-context", &["context_propose"])),
    ("POST /api/context/refine", Prompt("kiln-context", &["context_propose"])),
    ("POST /api/context/*", Tool(&["context_edit"])),
    ("POST /api/chat", Excluded("D2: the chat is the page's own agent")),
    ("GET /api/chat", Excluded("D2: the chat is the page's own agent")),
];

const MAIN: &str = include_str!("../src/main.rs");

/// Every command of the CLI has a tool or an explicit exclusion.
const COMMANDS: &[(&str, Cover)] = &[
    ("index", Tool(&["catalog", "status"])),
    ("check", Tool(&["check"])),
    ("tools", Tool(&["tools"])),
    ("mcp", Excluded("the MCP server itself")),
    ("trust", Excluded("the person's own decision: the agent must not be able to grant it")),
    ("serve", Excluded("the page; its routes are covered by TABLE")),
    ("stop", Excluded("the person stops the core that serves the MCP itself")),
    ("build", Tool(&["build"])),
    ("generate", Tool(&["generate"])),
    ("verify", Tool(&["verify"])),
    ("publish", Tool(&["publish"])),
    ("paint", Tool(&["paint"])),
    ("review", Tool(&["review"])),
    ("godot", Tool(&["godot_measure"])),
    ("content-manifest", Tool(&["content_manifest"])),
];

/// The commands of `main`: the `"name" =>` arms after `match args.cmd`.
fn commands(src: &str) -> BTreeSet<String> {
    let body = src.split("match args.cmd.as_str()").nth(1).unwrap_or_default();
    body.lines()
        .filter_map(|l| {
            let l = l.trim_start().strip_prefix('"')?;
            let (name, rest) = l.split_once('"')?;
            let ok = rest.trim_start().starts_with("=>")
                && name.starts_with(|c: char| c.is_ascii_lowercase())
                && name.chars().all(|c| c.is_ascii_lowercase() || c == '-');
            ok.then(|| name.to_string())
        })
        .collect()
}

#[test]
fn every_cli_command_is_covered() {
    let found = commands(MAIN);
    assert!(found.len() >= 12, "extractor found too few commands: {found:?}");
    let known: BTreeSet<&str> = COMMANDS.iter().map(|(k, _)| *k).collect();
    let missing: Vec<_> = found.iter().filter(|c| !known.contains(c.as_str())).collect();
    assert!(missing.is_empty(), "CLI commands without tool or exclusion: {missing:?}");
    let stale: Vec<_> = known.iter().filter(|k| !found.contains(**k)).collect();
    assert!(stale.is_empty(), "rows whose command is gone from main.rs: {stale:?}");
}

#[test]
fn command_tools_exist() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools: BTreeSet<String> =
        m.tools().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    for (cmd, cover) in COMMANDS {
        match cover {
            Tool(names) => {
                for n in *names {
                    assert!(tools.contains(*n), "{cmd}: tool {n} not in tools/list");
                }
            }
            Excluded(reason) => assert!(!reason.is_empty(), "{cmd}: exclusion needs a reason"),
            Prompt(..) => unreachable!("commands are covered by tools"),
        }
    }
}

/// Routes of the `/api/` match arms: literal `("GET"|"POST", "/api/..")` and
/// prefix `("GET"|"POST", p) if p.starts_with("/api/..")`. Test code is skipped.
fn routes(src: &str) -> BTreeSet<String> {
    let src = src.split("#[cfg(test)]").next().unwrap();
    let mut out = BTreeSet::new();
    for line in src.lines() {
        let line = line.trim_start();
        if line.starts_with("//") {
            continue;
        }
        for method in ["GET", "POST"] {
            let lit = format!("(\"{method}\", \"");
            if let Some(i) = line.find(&lit) {
                let rest = &line[i + lit.len()..];
                if let Some(end) = rest.find("\")")
                    && rest[..end].starts_with("/api/")
                {
                    out.insert(format!("{method} {}", &rest[..end]));
                }
            }
            // a route answered before the match: `req.method == "GET" && req.path == "/api/.."`
            let cond = format!("req.method == \"{method}\" && req.path == \"");
            if let Some(i) = line.find(&cond) {
                let rest = &line[i + cond.len()..];
                if let Some(end) = rest.find('"')
                    && rest[..end].starts_with("/api/")
                {
                    out.insert(format!("{method} {}", &rest[..end]));
                }
            }
            let pre = format!("(\"{method}\", p) if p.starts_with(\"");
            if let Some(i) = line.find(&pre) {
                let rest = &line[i + pre.len()..];
                if let Some(end) = rest.find("\")")
                    && rest[..end].starts_with("/api/")
                {
                    out.insert(format!("{method} {}*", &rest[..end]));
                }
            }
        }
    }
    out
}

fn uncovered(found: &BTreeSet<String>) -> Vec<String> {
    let known: BTreeSet<&str> = TABLE.iter().map(|(k, _)| *k).collect();
    found.iter().filter(|r| !known.contains(r.as_str())).cloned().collect()
}

fn stale(found: &BTreeSet<String>) -> Vec<&'static str> {
    TABLE.iter().map(|(k, _)| *k).filter(|k| !found.contains(*k)).collect()
}

#[test]
fn extractor_finds_literal_and_prefix_arms() {
    let sample = r#"
        ("GET", "/api/one") => a,
        ("POST", p) if p.starts_with("/api/two/") => b,
        ("GET", "/index.html") => c,
        // ("GET", "/api/commented") => d,
    "#;
    let r = routes(sample);
    assert_eq!(r.into_iter().collect::<Vec<_>>(), ["GET /api/one", "POST /api/two/*"]);
}

#[test]
fn extractor_finds_a_route_answered_before_the_match() {
    let sample = r#"    if req.method == "GET" && req.path == "/api/early" {"#;
    assert!(routes(sample).contains("GET /api/early"));
}

#[test]
fn a_new_route_without_tool_is_reported() {
    let sample = format!("(\"POST\", \"/api/invented\") => x,\n{SERVE}");
    let r = routes(&sample);
    assert_eq!(uncovered(&r), ["POST /api/invented"]);
}

#[test]
fn every_api_route_is_covered() {
    let found = routes(SERVE);
    assert!(found.len() >= 20, "extractor found too few routes: {found:?}");
    assert_eq!(uncovered(&found), Vec::<String>::new(), "routes without tool, prompt or exclusion");
}

#[test]
fn table_has_no_stale_rows() {
    assert_eq!(stale(&routes(SERVE)), Vec::<&str>::new(), "table rows whose route is gone from serve.rs");
}

#[test]
fn named_tools_and_prompts_exist() {
    let p = temp_project("");
    let mut m = Mcp::start(p.path());
    let tools: BTreeSet<String> =
        m.tools().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    let prompts: BTreeSet<String> = m.request("prompts/list", serde_json::json!({}))["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    for (route, cover) in TABLE {
        let (prompt, names): (Option<&str>, &[&str]) = match cover {
            Tool(n) => (None, n),
            Prompt(p, n) => (Some(p), n),
            Excluded(reason) => {
                assert!(!reason.is_empty(), "{route}: exclusion needs a reason");
                continue;
            }
        };
        for n in names {
            assert!(tools.contains(*n), "{route}: tool {n} not in tools/list ({tools:?})");
        }
        if let Some(p) = prompt {
            assert!(prompts.contains(p), "{route}: prompt {p} not in prompts/list ({prompts:?})");
        }
    }
}
