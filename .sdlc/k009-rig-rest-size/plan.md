# Plan: medir en reposo las piezas con esqueleto (K009)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-18); rama local, push y PR al cierre de ASTRA T010-C2
- **Nothing is implemented until this file is approved.**

## Done means

Un humanoide cuya pose inicial se sale de su caja en reposo da en
`report.json` su tamaño en reposo. `cargo test --workspace`, Clippy y los
tests ignorados de `rig` en verde. En ASTRA, `crew_guard` y `crew_porter`
pasan `kiln verify` y el técnico reconstruido sigue igual al publicado.

## Stop and ask if

- Un test existente falla por algo ajeno a este cambio.
- El GLB de una pieza con esqueleto cambia.
- Hay que tocar algo más que `assemble.py`, el fixture, el test y el README.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Fixture con `reach` y prueba roja `build_measures_a_rig_at_rest` | `crates/kiln-cli/tests/fixtures/rig_humanoid.py`, `crates/kiln-cli/tests/blender_rig.rs` | La prueba falla antes del arreglo con una profundidad mayor que 0,22 m | R1 | main | — | 5m/1M | done (cause: none) |
| T2 | Medir las armaduras en reposo y restaurar | `crates/kiln-cli/blender/assemble.py` | `build_measures_a_rig_at_rest` y `build_keeps_rig` en verde | R1, R2 | main | T1 | 3m/500k | done (cause: none) |
| T3 | Batería: `cargo test --workspace`, Clippy, tests ignorados de `rig`, `blender_contract` y `blender_piece`; en ASTRA, `verify` de guardia y mozo y técnico igual al publicado | — | Todo en verde | R2, R3 | main | T2 | 8m/1M | done (cause: none) |
| T4 | Una línea en la sección de personajes del README | `README.md` | Lectura | R1 | main | T3 | 2m/200k | done (cause: none) |

## Schedule

Secuencial, en la sesión principal: ~18 min, ~3M de tokens.

## Test strategy

TDD: la prueba roja primero, con Blender real (test ignorado, como los de
K008). Si una prueba falla, se arregla el código.

## Verification commands

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p kiln-cli -- --ignored rig
cargo test -p kiln-cli -- --ignored blender_contract blender_piece
```

## Implementation risks

- `assemble.py` es común a todas las piezas: el cambio solo actúa sobre
  objetos de tipo armadura.

## Review

- **bugs**: no important findings. Blender 5.2.3: the four `blender_rig` tests pass; the measurement runs after the save and the export, so neither file changes; a shared armature datablock restores correctly; models without a skeleton lose their armatures on import, so they are measured as before. Nits kept as they are: the loop covers every armature object rather than only the built rigs (equivalent today); no `try/finally` round the restore (nothing can fail in between and the process ends).
- **security**: no findings. No new input reaches Blender or the filesystem.
- **conformance**: no important findings. R1 reproduced red by the reviewer with the `assemble.py` of `main` (0.62 x 1.67 x 0.464 m) and green with the change; R2 holds by construction and by `build_keeps_rig`; R3 by the Blender tests. Nit fixed: the stop rule now lists `README.md`, which T4 touches.

Build result (2026-11-18): red test measured 0.62 x 1.67 x 0.464 m; green after the fix. `cargo test --workspace` 423 passed; Clippy clean; ignored tests of `blender_rig` (4), `godot_verify` rig (3), `blender_contract`, `blender_piece`, `blender_turn` and `blender_anchor` green. In ASTRA: the technician rebuilt equals the published version 6; `crew_guard` and `crew_porter` pass `verify`.

## Outside this plan

- Envolvente de animaciones, tolerancia de `glb_size`.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
