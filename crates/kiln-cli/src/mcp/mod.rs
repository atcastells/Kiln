//! `kiln mcp`: Kiln as an MCP server over stdio. The calling agent does the
//! agent work; Kiln reads, checks, writes and runs Blender and Godot. Only the
//! protocol goes to stdout; everything else goes to stderr.
//!
//! One server serves any project: the session has one open project at a time
//! and switches with `project_open` or `project_create`.

mod context;
mod materials;
mod new_game;
mod production;
mod projects;
mod prompts;
mod read;
mod sources;
mod specs;

use crate::agent::{Ctx, Jobs};
use crate::projects as registry;
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::service::{NotificationContext, RoleServer};
use rmcp::{ErrorData, ServerHandler, ServiceExt, prompt_handler, tool_handler};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// What a tool returns: a result the agent reads, or an error it can act on.
pub type Reply = Result<CallToolResult, ErrorData>;

/// The project this session works on.
#[derive(Default)]
pub struct Active {
    /// The open folder; `None` until one is opened.
    pub dir: Option<PathBuf>,
    /// `None` while the folder has no `kiln.toml` (start with `new_game`).
    pub ctx: Option<Arc<Ctx>>,
    /// Opened on purpose (`--project` or a tool), so the client's roots do
    /// not replace it.
    pub chosen: bool,
}

#[derive(Clone)]
pub struct Kiln {
    pub active: Arc<RwLock<Active>>,
    /// Where the registry of projects lives (`None`: not kept).
    pub home: Option<PathBuf>,
    pub jobs: Jobs,
    tool_router: ToolRouter<Kiln>,
    prompt_router: PromptRouter<Kiln>,
}

const INSTRUCTIONS: &str = "Kiln prepara los assets y el contenido de un juego. \
Empieza por `status`: dice qué proyecto está abierto. Con `projects`, `project_open` y \
`project_create` se cambia de juego sin salir de la conversación. El trabajo de agente \
lo haces tú siguiendo los prompts `kiln-asset` y `kiln-context`; Kiln comprueba y \
escribe. Montar, pintar, medir con Godot, revisar y traer son trabajos largos: devuelven \
un id que se sigue con `job`.";

const NO_PROJECT: &str = "no hay ningún proyecto abierto: usa `projects` para ver los recientes, \
`project_open` con la ruta de uno, o `project_create` para empezar uno nuevo";

impl Kiln {
    /// With `project`, that folder is open as before; without it, the working
    /// folder if it holds a project, else the newest registered one. The
    /// client's roots may replace that choice once it is connected.
    pub fn new(project: Option<PathBuf>, config: Option<PathBuf>) -> Result<Self, String> {
        let kiln = Kiln {
            active: Default::default(),
            home: registry::home(),
            jobs: Default::default(),
            tool_router: Self::read_router()
                + Self::production_router()
                + Self::specs_router()
                + Self::context_router()
                + Self::materials_router()
                + Self::sources_router()
                + Self::new_game_router()
                + Self::projects_router(),
            prompt_router: Self::prompts_router(),
        };
        match project {
            Some(p) => kiln.open(&p, config, true)?,
            None => {
                let here: Vec<PathBuf> = std::env::current_dir().into_iter().collect();
                let recent = kiln.home.as_deref().map(registry::list).unwrap_or_default();
                if let Some(p) = registry::pick(&here, &recent)
                    && let Err(e) = kiln.open(&p, None, false)
                {
                    eprintln!("no se pudo abrir {}: {e}", p.display());
                }
            }
        }
        Ok(kiln)
    }

    /// Makes `dir` the open project and notes it in the registry. A folder
    /// without `kiln.toml` stays open with no project yet.
    pub fn open(&self, dir: &Path, config: Option<PathBuf>, chosen: bool) -> Result<(), String> {
        let has_toml = config.is_some() || registry::kind(dir).kiln;
        let ctx = if has_toml { Some(Arc::new(crate::ops::ctx(dir, config)?)) } else { None };
        *self.active.write().unwrap() = Active { dir: Some(dir.to_path_buf()), ctx, chosen };
        self.remember(dir);
        Ok(())
    }

    /// Notes `dir` as the newest project; a failure only goes to stderr.
    pub fn remember(&self, dir: &Path) {
        let Some(home) = &self.home else { return };
        let k = registry::kind(dir);
        if !(k.kiln || k.godot) {
            return;
        }
        let name = kiln_core::Config::load(dir)
            .map(|c| c.project.name)
            .ok()
            .filter(|n| !n.trim().is_empty())
            .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_default();
        if let Err(e) = registry::touch(home, dir, &name, registry::now()) {
            eprintln!("no se pudo apuntar el proyecto en {}: {e}", home.display());
        }
    }

