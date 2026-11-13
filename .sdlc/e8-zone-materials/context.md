# Context: e8-zone-materials

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data.

## Decisions that shape the code

- A zone is a material slot of a piece, named as the model names it. A filler box has one zone, `body`.
- A spec part carries `materials = { zone = "palette key" }`; the old `material` stays as "every zone".
- Resolution order for a zone: spec `materials[zone]` > spec `material` > catalog binding for the slot > the model's own material.
- The assembled GLB names each material `part/zone` (part id, not copy name); copies of a part share them.
- Variants: `[[variants]]` in the spec with `id` and `materials = { "part/zone" = "key" }`, only what changes from the base.
- Validating moves a part's zone choices from the spec to the catalog bindings (`kiln/bindings.toml`) of its source piece.
- Godot: Kiln writes `_subresources.materials` in the built GLB's `.import`; material path from the template `[materials] godot = "res://materials/{key}.tres"`.

## Conventions

- Code, identifiers, comments in English; user-facing messages (findings, errors) in Spanish, as the existing ones.
- Rust 2024. `kiln-core` has no network, no processes and only serde, serde_json, toml, glob. No new dependencies.
- Unit tests live in a `#[cfg(test)] mod` at the bottom of the file they test. Red test first.
- Comments only when the logic is not obvious. No emojis. No mention of AI.
- Run only your own tests (`cargo test -p kiln-core <name>`); the full suite and clippy are run by the main session.
- New serde fields use `#[serde(default)]` so existing files keep reading; maps use `BTreeMap`.

## Key paths

- `crates/kiln-core/src/spec.rs`: the asset spec, its checks, `set_source`.
- `crates/kiln-core/src/build.rs`: the assembly plan (`Plan`, `Item`, `Paint`).
- `crates/kiln-core/src/bindings.rs`: slot -> palette bindings per catalog piece.
- `crates/kiln-core/src/godot.rs`, `config.rs`: Godot measures; `kiln.toml` sections.
- `crates/kiln-core/tests/fixtures/`: recorded files for tests.
- `examples/`: the invented game Puerto Niebla.

## Do not

- Do not run any git write command.
- Do not touch files outside the ones your task lists.
- Do not name any real game or project; examples are generic (Puerto Niebla).
- Do not change an existing test to make it pass.

## Learned

- Godot 4.7.1 keeps `"use_external/enabled": true` + `"use_external/path"` under `_subresources={"materials": {"<name>": {...}}}` and adds `"use_external/fallback_path"` on reimport. Names with `/` work.
- A `.blend` in sight of Godot stops `godot --headless --import`; the build keeps it in `<build>/<id>/.kiln/`, and `review/` has a `.gdignore`.
- glTF repeats a vertex per face at hard edges: mesh islands must be found by vertex position, not index.
- Kiln prepends `paint.py` (and `split.py`) to the Blender scripts it writes; `wear`, `zone_of`, `worn` live there.
- `Piece.split` comes from `kiln/zones.toml`; a split piece's zones (`part_N`, `color_N`) only exist after a build, so `check` does not flag them.
- The example runs in place: `kiln check --project examples`.
