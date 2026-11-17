# Context: k006-art-pipeline

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

## Decisions that shape the code

- Kiln never runs scripts from the project or from an MCP client: only its own embedded Blender/Godot scripts, with validated parameters.
- External validators are optional: a missing tool is reported as absent, never as an error.
- A project without `[anchors]`, `[publish]` or `.blend` pieces must behave exactly as before (same findings, same serialized specs).
- "Equivalent" builds are compared by the structural hash, not by file bytes.
- No new Rust dependency other than `sha2`.

## Conventions

- Code, identifiers and comments in English; user-facing messages (findings, errors, `say` lines) in Spanish, like the existing ones.
- Comments only when the logic is not obvious. No emojis. No mention of AI.
- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the file; integration tests in `crates/kiln-cli/tests/`.
- Tests that need Blender or Godot are `#[ignore]`; never run them from a subagent.
- Findings use `catalog::Finding`; look at `check.rs` for the shape and at the existing codes (snake_case).
- A subagent runs only its own tests (`cargo test -p <crate> <filter>`), never the whole suite, never git.
- Edit repo files with Edit or Write, not with shell heredocs (a hook blocks them).

## Key paths

- `crates/kiln-core/src/config.rs`: `kiln.toml` (`Tools`, and the new `anchors`, `publish`).
- `crates/kiln-core/src/glb.rs`: GLB reader. `crates/kiln-core/src/spec.rs`: asset spec.
- `crates/kiln-core/src/{tools,validate,asset}.rs`: new modules of this plan.
- `crates/kiln-cli/src/run.rs`: the single launcher for external tools.
- `crates/kiln-cli/src/production.rs`: build, paint, measure. `ops/`: logic shared by page and MCP. `mcp/`: tools.
- `examples/`: the sample project used by tests.

## Do not

- Do not edit existing tests to make them pass; fix the code.
- Do not touch files outside the task's list.
- Do not change the text of existing error messages or findings.
- Do not touch the ASTRA repository.

## Learned
- Clippy here (1.94) rejects nested `if let` + `if` (use let-chains), `x % n != 0` (use `is_multiple_of`) and float literals close to constants like `0.7071068`.
- Format one file with `rustfmt --edition 2024 --config skip_children=true <file>`; `cargo fmt` touches other files.
- Under zsh, quote globs: `grep --include="*.rs"`.
- A shell command containing the word "production" can be blocked by a hook: edit `production.rs` with Edit/Write, and run scripts from files.
- Launch tools only through `crate::run::{blender, godot}`: they set the cwd, the timeout and the log (`<work>/.kiln/logs/<name>.log`); on failure use `Output::failure("...")`.
- The project path is already absolute (canonicalized in `main.rs`).
- Anchor findings use the subject `anchor:<asset>/<id>`.
- `glb::tests` has `hnode`, `base`, `hash_of` helpers; `validate::tests` has its own GLB builder.
- `ops::read::tools(cfg)` gives the detected tools, cached for the process.