    /// The open folder, or what to do when there is none.
    pub fn dir(&self) -> Result<PathBuf, String> {
        self.active.read().unwrap().dir.clone().ok_or_else(|| NO_PROJECT.into())
    }

    /// The project, or what to do when there is none yet.
    pub fn ctx(&self) -> Result<Arc<Ctx>, String> {
        let a = self.active.read().unwrap();
        if let Some(c) = &a.ctx {
            return Ok(c.clone());
        }
        match &a.dir {
            None => Err(NO_PROJECT.into()),
            Some(d) if registry::kind(d).godot => Err(format!(
                "{} es un proyecto de Godot sin kiln.toml: usa `new_game_options` y `new_game` para crear aquí el de Kiln",
                d.display()
            )),
            Some(_) => Err(
                "esta carpeta aún no tiene kiln.toml: usa `new_game_options` y `new_game` para crear el proyecto".into(),
            ),
        }
    }

    /// Opens the first root of the client that holds a project, unless a
    /// project was already opened on purpose. Roots are deprecated in the
    /// protocol (SEP-2577) but still the only way some clients say where
    /// they work; without them the working folder decides.
    #[allow(deprecated)]
    async fn open_from_roots(&self, cx: NotificationContext<RoleServer>) {
        let asks = cx.peer.peer_info().is_some_and(|i| i.capabilities.roots.is_some());
        if !asks || self.active.read().unwrap().chosen {
            return;
        }
        let Ok(r) = cx.peer.list_roots().await else { return };
        let dirs: Vec<PathBuf> = r.roots.iter().filter_map(|r| file_path(&r.uri)).collect();
        let Some(p) = registry::pick(&dirs, &[]) else { return };
        if self.active.read().unwrap().chosen {
            return;
        }
        match self.open(&p, None, false) {
            Ok(()) => eprintln!("proyecto de las roots del cliente: {}", p.display()),
            Err(e) => eprintln!("no se pudo abrir {}: {e}", p.display()),
        }
    }
}

/// The local path of a `file://` URI, with `%XX` decoded.
fn file_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let b = rest.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%' && i + 3 <= b.len())
            .then(|| std::str::from_utf8(&b[i + 1..i + 3]).ok())
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(v) => {
                out.push(v);
                i += 3;
            }
            None => {
                out.push(b[i]);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok().map(PathBuf::from)
}

/// A JSON value as the tool's result.
pub fn ok(v: impl serde::Serialize) -> Reply {
    Ok(CallToolResult::success(vec![ContentBlock::json(v)?]))
}

/// A message the agent should read and act on (not a protocol error).
pub fn fail(e: impl Into<String>) -> Reply {
    Ok(CallToolResult::error(vec![ContentBlock::text(e.into())]))
}

/// `ok` or `fail` from a result.
pub fn reply<T: serde::Serialize>(r: Result<T, String>) -> Reply {
    match r {
        Ok(v) => ok(v),
        Err(e) => fail(e),
    }
}

/// Runs blocking Kiln work off the protocol thread.
pub async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, ErrorData> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

#[tool_handler(router = self.tool_router)]
#[prompt_handler(router = self.prompt_router)]
impl ServerHandler for Kiln {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_prompts().build())
            .with_server_info(Implementation::new("kiln", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn on_initialized(&self, cx: NotificationContext<RoleServer>) {
        let kiln = self.clone();
        tokio::spawn(async move { kiln.open_from_roots(cx).await });
    }
}

pub fn run(project: Option<PathBuf>, config: Option<PathBuf>) -> Result<(), String> {
    let kiln = Kiln::new(project, config)?;
    match &kiln.active.read().unwrap().dir {
        Some(d) => eprintln!("Kiln MCP por stdio, proyecto {}", d.display()),
        None => eprintln!("Kiln MCP por stdio, sin proyecto abierto"),
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let server = kiln
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| format!("no se pudo abrir MCP: {e}"))?;
        server.waiting().await.map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::file_path;
    use std::path::PathBuf;

    #[test]
    fn file_uris_become_paths() {
        assert_eq!(file_path("file:///Users/a/Mi%20Juego"), Some(PathBuf::from("/Users/a/Mi Juego")));
        assert_eq!(file_path("file://localhost/tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(file_path("file:///tmp/100%"), Some(PathBuf::from("/tmp/100%")));
        assert_eq!(file_path("https://example.com/x"), None);
        assert_eq!(file_path("file://host/x"), None);
    }
}
