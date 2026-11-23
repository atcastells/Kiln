//! Tools: from nothing to a first asset. Find the programs, make an example
//! game, change the essential settings, and see the asset in the
//! conversation or in the page.

use super::{Kiln, Reply, blocking, fail, ok, reply};
use crate::ops;
use crate::projects as registry;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SetupArgs {
    /// Solo si la persona elige entre varios: el programa de cada herramienta (`blender`, `godot`, `claude`), uno de los que `setup` listó en `path` u `others`. Kiln no acepta otro: una ruta que la persona escriba la guarda ella con `kiln setup --blender <ruta>`.
    #[serde(default)]
    choose: BTreeMap<String, String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ExampleArgs {
    /// Ruta absoluta de una carpeta nueva o vacía para el juego de ejemplo (p. ej. `~/Documentos/Mi primer juego`, ya expandida). Se crea si falta; su carpeta madre debe existir.
    path: String,
    /// Nombre del juego; por defecto «Mi primer juego».
    #[serde(default)]
    name: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SetArgs {
    /// La clave, una de las que lista `config_get` (p. ej. `publish.dir`).
    key: String,
    /// El valor nuevo, del tipo que dice `config_get`.
    value: serde_json::Value,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct PaletteArgs {
    /// Clave del material: letras minúsculas sin acentos, cifras, `_` y `-` (p. ej. `laton`).
    id: String,
    /// Nombre para la persona (p. ej. «Latón»).
    name: String,
    /// Color [r, g, b], cada valor de 0 a 1.
    color: [f64; 3],
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct PreviewArgs {
    /// Id del asset.
    asset: String,
    /// Solo para un asset generado: la semilla; por defecto la de su ficha.
    #[serde(default)]
    seed: Option<u32>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ViewerArgs {
    /// Id del asset que se quiere ver; sin él, la página en su inicio.
    #[serde(default)]
    asset: Option<String>,
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// Opens `url` in the person's browser, unless `KILN_NO_BROWSER` is set.
fn browse(url: &str) -> Result<bool, String> {
    if std::env::var_os("KILN_NO_BROWSER").is_some() {
        return Ok(false);
    }
    let program = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(program)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| true)
        .map_err(|e| format!("no se pudo abrir el navegador ({program}): {e}"))
}

#[tool_router(router = start_router, vis = "pub(super)")]
impl Kiln {
    /// Busca Blender, Godot y Claude Code aunque no estén en el PATH (apps de /Applications, Steam, Homebrew, carpetas del usuario) y guarda el que encuentra para todos los proyectos, sin tocar ningún kiln.toml. Dice la versión de cada uno, si sirve y, si falta, el enlace de descarga. Blender monta los assets de piezas y formas; Godot genera los de receta y verifica; Claude solo lo usa la página. Llámala al empezar y cada vez que la persona instale algo.
    #[tool(annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true))]
    async fn setup(&self, Parameters(a): Parameters<SetupArgs>) -> Reply {
        let file = registry::tools_file();
        reply(blocking(move || ops::setup::setup(file.as_deref(), &a.choose, Path::new("/"), user_home().as_deref())).await?)
    }

    /// Crea un juego de ejemplo en una carpeta nueva o vacía y lo deja abierto y de confianza: un proyecto de Kiln con sus respuestas básicas (estilizado, tercera persona, CC0), una paleta de siete materiales y la ficha de un farol hecho solo de formas, listo para `build`. Es el camino más corto al primer asset para quien aún no tiene juego. Si la persona ya tiene uno, usa `project_open` o `project_create`.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn example_create(&self, Parameters(a): Parameters<ExampleArgs>) -> Reply {
        let name = a.name.unwrap_or_else(|| "Mi primer juego".into());
        if let Err(e) = ops::check_len("el nombre", &name, 200) {
            return fail(e);
        }
        let path = a.path.trim().to_string();
        let path = match path.strip_prefix("~/").zip(user_home()) {
            Some((rest, h)) => h.join(rest).to_string_lossy().into_owned(),
            None => path,
        };
        if let Err(e) = ops::check_len("la ruta", &path, 1000) {
            return fail(e);
        }
        let dir = PathBuf::from(&path);
        if !dir.is_absolute() {
            return fail(format!("la ruta debe ser absoluta: {path}"));
        }
        let made = blocking(move || {
            if !registry::fresh(&dir) {
                return Err(format!("{} ya tiene cosas: el ejemplo va en una carpeta nueva o vacía", dir.display()));
            }
            if !dir.is_dir() {
                if !dir.parent().is_some_and(Path::is_dir) {
                    return Err(format!("no existe la carpeta madre de {}", dir.display()));
                }
                std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            let dir = std::fs::canonicalize(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            ops::start::example(&dir, &name).map(|r| (dir, r))
        })
        .await?;
        let (dir, (files, ctx)) = match made {
            Ok(r) => r,
            Err(e) => return fail(e),
        };
        self.made(&dir, ctx, true);
        ok(json!({
            "open": dir,
            "created": files,
            "example": "farol",
            "next": "Enséñale la ficha del farol (`spec_get` farol) en dos frases, comprueba las herramientas con `setup` y lanza `build` con el farol. Cuando el trabajo termine, enséñaselo con `preview` y pregúntale qué quiere cambiar o qué asset quiere crear.",
        }))
    }

    /// Los ajustes esenciales del kiln.toml del proyecto abierto, con su valor y qué significan: nombre, licencias, triángulos por pieza, carpeta donde se publica, carpeta del proyecto de Godot y tiempo máximo. Se cambian con `config_set`.
    #[tool(annotations(read_only_hint = true))]
    async fn config_get(&self) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::start::config_get(&ctx)).await?)
    }

    /// Cambia un ajuste esencial del kiln.toml (las claves de `config_get`) sin tocar el resto del fichero ni sus comentarios. Kiln comprueba el valor y que el fichero siga siendo válido. No cambia los programas que se ejecutan: esos los elige la persona con `kiln setup`. Pide antes a la persona el valor, salvo que ya lo haya dicho.
    #[tool(annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true))]
    async fn config_set(&self, Parameters(a): Parameters<SetArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::start::config_set(&ctx, &a.key, &a.value)).await?)
    }

    /// Añade un color liso a la paleta del juego (la primera tabla de `[materials] tables`), para que las zonas de una ficha lo usen por su clave. Úsalo cuando la paleta no tenga el material que pide la persona; los que ya hay salen en `catalog` con `materials`.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn palette_add(&self, Parameters(a): Parameters<PaletteArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        reply(blocking(move || ops::start::palette_add(&ctx, &a.id, &a.name, a.color)).await?)
    }

    /// Una imagen del asset montado (o generado) para enseñársela a la persona en la conversación: frente, perfil, planta y tres cuartos en una hoja, con el color de cada material, sus medidas y sus triángulos. Kiln la dibuja al momento, sin Blender ni Godot, y la guarda junto al modelo (`image`): si la persona no ve imágenes en la conversación, dale esa ruta. Para girarlo en 3D, `open_viewer`.
    #[tool(annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true))]
    async fn preview(&self, Parameters(a): Parameters<PreviewArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        match blocking(move || ops::start::preview(&ctx, &a.asset, a.seed)).await? {
            Ok((info, png)) => Ok(CallToolResult::success(vec![
                ContentBlock::text(info.to_string()),
                ContentBlock::image(super::production::base64(&png), "image/png"),
            ])),
            Err(e) => fail(e),
        }
    }

    /// Abre en el navegador de la persona la página de Kiln con el asset en 3D (se puede girar y acercar) y devuelve su enlace. La página sigue al proyecto abierto en esta conversación.
    #[tool(annotations(read_only_hint = true, open_world_hint = false))]
    async fn open_viewer(&self, Parameters(a): Parameters<ViewerArgs>) -> Reply {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let port = self
            .shared
            .as_ref()
            .and(self.home.as_deref())
            .and_then(crate::core::info)
            .and_then(|i| i.port);
        let Some(port) = port else {
            return fail(
                "la página de Kiln no está en marcha en esta sesión: pide a la persona que ejecute `kiln serve` en su terminal \
                 y que abra http://127.0.0.1:4330, o enséñale el asset con `preview`",
            );
        };
        let route = match &a.asset {
            None => String::new(),
            Some(id) => {
                if let Err(e) = ops::check_id("el asset", id) {
                    return fail(e);
                }
                let id2 = id.clone();
                let generated = blocking(move || {
                    ops::read::load(&ctx).map(|cat| cat.specs.iter().any(|s| s.asset.id == id2 && s.generator.is_some()))
                })
                .await?;
                match generated {
                    Ok(true) => format!("#/spec/{id}"),
                    Ok(false) => format!("#/build/{id}"),
                    Err(e) => return fail(e),
                }
            }
        };
        let url = format!("http://127.0.0.1:{port}/{route}");
        let opened = match browse(&url) {
            Ok(o) => o,
            Err(e) => return fail(format!("{e}. El enlace es {url}")),
        };
        ok(json!({ "url": url, "opened": opened, "next": "dale el enlace a la persona por si el navegador no se abrió delante" }))
    }
}
