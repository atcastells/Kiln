//! The tree of the project, by levels: the game at the top, then whatever
//! levels the project declares (place, building, room...) down to the assets,
//! which are the leaves. Each node knows how many of the assets under it
//! already have a model.

use crate::catalog::Catalog;
use crate::config::{Config, LevelSource};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Tree {
    pub root: String,
    pub nodes: BTreeMap<String, TreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    /// `level:id`, unique in the tree.
    pub key: String,
    pub id: String,
    pub level: String,
    pub name: String,
    /// Where it is declared, relative to the project root.
    pub file: Option<String>,
    pub children: Vec<String>,
    /// For a leaf: `model` (a catalog piece), `spec` (a spec, no model yet) or
    /// `missing`. For an inner node: `group`.
    pub status: String,
    /// The catalog piece of a leaf that has a model.
    pub piece: Option<String>,
    /// Distinct assets under the node, and how many have a model.
    pub assets: usize,
    pub modelled: usize,
}

const ASSET: &str = "asset";

pub fn build(root: &Path, config: &Config, cat: &Catalog) -> Tree {
    let mut nodes: BTreeMap<String, TreeNode> = BTreeMap::new();
    let add = |nodes: &mut BTreeMap<String, TreeNode>,
               level: &str,
               id: &str,
               name: Option<String>,
               file: Option<String>,
               children: Vec<String>| {
        let key = format!("{level}:{id}");
        let n = nodes.entry(key.clone()).or_insert_with(|| TreeNode {
            key: key.clone(),
            id: id.into(),
            level: level.into(),
            name: id.into(),
            file: None,
            children: Vec::new(),
            status: "group".into(),
            piece: None,
            assets: 0,
            modelled: 0,
        });
        if let Some(name) = name {
            n.name = name;
        }
        if n.file.is_none() {
            n.file = file;
        }
        for c in children {
            if !n.children.contains(&c) {
                n.children.push(c);
            }
        }
        key
    };

    for src in &config.levels {
        for (file, row) in rows(root, src) {
            let Some(id) = row.get(&src.id).and_then(|v| v.as_str()) else {
                continue;
            };
            let name = src
                .name
                .as_ref()
                .and_then(|k| row.get(k))
                .and_then(|v| v.as_str())
                .map(String::from);
            let children = src
                .children
                .iter()
                .flat_map(|c| {
                    child_ids(&row, &c.key, &c.field)
                        .into_iter()
                        .map(move |id| format!("{}:{id}", c.level))
                })
                .collect();
            add(&mut nodes, &src.level, id, name, Some(file), children);
        }
    }
    for s in &cat.specs {
        let level = s.asset.level.as_deref().unwrap_or(ASSET);
        let children = s
            .asset
            .children
            .iter()
            .map(|c| {
                if c.contains(':') {
                    c.clone()
                } else {
                    format!("{ASSET}:{c}")
                }
            })
            .collect();
        add(
            &mut nodes,
            level,
            &s.asset.id,
            Some(s.asset.name.clone()),
            Some(s.file.clone()),
            children,
        );
    }
    // Every child named but not declared is a node of its own (a leaf, usually).
    let named: Vec<String> = nodes.values().flat_map(|n| n.children.clone()).collect();
    for key in &named {
        if !nodes.contains_key(key) {
            let (level, id) = key.split_once(':').unwrap_or((ASSET, key));
            add(&mut nodes, level, id, None, None, Vec::new());
        }
    }
    // The game holds every node nobody else holds.
    let held: BTreeSet<String> = nodes.values().flat_map(|n| n.children.clone()).collect();
    let tops: Vec<String> = nodes
        .keys()
        .filter(|k| !held.contains(*k))
        .cloned()
        .collect();
    let root_key = add(
        &mut nodes,
        "game",
        "game",
        Some(config.project.name.clone()),
        Some(crate::config::FILE.into()),
        tops,
    );

    // Leaves: does the catalog have a model, or is there a spec?
    for n in nodes
        .values_mut()
        .filter(|n| n.children.is_empty() && n.key != root_key)
    {
        let low = n.id.to_lowercase();
        if let Some(p) = cat
            .pieces
            .iter()
            .find(|p| p.key.to_lowercase() == low || p.name.to_lowercase() == low)
        {
            n.status = "model".into();
            n.piece = Some(p.key.clone());
        } else if cat.specs.iter().any(|s| s.asset.id == n.id) {
            n.status = "spec".into();
        } else {
            n.status = "missing".into();
        }
    }
    // Coverage: distinct leaves under each node.
    let keys: Vec<String> = nodes.keys().cloned().collect();
    for k in keys {
        let mut leaves = BTreeSet::new();
        let mut stack = vec![k.clone()];
        let mut seen = BTreeSet::new();
        while let Some(c) = stack.pop() {
            if !seen.insert(c.clone()) {
                continue;
            }
            let Some(n) = nodes.get(&c) else { continue };
            if n.children.is_empty() && n.status != "group" {
                leaves.insert(c.clone());
            }
            stack.extend(n.children.iter().cloned());
        }
        let modelled = leaves
            .iter()
            .filter(|l| nodes.get(*l).is_some_and(|n| n.status == "model"))
            .count();
        let n = nodes.get_mut(&k).unwrap();
        n.assets = leaves.len();
        n.modelled = modelled;
    }
    Tree {
        root: root_key,
        nodes,
    }
}

