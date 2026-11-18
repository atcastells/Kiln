# Plan: personajes con esqueleto y animaciones a través de Kiln (K008)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-18)
- **Nothing is implemented until this file is approved.**

## Done means

El humanoide de prueba (6 huesos, pesos, `idle` y `walk`) pasa
`kiln build → verify → publish` en una copia de `examples` con su kit en
una carpeta fuera de `godot/` (`kits_src/`), y el GLB
publicado tiene 1 `skin` de 6 huesos, `JOINTS_0`/`WEIGHTS_0` y las dos
animaciones. El mismo recorrido con un GLB sin animaciones hace fallar
`verify`. `cargo test --workspace` y Clippy en verde, los tests ignorados
pasan con Blender y Godot reales, y el hash estructural de `hoist` no cambia.

## Stop and ask if

- The task needs to open windows or apps on the developer's screen.
- El hash estructural de `hoist` o de una pieza de ASTRA cambia.
- La ida y vuelta por Blender deforma el esqueleto y hay que cambiar de
  enfoque (paso íntegro del GLB sin montar).
- Hay que tocar `paint.py`, `repaint.py` o `split.py`.
- Un test existente falla por algo ajeno a este cambio.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Leer esqueletos y animaciones del GLB en `Structure` y meterlos en el hash solo cuando existen | `crates/kiln-core/src/glb.rs` | `glb::tests`: lee 1 piel de 2 huesos y 2 animaciones de un glTF de prueba; quitar una animación o la piel cambia el hash (los huesos ya entran como nodos); un glTF sin esqueleto da el hash de antes | R3 | implementer | — | 1m/170k | done |
| T2 | Campo `animations` en `[asset]` y error de ficha si se declara con `count` > 1 | `crates/kiln-core/src/spec.rs` | `spec::tests`: se lee la lista; ficha sin el campo queda vacía; `count = 2` con `animations` da error | R5 | implementer | — | 1m/170k | done |
| T3 | Hallazgos de esqueleto y animación mal formados | `crates/kiln-core/src/validate.rs` | `validate::tests`: un test por hallazgo (piel sin `JOINTS_0`, sin `WEIGHTS_0`, hueso inexistente, animación sin canales, canal a nodo inexistente) y uno sin hallazgos | R4 | implementer | T1 | 1m/170k | done |
| T4 | `asset_findings`: animaciones declaradas que faltan y ficha con animaciones sobre un GLB sin esqueleto | `crates/kiln-core/src/validate.rs` | `validate::tests`: falta `walk` da error; sin `skins` da error; ficha sin `animations` no añade hallazgos | R5 | implementer | T2, T3 | 1m/170k | done |
| T5 | Guion que genera el humanoide de prueba y `export_piece.py` que conserva armadura, pieles y acciones | `crates/kiln-cli/tests/fixtures/rig_humanoid.py`, `crates/kiln-cli/blender/export_piece.py`, `crates/kiln-cli/tests/blender_rig.rs` | `blender_rig::piece_keeps_rig` (ignorado, Blender real): el GLB de la pieza tiene 6 huesos y 2 animaciones; `blender_piece` sigue en verde | R1 | main | T1 | 5m/2M | done |
| T6 | `assemble.py` monta la pieza con esqueleto (la raíz es la armadura) sin hornear, unir, recentrar ni escalar; importa con `disable_bone_shape`; triángulos y tamaño salen de las mallas hijas; detiene el montaje si se le piden varias copias o ajuste de tamaño | `crates/kiln-cli/blender/assemble.py`, `crates/kiln-cli/tests/blender_rig.rs`, `crates/kiln-cli/tests/fixtures/rig_deform.gd` | `blender_rig::build_keeps_rig` (ignorado, Blender y Godot reales): frente al GLB de la pieza, mismos huesos y pesos por vértice y deformación con menos de 0,0001 m de diferencia en 9 instantes de cada animación; `report.json` da 72 triángulos y el tamaño del humanoide; `count = 2` detiene el montaje; `blender_contract` da el mismo hash para `hoist` | R2 | main | T5 | 5m/2M | done (cause: none) |
| T7 | `import_check.gd` informa de huesos y animaciones y de si mueven huesos; una función pura compara ese informe con las animaciones declaradas | `crates/kiln-cli/godot/import_check.gd`, `crates/kiln-core/src/godot.rs` | `godot::tests` (módulo nuevo): informe con `idle` y `walk` pasa; sin `walk` o con una que no mueve huesos da hallazgo; el informe de hoy, sin esos campos, se sigue leyendo | R6 | implementer | — | 1m/170k | done |
| T8 | `verify` usa esa comparación: `godot_import` falla si falta una animación declarada o no mueve ningún hueso | `crates/kiln-cli/src/verify.rs`, `crates/kiln-cli/tests/godot_verify.rs` | `godot_verify::rig_*` (ignorado, Blender y Godot reales): el humanoide pasa; el mismo sin `walk` falla | R6 | main | T2, T5, T7 | 1m/170k | done (cause: run on main instead of an implementer, its tests need Blender and Godot) |
| T9 | Bloque `rig` en `asset.json` | `crates/kiln-core/src/asset.rs` | `asset::tests`: con esqueleto lista huesos y animaciones con duración; sin él, el contrato es idéntico al de hoy | R7 | implementer | T1 | 1m/170k | done |
| T10 | Batería entera, tests ignorados y recorrido real en una copia de `examples` con el kit del humanoide en `kits_src/`, en positivo y en negativo; escribir `e2e.md` | `.sdlc/k008-rigged-characters/e2e.md` | El recorrido mismo: GLB publicado con esqueleto y animaciones; el GLB sin animaciones falla en `verify` | R7, R8 | main | T4, T6, T8, T9 | 5m/2M | done (cause: none) |
| T11 | Documentar la ficha con animaciones y sus límites | `README.md`, `skills/asset/SKILL.md` | Lectura: cada límite del spec aparece; los ejemplos de ficha se leen con `kiln check` | R8 | implementer | T10 | 1m/170k | done (cause: none) |
| T12 | Verificación independiente del cambio y dos flujos vecinos (`hoist` completo, `kiln index` de `examples`) | — | Informe del verificador | R2, R8 | verifier | T11 | 5m/1.2M | done |

