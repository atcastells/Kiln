# Context: k008-rigged-characters

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

## Decisions that shape the code

- One path: the existing build learns to keep the rig. No second, pass-through path.
- A GLB or a spec without skeleton, skins or animations must behave exactly as before: same structural hash, same findings, same serialized JSON (new fields are skipped when empty).
- The spec declares what it expects with `[asset] animations = ["idle", "walk"]`; nothing is required when the list is empty.
- Weight normalisation is left to the Khronos glTF Validator; Kiln does not read weight values.
- A declared animation that is missing is an error; one that moves no bone is a warning.
- No new Rust dependency.

## Conventions

- Code, identifiers and comments in English; user-facing messages (findings, errors, `say` lines) in Spanish, like the existing ones.
- Comments only when the logic is not obvious. No emojis. No mention of AI.
- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the file; integration tests in `crates/kiln-cli/tests/`.
- Tests that need Blender or Godot are `#[ignore]`; a subagent never runs them.
- Findings use `catalog::Finding`; codes are snake_case; look at `validate.rs` for the shape.
- A subagent runs only its own tests (`cargo test -p <crate> <filter>`), never the whole suite, never git.
- Red test first: write the test, see it fail, then implement.
- Edit repo files with Edit or Write, not with shell heredocs (a hook blocks them).
- Under 100 new lines per task.

## Key paths

- `crates/kiln-core/src/glb.rs`: GLB reader, `Structure`, `structure_hash`.
- `crates/kiln-core/src/validate.rs`: `glb_findings` (file), `asset_findings` (file against spec), `Report`.
- `crates/kiln-core/src/spec.rs`: asset spec. `crates/kiln-core/src/asset.rs`: `asset.json` contract.
- `crates/kiln-core/src/godot.rs`: Godot-side helpers. `crates/kiln-cli/src/verify.rs`: runs the three checks.
- `crates/kiln-cli/blender/*.py`, `crates/kiln-cli/godot/*.gd`: embedded scripts.
- `examples/`: the sample project used by tests.

## Do not

- Do not edit existing tests to make them pass; fix the code.
- Do not touch files outside the task's list.
- Do not change the text of existing error messages or findings.
- Do not touch the ASTRA repository.

## Learned
- `glb::Structure` has no `Default`: every literal needs `skins: vec![], animations: vec![]` (one lives in the tests of `validate.rs`).
- `Structure.skins[].joints` are node names; `Structure.animations[]` carry `name`, `channels`, `duration`.
- `Asset.animations` is the list a spec declares; `godot::rig_import_findings(subject, report, expected)` compares Godot's report with it.
- Other tasks may be editing the crate at the same time: if a compile error points at a file that is not yours, wait and retry once, then report it.
- New finding codes: `skin_no_joints_attr`, `skin_no_weights_attr`, `animation_empty`, `rig_count`, `rig_no_skeleton`, `rig_animation_missing`, `godot_no_skeleton`, `godot_animation_missing`, `godot_animation_static` (warning); dangling skin, joint and animation indices reuse `glb_broken_ref`.
- `asset.json` gains `outputs[].rig = { bones, animations: [{ name, duration_s }] }` only for a rigged GLB.
- `assemble.py` places a rigged piece once, as authored (no bake, fit, recentring); `count > 1`, a resize or a split stop the build.
- `rustfmt` needs `--edition 2024`; the repo is not fmt-clean, so only check your own lines.
