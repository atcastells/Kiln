# Context: k011-recipe-verify-publish

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

## Decisions that shape the code

- An asset whose spec has `[generator]` is verified by generating a fixed batch of seeds and measuring every mesh; it is published as a recipe, a script and resolved seeds, never as a GLB.
- Radius lower bound of a recipe, in units of the sphere radius: starts at 1; each `stretch` multiplies it by the smallest of its three factors (the LOW end of a range; an absent axis is 1); each `noise` subtracts its amplitude (the HIGH end of a range). At 0 or below at any step: error `generator_radius`. Sum of the high ends of all noise amplitudes above 0.5: warning `generator_amplitude`.
- Mesh measures are computed in Rust from the GLB, never by the Godot script. Degenerate triangle: |cross(b-a, c-a)| < 1e-9 * |centroid|^2. Flipped face: normal · centroid < 0. Steep face: the angle between the normal and the centroid direction is over 60 degrees (|cos| < 0.5), measured after dividing every position by the total stretch of the resolved recipe (product of all `stretch` ops, per axis).
- A sphere of S segments has exactly 2 * S degenerate triangles (the poles) and `vocab::sphere_tris(S)` triangles.
- Per-seed verdict: errors `generated_flipped` (any flipped face), `generated_degenerate` (more than 2 * segments), `generated_tris` (triangles differ from `sphere_tris`, or exceed `budget_tris` when the spec has one, or exceed `rules.max_tris` when set); warning `generated_steep` (steep fraction above `[generate] steep_max`, default 0.10).
- Batch of seeds: `[generate] verify_seeds` (default 16, 1..=64). Seed 0 of the batch is the spec's own seed; seed k (from 1) is the low 32 bits of `resolve::mix("<seed>/verify/<k>")`.
- Verification of a recipe writes `<build>/<id>/validation.json` with today's shape plus a `recipe` block. A parts asset must serialize exactly as before.
- Publish gate for a recipe: the validation is valid AND its recipe hash, generator version and script hash equal the current ones.
- Published files in `[publish] dir/<id>/`: `recipe.json`, `kiln_generator.gd`, `asset.json` (with `version`). No `.glb`. No Godot run.
- `kiln_generator.gd` builds the mesh (`extends RefCounted`, `static func build(job: Dictionary) -> Dictionary`); `generate.gd` is only the runner. Splitting must not change a single byte of the GLBs.
- `build`, `paint` and `review` keep refusing a generated asset; `verify` and `publish` stop refusing.
- No new Rust dependency. No new MCP tool or route.

## Conventions

- Code, identifiers and comments in English; user-facing messages (findings, errors, `say` lines) in Spanish, like the existing ones.
- Comments only when the logic is not obvious. No emojis. No mention of AI.
- Unit tests in a `#[cfg(test)] mod tests` at the bottom of the file; integration tests in `crates/kiln-cli/tests/`.
- Tests that need Blender or Godot are `#[ignore]`; a subagent never runs them.
- Findings use `catalog::Finding { severity, subject, code, message }`; codes are snake_case; the asset subject is `asset:<id>`.
- A subagent runs only its own tests (`cargo test -p <crate> <filter>`), never the whole suite, never git.
- Red test first: write the test, see it fail, then implement.
- Edit repo files with Edit or Write, never with shell heredocs or scripts.
- A shell command whose text contains the word "production" is blocked by a hook: read and edit such files with the tools, and use test filters without that word.
- Under 100 new non-test lines per task. Touch only the files the task lists; if another file must change to compile, say so in the report.
- K009 (unmerged) edits `verify.rs`: in `verify.rs` and `ops/publish.rs` only the branch to the new code and visibility changes are allowed.

## Key paths

- `crates/kiln-core/src/generate/`: `mod.rs` (`Generator`, `Op`, `Value::{Fixed, Range}`), `vocab.rs` (`OPS`, `op`, `sphere_tris`), `check.rs` (`check(asset_id, &Generator, budget_tris, palette) -> Vec<Finding>`), `resolve.rs` (`resolve`, `mix`, `Resolved`, `ResolvedOp { op, params: BTreeMap<String, f64>, noise_seed }`).
- `crates/kiln-core/src/glb.rs`: `json_chunk`, `structure`, `structure_hash`; it does not decode vertex buffers.
- `crates/kiln-core/src/asset.rs`: `sha256_file`, `sha256_hex`, `write_replacing`.
- `crates/kiln-core/src/validate.rs`: `Report`, `Check`, `glb_findings`.
- `crates/kiln-cli/src/generate.rs`: `generate_asset(project, config_file, id, seed, say) -> Result<Generated, String>`, `GENERATOR_VERSION`, `SCRIPT`; tests module `pub(crate) mod tests` with `FAKE` (fake Godot), `ROCK`, `project(tag)`.
- `crates/kiln-cli/src/verify.rs`: `verify_asset`, private `khronos` and `godot_import`.
- `crates/kiln-cli/src/ops/publish.rs`: `publish_asset`, `stage`, `commit`, `Published`.
- `crates/kiln-cli/src/ops/mod.rs`: `refuse_generated`, `check_id`, `rel`, `ctx`.
- `.sdlc/k011-recipe-verify-publish/spec.md`: the design and the measured table.

