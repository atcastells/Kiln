# Plan: piezas modeladas por el MCP (`source.kind = "model"`)

- **Derived from**: `spec.md` (borrador)
- **Status**: APPROVED BY ENGINEER, Aaron Castells, 2026-11-22 («Apruebo el plan»). Con él quedan aceptados el intent y el spec, y la propuesta de la pregunta 5 (texturas fuera).
- **Nothing is implemented until this file is approved.**
- **Repos**: código en `~/Kiln`. ASTRA solo se lee, y la prueba de punta a
  punta va sobre una copia en el scratchpad.

## Rama de partida

- La copia de trabajo está en `k015-derived-mesh-verify`, ya unida a `main`
  (PR #36).
- Por la respuesta 3 del intent, la rama nueva sale de `origin/k016-shared-core`
  (PR #37, abierto): `git switch -c k017-modeled-parts origin/k016-shared-core`.
  La crea Claude al aprobarse este plan, porque el desarrollador pidió partir
  de K016. Los ficheros sin seguimiento de `.sdlc/` pasan intactos.
- Si K016 cambia antes de unirse, esta rama se queda sobre la cabeza que
  tenía. El PR de K017 apunta a `k016-shared-core` hasta que el #37 se una, y
  después a `main`.

## Done means

- Una ficha con una pieza `model` pasa `kiln check`, se monta sin cajas ni
  `todo` y su GLB pasa `kiln verify` con sus zonas pintadas.
- R8 cumplido sobre una copia de ASTRA, con capturas lado a lado en
  `.sdlc/k017-modeled-parts/shots/` y las cifras (triángulos, medidas, zonas)
  en `e2e.md`.
- `cargo test --workspace` sin fallos y con las pruebas existentes sin
  cambios; `clippy` limpio; las ignoradas de Blender de esta entrega pasan.

## Stop and ask if

- Hace falta tocar el repo de ASTRA (no su copia).
- Una prueba existente falla por algo ajeno a esta entrega (la línea base se
  mide antes en T0).
- El vocabulario de R1 no basta para `workbench` y hay que añadir una forma.
- La prueba de punta a punta no converge en una pasada.
- The task needs to open windows or apps on the developer's screen.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T0 | Crear la rama desde `origin/k016-shared-core`; línea base: `cargo test --workspace` y `clippy`, salida en fichero | `.sdlc/k017-modeled-parts/baseline/` | — | R9 | main | — | 5m/300k | done (cause: none; 682/0/30, clippy limpio) |
| T1 | `model.rs`: `Shape` (`box`, `para`, `tube` con `zone`), tabla de límites, validación, `tris()`, `bounds()`, texto del vocabulario | `crates/kiln-core/src/model.rs`, `crates/kiln-core/src/lib.rs` | `model::tests`: tabla de formas rotas → código (R6); `tris` y `bounds` de las tres formas | R1, R5, R6 | implementer | T0 | 2m/250k | done (cause: main added strict parsing via try_from and exact tube rims after review; the tube bounds by circle could miss ±1 mm) |
| T2 | `spec.rs`: `"model"` en `KINDS`, `Part.shapes`, y en `spec::check`: `shapes` ⇔ `model`, `model_shape`, `model_zone`, `model_size`, triángulos de las formas sumados a `budget` (incluye T4: mismo fichero) | `crates/kiln-core/src/spec.rs` | `spec::tests`: una ficha `model` se lee sin avisos; una tabla con cada código y su ficha mínima | R1, R3, R5, R6 | implementer | T1 | 3m/350k | done (cause: none; main reworded the budget message) |
| T3 | `config.rs`: `[assets] uv_per_m` opcional, > 0 | `crates/kiln-core/src/config.rs` | `config::tests`: por defecto 1.0, 0 o negativo rechazado | R4 | implementer | T0 | 1m/200k | done (cause: none) |
| T4 | Fusionada en T2: las comprobaciones de la ficha viven en `spec::check` (`spec.rs:352`), no en `validate.rs` | — | — | — | — | — | 0m/0k | done (cause: merged into T2) |
| T5 | `build.rs`: `Item.shapes`, `model = None`, `fit = false`, `todo = None`, `size_m` de `bounds()`; `Plan.uv_per_m` | `crates/kiln-core/src/build.rs` | `build::tests`: el item de una pieza `model` lleva sus formas y no tiene `todo` | R2, R4 | implementer | T2, T3 | 2m/250k | done (cause: none; main added the `model` label in `ui/app/views/spec.js`) |
| T6 | `assemble.py`: malla desde `shapes` (ranura por zona, `to_blender`, normales, UV en caja con `uv_per_m`, `base_to_origin`) | `crates/kiln-cli/blender/assemble.py`, `crates/kiln-cli/tests/model_build.rs` | Ignorada (pide Blender): ficha con una forma asimétrica de cada tipo → GLB con los triángulos de `tris()`, la caja de `bounds()` en ejes de glTF, una ranura por zona y la U esperada | R2, R3, R4 | implementer | T5 | 3m/400k | done (cause: Blender runs; 20 min wall, mutation proved the test; UV scale not asserted, checked in T9) |
| T7 | Texto del vocabulario en el prompt `kiln-asset` y en la petición del agente; sección «Una pieza modelada» en la skill; README | `crates/kiln-cli/src/mcp/prompts.rs`, `crates/kiln-cli/src/agent.rs`, `skills/asset/SKILL.md`, `README.md`, `crates/kiln-cli/tests/mcp_prompts.rs` | `mcp_prompts` y `agent::tests` piden «Vocabulario de modelado» | R7 | implementer | T1 | 2m/250k | done (cause: none; session cut at the end, diff checked by main) |
| T8 | Batería L3 (`cargo test --workspace`, `clippy`, ignoradas de T6) y `verifier` (cambio + `examples/` montado y publicado igual + un asset generado sin cambios) | `.sdlc/k017-modeled-parts/review/` | — | R9 | main + verifier | T4, T6, T7 | 10m/1.5M | done (cause: verifier waited on a permission prompt; amber, I1 fixed) |
| T9 | De punta a punta (R8): copia de ASTRA sin `.godot` ni `target/`, `uv_per_m = 0.5` en la copia; `claude -p` con solo `kiln mcp` (y tope de turnos) rehace `workbench` como `workbench_model`; `build`, `review`, `verify`, `publish` en la copia; capturas del original y del modelado con `render.py` | `.sdlc/k017-modeled-parts/e2e.md`, `.sdlc/k017-modeled-parts/shots/` | Cifras de R8 medidas en los dos GLB | R8 | main | T8 | 30m/5M | done (cause: two passes; R8 tris ±10 % and same zones not met as written, see e2e.md) |
| T9b | Arreglo que pidió T9: herramienta `guide` (`asset`/`review`/`context`) con el texto de los prompts, porque `claude -p` no pasa los prompts MCP al modelo y la primera pasada sacó el formato de los errores. Mención en `INSTRUCTIONS`; `guide` en la superficie fijada de `mcp_guards` | `crates/kiln-cli/src/mcp/prompts.rs`, `crates/kiln-cli/src/mcp/mod.rs`, `crates/kiln-cli/tests/mcp_prompts.rs`, `crates/kiln-cli/tests/mcp_guards.rs` | `mcp_prompts::guide_tool_gives_the_prompt_texts` | R7 | main | T9 | 5m/500k | done (cause: not in the plan; found by T9 pass A) |
| T10 | Revisión en tres pasadas (bugs, seguridad, conformidad), arreglos, `CLAUDE.md` (recuentos), PR contra `k016-shared-core` | `.sdlc/k017-modeled-parts/review.md`, `CLAUDE.md` | — | todos | main + 3 reviewer | T9 | 20m/3M | done (cause: seven important findings fixed with tests; PR waits on the developer's commit) |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 0 | T0 | 5m | — |
| 1 | T1, T3 | 2m | `git diff --stat` |
| 2 | T2, T7 | 3m | `git diff --stat` |
| 3 | T5 | 2m | `git diff --stat`, L3 |
| 4 | T6 | 3m | Blender ignoradas |
| 5 | T8 | 10m | — |
| 6 | T9 | 30m | — |
| 7 | T10 | 20m | — |

- **Estimated wall-clock**: ~75 min, más las comprobaciones entre olas.
- **Sequential sum**: 79 min.
- **Estimated tokens**: ~11,7 M (T9 y T10 son el grueso).

## Order and dependencies

`model.rs` es la tabla única; `spec.rs` la usa para leer, las comprobaciones
y `build.rs` para medir. `assemble.py` va después de `build.rs` porque lee el
`plan.json` que este escribe. El texto del vocabulario (T7) solo depende de
la tabla. La prueba de punta a punta va sobre la batería en verde.

## Test strategy

- **Unit**: formas, límites, triángulos y caja (`model`); lectura de la ficha
  (`spec`); códigos de `kiln check`; el item del plan (`build`).
- **Integration**: la prueba ignorada de T6 con Blender real; `mcp_prompts`.
- **Manual verification**: T9 (capturas lado a lado, cifras) y el `verifier`
  de T8 (dos flujos vecinos: `examples/` y un asset generado).

## Verification commands

```
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p kiln-cli --test model_build -- --ignored
```

## Implementation risks

- Ejes: glTF (Y arriba, frente +Z) en la ficha frente a Blender (Z arriba)
  en `assemble.py`. T6 lo mide con una forma asimétrica.
- `dress()` nombra la zona con `zone_of(slot)`: las ranuras de la malla
  modelada deben llamarse como la zona, sin sufijos, o se pintan mal.
- `Item` se serializa a `plan.json`: el campo nuevo va con `skip_serializing_if`
  para no cambiar los planes sin piezas `model` (R9).

## Review

Detalle en `review.md`. Siete importantes, todos arreglados y probados.

- **verifier**: ámbar, 1 importante (`build` no aplicaba R6), arreglado (I1).
- **bugs**: 2 importantes (eje del tubo en float32, zona larga sin pintar), arreglados (I2, I3).
- **security**: 3 importantes (I1, memoria de `bounds()`, `count` sin tope), arreglados (I1, I4, I5).
- **conformance**: pasa, con 4 importantes. README arreglado (I6) y batería final repetida (I7). Quedan dos decisiones del product owner: `guide` como herramienta nueva (D1) y las dos cifras de R8 (D2).

Batería final, sobre el árbol definitivo, antes de la línea del vocabulario:
`cargo test --workspace` da 694 pasan, 0 fallan, 31 ignoradas; clippy está
limpio; `model_build` (Blender) pasa. `scripts/ui_test.sh` sale con 1 por un
único fallo, «Contexto: search "electricidad" narrows (4 dimmed)». Es el
mismo fallo que el PR #37 (K016) ya documenta en `main`, y K017 no toca esa
vista. Tras la línea del vocabulario se repitieron `model::` (3/0) y
`mcp_prompts` (7/0). Registros en `review/final-*.log`.

## Estimate vs actual

Desde `actuals.jsonl`.

| # | Runs on | Est | Actual | Deviation (min / tok) | Cause |
|---|---------|-----|--------|-----------|-------|
| T0 | main | 5m/300k | 1.6m/2088k | −68 % / +596 % | la lectura inicial del repo cae en el reloj de T0 |
| T1 | implementer | 2m/250k | 1.4m/186k | −30 % / −26 % | — |
| T2 | implementer | 3m/350k | 6.5m/155k | +117 % / −56 % | sin causa medida (compiló a la vez que T7) |
| T3 | implementer | 1m/200k | 1.1m/264k | +10 % / +32 % | — |
| T5 | implementer | 2m/250k | 7.5m/192k | +275 % / −23 % | sin causa medida |
| T6 | implementer | 3m/400k | 4m/735k | +33 % / +84 % | corridas de Blender y mutante |
| T7 | implementer | 2m/250k | 15.2m/282k | +660 % / +13 % | el equipo se suspendió a mitad de la respuesta (error de API) |
| T8 | verifier | 7m/1300k | 105.4m/912k | +1406 % / −30 % | esperó la aprobación de un permiso de Bash |
| T9b | main | 5m/500k | 66.1m/2764k | +1222 % / +453 % | fuera del plan; el reloj de main incluye las dos pasadas de T9 y las capturas |
| T10 | reviewer ×3 | 3m/750k c/u | 2m, 4.5m, 5.4m / 1.0–2.0M | hasta +80 % / +162 % | la de bugs llegó al tope de turnos y hubo que pedirle el informe |

Las de |desviación| > 50 % y su causa están en la tabla.

## Outside this plan

- Migrar los guiones de los kits de ASTRA.
- Texturas generadas.
- Biseles, booleanas, curvas y deformaciones.
- Usar piezas modeladas en el repo de ASTRA (lo decide el desarrollador).

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
