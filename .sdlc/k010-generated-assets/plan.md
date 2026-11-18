# Plan: assets generados en el juego a partir de una receta (K010)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-18)
- **Nothing is implemented until this file is approved.**

## Done means

Con `kiln serve` sobre un proyecto que tiene una ficha con `[generator]`: la
página muestra sus controles, Aplicar guarda la receta, Godot sin ventana
deja `<cache>/generated/<id>/<semilla>.glb` y la vista 3D lo carga; la misma
receta y semilla dan el mismo hash estructural. `cargo test --workspace`, los
`--ignored`, `clippy` y `scripts/ui_test.sh` pasan. `e2e.md` anota la petición
«crear un asteroide» con el agente real.

## Stop and ask if

- Un test que ya existía falla por algo ajeno a este cambio.
- Hace falta tocar `verify.rs` más allá de hacer visible `scratch_dir` y del
  rechazo de T6.
- El generador necesita una operación o un parámetro fuera del vocabulario
  de la spec.
- El guion de Godot no da el mismo hash estructural en dos ejecuciones.
- Antes de T20: lanza al agente y gasta de la suscripción.
- The task needs to open windows or apps on the developer's screen.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Tipos de la receta (`Generator`, `Op`, valor fijo o rango) y `Spec.generator`; `no_parts` no salta con receta; `generator_and_parts` | `crates/kiln-core/src/generate/mod.rs` (nuevo, declara `vocab`, `check` y `resolve` vacíos), `crates/kiln-core/src/lib.rs`, `crates/kiln-core/src/spec.rs` | Unitarios: la ficha de la spec se lee y se escribe igual; con piezas y receta da `generator_and_parts` | R1 | implementer | — | 2m/300k | pending |
| T2 | Tabla del vocabulario: operaciones, parámetros, tipo, límites, valor por defecto y cálculo de triángulos | `crates/kiln-core/src/generate/vocab.rs` | Unitarios: las cuatro operaciones de la spec con sus límites; 64 segmentos dan 4 224 triángulos | R2 | implementer | T1 | 1m/200k | pending |
| T3 | Comprobar la receta: `generator_op`, `generator_param`, `generator_range`, `generator_budget`, `sphere` primera, material en la paleta, semilla de 32 bits | `crates/kiln-core/src/generate/check.rs`, llamada en `spec.rs` | Un unitario por código de hallazgo | R2, R3 | implementer | T2 | 2m/300k | pending |
| T4 | `resolve(receta, semilla)` con el algoritmo de la spec | `crates/kiln-core/src/generate/resolve.rs` | Vectores fijos; un valor fijo no cambia; otra semilla cambia algún valor; enteros hacia abajo | R4 | implementer | T2 | 1m/200k | pending |
| T5 | Guion `generate.gd`: esfera, `stretch`, `noise`, `facet`; nombra la ranura con la clave del material; exporta GLB y escribe medidas | `crates/kiln-cli/godot/generate.gd` (nuevo), `crates/kiln-cli/tests/generate_vocab.rs` (nuevo) | Cada operación y parámetro de la tabla aparece en el texto del guion (suite normal) | R2, R5 | implementer | T2 | 2m/300k | pending |
| T6 | Rechazar un asset generado donde entran la página, MCP y la línea de órdenes: una función común y su llamada al principio de `build_asset`, `paint_asset`, `verify_asset`, `publish_asset` y `review::run` | `crates/kiln-cli/src/ops/mod.rs`, `crates/kiln-cli/src/production.rs`, `crates/kiln-cli/src/verify.rs`, `crates/kiln-cli/src/ops/publish.rs`, `crates/kiln-cli/src/review.rs` | Cada una devuelve el mensaje con `[tools]` apuntando a un programa que no existe | R9 | implementer | T1 | 2m/300k | pending |
| T7 | El vocabulario viaja en el catálogo que lee la página | `crates/kiln-cli/src/ops/read.rs` | El JSON del catálogo trae operaciones, parámetros y límites | R6 | implementer | T2 | 1m/200k | pending |
| T8 | La skill decide entre montar y generar; el vocabulario de la tabla se añade al mensaje del agente de la página y al prompt MCP `kiln-asset` | `skills/asset/SKILL.md`, `crates/kiln-cli/src/agent.rs`, `crates/kiln-cli/src/mcp/prompts.rs`, `crates/kiln-cli/tests/mcp_prompts.rs` | Los dos textos contienen cada operación de la tabla | R8 | implementer | T2 | 2m/300k | pending |
| T9 | `spec_write` acepta una ficha con receta y rechaza una receta inválida | `crates/kiln-cli/src/ops/specs.rs`, `crates/kiln-cli/tests/mcp_specs.rs` | Una receta válida se escribe; una con una operación desconocida se rechaza y no toca el fichero | R1, R8 | implementer | T3 | 1m/200k | pending |
| T10 | seam: el lanzador usa el proyecto vacío de `verify.rs`. Resolver, escribir el encargo y lanzar el guion en la caché | `crates/kiln-cli/src/generate.rs` (nuevo), `crates/kiln-cli/src/main.rs` (solo `mod`), `crates/kiln-cli/src/verify.rs` (`scratch_dir` visible) | Con un Godot de pega: el encargo lleva los valores resueltos y la salida queda bajo `<cache>/generated/<id>/` | R5 | implementer | T4, T5, T6 | 2m/300k | pending |
| T11 | `generated.json` (receta, semilla, valores, medidas, triángulos, hash estructural, versiones) y la prueba con Godot de verdad | `crates/kiln-cli/src/generate.rs`, `crates/kiln-cli/tests/godot_generate.rs` (nuevo) | `--ignored`: misma semilla, mismo hash estructural; otra semilla, otro; triángulos igual al cálculo estático | R5 | implementer | T10 | 2m/300k | pending |
| T12 | Operación `generate(asset, seed, receta?)` como trabajo: valida, escribe la ficha con `spec_write`, genera | `crates/kiln-cli/src/ops/production.rs` | Una receta inválida no escribe la ficha ni lanza Godot; una válida deja el GLB con el Godot de pega | R5, R6 | implementer | T9, T11 | 2m/300k | pending |
| T13 | Orden `kiln generate --asset ID [--seed N]` | `crates/kiln-cli/src/main.rs` | La orden sale en la ayuda y, sin Godot, falla con el mensaje de la herramienta | R5 | implementer | T12 | 1m/200k | pending |
| T14 | Herramienta MCP `generate` | `crates/kiln-cli/src/mcp/production.rs`, `crates/kiln-cli/tests/mcp_production.rs` | La herramienta arranca el trabajo y devuelve su número | R7 | implementer | T12 | 1m/200k | pending |
| T15 | Ruta `POST /api/generate` con la cabecera de la página y su fila en la tabla de cobertura | `crates/kiln-cli/src/serve.rs`, `crates/kiln-cli/tests/coverage.rs`, `crates/kiln-cli/tests/routes.rs` | `coverage.rs` pasa; sin la cabecera la ruta se niega | R7 | implementer | T14 | 1m/200k | pending |
| T16 | Controles de la receta: uno por parámetro, doble si es rango, y la semilla | `crates/kiln-cli/ui/app/views/generator.js` (nuevo), su test en `crates/kiln-cli/ui/tests/unit/` | Unitario de node: de receta y vocabulario salen los controles con sus límites, y de los controles la receta | R6 | implementer | T7 | 2m/300k | pending |
| T17 | Aplicar en la vista de la ficha: envía, espera el trabajo y carga el GLB en la vista 3D; ficha generada y Godot de pega en el proyecto de prueba de la página | `crates/kiln-cli/ui/app/views/spec.js`, `crates/kiln-cli/ui/app/api.js`, alta de `generator.js` en `serve.rs`, `scripts/ui_fixture.sh`, `crates/kiln-cli/ui/tests/app_views2.mjs` | `scripts/ui_test.sh`: la ficha muestra los controles, Aplicar llama a la ruta y la vista pide el GLB devuelto | R6 | implementer | T15, T16 | 3m/400k | pending |
| T18 | Batería entera, `--ignored`, `clippy` y `ui_test.sh` en `main`; el resultado pasa al verificador | — | Todo verde; el verificador mira Aplicar con Godot de verdad, montar un asset de piezas y el catálogo | todos | main + verifier | T13, T17 | 8m/2.8M | pending |
| T19 | README (entrega nueva) y cuentas de tests de `CLAUDE.md` | `README.md`, `CLAUDE.md` | Las cifras salen de la ejecución de T18 | — | main | T18 | 3m/1.7M | pending |
| T20 | Prueba de punta a punta con el agente real: «crear un asteroide» | `.sdlc/k010-generated-assets/e2e.md` | La ficha escrita lleva receta y pasa `kiln check`; Aplicar enseña la malla | R8 | main | T18 | 5m/1.7M | pending |
| T21 | Orquestación: encargos, lectura de cada diff y `context.md` | `.sdlc/k010-generated-assets/context.md` | — | — | main | — | 20m/6M | pending |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1 | 2m | leer el diff, `context.md` |
| 2 | T2, T6 | 2m | leer los diffs |
| 3 | T3, T4, T5, T7, T8 | 2m | `git diff --stat`, tests de `kiln-core` |
| 4 | T9, T10, T16 | 2m | leer los diffs |
| 5 | T11 | 2m | correr sus `--ignored` |
| 6 | T12 | 2m | leer el diff |
| 7 | T13, T14 | 1m | leer los diffs |
| 8 | T15 | 1m | `coverage.rs` |
| 9 | T17 | 3m | `scripts/ui_test.sh` |
| 10 | T18 | 8m | T19, luego T20 con permiso |

