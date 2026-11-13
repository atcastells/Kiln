//! Searches the asset sites for a `search` part and brings in the model the
//! person picks: the network goes through `curl`, which already knows the
//! machine's proxy and certificates. Keys come from the environment
//! (`KILN_POLYPIZZA_KEY`, `KILN_SKETCHFAB_TOKEN`) or `~/.config/kiln/keys.toml`
//! and never reach the page or the agent.

use kiln_core::sources::{self, Candidate, Origin};
use kiln_core::{Config, spec};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PER_SITE: usize = 12;

#[derive(Debug, Serialize)]
pub struct Found {
    pub query: String,
    pub size_m: Option<[f64; 3]>,
    pub budget_tris: Option<u64>,
    pub candidates: Vec<Candidate>,
    /// One line per site that could not answer, or what was left out.
    pub notes: Vec<String>,
}

fn key(name: &str) -> Option<String> {
    let env = format!("KILN_{}", name.to_uppercase());
    if let Ok(v) = std::env::var(&env) {
        return Some(v.trim().to_string()).filter(|v| !v.is_empty());
    }
    let home = std::env::var("HOME").ok()?;
    let text = std::fs::read_to_string(Path::new(&home).join(".config/kiln/keys.toml")).ok()?;
    let t: toml::Table = toml::from_str(&text).ok()?;
    let short = name.split('_').next().unwrap_or(name);
    t.get(short)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// GET through curl; the header (a key) goes in by stdin, not the command line.
fn curl(url: &str, header: Option<String>, out: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "120", "-A", "kiln/0.1"]);
    if header.is_some() {
        cmd.args(["-H", "@-"]);
    }
    if let Some(o) = out {
        cmd.arg("-o").arg(o);
    }
    cmd.arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("no se puede lanzar curl: {e}"))?;
    if let (Some(h), Some(mut stdin)) = (header, child.stdin.take()) {
        let _ = stdin.write_all(h.as_bytes());
    }
    let res = child.wait_with_output().map_err(|e| e.to_string())?;
    if !res.status.success() {
        let host = url.split('/').nth(2).unwrap_or(url);
        let err = String::from_utf8_lossy(&res.stderr);
        return Err(format!(
            "{host}: {}",
            err.trim().trim_start_matches("curl: ")
        ));
    }
    Ok(res.stdout)
}

fn get_json(url: &str, header: Option<String>) -> Result<Value, String> {
    let bytes = curl(url, header, None)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("respuesta ilegible de {url}: {e}"))
}

fn enc(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn search_site(site: &str, query: &str, cfg: &Config) -> Result<Vec<Candidate>, String> {
    match site {
        "polyhaven" => {
            let list = get_json("https://api.polyhaven.com/assets?t=models", None)?;
            Ok(sources::parse_polyhaven(&list, query, PER_SITE))
        }
        "polypizza" => {
            let k = key("polypizza_key").ok_or(
                "Poly Pizza pide una clave gratuita: ponla en KILN_POLYPIZZA_KEY o en ~/.config/kiln/keys.toml (polypizza = \"…\")",
            )?;
            let v = get_json(
                &format!(
                    "https://api.poly.pizza/v1/search/{}?Limit={PER_SITE}",
                    enc(query)
                ),
                Some(format!("X-Auth-Token: {k}")),
            )?;
            Ok(sources::parse_polypizza(&v))
        }
        "sketchfab" => {
            let mut out = Vec::new();
            for lic in sources::sketchfab_licenses(&cfg.rules.licenses) {
                let v = get_json(
                    &format!(
                        "https://api.sketchfab.com/v3/search?type=models&downloadable=true&count={}&license={lic}&q={}",
                        PER_SITE / 2,
                        enc(query)
                    ),
                    None,
                )?;
                out.extend(sources::parse_sketchfab(&v));
            }
            Ok(out)
        }
        other => Err(format!("sitio desconocido «{other}»")),
    }
}

fn find_part<'a>(
    cat: &'a kiln_core::Catalog,
    asset: &str,
    part: &str,
) -> Result<(&'a kiln_core::Spec, &'a spec::Part), String> {
    let s = cat
        .specs
        .iter()
        .find(|s| s.asset.id == asset)
        .ok_or(format!("no hay ficha con id «{asset}»"))?;
    let p = s
        .parts
        .iter()
        .find(|p| p.id == part)
        .ok_or(format!("la ficha no tiene la pieza «{part}»"))?;
    Ok((s, p))
}