Cada tarea delegada cubre un punto y menos de 100 líneas nuevas, y corre solo
sus tests; la batería entera corre una vez, en T10.

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1, T2, T7 | 1m | `git diff --stat`, leer los diffs |
| 2 | T3, T9 | 1m | ídem |
| 3 | T4, T5 | 5m | ídem |
| 4 | T6, T8 | 5m | `git diff --stat`, leer los diffs |
| 5 | T10 | 5m | — |
| 6 | T11 | 1m | leer el diff |
| 7 | T12 | 5m | consolidar |

- **Estimated wall-clock**: 23 min más las comprobaciones de main (~10 min)
- **Sequential sum**: 28 min
- **Estimated tokens**: ~8,6M

## Order and dependencies

Primero lo que Kiln lee (T1, T2, T7), porque sin ello nada puede fallar en
rojo. Después lo que comprueba (T3, T4, T8, T9). Los guiones de Blender (T5,
T6) van al final y en la sesión principal: son el punto incierto y sus tests
necesitan lo anterior para medir. T3 y T4 comparten fichero y van en olas
distintas.

## Test strategy

TDD: test rojo antes de implementar. Si un test falla, se arregla el código.

- **Unit**: lectura del GLB, hash, hallazgos, ficha, contrato y lectura del
  informe de Godot, con glTF escritos a mano; sin herramientas externas.
- **Integration**: `blender_rig` y `godot_verify` (ignorados) con Blender
  5.2.3 y Godot 4.7.1 reales, sin ventana.
- **Manual verification**: el recorrido de T10 y lo que mire el verificador.
  Antes de diagnosticar un rojo de los tests ignorados se mide la línea base
  en `main`.

## Verification commands

```
cargo build --workspace
cargo test --workspace
cargo test -p kiln-cli -- --ignored
cargo clippy --workspace --all-targets -- -D warnings
```

## Implementation risks

- `assemble.py` lo usan todos los assets: la rama nueva solo entra cuando el
  modelo importado trae armadura.
- `export_piece.py` corre con `export_apply=True`; con armadura hay que
  comprobar que no aplica el modificador de armadura (lo mide T5).
- `structure_hash` lo usan `validation.json` y `blender_contract`: los
  bloques nuevos no entran en el texto del hash si están vacíos.
- Una pieza con esqueleto no es una malla: `tris`, `bound_box`, `dress` y
  `src.copy()` de `assemble.py` dan por hecho que sí. T6 los resuelve sobre
  las mallas hijas.
- Godot 4.7.1 puede nombrar el esqueleto y el reproductor de otra forma según
  la jerarquía: `import_check.gd` busca por clase, no por nombre.

## Review

Filled in Deploy. A plan is not closed while this section is empty.

- **bugs**:
- **security**:
- **conformance**:

## Estimate vs actual

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

- Fases 2–4 de T010-C1 (técnico, integración en Godot, capturas): cadena de
  ASTRA, que parte de `task-010b-access-workshop-identity`.
- Variantes pintadas de un asset con esqueleto.
- Fijar el bucle de las animaciones al publicar.
- Que `publish` falle con un `.blend` dentro del proyecto de Godot.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
