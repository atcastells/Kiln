# Plan: verificar y publicar un asset generado por receta (K011)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-18)
- **Nothing is implemented until this file is approved.**

## Done means

Sobre un proyecto con un asset generado: `kiln verify --asset ID` genera la
tanda de 16 semillas, las mide y escribe `validation.json` con el peor caso;
`kiln publish --asset ID` deja `recipe.json`, `kiln_generator.gd` y
`asset.json` con versión en `[publish] dir/<id>/`, sin GLB; una receta cuya
cota del radio llega a cero no pasa `check` ni se puede publicar. Lo mismo
por las herramientas MCP `verify` y `publish`. `cargo test --workspace`, los
`--ignored` y `clippy` pasan. `e2e.md` anota el recorrido con Godot de
verdad.

## Stop and ask if

- Un test que ya existía falla por algo ajeno a este cambio.
- Partir el guion cambia el sha256 de un GLB respecto al de antes de T6.
- Hace falta cambiar la forma de `validation.json` que ya leen `publish` o
  `spec_get` para los assets de piezas.
- Una receta con cota del radio positiva sale con caras vueltas.
- Hay que tocar `verify.rs` o `ops/publish.rs` más que la bifurcación y la
  visibilidad de sus funciones (K009 toca esos ficheros y sigue sin unir).
