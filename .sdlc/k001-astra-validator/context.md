# Context: k001-astra-validator

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

## Decisions that shape the code

- Config section is `[integrations.astra]`: `Config.integrations: IntegrationsSection` (`#[serde(default)]`), holding `astra: Option<AstraSection>`.
- `AstraSection { enabled: bool = false, definitions: Vec<String> (globs, relative to root), godot_manifest: Option<String>, greybox: bool = false }`.
- No `deny_unknown_fields` anywhere: a kiln.toml without the section must behave exactly as before.
- RON definitions are a list of `enum Definition { Sector{id, scene}, Door{id, connects:(String,String)}, Object{id, asset}, Installation{id, sector, objects:Vec<String>} }`, provisional schema.
- `Finding` is not changed. `subject` = `<file>#<id>` (file relative to project root); for syntax errors `subject` = `<file>` and the message carries line:column.
- Stable codes, all prefixed `astra_` (see spec.md "Códigos"). Severity per that table.
- IDs are static content strings, never generated, never `PersistentId`.
- Module layout: `kiln-core/src/astra/mod.rs`, `astra/refs.rs`, `astra/manifest.rs`.

## Conventions

- Rust 2024. Identifiers and code in English; user-facing messages (`Finding.message`) in Spanish, like `check.rs`.
- Comments only when the logic is not obvious; no emojis.
- Unit tests in a `#[cfg(test)] mod tests` at the bottom of the same file. Temporary dirs via `std::env::temp_dir().join(format!("kiln-<name>-{}", std::process::id()))`, as in `tree.rs:268`.
- Run only your own tests: `cargo test -p kiln-core <module>`; the main session runs the whole battery.

## Key paths

- `crates/kiln-core/src/catalog.rs:86-99`: `Severity` and `Finding`.
- `crates/kiln-core/src/config.rs`: `Config` and its sections.
- `crates/kiln-core/src/check.rs`: style reference for producing findings.
- `crates/kiln-core/src/lib.rs`: module list and `open_with`.

## Do not

- Do not touch files outside your task's list. Do not run git writes.
- Do not change `Finding`, `Severity`, or any existing check.
- Do not wire anything into `open_with` before T6.

## Learned

- `Config::astra()` returns the section only when present and `enabled`; tests build Config with `Config::parse("[project]\nname = \"demo\"\n...")`.
- ron 0.12: `SpannedError.span.start.line/col` (1-based). The reported line is where ron notices the error, not always where the omission is.
- `astra::Parsed { file, defs }` and `Definition::{id(), kind()}` exist in `astra/mod.rs`; kinds are "sector", "door", "object", "installation".
- `astra::refs::check(&[Parsed]) -> Vec<Finding>` and `astra::manifest::{load(root, rel), compare(files, &Manifest, rel)}` exist; `load` returns the missing/syntax Finding as `Err`.
- Implementers must show the red run: write the test, run it failing, then implement.
