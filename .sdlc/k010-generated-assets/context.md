# Context: k010-generated-assets

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

## Decisions that shape the code

- A spec carries either `[[parts]]` or `[generator]`, never both. A spec without `[generator]` must behave and serialize exactly as before (new fields skipped when empty).
- The recipe is `[generator]` with `seed` (u32), `material` (palette key) and `[[generator.ops]]`; every op has `op = "<id>"` plus its parameters. A parameter is a fixed number or a range `[min, max]`.
- The vocabulary is closed and lives in ONE table, `kiln_core::generate::vocab`. Checks, the page controls, the agent's text and the Godot script all read or mirror that table. First vocabulary: `sphere` (`radius_m` 0.1..10000, `segments` 8..256 integer), `stretch` (`x`, `y`, `z` 0.2..5, default 1), `noise` (`amplitude` 0..1, `size` 0.01..4; repeatable), `facet` (no parameters). `sphere` is required and first.
- Triangles of a sphere = 2 * segments * (segments / 2 + 1); 64 segments give 4224.
- Ranges are drawn in Rust (`generate::resolve`), never in GDScript. Godot receives resolved values plus a 32-bit noise seed per `noise` op.
- Draw algorithm: key = "<seed>/<op index>/<param>"; h = FNV-1a 64 of the key, then one SplitMix64 step; u = (h >> 11) / 2^53; value = min + (max - min) * u; integers floor. Noise seed = low 32 bits of h for the key "<seed>/<op index>/noise".
- Godot runs only Kiln's own script, headless, in an empty scratch project inside the cache (like `verify.rs`), never in the game's project.
- Output: `<cache>/generated/<id>/<seed>.glb` and `generated.json` beside it.
- Build, paint, verify, publish and review refuse a generated asset with: «es un asset generado: aún no se monta ni se publica».
- No new Rust dependency.

## Conventions

- Code, identifiers and comments in English; user-facing messages (findings, errors, `say` lines, UI texts) in Spanish, like the existing ones.
- Comments only when the logic is not obvious. No emojis. No mention of AI.
- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the file; integration tests in `crates/kiln-cli/tests/`.
- Tests that need Blender or Godot are `#[ignore]`; a subagent never runs them.
- Findings use `catalog::Finding`; codes are snake_case; follow the `push(Severity, subject, code, message)` shape in `spec.rs`.
- A subagent runs only its own tests (`cargo test -p <crate> <filter>`), never the whole suite, never git.
- Red test first: write the test, see it fail, then implement.
- Edit repo files with Edit or Write, never with shell heredocs or scripts (a hook blocks commands that mention `production.rs`).
- Under 100 new lines per task. Touch only the files the task lists; if another file must change to compile, say so in the report instead of widening silently beyond a one-line fix.

## Key paths

- `crates/kiln-core/src/spec.rs`: `Spec`, `check` (the `no_parts` finding is near line 340).
- `crates/kiln-core/src/generate/`: `mod.rs` (types), `vocab.rs`, `check.rs`, `resolve.rs`.
- `crates/kiln-core/src/glb.rs`: `structure`, `structure_hash`.
- `crates/kiln-cli/godot/`: Kiln's Godot scripts (`measure.gd` shows the GLB export).
- `crates/kiln-cli/src/run.rs`: `godot(cfg, args, work, name)`.
- `crates/kiln-cli/src/verify.rs`: `scratch_dir` and the empty scratch project.
- `crates/kiln-cli/src/ops/`: what the page and MCP share; `ops/specs.rs` has `spec_write`.
- `crates/kiln-cli/tests/coverage.rs`: every `/api/*` route needs an MCP tool.
- `.sdlc/k010-generated-assets/spec.md`: the recipe example and the vocabulary table.

## Do not

- Do not add an asteroid to `examples/` (a medieval harbour).
- Do not change an existing test to make it pass.
- Do not run git, the whole test suite, Blender, Godot or `claude`.

## Learned