- **Estimated wall-clock**: alrededor de una hora (25 min de tandas, más la
  lectura de diffs de `main`, T19 y T20)
- **Sequential sum**: 65 min
- **Estimated tokens**: unos 17 M

## Order and dependencies

T1 fija los tipos y T2 la tabla de la que cuelga todo lo demás. T3 y T4
escriben ficheros distintos del mismo módulo. T6 y T10 tocan `verify.rs`, y
T10 y T13 `main.rs`, por eso van en tandas distintas. La herramienta MCP
(T14) va antes que la ruta (T15) porque `coverage.rs` exige que una ruta
nueva tenga ya su herramienta. T17 es la única que une página y servidor.

## Test strategy

TDD: test rojo antes de implementar. Si un test falla, se arregla el código.

- **Unit**: lectura de la receta, hallazgos, sorteo, mensaje del agente,
  controles de la página.
- **Integration**: `coverage.rs`, MCP, y con Godot (`--ignored`) el
  generador y su determinismo.
- **Manual verification**: el verificador comprueba Aplicar de punta a punta
  con Godot de verdad y dos flujos vecinos: montar un asset de piezas
  (`hoist`) y leer el catálogo. Antes de diagnosticar un rojo con Godot se
  mide la línea base sin el cambio.

## Verification commands