/// Each row of a level source: a whole file, or each table of an array in it.
fn rows(root: &Path, src: &LevelSource) -> Vec<(String, toml::Table)> {
    let mut out = Vec::new();
    for pattern in &src.files {
        let full = root.join(pattern).to_string_lossy().replace('\\', "/");
        let mut files: Vec<_> = glob::glob(&full)
            .map(|g| g.filter_map(Result::ok).collect())
            .unwrap_or_default();
        files.sort();
        for f in files {
            let rel = f
                .strip_prefix(root)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            let Ok(table) = std::fs::read_to_string(&f)
                .unwrap_or_default()
                .parse::<toml::Table>()
            else {
                continue;
            };
            match &src.table {
                Some(t) => {
                    for row in table
                        .get(t)
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.as_table())
                    {
                        out.push((rel.clone(), row.clone()));
                    }
                }
                None => out.push((rel, table)),
            }
        }
    }
    out
}

/// The ids a row names as children: `key` is an array of tables (take `field`
/// of each) or of strings.
fn child_ids(row: &toml::Table, key: &str, field: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(one) = row.get(key).and_then(|v| v.as_str()) {
        return vec![one.to_string()];
    }
    for v in row
        .get(key)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let id = v.as_str().map(String::from).or_else(|| {
            v.as_table()
                .and_then(|t| t.get(field))
                .and_then(|f| f.as_str())
                .map(String::from)
        });
        if let Some(id) = id
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_levels_and_coverage() {
        let dir = std::env::temp_dir().join(format!("kiln-tree-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(
            dir.join("data/house.toml"),
            "id = \"house\"\nneeds = [{ use = \"kitchen\" }]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("data/rooms.toml"),
            "[[rooms]]\nuse = \"kitchen\"\nfixtures = [{ id = \"barrel\" }, { id = \"stove\" }]\n",
        )
        .unwrap();
        let config = Config::parse(
            r#"[project]
name = "Test"
[[levels]]
level = "building"
files = ["data/house.toml"]
id = "id"
children = [{ key = "needs", field = "use", level = "room" }]
[[levels]]
level = "room"
files = ["data/rooms.toml"]
table = "rooms"
id = "use"
children = [{ key = "fixtures", field = "id", level = "asset" }]
"#,
        )
        .unwrap();
        let mut cat = crate::catalog::index(&dir, &config);
        cat.pieces.push(crate::catalog::Piece {
            key: "furniture/barrel".into(),
            kit: "furniture".into(),
            name: "barrel".into(),
            file: None,
            format: "glb".into(),
            size_m: None,
            min: None,
            tris: None,
            material_slots: vec![],
            images: None,
            licenses: vec![],
            credit: None,
            manifest: Default::default(),
        });
        let tree = build(&dir, &config, &cat);
        let game = &tree.nodes[&tree.root];
        assert_eq!(game.children, ["building:house"]);
        assert_eq!((game.assets, game.modelled), (2, 1));
        assert_eq!(tree.nodes["asset:stove"].status, "missing");
        assert_eq!(
            tree.nodes["asset:barrel"].piece.as_deref(),
            Some("furniture/barrel")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
