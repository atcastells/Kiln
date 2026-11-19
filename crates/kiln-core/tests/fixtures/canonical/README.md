# Canonical geometry fixtures

`asteroid_64_seed42.bin`: canonical container (spec D3, 73 784 bytes, 2050
vertices, 4096 triangles) of asset `asteroid_64`, seed 42, the positive that
`tests/canonical_cases.rs` mutates (via C of K013).

- Source GLB: exported by `crates/kiln-cli/godot/generate.gd` with Godot 4.7.1
  on arm64, headless, from the seed 42 ops of `asteroid_64/recipe.json`
  including `facet` (K013 task T1, `ast42.glb`).
- Built with `canonical::build(positions, indices, 64)` over `mesh::read`.
- `geometry_sha256` = `2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a`.
- Regenerate (builds, validates, writes and prints the hash):
  `cargo run -p kiln-core --example canonical_fixture -- <ast42.glb>
  crates/kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin`.
