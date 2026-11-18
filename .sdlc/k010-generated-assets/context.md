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

- Types: `kiln_core::generate::{Generator { seed: u32, material: Option<String>, ops: Vec<Op> }, Op { op: String, params: BTreeMap<String, Value> }, Value::{Fixed(f64), Range([f64; 2])}}`; `Spec.generator: Option<Generator>`.
- Vocabulary: `generate::vocab::{OPS, op(id) -> Option<&OpDef>, OpDef { id, label, params, repeatable }, OpDef::param(id), Param { id, label, unit, min, max, default, integer }, sphere_tris(segments)}`; all `Serialize`. `serde_json` is already a dependency of kiln-core.
- `spec::check(&spec, &rules, &mut cat)`: `push(Severity, subject: String, code: &str, msg: String)` is a closure over a local vector; the asset subject is `asset:<id>`; the palette is `cat.materials` (`.key`). Tests: `spec::parse(text)`, `Catalog::default()`, `&Default::default()` for the rules.
- kiln-cli: `crate::load(project, config_file)` returns the `Catalog` (`cat.specs`); `crate::production::config(project, config_file)` returns the `Config`; `crate::ops::refuse_generated(&cat.specs, id)`.
- Minimal temp project for ops tests: `kiln.toml` with `[project] name`, `[assets] specs = "specs"`, `[tools]`, plus `specs/<id>.toml`. A minimal generated spec: `[asset] id/name`, `[generator]`, one `sphere` op.
- `generate::check::check(asset_id, &Generator, budget_tris: Option<u64>, palette: &[&str]) -> Vec<Finding>`; empty means valid. `spec::check` already calls it.
- `generate::resolve::{resolve(&Generator, seed: u32) -> Resolved, Resolved { seed, material, ops: Vec<ResolvedOp> }, ResolvedOp { op, params: BTreeMap<String, f64>, noise_seed: Option<u32> }}`, all `Serialize`: a `ResolvedOp` serializes exactly as the op objects `generate.gd` reads.
- `crates/kiln-cli/godot/generate.gd`, run as `godot --headless --path <empty project> --script <abs path> -- job.json out.json`. job.json: `{ id, glb (absolute path), material (string or null), ops: [ { op, params: {..}, noise_seed? } ] }`. out.json: `{ size_m: [x,y,z], min: [x,y,z], tris, godot }` or `{ error }` with exit code 1. Proven in real Godot 4.7.1: same job gives a byte-identical GLB; 64 segments give 4224 triangles with and without `facet`.
- The page's catalog (`GET /api/catalog`) is built in `serve.rs::catalog_reply` by serializing `Catalog`; it now carries `generator_ops` (the vocabulary table). Each spec in `specs[]` carries its `generator` when it has one. The MCP `catalog` tool (`ops::read::catalog`) returns paged lists and does not carry the vocabulary.
- `agent::generator_vocabulary()` renders the table as text for the agent; the page prompt is built by `agent::ask_prompt`.
- `ops::specs::spec_write(ctx: &Ctx, id, toml_text) -> Result<Written { file, findings }, Rejected { error, findings }>` writes, reloads the catalog and restores the old file on Error findings; it already accepts a recipe spec and rejects an invalid recipe.
- `crate::generate::generate_asset(project, config_file, id, seed: Option<u32>, say) -> Result<Generated { glb, rel, seed, resolved, out }, String>`; work folder `<cache>/generated/<id>/.project`, GLB `<cache>/generated/<id>/<seed>.glb`; default cache `kiln/cache`. It and `Generated` carry `#[allow(dead_code)]` until a caller exists: remove it when you add one.
- `crate::run::godot` returns `Ok(Output)` even on a non-zero exit: check `run.success()`, use `run.failure(..)`. `crate::ops::rel(&canonical_root, &path)` gives the forward-slash relative path.
- Fake Godot in tests: a shell script the test writes and points `[tools] godot` at (see the tests of `generate.rs`).
- Page: `views/generator.js` exports `controlsOf`, `recipeOf`, `setSeed`, `clamp`, `generatorPanel(spec, vocabulary)` (returns house `html`) and `bindGenerator(root, spec, vocabulary, { onApply })` (call once per new root; `onApply(recipe, seed)`). Views are `mount(el, ctx)` modules returning a cleanup; static files are listed in `serve.rs` (~line 31) as `("app/views/x.js", include_str!(...))`.
- `ops::production::generate(jobs, ctx: Arc<Ctx>, asset, seed: Option<u32>, recipe: Option<&str>) -> Result<u64, String>`: `recipe` is the whole spec as TOML; it is written with `spec_write` before the job starts (a rejected one returns Err, starts no job). The job's result text (`Job.spec`, shown as `JobView.file`) is the GLB path relative to the project, e.g. `kiln/cache/generated/rock/7.glb`. It carries `#[allow(dead_code)]` until a caller exists.
- `generate_asset` also writes `<cache>/generated/<id>/generated.json` (asset, seed, file, glb_sha256, structure_sha256, recipe, resolved, size_m, min, tris, tris_expected, godot, generator_version).
- A route that passes `recipe: Some(..)` must call `drop_catalog(&ctx.project)` in `serve.rs` after success, as the other write routes do.
- Test helpers in `generate.rs`: `pub(crate) mod tests` with `FAKE`, `ROCK`, `project(tag)` (unique tag per test); `crate::ops::ctx(&p, None)` builds a `Ctx`.
- CLI: `kiln generate --project DIR --asset ID [--seed N]` prints the GLB path; exit 2 on error. MCP tool `generate { asset, seed?, recipe? }`.
- Route: `POST /api/generate`, header `X-Kiln: 1`, JSON body `{ asset, seed?, recipe? }` (`recipe` = whole spec as TOML text). Success `200 {"job": <id>}`; then poll `/api/jobs/<id>`, whose `file` is the GLB path relative to the project (serve it through `/files/<path>`). Errors `400 {"error": "..."}`. `handle` drops the cached catalog after every POST.
- The structural hash does not cover vertex positions: two seeds of one recipe share it. Geometry is compared by `glb_sha256` and `size_m`.
- A shell command whose text contains the word "production" is blocked by a hook: use Read/Edit for those files and test filters without that word.

