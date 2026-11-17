//! The contract of a built asset (`asset.json`): what it is, what it was
//! made from, what came out and with which tools.

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Only regular files are read (never a device or a pipe), and in pieces, so
/// a large file does not have to fit in memory.
pub fn sha256_file(path: &std::path::Path) -> Result<String, String> {
    use sha2::Digest;
    use std::io::Read;
    let unreadable =
        |e: &dyn std::fmt::Display| format!("no se puede leer {}: {e}", path.display());
    let mut file = std::fs::File::open(path).map_err(|e| unreadable(&e))?;
    if !file.metadata().map_err(|e| unreadable(&e))?.is_file() {
        return Err(unreadable(&"no es un fichero"));
    }
    let mut hasher = sha2::Sha256::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut chunk).map_err(|e| unreadable(&e))?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Writes a file Kiln owns in place of whatever has that name. A link left
/// there is removed, never followed: the folders Kiln writes in belong to a
/// project that anyone may have prepared.
pub fn write_replacing(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}

/// The sha256 of a file of the project, named relative to its root. Nothing
/// when the name leaves the project (absolute, `..`, or a link that leads
/// out) or is not a regular file: such names come from files of the project
/// that anyone may have written.
pub fn sha256_inside(root: &std::path::Path, file: &str) -> Option<String> {
    let rel = std::path::Path::new(file);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return None;
    }
    let real = std::fs::canonicalize(root.join(rel)).ok()?;
    if !real.starts_with(std::fs::canonicalize(root).ok()?) {
        return None;
    }
    sha256_file(&real).ok()
}

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::{Catalog, Spec, glb, tools::Tool};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Contract {
    pub id: String,
    pub name: String,
    pub function: Function,
    pub source: Source,
    pub outputs: Vec<Output>,
    pub size_m: Option<[f64; 3]>,
    pub tris: Option<u64>,
    pub axes: Axes,
    pub anchors: Vec<crate::spec::Anchor>,
    /// GLB material name (`part/zone`) -> palette key it wears.
    pub materials: BTreeMap<String, Option<String>>,
    pub licenses: Vec<String>,
    pub credits: Vec<String>,
    pub tools: Vec<ToolRef>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Function {
    pub used_by: Vec<String>,
    #[serde(rename = "where")]
    pub where_: String,
    pub states: Vec<String>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Source {
    pub spec: FileRef,
    pub pieces: Vec<PieceRef>,
    /// Ids of the assets it is made of.
    pub assets: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileRef {
    /// Relative to the project root, `/` separators.
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PieceRef {
    pub key: String,
    pub file: Option<String>,
    pub sha256: Option<String>,
    pub format: String,
    pub licenses: Vec<String>,
    pub credit: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Output {
    /// None is the base look; Some is a colour variant.
    pub variant: Option<String>,
    /// File name only.
    pub file: String,
    pub sha256: String,
    pub structure_sha256: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Axes {
    pub up: String,
    pub units: String,
    pub origin: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolRef {
    pub name: String,
    pub version: Option<String>,
}

/// What the build knows when it writes the contract.
pub struct Built<'a> {
    pub root: &'a Path,
    pub spec: &'a Spec,
    pub catalog: &'a Catalog,
    pub size_m: Option<[f64; 3]>,
    pub tris: Option<u64>,
    pub materials: BTreeMap<String, Option<String>>,
    /// (variant, GLB file) of every look that exists on disk; the base first.
    pub outputs: Vec<(Option<String>, PathBuf)>,
    pub tools: &'a [Tool],
}

pub fn contract(b: &Built) -> Result<Contract, String> {
    let spec = b.spec;
    let spec_path = spec.file.replace('\\', "/");
    let spec_ref = FileRef {
        sha256: sha256_file(&b.root.join(&spec.file))?,
        path: spec_path,
    };

    let mut keys = BTreeSet::new();
    let mut assets = BTreeSet::new();
    for part in &spec.parts {
        let Some(from) = &part.source.from else {
            continue;
        };
        if part.source.kind == "asset" {
            assets.insert(from.clone());
        } else {
            keys.insert(from.clone());
        }
    }
    let mut pieces = Vec::new();
    for key in &keys {
        let Some(p) = b.catalog.pieces.iter().find(|p| &p.key == key) else {
            continue;
        };
        let sha256 = p
            .file
            .as_ref()
            .map(|f| b.root.join(f))
            .filter(|f| f.is_file())
            .and_then(|f| sha256_file(&f).ok());
        pieces.push(PieceRef {
            key: p.key.clone(),
            file: p.file.clone(),
            sha256,
            format: p.format.clone(),
            licenses: p.licenses.clone(),
            credit: p.credit.clone(),
        });
    }
    let licenses: BTreeSet<String> = pieces.iter().flat_map(|p| p.licenses.clone()).collect();
    let credits: BTreeSet<String> = pieces.iter().filter_map(|p| p.credit.clone()).collect();

    let mut outputs = Vec::new();
    for (variant, path) in &b.outputs {
        let structure = glb::structure(path)?;
        outputs.push(Output {
            variant: variant.clone(),
            file: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            sha256: sha256_file(path)?,
            structure_sha256: glb::structure_hash(&structure),
        });
    }

    Ok(Contract {
        id: spec.asset.id.clone(),
        name: spec.asset.name.clone(),
        function: Function {
            used_by: spec.asset.used_by.clone(),
            where_: spec.asset.where_.clone(),
            states: spec.asset.states.clone(),
            level: spec.asset.level.clone(),
        },
        source: Source {
            spec: spec_ref,
            pieces,
            assets: assets.into_iter().collect(),
        },
        outputs,
        size_m: b.size_m,
        tris: b.tris,
        axes: Axes {
            up: "+Y".into(),
            units: "m".into(),
            origin: "base centre".into(),
        },
        anchors: spec.anchors.clone(),
        materials: b.materials.clone(),
        licenses: licenses.into_iter().collect(),
        credits: credits.into_iter().collect(),
        tools: b
            .tools
            .iter()
            .filter(|t| t.found)
            .map(|t| ToolRef {
                name: t.name.clone(),
                version: t.version.clone(),
            })
            .collect(),
    })
}

/// The pieces of a written contract whose source file is no longer the one
/// the asset was built from: changed since, or gone. Keys, sorted.
pub fn changed_pieces(root: &std::path::Path, contract: &serde_json::Value) -> Vec<String> {
    let mut out: Vec<String> = contract["source"]["pieces"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let (key, file, sha) = (
                p["key"].as_str()?,
                p["file"].as_str()?,
                p["sha256"].as_str()?,
            );
            let now = sha256_inside(root, file);
            (now.as_deref() != Some(sha)).then(|| key.to_string())
        })
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Piece;

    const SPEC: &str = r#"
[asset]
id = "shed"
name = "Shed"
used_by = ["world"]
where = "yard"

[[parts]]
id = "wall"
what = "wall"
[parts.source]
kind = "reuse"
from = "kit/wall"

[[parts]]
id = "wall2"
what = "wall again"
[parts.source]
kind = "reuse"
from = "kit/wall"

[[parts]]
id = "door"
what = "door"
[parts.source]
kind = "reuse"
from = "kit/missing"

[[parts]]
id = "kid"
what = "child"
[parts.source]
kind = "asset"
from = "child"
"#;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kiln-asset-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn piece(key: &str, file: Option<&str>, lic: &[&str], credit: Option<&str>) -> Piece {
        Piece {
            key: key.into(),
            kit: "kit".into(),
            name: key.into(),
            file: file.map(String::from),
            format: "glb".into(),
            size_m: None,
            min: None,
            tris: None,
            material_slots: vec![],
            split: None,
            images: None,
            licenses: lic.iter().map(|s| s.to_string()).collect(),
            credit: credit.map(String::from),
            manifest: BTreeMap::new(),
            measured_by: None,
            preview: None,
            bindings: BTreeMap::new(),
        }
    }

    fn glb_bytes(node: &str) -> Vec<u8> {
        let mut json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scenes":[{{"nodes":[0]}}],"nodes":[{{"name":"{node}"}}]}}"#
        )
        .into_bytes();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend((20 + json.len() as u32).to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json);
        out
    }

    struct World {
        dir: PathBuf,
        spec: Spec,
        catalog: Catalog,
    }

    fn world(name: &str) -> World {
        let dir = tmp(name);
        std::fs::create_dir_all(dir.join("specs")).unwrap();
        std::fs::write(dir.join("specs/shed.toml"), SPEC).unwrap();
        std::fs::write(dir.join("wall.glb"), glb_bytes("wall")).unwrap();
        let mut spec = crate::spec::parse(SPEC).unwrap();
        spec.file = "specs/shed.toml".into();
        let mut catalog = Catalog::default();
        catalog.pieces.push(piece(
            "kit/wall",
            Some("wall.glb"),
            &["MIT", "CC0"],
            Some("B credit"),
        ));
        catalog
            .pieces
            .push(piece("kit/door", Some("gone.glb"), &["CC0", "CC-BY"], None));
        World { dir, spec, catalog }
    }

    fn built<'a>(w: &'a World, outputs: Vec<(Option<String>, PathBuf)>) -> Built<'a> {
        Built {
            root: &w.dir,
            spec: &w.spec,
            catalog: &w.catalog,
            size_m: Some([1., 2., 3.]),
            tris: Some(12),
            materials: BTreeMap::new(),
            outputs,
            tools: &[],
        }
    }

    #[test]
    fn pieces_are_deduplicated_and_assets_listed() {
        let w = world("dedup");
        let c = contract(&built(&w, vec![])).unwrap();
        assert_eq!(c.source.pieces.len(), 1);
        let p = &c.source.pieces[0];
        assert_eq!(p.key, "kit/wall");
        assert_eq!(p.sha256, Some(sha256_hex(&glb_bytes("wall"))));
        assert_eq!(p.licenses, ["MIT", "CC0"]);
        assert_eq!(p.credit.as_deref(), Some("B credit"));
        assert_eq!(c.source.assets, ["child"]);
        assert_eq!(c.source.spec.path, "specs/shed.toml");
        assert_eq!(c.source.spec.sha256, sha256_hex(SPEC.as_bytes()));
        assert_eq!(c.axes.up, "+Y");
        std::fs::remove_dir_all(&w.dir).unwrap();
    }

    #[test]
    fn licenses_and_credits_are_the_sorted_union() {
        let mut w = world("union");
        w.spec.parts[2].source.from = Some("kit/door".into());
        let c = contract(&built(&w, vec![])).unwrap();
        assert_eq!(c.licenses, ["CC-BY", "CC0", "MIT"]);
        assert_eq!(c.credits, ["B credit"]);
        std::fs::remove_dir_all(&w.dir).unwrap();
    }

    #[test]
    fn missing_piece_file_has_no_hash() {
        let mut w = world("nofile");
        w.spec.parts[2].source.from = Some("kit/door".into());
        let c = contract(&built(&w, vec![])).unwrap();
        let door = c
            .source
            .pieces
            .iter()
            .find(|p| p.key == "kit/door")
            .unwrap();
        assert_eq!(door.sha256, None);
        std::fs::remove_dir_all(&w.dir).unwrap();
    }

    #[test]
    fn outputs_carry_both_hashes_and_serialize_identically() {
        let w = world("outputs");
        let base = w.dir.join("shed.glb");
        let red = w.dir.join("shed_red.glb");
        std::fs::write(&base, glb_bytes("shed")).unwrap();
        std::fs::write(&red, glb_bytes("shed")).unwrap();
        let outs = vec![(None, base.clone()), (Some("red".to_string()), red)];
        let a = contract(&built(&w, outs.clone())).unwrap();
        let b = contract(&built(&w, outs)).unwrap();
        assert_eq!(a.outputs.len(), 2);
        assert_eq!(a.outputs[0].variant, None);
        assert_eq!(a.outputs[0].file, "shed.glb");
        assert_eq!(a.outputs[0].sha256, sha256_file(&base).unwrap());
        assert_eq!(a.outputs[0].structure_sha256.len(), 64);
        assert_eq!(a.outputs[1].variant.as_deref(), Some("red"));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        std::fs::remove_dir_all(&w.dir).unwrap();
    }

    #[test]
    fn missing_output_is_an_error() {
        let w = world("noout");
        let r = contract(&built(&w, vec![(None, w.dir.join("nope.glb"))]));
        assert!(r.is_err());
        std::fs::remove_dir_all(&w.dir).unwrap();
    }

    #[test]
    fn sha256_of_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_file_reads_and_reports_missing() {
        let p = std::env::current_exe().unwrap();
        assert_eq!(sha256_file(&p).unwrap().len(), 64);
        let err = sha256_file(std::path::Path::new("/no/such/file")).unwrap_err();
        assert!(err.starts_with("no se puede leer /no/such/file"));
    }

    #[test]
    fn changed_pieces_names_the_sources_that_are_no_longer_the_built_ones() {
        let root = std::env::temp_dir().join(format!("kiln-asset-changed-{}", std::process::id()));
        std::fs::create_dir_all(root.join("kits/k")).unwrap();
        std::fs::write(root.join("kits/k/a.blend"), b"one").unwrap();
        std::fs::write(root.join("kits/k/b.glb"), b"two").unwrap();
        let contract = serde_json::json!({ "source": { "pieces": [
            { "key": "k/a", "file": "kits/k/a.blend", "sha256": sha256_hex(b"one") },
            { "key": "k/b", "file": "kits/k/b.glb", "sha256": sha256_hex(b"two") },
            { "key": "k/gone", "file": "kits/k/gone.glb", "sha256": sha256_hex(b"x") },
            { "key": "k/unhashed", "file": null, "sha256": null },
        ] } });
        assert_eq!(changed_pieces(&root, &contract), ["k/gone"]);
        std::fs::write(root.join("kits/k/a.blend"), b"one, retouched").unwrap();
        assert_eq!(changed_pieces(&root, &contract), ["k/a", "k/gone"]);
        assert!(changed_pieces(&root, &serde_json::json!({})).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn hashing_stays_inside_the_project_and_on_regular_files() {
        let base = std::env::temp_dir().join(format!("kiln-asset-inside-{}", std::process::id()));
        let (root, out) = (base.join("game"), base.join("out"));
        std::fs::create_dir_all(root.join("kits")).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(root.join("kits/a.glb"), b"abc").unwrap();
        std::fs::write(out.join("secret"), b"abc").unwrap();
        std::os::unix::fs::symlink(out.join("secret"), root.join("kits/link.blend")).unwrap();
        std::os::unix::fs::symlink("/dev/zero", root.join("kits/zero.blend")).unwrap();
        assert_eq!(sha256_inside(&root, "kits/a.glb"), Some(sha256_hex(b"abc")));
        assert_eq!(sha256_inside(&root, "kits/link.blend"), None);
        assert_eq!(sha256_inside(&root, "kits/zero.blend"), None);
        assert_eq!(sha256_inside(&root, "/dev/zero"), None);
        assert_eq!(sha256_inside(&root, "../out/secret"), None);
        assert!(sha256_file(std::path::Path::new("/dev/zero")).is_err());
        // a contract that names such files is answered at once, as changed
        let contract = serde_json::json!({ "source": { "pieces": [
            { "key": "k/zero", "file": "/dev/zero", "sha256": "00" },
        ] } });
        assert_eq!(changed_pieces(&root, &contract), ["k/zero"]);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn write_replacing_replaces_a_link_instead_of_following_it() {
        let dir = std::env::temp_dir().join(format!("kiln-asset-replace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (victim, file) = (dir.join("victim"), dir.join("report.log"));
        std::fs::write(&victim, b"untouched").unwrap();
        std::os::unix::fs::symlink(&victim, &file).unwrap();
        write_replacing(&file, b"new").unwrap();
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
        assert_eq!(std::fs::read(&file).unwrap(), b"new");
        assert!(!file.symlink_metadata().unwrap().is_symlink());
        write_replacing(&file, b"again").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"again");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
