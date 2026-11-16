# Context: k004-mcp

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data.

## Where

- Repo `~/Kiln`, branch `k004-mcp` (from `main` at `1f0b479`). Spec and plan:
  `.sdlc/k004-mcp/{spec.md,plan.md}`. Do not touch `.sdlc/k003-ui/`.

## Decisions that shape the code

- `kiln mcp --project DIR` serves MCP over stdio with `rmcp` (exact version).
  Only that subcommand uses `tokio`; the rest of Kiln stays synchronous.
- The page and the MCP share one logic: `crates/kiln-cli/src/ops/<area>.rs`.
  Functions take Rust types and return `Result<serde_json::Value, String>`
  (or a typed value); no HTTP, no MCP types inside `ops`.
- Logic moved from `serve.rs` keeps its behaviour and its error messages
  word for word. Only T10 edits `serve.rs`.
- The calling agent does the agent work: from the MCP, Kiln never launches
  `claude -p`, except the visual reviewer (`review`, decision D1).
- Context has two voices: `context_edit` = the person (pins), `context_propose`
  = the agent (goes through `Context::guard_pinned`).
- `spec_write` never writes a spec that fails `spec::parse` or has errors from
  `spec::check`; it writes only inside `[assets] specs`.
- Long work is a job (`agent::job`, `Jobs`); waiting is capped at 30 s.
- API keys (`KILN_POLYPIZZA_KEY`, `KILN_SKETCHFAB_TOKEN`, `keys.toml`) never
  appear in any output.

## Conventions

- Code, identifiers and doc comments in English; user-facing messages in
  Spanish, like the existing ones. Comments only when not obvious.
- Rust 2024 edition. Tests: unit in `#[cfg(test)] mod tests` of the file;
  integration in `crates/kiln-cli/tests/`, sharing `tests/common/mod.rs`.
- Tests that write use a temp copy of `examples` (`tests/common`), never the
  repo's `examples` itself.
- Run only your own tests: `cargo test -p kiln-cli <filter>`.

## Key paths

- `crates/kiln-cli/src/serve.rs`: HTTP routes (`handle_new` 99-134,
  `handle` 179-480), helpers `spec_file`, `rel`, `file` (root confinement).
- `crates/kiln-cli/src/agent.rs`: `Ctx`, `Job`, `Jobs`, `job`, `start`.
- `crates/kiln-cli/src/{production,review,fetch}.rs`: Blender, Godot, review
  loop, external sources.
- `crates/kiln-core/src/{context,spec,bindings,zones,new_game}.rs`.
- `examples/`: Puerto Niebla (kiln.toml, specs, context); `examples/content`.
- `skills/{asset,context,review}/SKILL.md`.

## Do not

- No git writes. No new dependencies except `rmcp` and `tokio` in T11.
- Do not change what a page route returns. Do not "fix" behaviour on the way:
  note it in your report.
- No files outside the ones your task lists.

## Learned

- Baseline before K004: `cargo test --workspace` 156 passed, 1 ignored.
- After wave 1: 172 passed. `ops/mod.rs` gives `ctx(project, config)` (copy of
  `serve::make_ctx`), `spec_file`, `rel`, `check_id(what, id)`,
  `check_len(what, text, max)`, `confine(&ctx, path)`. Use them; do not
  re-implement. Do not edit `ops/mod.rs` (main only).
- `tests/common/mod.rs` gives `temp_project("")`/`temp_project("content")`,
  `empty_dir()`, `Server::start`, `get`, `post(path, json, page)`, `error_of`,
  `json_of`, `TempDir::{read, exists}`. Unit tests inside `src/ops/` cannot use
  it: copy `examples` to a temp dir with a small local helper or only read.
- After wave 3 (T10): `serve.rs` calls `ops::*`; page behaviour pinned by
  `tests/routes.rs` (12). ops signatures: see `grep -n "^pub fn" src/ops/*.rs`.
- After T11: `rmcp =3.8.0` (features server, macros, transport-io), `tokio`
  (rt). `src/mcp/mod.rs` (main only, do not edit) has `Kiln { project, config,
  slot, jobs }`, `self.ctx() -> Result<Arc<Ctx>, String>` (error says to use
  `new_game` when there is no kiln.toml), `ok(v)`, `fail(msg)`, `reply(result)`,
  `blocking(closure).await?` (run every ops call inside it), type `Reply`.
- Tool pattern (copy `src/mcp/read.rs` `status`): one
  `#[tool_router(router = <area>_router, vis = "pub(super)")] impl Kiln { .. }`
  per area file (remove `allow_empty` once it has a tool). Each tool:
  `/// descripción en castellano` (becomes the tool description; say when to
  use it), `#[tool(annotations(read_only_hint = true))]` for reads; writes:
  `read_only_hint = false`, `destructive_hint = true` when they overwrite
  something, `idempotent_hint` when true; `open_world_hint = true` for network.
  Args: `async fn x(&self, Parameters(a): Parameters<XArgs>) -> Reply` with
  `use rmcp::handler::server::wrapper::Parameters; use rmcp::{schemars, tool, tool_router};`
  and `#[derive(serde::Deserialize, schemars::JsonSchema)] struct XArgs { /// doc per field }`.
  Optional fields `#[serde(default)] Option<T>`. Errors the agent can act on
  go through `fail`/`reply` (isError result), never panics.
- MCP-layer validation (R12), on top of ops: ids with `ops::check_id`, free
  texts with `ops::check_len` (request 500, context texts 2000, spec 200_000),
  paths with `ops::confine`. ops keep the page's behaviour.
- Integration tests: one file per area, `crates/kiln-cli/tests/mcp_<area>.rs`
  with `mod common;` and `common::{Mcp, temp_project, empty_dir}`:
  `Mcp::start(dir)`, `m.tools()`, `m.call(name, json) -> Called { is_error,
  text, raw, json() }`, `m.request(method, params)`, `m.stop() -> stderr`.
  Never on the repo `examples` itself: always `temp_project("")`.
- After wave 5: 27 tools + 3 prompts, 243 tests green, Clippy clean. Tree
  node ids are the child's `key` (`level:id`). Context file in `examples` is
  the root `context.toml`. A missing or mistyped tool argument comes back as
  an isError result in English from rmcp («failed to deserialize parameters:
  missing field `asset`»), checked by main with `build {}` and `spec_get {}`.
  Prompt arguments that a prompt rejects are protocol errors; `Mcp::request`
  panics on those, so such tests send raw JSON-RPC (see `tests/mcp_prompts.rs`).
  Image blocks: `raw["content"][i]` with `type: "image"`, `mimeType`, `data`.
- `/api/settle` on `examples` returns 400 today (missing intent paragraph and
  pinned claims per layer); layer names in messages are display names.
