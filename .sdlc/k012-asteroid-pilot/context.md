# Context: k012-asteroid-pilot

Shared by every task of the plan. Read it before anything else. The main
session is the only writer. Committed: no personal data, credentials or
sensitive business data (prices, margins, stock).

The code of this delivery lives in ASTRA: `/Users/acastells/.t3/projects/astra`,
branch `k012-asteroid-pilot`. This file and the chain live in Kiln.

## Decisions that shape the code

- The pilot is independent of the playable station: nothing under `crates/`, `godot/proto/`, `godot/net/`, `godot/station/` or `content/` changes. No server, no network.
- Three published packages in `godot/assets/kiln/asteroid_32/`, `asteroid_64/`, `asteroid_128/`, each with `recipe.json`, `kiln_generator.gd` (+ `.uid`), `asset.json`. No GLB anywhere.
- `recipe.json`: `{ asset, name, generator_version, script, script_sha256, recipe_sha256, recipe, budget_tris, seeds: [ { seed, material, ops: [ { op, params: {..numbers..}, noise_seed? } ], tris, size_m: [x,y,z] } ], worst }`. 16 seeds per package; the first is 42.
- `asset.json`: `{ asset, kind: "generated", version, files, recipe_sha256, recipe_json_sha256, script_sha256, generator_version, validation }`.
- The published script: `load(".../kiln_generator.gd").build({"material": entry.material, "ops": entry.ops}) -> {"mesh": ArrayMesh, "tris": int} | {"error": String}`. It draws nothing itself and does NOT validate its job: a malformed job is a script error, so the consumer checks the shape first.
- The consumer never draws a seed: only seeds listed in `recipe.json` can be built.
- The consumer loads the script only from the package folder and only when its sha256 equals `asset.json`'s `script_sha256`; `recipe.json` must match `recipe_json_sha256`. `FileAccess.get_sha256(path)` gives the hex hash.
- Supported `generator_version`: 1.
- Tolerance when comparing a built mesh with the package: triangles exact; each size within 1 part in 100 000 (measured: 6 parts in 100 million).
- Everything is measured on this Mac. No Docker, no Linux comparison.
- Vocabulary of ops (ids and params): `sphere` (`radius_m`, `segments`), `stretch` (`x`, `y`, `z`), `noise` (`amplitude`, `size`, plus `noise_seed` beside `params`), `facet` (no params).

## Conventions

- GDScript: code and identifiers in English; comments only when the logic is not obvious; no emojis; no mention of AI. Texts the person sees (labels, error messages) in Spanish.
- Test scripts follow `godot/tests/ui_theme_test.gd`: `extends SceneTree`, `_initialize` calls `_run.call_deferred()`, failures print lines starting with `FAIL`, success prints one exact `OK: ...` line, then `quit()`.
- Arguments after `--` are read with `OS.get_cmdline_user_args()` as `--name=value`.
- A task may run Godot headless for its own test, from the ASTRA root: `godot --headless --path godot -s tests/<file>.gd`. Never open a window, never start the Rust server, never run `scripts/godot_station.sh` whole, never run git.
- A run is green when it prints its `OK:` line and the output has no `SCRIPT ERROR`, `Parse Error` or `FAIL`.
- Godot writes a `.uid` beside every new `.gd`: leave it, it is committed.
- Red test first: write the test, see it fail, then implement.
- Edit files with Edit or Write, never with shell heredocs. A shell command whose text contains the word "production" is blocked by a hook.
- Under 100 new non-test lines per task. Touch only the files the task lists.

## Key paths (ASTRA)

- `kiln/specs/asteroid_{32,64,128}.toml`: the three recipes.
- `godot/assets/kiln/asteroid_*/`: the published packages.
- `godot/procedural/recipe_package.gd`: the consumer.
- `godot/tests/`: test scripts, one `.gd` each; `ui_theme_test.gd` is the pattern, `crew_shots.gd` the pattern of a windowed capture script with `cost.json`.
- `scripts/godot_station.sh`: phases (`phase_<name>`, `run_test`) and windowed modes (`--crew-shots DIR` is the no-server precedent).
- `godot/ui/astra_theme.tres`: the UI theme.

## Do not

- Do not touch Rust, the protocol, `godot/station/`, `godot/net/` or Kiln's code.
- Do not export or commit a GLB.
- Do not change an existing test to make it pass.
- Do not leave files in `user://` after a test.

## Learned

- The consumer, `godot/procedural/recipe_package.gd` (load it with `const Package := preload("res://procedural/recipe_package.gd")`):
  - `Package.open(id: String, base := Package.PUBLISHED) -> Dictionary`: `{"error": String}` or `{"id", "dir", "contract", "recipe", "generator", "seeds": Array[int]}`.
  - `Package.build(package, seed: int) -> Dictionary`: `{"error": String}` or `{"mesh": ArrayMesh, "seed", "tris", "vertices", "size_m": Vector3, "ms": float, "hash": String, "expected_tris", "expected_size_m"}`. It refuses a seed outside the batch, a malformed entry and a mesh that does not match the package.
- Measured on this Mac, headless: average build 0.7 ms (32 segments, 3 264 vertices), 2.6 ms (64, 12 672), 9.9 ms (128, 49 920). The asteroids are 2.0 to 3.3 km long.
- The published script sets a default white `StandardMaterial3D` on surface 0: a scene gives the look with `material_override`.
- JSON numbers arrive as float: use `int()`. `:=` cannot infer from untyped loop variables: write the type.
- Copying into `user://`: `FileAccess.get_file_as_bytes` then `FileAccess.open(path, WRITE).store_buffer`, after `DirAccess.make_dir_recursive_absolute`; `load()` of a `.gd` in `user://` works.
- A windowed capture that works here (Metal): add the nodes under `root`, `await process_frame` a few times, `await RenderingServer.frame_post_draw`, then `root.get_texture().get_image().save_png(path)`. For objects kilometres across set `Camera3D.far` and `DirectionalLight3D.directional_shadow_max_distance` accordingly.

