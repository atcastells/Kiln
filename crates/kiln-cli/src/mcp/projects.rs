//! Tools: the projects of this user and the one this session works on.

use super::{Kiln, Reply, blocking, fail, ok};
use crate::projects as registry;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct OpenArgs {
    /// Ruta absoluta de la carpeta del juego (la que tiene `kiln.toml` o `project.godot`).
    path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CreateArgs {
    /// Ruta absoluta de la carpeta del juego. Se crea si no existe; su carpeta madre debe existir.
    path: String,
    /// Nombre del juego.
    name: String,
    /// La respuesta a cada pregunta básica, por su id (las de `new_game_options`).
    #[serde(default)]
    basics: BTreeMap<String, String>,
}

/// An absolute path of bounded length, canonicalized when it exists.
fn absolute(path: &str) -> Result<PathBuf, String> {
    crate::ops::check_len("la ruta", path, 1000)?;
    let p = Path::new(path.trim());
    if !p.is_absolute() {
        return Err(format!("la ruta debe ser absoluta: {path}"));
    }
    Ok(std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()))
}

#[tool_router(router = projects_router, vis = "pub(super)")]
impl Kiln {
    /// Los proyectos abiertos antes con Kiln, el más reciente primero: nombre, ruta, si la carpeta sigue existiendo, si tiene `kiln.toml` o `project.godot`, y cuál está abierto ahora. Para cambiar de juego, `project_open` con su ruta.
    #[tool(annotations(read_only_hint = true))]
    async fn projects(&self) -> Reply {
        let home = self.home.clone();
        let open = self.active.read().unwrap().dir.clone();
        let list = blocking(move || {
            let recent = home.as_deref().map(registry::list).unwrap_or_default();
            recent
                .into_iter()
                .map(|e| {
                    let k = registry::kind(&e.path);
                    json!({
                        "name": e.name,
                        "path": e.path,
                        "last_opened": e.last_opened,
                        "exists": k.exists,
                        "kiln": k.kiln,
                        "godot": k.godot,
                        "trusted": home.as_deref().is_some_and(|h| registry::is_trusted(h, &e.path)),
                        "open": open.as_deref() == Some(e.path.as_path()),
                    })
                })
                .collect::<Vec<_>>()
        })
        .await?;
        let open = self.active.read().unwrap().dir.clone();
        ok(json!({ "open": open, "projects": list }))
    }

    /// Abre otro juego en esta sesión; a partir de aquí todas las herramientas trabajan sobre él. La carpeta debe tener `kiln.toml` o `project.godot`. Si solo tiene `project.godot`, queda abierta y falta crear el proyecto de Kiln con `new_game_options` y `new_game`. Los trabajos lanzados antes siguen corriendo. Kiln no lanza Blender, Godot ni Claude en un proyecto abierto así hasta que la persona confíe en él con `kiln trust` (lo dice `trusted`).
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn project_open(&self, Parameters(a): Parameters<OpenArgs>) -> Reply {
        let dir = match absolute(&a.path) {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let k = registry::kind(&dir);
        if !k.exists {
            return fail(format!("no existe la carpeta {}", dir.display()));
        }
        if !(k.kiln || k.godot) {
            return fail(format!(
                "{} no tiene kiln.toml ni project.godot: para empezar un juego ahí usa `project_create`",
                dir.display()
            ));
        }
        let kiln = self.clone();
        let d = dir.clone();
        if let Err(e) = blocking(move || kiln.open(&d, None, true)).await? {
            return fail(e);
        }
        let trusted = self.trusted();
        let next = if !k.kiln {
            "es un proyecto de Godot sin kiln.toml: usa `new_game_options` y `new_game` para crear aquí el de Kiln"
        } else if !trusted {
            "se puede leer y editar, pero Kiln no lanzará Blender, Godot ni Claude hasta que la persona ejecute `kiln trust --project` con esta ruta en su terminal; díselo y sigue con `status`"
        } else {
            "llama a `status`"
        };
        ok(json!({ "open": dir, "kiln": k.kiln, "godot": k.godot, "trusted": trusted, "next": next }))
    }

    /// Empieza un juego nuevo en la carpeta dada y lo deja abierto: la crea si falta y escribe kiln.toml, una paleta vacía y el contexto con cada respuesta fijada, como `new_game`. Sirve también para una carpeta de Godot que aún no tiene kiln.toml. Pide antes a la persona la carpeta, el nombre y cada pregunta básica (las respuestas sugeridas salen de `new_game_options`). Falla si la carpeta ya tiene kiln.toml: entonces usa `project_open`.
    #[tool(annotations(read_only_hint = false, destructive_hint = false))]
    async fn project_create(&self, Parameters(a): Parameters<CreateArgs>) -> Reply {
        if let Err(e) = crate::ops::check_len("el nombre", &a.name, 200) {
            return fail(e);
        }
        let dir = match absolute(&a.path) {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let game = kiln_core::new_game::NewGame { name: a.name, basics: a.basics, ..Default::default() };
        let d = dir.clone();
        let made = blocking(move || {
            let fresh = registry::fresh(&d);
            if !d.is_dir() {
                if !d.parent().is_some_and(Path::is_dir) {
                    return Err(format!("no existe la carpeta madre de {}", d.display()));
                }
                std::fs::create_dir(&d).map_err(|e| format!("{}: {e}", d.display()))?;
            }
            let d = std::fs::canonicalize(&d).map_err(|e| format!("{}: {e}", d.display()))?;
            crate::ops::new_game::create(&d, game).map(|r| (d, fresh, r))
        })
        .await?;
        let (dir, fresh, (files, ctx)) = match made {
            Ok(r) => r,
            Err(e) => return fail(e),
        };
        self.made(&dir, ctx, fresh);
        ok(json!({ "open": dir, "created": files, "next": "ordena lo que explique la persona con `context_propose` y sigue con `status`" }))
    }
}