- The task needs to open windows or apps on the developer's screen.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Reglas estáticas: cota del radio (`generator_radius`, error) y suma de amplitudes sobre 0,5 (`generator_amplitude`, aviso) | `crates/kiln-core/src/generate/check.rs` | Unitarios con las recetas medidas: «estirar 0,2 y ruido 0,9» error; «ruido 0,9 y estirar 0,2» no; la del README, ninguno; rangos usan el peor extremo | R1 | implementer | — | 1m/200k | pending |
| T2 | Lector de mallas de un GLB: posiciones e índices de cada primitiva de triángulos, con cada desplazamiento comprobado y tope de vértices | `crates/kiln-core/src/mesh.rs` (nuevo), `crates/kiln-core/src/lib.rs` | Un tetraedro hecho a mano con y sin índices, `u16` y `u32`; GLB truncado, índice fuera de rango y `bufferView` fuera del fichero dan error | R2 | implementer | — | 2m/300k | pending |
| T3 | Medir una malla: triángulos, sin área, caras vueltas y fracción empinada con el estirado deshecho | `crates/kiln-core/src/generate/measure.rs` (nuevo), `crates/kiln-core/src/generate/mod.rs` | Tetraedro limpio; con una cara vuelta; con un triángulo sin área; un elipsoide estirado da 0 empinadas al deshacer el estirado | R2 | implementer | T2 | 1m/200k | pending |
| T4 | `[generate] verify_seeds` (16, de 1 a 64) y `steep_max` (0,10) en `kiln.toml` | `crates/kiln-core/src/config.rs` | Por defecto, leídos y fuera de rango rechazados | R3, R4 | implementer | — | 1m/200k | pending |
| T5 | La tanda de semillas: la de la ficha y las derivadas con `mix` | `crates/kiln-core/src/generate/resolve.rs` | Valores fijados, calculados aparte; `n` semillas distintas; la primera es la de la ficha | R3 | implementer | — | 1m/200k | pending |
| T6 | seam: el guion y su lanzador van juntos. Partir el guion: `kiln_generator.gd` construye la malla, `generate.gd` la pide, exporta y mide; el lanzador escribe los dos | `crates/kiln-cli/godot/kiln_generator.gd` (nuevo), `crates/kiln-cli/godot/generate.gd`, `crates/kiln-cli/src/generate.rs`, `crates/kiln-cli/tests/generate_vocab.rs` | El test del vocabulario apunta al fichero nuevo; `main` corre después las pruebas con Godot y compara el sha256 con el de antes | R8 | implementer | — | 2m/300k | pending |
| T7 | Hash de la receta: `[generator]` y `budget_tris` en JSON con claves ordenadas | `crates/kiln-core/src/generate/hash.rs` (nuevo), `crates/kiln-core/src/generate/mod.rs` | Mismo hash con las claves en otro orden; cambia con un valor, con la semilla y con el presupuesto | R6 | implementer | T3 | 1m/200k | pending |
| T8 | Generar en la carpeta que se pida: la tanda va a `<cache>/generated/<id>/verify/` con su carpeta de trabajo, sin tocar la vista previa | `crates/kiln-cli/src/generate.rs` | Con el Godot de pega: generar para verificar no borra ni pisa el GLB ni el `generated.json` de la vista previa | R3 | implementer | T6 | 1m/200k | pending |
| T9 | Juzgar una semilla: de sus medidas y lo esperado salen `generated_flipped`, `generated_degenerate`, `generated_tris` (error) y `generated_steep` (aviso) | `crates/kiln-core/src/generate/measure.rs` | Un unitario por código; el presupuesto usa `budget_tris` si lo hay y `rules.max_tris` siempre | R4 | implementer | T3 | 1m/200k | pending |
| T10 | Verificar una receta: reglas estáticas, tanda, medir y juzgar; `verify_asset` deriva aquí cuando la ficha lleva receta | `crates/kiln-cli/src/verify_recipe.rs` (nuevo), `crates/kiln-cli/src/main.rs` (solo `mod`), `crates/kiln-cli/src/verify.rs` (la bifurcación) | Con el Godot de pega: genera las 16 semillas; con error estático no lanza Godot; una malla de pega con una cara vuelta falla | R3, R4 | implementer | T1, T4, T5, T8, T9 | 2m/300k | pending |
| T11 | El informe: `validation.json` con la forma de hoy más el bloque `recipe` (hash de la receta, versión y hash del guion, y por semilla valores, triángulos, medidas y empinadas; peor caso) | `crates/kiln-cli/src/verify_recipe.rs`, `crates/kiln-core/src/validate.rs` (campo opcional) | El informe de un asset de piezas se escribe igual que antes; el de una receta trae el bloque y `valid` | R5 | implementer | T7, T10 | 2m/300k | pending |
| T12 | Comprobaciones de fuera: las propias del GLB en todas las semillas; Khronos e importación en Godot solo en la de la ficha | `crates/kiln-cli/src/verify_recipe.rs`, `crates/kiln-cli/src/verify.rs` (visibilidad de `khronos` y `godot_import`) | Con el Godot de pega: `checks` trae las tres; Khronos ausente no cuenta como fallo | R3 | implementer | T11 | 1m/200k | pending |
| T13 | Publicar, la puerta: sin verificar, verificación fallida, o receta, versión o guion distintos de los verificados, se niega sin escribir | `crates/kiln-cli/src/ops/publish_recipe.rs` (nuevo), `crates/kiln-cli/src/ops/mod.rs` (solo `mod`) | Un test por cada negativa, con el mensaje que dice qué hacer | R6 | implementer | T11 | 2m/300k | pending |
| T14 | Publicar, lo escrito: `recipe.json`, `kiln_generator.gd` y `asset.json` con versión; `publish_asset` deriva aquí cuando la ficha lleva receta | `crates/kiln-cli/src/ops/publish_recipe.rs`, `crates/kiln-cli/src/ops/publish.rs` (la bifurcación y la visibilidad de la carpeta de destino) | Los tres ficheros y ningún `.glb`; repetir no sube la versión; cambiar la receta, verificar y publicar, sí; un enlace en el destino no se sigue | R7 | implementer | T13 | 2m/300k | pending |
| T15 | Los rechazos pasan de cinco a tres: `build`, `paint` y `review`; mensaje «es un asset generado: no se monta, se genera y se verifica» | `crates/kiln-cli/src/ops/mod.rs` | `the_five_operations_refuse_…` pasa a llamarse por tres y comprueba el mensaje nuevo; verificar y publicar ya no devuelven el rechazo | R9 | implementer | T10, T14 | 1m/200k | pending |
| T16 | Por MCP: `verify` y `publish` sobre un asset generado | `crates/kiln-cli/tests/mcp_production.rs` | Con un Godot que no existe: el trabajo de `verify` falla nombrando la herramienta; `publish` sin verificar se niega | R9 | implementer | T15 | 1m/200k | pending |
| T17 | Pruebas con Godot de verdad: verificar 16 semillas dos veces da el mismo informe; publicar; la receta rota no se publica | `crates/kiln-cli/tests/godot_recipe.rs` (nuevo) | `--ignored`, las corre `main` | R3, R6, R7 | implementer | T15 | 1m/200k | pending |
| T18 | Batería entera, `--ignored` y `clippy` en `main`; el resultado pasa al verificador | — | Todo verde; el verificador mira verificar y publicar una receta, montar-verificar-publicar un asset de piezas (`hoist`) y Aplicar en la vista previa | todos | main + verifier | T16, T17 | 8m/5M | pending |
| T19 | README (entrega nueva, formato de lo publicado y cómo lo llama un juego) | `README.md` | Las cifras salen de T18 | — | main | T18 | 3m/2M | pending |
| T20 | Recorrido real anotado: verificar, publicar, rechazo, tiempos y tamaños | `.sdlc/k011-recipe-verify-publish/e2e.md` | Con Godot de verdad sobre una copia de `examples` | Señal | main | T18 | 4m/3M | pending |
| T21 | Orquestación: encargos, lectura de cada diff, `context.md`, sha256 de antes y después de T6 | `.sdlc/k011-recipe-verify-publish/context.md` | — | — | main | — | 20m/8M | pending |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 0 | — | — | sha256 de los GLB de referencia con el guion de ahora |
| 1 | T1, T2, T4, T5, T6 | 2m | pruebas con Godot de K010 y comparar el sha256 (T6) |
| 2 | T3, T8 | 1m | leer los diffs |
| 3 | T7, T9 | 1m | tests de `kiln-core` |
| 4 | T10 | 2m | leer el diff |
| 5 | T11 | 2m | leer el diff |
| 6 | T12, T13 | 2m | leer los diffs |
| 7 | T14 | 2m | leer el diff |
| 8 | T15 | 1m | — |
| 9 | T16, T17 | 1m | correr `godot_recipe` con Godot |
| 10 | T18 | 8m | T19, T20 |