/// The candidates of every active site for one part, with the licences
/// the project does not admit left out.
pub fn search(
    project: &Path,
    config: Option<&Path>,
    asset: &str,
    part: &str,
) -> Result<Found, String> {
    let cfg = crate::production::config(project, config)?;
    let cat = crate::load(project, config)?;
    let (s, p) = find_part(&cat, asset, part)?;
    let query = p
        .source
        .query
        .clone()
        .filter(|q| !q.trim().is_empty())
        .unwrap_or(p.what.clone());
    let mut all = Vec::new();
    let mut notes = Vec::new();
    // the sites answer at once, each in its own thread
    let answers: Vec<(String, Result<Vec<Candidate>, String>)> = std::thread::scope(|sc| {
        let hs: Vec<_> = cfg
            .sources
            .sites
            .iter()
            .map(|site| {
                let (q, c) = (&query, &cfg);
                (site.clone(), sc.spawn(move || search_site(site, q, c)))
            })
            .collect();
        hs.into_iter()
            .map(|(site, h)| (site, h.join().unwrap_or(Err("falló".into()))))
            .collect()
    });
    for (site, r) in answers {
        match r {
            Ok(c) => all.extend(c),
            Err(e) => notes.push(format!("{site}: {e}")),
        }
    }
    let (candidates, dropped) = sources::admitted(all, &cfg.rules.licenses);
    if dropped > 0 {
        notes.push(format!(
            "{dropped} sin una licencia que admita el proyecto ({})",
            cfg.rules.licenses.join(", ")
        ));
    }
    Ok(Found {
        query,
        size_m: p.size_m,
        budget_tris: s.asset.budget_tris,
        candidates,
        notes,
    })
}

/// Today as YYYY-MM-DD (UTC).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The candidate as the site describes it now: the licence is read again
/// here, never taken from the page.
fn lookup(site: &str, id: &str) -> Result<Candidate, String> {
    let one = |v: Value| serde_json::json!({ "results": [v] });
    let c = match site {
        "polyhaven" => {
            let v = get_json(&format!("https://api.polyhaven.com/info/{}", enc(id)), None)?;
            Some(sources::polyhaven_candidate(id, &v))
        }
        "polypizza" => {
            let k =
                key("polypizza_key").ok_or("falta la clave de Poly Pizza (KILN_POLYPIZZA_KEY)")?;
            let v = get_json(
                &format!("https://api.poly.pizza/v1/model/{}", enc(id)),
                Some(format!("X-Auth-Token: {k}")),
            )?;
            sources::parse_polypizza(&one(v)).pop()
        }
        "sketchfab" => {
            let v = get_json(
                &format!("https://api.sketchfab.com/v3/models/{}", enc(id)),
                None,
            )?;
            sources::parse_sketchfab(&one(v)).pop()
        }
        other => return Err(format!("sitio desconocido «{other}»")),
    };
    c.ok_or(format!("{site} no describe el modelo {id}"))
}

/// Downloads the model into `dir`; returns the main file.
fn download(c: &Candidate, dir: &Path, res: &str, say: &dyn Fn(String)) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let slug = c.slug();
    match c.site.as_str() {
        "polyhaven" => {
            let files = get_json(
                &format!("https://api.polyhaven.com/files/{}", enc(&c.id)),
                None,
            )?;
            let list = sources::polyhaven_files(&files, res)
                .or_else(|| sources::polyhaven_files(&files, "1k"))
                .ok_or("Poly Haven no da glTF de este modelo")?;
            say(format!(
                "Bajando {} ficheros de Poly Haven ({res})",
                list.len()
            ));
            let mut main = None;
            for (rel, url) in &list {
                let safe = !rel.is_empty()
                    && !rel.starts_with('/')
                    && Path::new(rel)
                        .components()
                        .all(|p| matches!(p, std::path::Component::Normal(_)));
                if !safe {
                    return Err(format!("ruta rara en el glTF: {rel}"));
                }
                let to = dir.join(rel);
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                curl(url, None, Some(&to))?;
                main.get_or_insert(to);
            }
            main.ok_or("sin ficheros".into())
        }
        "polypizza" => {
            let url = c.download.clone().ok_or("Poly Pizza no da el GLB")?;
            let to = dir.join(format!("{slug}.glb"));
            say("Bajando el GLB de Poly Pizza".into());
            curl(&url, None, Some(&to))?;
            Ok(to)
        }
        "sketchfab" => {
            let token = key("sketchfab_token").ok_or(
                "Sketchfab pide tu token de API para bajar: ponlo en KILN_SKETCHFAB_TOKEN o en ~/.config/kiln/keys.toml (sketchfab = \"…\")",
            )?;
            let v = get_json(
                &format!(
                    "https://api.sketchfab.com/v3/models/{}/download",
                    enc(&c.id)
                ),
                Some(format!("Authorization: Token {token}")),
            )?;
            let url = v.pointer("/glb/url").and_then(Value::as_str).ok_or(
                "Sketchfab no ofrece GLB de este modelo (solo glTF en zip, que aún no leo)",
            )?;
            let to = dir.join(format!("{slug}.glb"));
            say("Bajando el GLB de Sketchfab".into());
            curl(url, None, Some(&to))?;
            Ok(to)
        }
        other => Err(format!("sitio desconocido «{other}»")),
    }
}

