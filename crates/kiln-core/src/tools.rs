//! The external programs Kiln calls: which ones are there, and their versions.

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Tool {
    pub name: String,
    pub program: String,
    pub found: bool,
    pub version: Option<String>,
    pub min: Option<String>,
    pub ok: bool,
    pub note: Option<String>,
}

/// First `digits(.digits)+` of the output; the validator prints
/// `..., version 2.0.0-dev.3.10`, so it is read after "version ".
pub fn parse_version(name: &str, output: &str) -> Option<String> {
    let text = if name == "gltf_validator" {
        output.split_once("version ").map_or(output, |(_, rest)| rest)
    } else {
        output
    };
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut dots = 0;
        while i < bytes.len() {
            if bytes[i].is_ascii_digit() {
                i += 1;
            } else if bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                dots += 1;
                i += 1;
            } else {
                break;
            }
        }
        if dots > 0 {
            return Some(text[start..i].to_string());
        }
    }
    None
}

fn parts(version: &str) -> Vec<u64> {
    version.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Numeric compare of dotted parts; a missing part counts as 0.
pub fn older(version: &str, min: &str) -> bool {
    let (a, b) = (parts(version), parts(min));
    (0..a.len().max(b.len())).find_map(|i| {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        (x != y).then_some(x < y)
    }) == Some(true)
}

pub fn probe(name: &str, program: &str, min: Option<&str>) -> Tool {
    let mut tool = Tool {
        name: name.to_string(),
        program: program.to_string(),
        found: false,
        version: None,
        min: min.map(str::to_string),
        ok: false,
        note: None,
    };
    let Ok(out) = std::process::Command::new(program).arg("--version").output() else {
        tool.note = Some(format!("no se encuentra «{program}»"));
        return tool;
    };
    tool.found = true;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    tool.version = parse_version(name, &text);
    let is_python = std::path::Path::new(program)
        .file_name()
        .is_some_and(|f| f.to_string_lossy().starts_with("python"));
    if is_python {
        tool.ok = true;
        tool.note = Some("Python con bpy: versión de Blender sin comprobar".into());
        return tool;
    }
    match (&tool.version, min) {
        (Some(v), Some(m)) if older(v, m) => {
            tool.note = Some(format!("versión {v}: Kiln está probado desde la {m}"));
        }
        _ => tool.ok = true,
    }
    tool
}

pub fn detect(config: &crate::Config) -> Vec<Tool> {
    let claude = std::env::var("KILN_CLAUDE").unwrap_or_else(|_| "claude".into());
    vec![
        probe("blender", &config.tools.blender, Some("5.0")),
        probe("godot", &config.tools.godot, Some("4.7")),
        probe("gltf_validator", &config.tools.gltf_validator, None),
        probe("gltf-transform", &config.tools.gltf_transform, None),
        probe("claude", &claude, None),
        probe("curl", "curl", None),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_version_of_each_tool() {
        let v = |n, o| parse_version(n, o).as_deref().map(str::to_owned);
        assert_eq!(v("blender", "Blender 5.2.3 LTS\n\tbuild date"), Some("5.2.3".into()));
        assert_eq!(v("godot", "4.7.1.stable.official.a13da4feb"), Some("4.7.1".into()));
        assert_eq!(v("curl", "curl 8.7.1 (x86_64-apple-darwin) libcurl/8.7.1"), Some("8.7.1".into()));
        assert_eq!(v("claude", "2.1.0 (Claude Code)"), Some("2.1.0".into()));
        assert_eq!(v("gltf-transform", "4.1.0"), Some("4.1.0".into()));
        assert_eq!(
            v("gltf_validator", "glTF 2.0 Validator, version 2.0.0-dev.3.10"),
            Some("2.0.0".into())
        );
        assert_eq!(v("curl", "no digits here"), None);
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(older("4.4.1", "4.7"));
        assert!(!older("5.2.3", "5.0"));
        assert!(!older("5.0", "5.0"));
        assert!(!older("4.10", "4.9"));
    }

    #[cfg(unix)]
    fn fake(dir: &std::path::Path, file: &str, says: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(file);
        std::fs::write(&path, format!("#!/bin/sh\necho '{says}'\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[cfg(unix)]
    #[test]
    fn probes_fake_programs() {
        let dir = std::env::temp_dir().join(format!("kiln-tools-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = fake(&dir, "good", "Blender 5.2.3 LTS");
        let old = fake(&dir, "old", "Blender 4.2.0");
        let missing = dir.join("nope").to_string_lossy().into_owned();

        let t = probe("blender", &good, Some("5.0"));
        assert!(t.found && t.ok && t.note.is_none());
        assert_eq!(t.version.as_deref(), Some("5.2.3"));

        let t = probe("blender", &old, Some("5.0"));
        assert!(t.found && !t.ok);
        assert_eq!(t.note.as_deref(), Some("versión 4.2.0: Kiln está probado desde la 5.0"));

        let t = probe("blender", &missing, Some("5.0"));
        assert!(!t.found && !t.ok && t.version.is_none());
        assert_eq!(t.note, Some(format!("no se encuentra «{missing}»")));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