- **Estimated wall-clock**: alrededor de una hora (22 min de tandas, más la
  lectura de diffs, T19 y T20)
- **Sequential sum**: 59 min
- **Estimated tokens**: unos 22 M (4 M de encargos, el resto `main`, que en
  este repo gasta el doble de lo que dice la tabla de partida)

## Order and dependencies

Lo que no depende de nada va primero y en paralelo (T1, T2, T4, T5, T6). T7
y T9 esperan a T3 porque los tres declaran su módulo en `generate/mod.rs` o
escriben en `measure.rs`. De T10 a T12 escriben el mismo fichero nuevo y van
en fila; lo mismo T13 y T14. T15 va cuando verificar y publicar ya derivan,
para que el test de los rechazos cambie con el comportamiento y no antes.

## Test strategy

TDD: test rojo antes de implementar. Si un test falla, se arregla el código.

- **Unit**: reglas estáticas, lector de mallas, medidas, juicio por semilla,
  tanda de semillas, hash de la receta, puerta de publicar.
- **Integration**: verificar y publicar con un Godot de pega que copia una
  malla hecha a mano; MCP; con Godot de verdad (`--ignored`), la tanda
  entera y su repetición.
- **Manual verification**: el verificador recorre verificar y publicar con
  Godot de verdad y dos flujos vecinos: el asset de piezas `hoist` de punta
  a punta y Aplicar en la vista previa. Antes de diagnosticar un rojo con
  Godot se mide la línea base sin el cambio.

## Verification commands

```
cargo build --release
cargo test --workspace
cargo test --workspace -- --ignored
cargo clippy --workspace --all-targets -- -D warnings
```

`scripts/ui_test.sh` no entra: esta entrega no toca la página, y esa prueba
ya termina en 1 por una comprobación ajena.

## Implementation risks

- K009 sigue sin unir y toca `verify.rs`: aquí solo entran la bifurcación y
  la visibilidad; lo nuevo va en ficheros nuevos.
- El Godot de pega de K010 copia un GLB mínimo sin índices ni triángulos de
  verdad: T10 necesita uno con malla que el lector pueda medir.
- `Report` gana un campo opcional: los assets de piezas deben serializar
  igual que antes (T11 lo comprueba).
- El test de los cinco rechazos cambia a propósito (R9); cualquier otro test
  existente que cambie es motivo para parar.
- Un comando de consola con la palabra «production» lo bloquea un gancho:
  esos ficheros se leen y editan con las herramientas, no desde la consola.

## Review

Filled in Deploy. A plan is not closed while this section is empty.

## Estimate vs actual

Filled at Build close.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

Lo que la spec deja fuera: el piloto en Godot y regenerar lo publicado en un
Godot limpio, botones en la página, ampliar el vocabulario, que ASTRA lea lo
publicado, publicar GLB y limpiar los GLB de la vista previa.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
