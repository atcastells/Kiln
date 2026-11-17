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
        output
            .split_once("version ")
            .map_or(output, |(_, rest)| rest)
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
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        (x != y).then_some(x < y)
    }) == Some(true)
}

/// Where the program is, without running it: a path as given, or a name
/// looked up in the folders of `PATH`.
pub fn locate(program: &str) -> Option<std::path::PathBuf> {
    let runnable = |p: &std::path::Path| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            p.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            p.is_file()
        }
    };
    let given = std::path::Path::new(program);
    if given.components().count() > 1 {
        return runnable(given).then(|| given.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|p| runnable(p))
}

/// What `<program> --version` prints, or nothing when it cannot be run or
/// does not answer in `PROBE_TIMEOUT`.
fn version_output(program: &str) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= PROBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Some(String::new());
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(_) => return Some(String::new()),
        }
    }
    let mut text = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut text);
    }
    text.push('\n');
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut text);
    }
    Some(text)
}

const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

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
    let Some(text) = version_output(program) else {
        tool.note = Some(format!("no se encuentra «{program}»"));
        return tool;
    };
    tool.found = true;
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

/// The tools Kiln knows of: name, configured program and oldest version tested.
fn known(config: &crate::Config) -> Vec<(&'static str, String, Option<&'static str>)> {
    let claude = std::env::var("KILN_CLAUDE").unwrap_or_else(|_| "claude".into());
    vec![
        ("blender", config.tools.blender.clone(), Some("5.0")),
        ("godot", config.tools.godot.clone(), Some("4.7")),
        ("gltf_validator", config.tools.gltf_validator.clone(), None),
        ("gltf-transform", config.tools.gltf_transform.clone(), None),
        ("claude", claude, None),
        ("curl", "curl".into(), None),
    ]
}

/// Runs each tool with `--version`. The programs come from `kiln.toml`, so
/// this is for a person or an agent that asked for it, never for a plain read.
pub fn detect(config: &crate::Config) -> Vec<Tool> {
    known(config)
        .into_iter()
        .map(|(name, program, min)| probe(name, &program, min))
        .collect()
}

/// Which tools are there, without running any: no versions.
pub fn present(config: &crate::Config) -> Vec<Tool> {
    known(config)
        .into_iter()
        .map(|(name, program, min)| {
            let found = locate(&program).is_some();
            Tool {
                name: name.to_string(),
                note: (!found).then(|| format!("no se encuentra «{program}»")),
                program,
                found,
                version: None,
                min: min.map(str::to_string),
                ok: found,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_version_of_each_tool() {
        let v = |n, o| parse_version(n, o).as_deref().map(str::to_owned);
        assert_eq!(
            v("blender", "Blender 5.2.3 LTS\n\tbuild date"),
            Some("5.2.3".into())
        );
        assert_eq!(
            v("godot", "4.7.1.stable.official.a13da4feb"),
            Some("4.7.1".into())
        );
        assert_eq!(
            v("curl", "curl 8.7.1 (x86_64-apple-darwin) libcurl/8.7.1"),
            Some("8.7.1".into())
        );
        assert_eq!(v("claude", "2.1.0 (Claude Code)"), Some("2.1.0".into()));
        assert_eq!(v("gltf-transform", "4.1.0"), Some("4.1.0".into()));
        assert_eq!(
            v(
                "gltf_validator",
                "glTF 2.0 Validator, version 2.0.0-dev.3.10"
            ),
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
        assert_eq!(
            t.note.as_deref(),
            Some("versión 4.2.0: Kiln está probado desde la 5.0")
        );

        let t = probe("blender", &missing, Some("5.0"));
        assert!(!t.found && !t.ok && t.version.is_none());
        assert_eq!(t.note, Some(format!("no se encuentra «{missing}»")));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn present_finds_tools_without_running_them() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("kiln-tools-present-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (tool, mark) = (dir.join("blender"), dir.join("ran"));
        std::fs::write(&tool, format!("#!/bin/sh\ntouch {}\n", mark.display())).unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let plain = dir.join("not-executable");
        std::fs::write(&plain, "x").unwrap();
        let config = crate::Config::parse(&format!(
            "[project]\nname = \"t\"\n[tools]\nblender = \"{}\"\ngodot = \"{}\"\n",
            tool.display(),
            plain.display()
        ))
        .unwrap();
        let found = present(&config);
        assert!(found[0].found && found[0].ok && found[0].version.is_none());
        assert!(
            !found[1].found && !found[1].ok,
            "a file that cannot run is not a tool"
        );
        assert!(!mark.exists(), "present must not run the tool");
        assert!(locate("sh").is_some() && locate("kiln-no-such-tool").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
