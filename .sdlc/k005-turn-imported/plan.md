# Plan: el montaje gira también las piezas con modelo (K005)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (2026-11-16)
- **Nothing is implemented until this file is approved.**

## Done means

- `cargo test -p kiln-cli -- --ignored turn` en verde con el arreglo y en
  rojo sin él (visto en rojo antes de tocar `assemble.py`).
- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`
  en verde (sin el test de Blender, como hoy).
- `cargo_quay` (el único `turn` de los ejemplos, 180° en la pieza-asset)
  montado antes y después: con el arreglo, el nodo lleva su rotación.
- La caseta `fisher_hut` de la prueba de K004 montada con el arreglo:
  `roof_1` y `roof_2` llevan su giro en el GLB y `measures.json` deja de
  decir que no se apoyan en nada por estar sin girar.
- README con la línea de R4. El `verifier` ha pasado.

## Stop and ask if

- El test sale en verde antes del arreglo (el diagnóstico de `e2e.md` sería
  incompleto).
- Con `rotation_mode = "XYZ"` el giro compuesto `[30, 45, 0]` de una pieza
  con modelo no coincide con el de la caja: el orden de ejes de la línea 152
  sería otro problema, que no entra aquí sin preguntar.
- Hace falta tocar algo más que `build()` de `assemble.py`.
- La rama: todo esto va en una rama nueva desde `main` actualizado, que
  creas tú (o me pides que la cree) antes de T1.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Test con Blender, `#[ignore]`: proyecto temporal con un kit de una pieza GLB (cubo de 1 m escrito por el test) y una ficha con una copia `reuse` y una caja `search`, las dos con `turn = [[0, 90, 0]]`, más una copia `reuse` con `[30, 45, 0]`; `kiln build` por el binario; lee `nodes[].rotation` del JSON del GLB y compara con la caja y con el cuaternión esperado | `crates/kiln-cli/tests/blender_turn.rs` | **Rojo** sobre el código de hoy: la copia con modelo sale sin `rotation` | R1, R2 | main | — | 6m/2M | done (cause: hubo que poner `count = 2`; rojo visto: modelos `[0,0,0,1]`, cajas giradas) |
| T2 | `o.rotation_mode = "XYZ"` antes de `o.rotation_euler` en `build()` | `crates/kiln-cli/blender/assemble.py` | T1 en verde | R1 | main | T1 | 2m/500k | done (cause: none; el giro compuesto coincide con la caja) |
| T3 | Antes y después: montar `cargo_quay` y la caseta `fisher_hut` (copiada de la prueba de K004) en copias del scratchpad; comparar rotaciones de los nodos y `measures.json` de una revisión solo de medida (sin el revisor: `render` y `measures` de Blender, sin `claude`) | — (scratchpad) | Rotaciones presentes; los faldones ya no salen «sin apoyo» por no girar | R3 | main | T2 | 6m/2M | done (cause: el `.blend` está en `.kiln/`; con la ficha de la vuelta 1, `floating` pasa de roof_1/roof_2 a 0,53 m a vacío) |
| T4 | README: aviso de rehacer los montajes con `turn` en piezas con modelo hechos antes del arreglo, y cómo correr los tests que necesitan Blender o Godot | `README.md` | Revisión | R4 | main | T2 | 2m/500k | done |
| T5 | Verificación: build, batería, Clippy, el test de Blender, y dos flujos de al lado: `kiln paint` sobre el montaje nuevo y la vista 3D de la página sirviendo el GLB | — | Informe del verifier | todos | verifier | T3, T4 | 5m/1.3M | done (verde; el test falla sin la línea; `paint` y `serve` conservan el giro; 2 nits) |

## Schedule

Todo en serie: son cambios pequeños y T2 depende de ver T1 en rojo.

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1 | 6m | ver el rojo |
| 2 | T2 | 2m | T1 en verde |
| 3 | T3, T4 | 6m | `git diff --stat` |
| 4 | T5 | 5m | leer el informe |

- **Estimated wall-clock**: unos 20 min más las comprobaciones.
- **Sequential sum**: 21 min.
- **Estimated tokens**: unos 6,3M.

## Order and dependencies

T1 antes que T2 para ver el rojo. T3 necesita el arreglo. T4 puede ir con
T3. El verifier al final.

## Test strategy

TDD: T1 en rojo primero. Si falla, se arregla el código, no el test.

- **Unit**: no hay lógica nueva en Rust.
- **Integración**: `tests/blender_turn.rs` con Blender real, `#[ignore]`.
- **Manual verification**: T3 (rotaciones y medidas de `cargo_quay` y de la
  caseta) y lo que mire el verifier.

## Verification commands

```
cargo build --workspace
cargo test --workspace
cargo test -p kiln-cli -- --ignored turn
cargo clippy --workspace --all-targets -- -D warnings
```

## Implementation risks

- El cubo del test tiene que importarse con `rotation_mode = 'QUATERNION'`
  como los modelos reales; si el importador lo deja en `XYZ` el test no
  reproduciría el fallo. Por eso T1 tiene que verse en rojo.
- Fichas con giros ajustados a ojo sobre montajes que no giraban cambiarán
  al rehacerlas (riesgo del spec): T3 lo mira con `cargo_quay`.

## Estimate vs actual

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|
| T1-T4 | main | 16m/5M | sin medir | — | las tareas de main no se pasaron a «in progress» antes de empezar |
| T5 | verifier | 5m/1.3M | 6,7m/352k | +34 % / −73 % | comprobó la variante con una ficha añadida en su copia |

## Outside this plan

- Avisar en `kiln check` de montajes viejos.
- Cómo atribuye causas el revisor.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