```
cargo build --release
cargo test --workspace
cargo test --workspace -- --ignored
cargo clippy --workspace --all-targets -- -D warnings
scripts/ui_test.sh
```

## Implementation risks

- `Spec` gana un campo: todo lo que construye un `Spec` a mano en los tests
  deja de compilar si no lleva `Default` o `serde(default)`.
- Quien recorre `spec.parts` (grafo, árbol, montaje) recibe una lista vacía
  con un asset generado; T6 cubre las operaciones, la página se mira en T17.
- La línea de órdenes llama a `build_asset`, `verify_asset`, `publish_asset`,
  `paint_asset` y `review::run` sin pasar por la capa de operaciones
  (`main.rs:167-208`): el rechazo va dentro de esas funciones.
- La prueba de la página no lanza Godot: usa uno de pega que copia un GLB ya
  hecho. Godot de verdad lo cubren T11 y el verificador.
- La vista 3D ya pide cada fichero con una marca de tiempo (`view3d.js:10`),
  así que un GLB regenerado con la misma semilla no sale de la caché del
  navegador. Sin comprobar: si cada Aplicar obliga a releer el catálogo.
- Que la vista pinte la malla con el color de su material se comprueba en
  T17; si no lo hace, queda anotado y no bloquea.
- `scripts/ui_test.sh` monta su proyecto con `scripts/ui_fixture.sh`: T17 le
  añade una ficha generada y puede mover la línea base de medidas.
- La rama sale de `main`, sin K009; `build.rs` y `verify.rs` pueden chocar al
  unir las dos.

## Review

Filled in Deploy. A plan is not closed while this section is empty.

## Estimate vs actual

Filled at Build close.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

Lo que la spec deja fuera: publicar la receta, la envolvente para ASTRA, la
vista en vivo, el barrido de semillas, cráteres y más materiales, colisión.
Tampoco se añade un asteroide a `examples/`, que es un puerto medieval.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
