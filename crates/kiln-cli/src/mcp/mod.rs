//! `kiln mcp`: Kiln as an MCP server over stdio. The calling agent does the
//! agent work; Kiln reads, checks, writes and runs Blender and Godot. Only the
//! protocol goes to stdout; everything else goes to stderr.

mod context;
mod materials;
mod new_game;
mod production;
mod prompts;
mod read;
mod sources;
mod specs;

use crate::agent::{Ctx, Jobs};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, ServiceExt, prompt_handler, tool_handler};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// What a tool returns: a result the agent reads, or an error it can act on.
pub type Reply = Result<CallToolResult, ErrorData>;

#[derive(Clone)]
pub struct Kiln {
    pub project: PathBuf,
    /// `None` while the folder has no `kiln.toml` (start with `new_game`).
    pub slot: Arc<RwLock<Option<Arc<Ctx>>>>,
    pub jobs: Jobs,
    tool_router: ToolRouter<Kiln>,
    prompt_router: PromptRouter<Kiln>,
}

const INSTRUCTIONS: &str = "Kiln prepara los assets y el contenido de un juego. \
Empieza por `status`. El trabajo de agente lo haces tú siguiendo los prompts \
`kiln-asset` y `kiln-context`; Kiln comprueba y escribe. Montar, pintar, medir con \
Godot, revisar y traer son trabajos largos: devuelven un id que se sigue con `job`.";

impl Kiln {
    pub fn new(project: PathBuf, config: Option<PathBuf>) -> Result<Self, String> {
        let new = config.is_none() && !project.join(kiln_core::config::FILE).exists();
        let ctx = if new { None } else { Some(Arc::new(crate::ops::ctx(&project, config.clone())?)) };
        Ok(Kiln {
            project,
            slot: Arc::new(RwLock::new(ctx)),
            jobs: Default::default(),
            tool_router: Self::read_router()
                + Self::production_router()
                + Self::specs_router()
                + Self::context_router()
                + Self::materials_router()
                + Self::sources_router()
                + Self::new_game_router(),
            prompt_router: Self::prompts_router(),
        })
    }

    /// The project, or what to do when there is none yet.
    pub fn ctx(&self) -> Result<Arc<Ctx>, String> {
        self.slot.read().unwrap().clone().ok_or_else(|| {
            "esta carpeta aún no tiene kiln.toml: usa `new_game_options` y `new_game` para crear el proyecto".into()
        })
    }
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
}

pub fn run(project: PathBuf, config: Option<PathBuf>) -> Result<(), String> {
    let kiln = Kiln::new(project.clone(), config)?;
    eprintln!("Kiln MCP por stdio, proyecto {}", project.display());
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