fn read_origins(file: &Path) -> BTreeMap<String, Origin> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Brings the model `id` of `site` in for the part, records its origin and
/// credit, and points the part at it. Returns the spec it changed.
pub fn import(
    project: &Path,
    config: Option<&Path>,
    asset: &str,
    part: &str,
    site: &str,
    id: &str,
    say: &dyn Fn(String),
) -> Result<String, String> {
    let cfg = crate::production::config(project, config)?;
    if !cfg.sources.sites.iter().any(|s| s == site) {
        return Err(format!("el sitio «{site}» no está en [sources] sites"));
    }
    let cat = crate::load(project, config)?;
    let (s, p) = find_part(&cat, asset, part)?;
    let (spec_file, want) = (s.file.clone(), p.size_m);
    say(format!("Leyendo el modelo en {site}"));
    let c = lookup(site, id)?;
    if !cfg.rules.licenses.contains(&c.license) {
        return Err(format!(
            "licencia «{}» ({}), que el proyecto no admite: no se trae",
            c.license_label, c.license
        ));
    }
    let kit = project.join(cfg.sources_root()).join(site);
    let slug = c.slug();
    let piece_dir = kit.join(&slug);
    let main = download(&c, &piece_dir, &cfg.sources.resolution, say)?;
    let info = kiln_core::glb::read(&main).map_err(|e| format!("lo bajado no se lee: {e}"))?;
    let size = info.size();
    say(format!(
        "{}: {} tris, {:.2} × {:.2} × {:.2} m",
        slug, info.tris, size[0], size[1], size[2]
    ));

    // origin, licence and credits
    let src_file = kit.join("sources.json");
    let mut origins = read_origins(&src_file);
    let rel_main = main
        .strip_prefix(&kit)
        .unwrap_or(&main)
        .to_string_lossy()
        .replace('\\', "/");
    origins.insert(slug.clone(), Origin::of(&c, &rel_main, &today()));
    let write =
        |f: &Path, t: String| std::fs::write(f, t).map_err(|e| format!("{}: {e}", f.display()));
    write(
        &src_file,
        serde_json::to_string_pretty(&origins).map_err(|e| e.to_string())?,
    )?;
    write(
        &kit.join("LICENSE.md"),
        sources::license_md(site, &origins.iter().collect::<Vec<_>>()),
    )?;
    let mut all = Vec::new();
    for sname in sources::SITES {
        for (piece, o) in read_origins(
            &project
                .join(cfg.sources_root())
                .join(sname)
                .join("sources.json"),
        ) {
            all.push((sname.to_string(), piece, o));
        }
    }
    let credits = project.join(&cfg.sources.credits);
    if let Some(d) = credits.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    write(&credits, sources::credits_md(&all))?;
    say(format!(
        "Licencia {} y crédito en {}/LICENSE.md y {}",
        c.license, site, cfg.sources.credits
    ));

    // the part now takes it: as it is if it already has the size, else fitted
    let close = want.is_some_and(|w| {
        let (mut a, mut b) = (w, size);
        a.sort_by(f64::total_cmp);
        b.sort_by(f64::total_cmp);
        a.iter()
            .zip(b)
            .all(|(x, y)| (x - y).abs() <= 0.15 * x.max(0.05))
    });
    let kind = if want.is_none() || close {
        "reuse"
    } else {
        "variant"
    };
    let from = format!("{site}/{slug}");
    let path = project.join(&spec_file);
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let why = format!("traída de {}: {}", site, c.credit());
    write(&path, spec::set_source(&text, part, kind, &from, &why)?)?;
    say(format!(
        "La pieza {part} pasa a {} de {from}{}",
        if kind == "reuse" {
            "reutilizar"
        } else {
            "variante"
        },
        if kind == "variant" {
            " (se escala a su medida al montar)"
        } else {
            ""
        }
    ));
    Ok(spec_file)
}