## Do not

- Do not change an existing test to make it pass, except the one the task names.
- Do not run git, the whole test suite, Blender, Godot or `claude`.
- Do not add anything to `examples/`.

## Learned

- `kiln_core::mesh::{Mesh { positions: Vec<[f32; 3]>, indices: Vec<u32> }, read(&Path) -> Result<Vec<Mesh>, String>, read_bytes(&[u8]), MAX_VERTICES}`: one `Mesh` per triangle primitive; non-indexed meshes come out with indices `0..count`; damaged files are `Err`, never a panic.
- `generate::check` now also emits `generator_radius` (Error) and `generator_amplitude` (Warn, sum above 0.5): a test recipe whose noise amplitudes sum over 0.5 carries that warning.
- `cfg.generate.verify_seeds` (u32, default 16, 1..=64) and `cfg.generate.steep_max` (f64, default 0.10); validated in `Config::parse`.
- `generate::resolve::batch(seed, n) -> Vec<u32>`: n distinct seeds, the first is `seed`.
- Godot scripts: `crates/kiln-cli/godot/kiln_generator.gd` (the builder, `static func build(job) -> {"mesh","tris"} | {"error"}`) and `generate.gd` (the runner, loads `res://kiln_generator.gd`). In `generate.rs`: `SCRIPT` (runner), `BUILDER`, `builder_sha256()`; both files are written into the work folder. Proven with real Godot: the split gives byte-identical GLBs.
- `generate::measure::{Measures { tris, degenerate, flipped, steep, size_m }, measure(&[Mesh], stretch: [f64; 3]) -> Measures, total_stretch(&Resolved) -> [f64; 3]}`. `Mesh` has no `Debug`/`Clone`. An outward tetrahedron for tests: vertices (1,1,1), (1,-1,-1), (-1,1,-1), (-1,-1,1), indices `[1,3,2, 0,2,3, 0,3,1, 0,1,2]`.
- `crate::generate::{cache_dir (pub(crate)), verify_dir(project, cfg, id) -> <cache>/generated/<id>/verify created empty, run_seed(cfg, project, id, out_dir, &Resolved, say) -> Run { glb, out }}`: `run_seed` takes the seed from `resolved.seed`, writes `<out_dir>/<seed>.glb`, wipes `<out_dir>/.project`, writes no `generated.json`. They carry `#[allow(dead_code)]` until a caller exists: remove it when you add one.
- `generate::hash::recipe_sha256(&Generator, budget_tris: Option<u64>) -> String`; `generate::measure::{judge(asset_id, seed, &Measures, &Limits) -> Vec<Finding>, Limits { segments, budget_tris, max_tris, steep_max }, segments_of(&Resolved)}`.
- `crate::verify_recipe::{run(project, cfg, spec, palette, say) -> Result<Outcome { seeds: Vec<SeedResult>, findings }, String>, verify(project, config_file, id, say) -> Result<PathBuf, String>}`; `verify_asset` branches to `verify` when the spec has a generator. A seed whose GLB cannot be read is a finding (`generated_unreadable`) and is absent from `seeds`.
- The report of a recipe is `kiln_core::build::out_dir(project, &cfg, id)/validation.json` (`kiln/build/<id>/`), today's `Report` plus `recipe`: `{ recipe_sha256, generator_version, script_sha256, verify_seeds, seeds: [ { seed, glb_sha256, resolved: { seed, material, ops: [ { op, params, noise_seed } ] }, tris, degenerate, flipped, steep, size_m } ], worst: { tris, steep, flipped, size_m } | null }`. `valid` is false and `seeds` empty when a static error stopped the batch.
- Test helpers in `verify_recipe::tests` (`pub(crate)`): `sphere_glb(segments, flip_one)` (8 segments: 80 triangles, 16 degenerate), `setup`, `report_of`.
- The fake Godot of the `generate.rs` tests copies `fixture.glb` (a GLB with no triangles to measure): a test that measures a mesh needs its own fixture with a real mesh.

