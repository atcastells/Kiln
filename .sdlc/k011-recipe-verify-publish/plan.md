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
| T1 | Reglas estáticas: cota del radio (`generator_radius`, error) y suma de amplitudes sobre 0,5 (`generator_amplitude`, aviso) | `crates/kiln-core/src/generate/check.rs` | Unitarios con las recetas medidas: «estirar 0,2 y ruido 0,9» error; «ruido 0,9 y estirar 0,2» no; la del README, ninguno; rangos usan el peor extremo | R1 | implementer | — | 1m/200k | done |
| T2 | Lector de mallas de un GLB: posiciones e índices de cada primitiva de triángulos, con cada desplazamiento comprobado y tope de vértices | `crates/kiln-core/src/mesh.rs` (nuevo), `crates/kiln-core/src/lib.rs` | Un tetraedro hecho a mano con y sin índices, `u16` y `u32`; GLB truncado, índice fuera de rango y `bufferView` fuera del fichero dan error | R2 | implementer | — | 2m/300k | done |
| T3 | Medir una malla: triángulos, sin área, caras vueltas y fracción empinada con el estirado deshecho | `crates/kiln-core/src/generate/measure.rs` (nuevo), `crates/kiln-core/src/generate/mod.rs` | Tetraedro limpio; con una cara vuelta; con un triángulo sin área; un elipsoide estirado da 0 empinadas al deshacer el estirado | R2 | implementer | T2 | 1m/200k | done |
| T4 | `[generate] verify_seeds` (16, de 1 a 64) y `steep_max` (0,10) en `kiln.toml` | `crates/kiln-core/src/config.rs` | Por defecto, leídos y fuera de rango rechazados | R3, R4 | implementer | — | 1m/200k | done |
| T5 | La tanda de semillas: la de la ficha y las derivadas con `mix` | `crates/kiln-core/src/generate/resolve.rs` | Valores fijados, calculados aparte; `n` semillas distintas; la primera es la de la ficha | R3 | implementer | — | 1m/200k | done |
| T6 | seam: el guion y su lanzador van juntos. Partir el guion: `kiln_generator.gd` construye la malla, `generate.gd` la pide, exporta y mide; el lanzador escribe los dos | `crates/kiln-cli/godot/kiln_generator.gd` (nuevo), `crates/kiln-cli/godot/generate.gd`, `crates/kiln-cli/src/generate.rs`, `crates/kiln-cli/tests/generate_vocab.rs` | El test del vocabulario apunta al fichero nuevo; `main` corre después las pruebas con Godot y compara el sha256 con el de antes | R8 | implementer | — | 2m/300k | done |
| T7 | Hash de la receta: `[generator]` y `budget_tris` en JSON con claves ordenadas | `crates/kiln-core/src/generate/hash.rs` (nuevo), `crates/kiln-core/src/generate/mod.rs` | Mismo hash con las claves en otro orden; cambia con un valor, con la semilla y con el presupuesto | R6 | implementer | T3 | 1m/200k | done |
| T8 | Generar en la carpeta que se pida: la tanda va a `<cache>/generated/<id>/verify/` con su carpeta de trabajo, sin tocar la vista previa | `crates/kiln-cli/src/generate.rs` | Con el Godot de pega: generar para verificar no borra ni pisa el GLB ni el `generated.json` de la vista previa | R3 | implementer | T6 | 1m/200k | done |
| T9 | Juzgar una semilla: de sus medidas y lo esperado salen `generated_flipped`, `generated_degenerate`, `generated_tris` (error) y `generated_steep` (aviso) | `crates/kiln-core/src/generate/measure.rs` | Un unitario por código; el presupuesto usa `budget_tris` si lo hay y `rules.max_tris` siempre | R4 | implementer | T3 | 1m/200k | done |
| T10 | Verificar una receta: reglas estáticas, tanda, medir y juzgar; `verify_asset` deriva aquí cuando la ficha lleva receta | `crates/kiln-cli/src/verify_recipe.rs` (nuevo), `crates/kiln-cli/src/main.rs` (solo `mod`), `crates/kiln-cli/src/verify.rs` (la bifurcación) | Con el Godot de pega: genera las 16 semillas; con error estático no lanza Godot; una malla de pega con una cara vuelta falla | R3, R4 | implementer | T1, T4, T5, T8, T9 | 2m/300k | done (report and the branch in verify.rs moved to T11) |
| T11 | El informe: `validation.json` con la forma de hoy más el bloque `recipe` (hash de la receta, versión y hash del guion, y por semilla valores, triángulos, medidas y empinadas; peor caso) | `crates/kiln-cli/src/verify_recipe.rs`, `crates/kiln-core/src/validate.rs` (campo opcional) | El informe de un asset de piezas se escribe igual que antes; el de una receta trae el bloque y `valid` | R5 | implementer | T7, T10 | 2m/300k | done |
| T12 | Comprobaciones de fuera: las propias del GLB en todas las semillas; Khronos e importación en Godot solo en la de la ficha | `crates/kiln-cli/src/verify_recipe.rs`, `crates/kiln-cli/src/verify.rs` (visibilidad de `khronos` y `godot_import`) | Con el Godot de pega: `checks` trae las tres; Khronos ausente no cuenta como fallo | R3 | implementer | T11 | 1m/200k | done |
| T13 | Publicar, la puerta: sin verificar, verificación fallida, o receta, versión o guion distintos de los verificados, se niega sin escribir | `crates/kiln-cli/src/ops/publish_recipe.rs` (nuevo), `crates/kiln-cli/src/ops/mod.rs` (solo `mod`) | Un test por cada negativa, con el mensaje que dice qué hacer | R6 | implementer | T11 | 2m/300k | done |
| T14 | Publicar, lo escrito: `recipe.json`, `kiln_generator.gd` y `asset.json` con versión; `publish_asset` deriva aquí cuando la ficha lleva receta | `crates/kiln-cli/src/ops/publish_recipe.rs`, `crates/kiln-cli/src/ops/publish.rs` (la bifurcación y la visibilidad de la carpeta de destino) | Los tres ficheros y ningún `.glb`; repetir no sube la versión; cambiar la receta, verificar y publicar, sí; un enlace en el destino no se sigue | R7 | implementer | T13 | 2m/300k | done |
| T15 | Los rechazos pasan de cinco a tres: `build`, `paint` y `review`; mensaje «es un asset generado: no se monta, se genera y se verifica» | `crates/kiln-cli/src/ops/mod.rs` | `the_five_operations_refuse_…` pasa a llamarse por tres y comprueba el mensaje nuevo; verificar y publicar ya no devuelven el rechazo | R9 | implementer | T10, T14 | 1m/200k | done (cause: three-line change, made in main) |
| T16 | Por MCP: `verify` y `publish` sobre un asset generado | `crates/kiln-cli/tests/mcp_production.rs` | Con un Godot que no existe: el trabajo de `verify` falla nombrando la herramienta; `publish` sin verificar se niega | R9 | implementer | T15 | 1m/200k | done |
| T17 | Pruebas con Godot de verdad: verificar 16 semillas dos veces da el mismo informe; publicar; la receta rota no se publica | `crates/kiln-cli/tests/godot_recipe.rs` (nuevo) | `--ignored`, las corre `main` | R3, R6, R7 | implementer | T15 | 1m/200k | done |
| T18 | Batería entera, `--ignored` y `clippy` en `main`; el resultado pasa al verificador | — | Todo verde; el verificador mira verificar y publicar una receta, montar-verificar-publicar un asset de piezas (`hoist`) y Aplicar en la vista previa | todos | main + verifier | T16, T17 | 8m/5M | done |
| T19 | README (entrega nueva, formato de lo publicado y cómo lo llama un juego) | `README.md` | Las cifras salen de T18 | — | main | T18 | 3m/2M | done |
| T20 | Recorrido real anotado: verificar, publicar, rechazo, tiempos y tamaños | `.sdlc/k011-recipe-verify-publish/e2e.md` | Con Godot de verdad sobre una copia de `examples` | Señal | main | T18 | 4m/3M | done |
| T21 | Orquestación: encargos, lectura de cada diff, `context.md`, sha256 de antes y después de T6 | `.sdlc/k011-recipe-verify-publish/context.md` | — | — | main | — | 20m/8M | done (cause: five review findings fixed in main, with tests) |

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

Tres pasadas en paralelo sobre el árbol sin commit (2026-11-18). El repo no
tiene `REVIEW.md` en esta rama; cada pasada usó los criterios de su encargo.

- **bugs**: 1 importante, corregido con test rojo primero. La versión se
  decidía comparando el `recipe.json` del disco: si fallaba la escritura del
  contrato, la siguiente publicación no subía la versión de algo que ya
  había cambiado. Ahora se decide contra el contrato anterior. La revisión
  confirmó con Godot que la cota del radio no se queda corta (cuatro recetas
  límite, 64 semillas cada una, 0 caras vueltas) y que el lector de mallas
  no entra en pánico con 200 000 ficheros mutados.
- **security**: 1 importante, corregido con test. Un enlace dejado en
  `<cache>/generated/<id>/verify` hacía que verificar borrase la carpeta del
  proyecto a la que apuntaba.
- **conformance**: 3 importantes, corregidos. Faltaba el test de que la
  receta del README no da ningún hallazgo; la prueba de que partir el guion
  no cambia las mallas no estaba escrita en ningún sitio (ahora en
  `e2e.md`, con los sha256); y `ops/publish.rs` cambió más de lo que este
  plan permitía sin preguntar: se extrajo la comprobación de la carpeta de
  destino y, de paso, cambió el orden de dos errores para los assets de
  piezas. El orden se ha devuelto al de antes; la extracción se queda.

Queda sin hacer, anotado como menor: un `validation.json` escrito a mano con
los hashes correctos pasa la puerta (como en los assets de piezas, y no da
más de lo que ya da editar la ficha); la puerta no mira `verify_seeds`,
`steep_max` ni `max_tris`; si Godot no arranca a media tanda queda en disco
el informe anterior; dos verificaciones a la vez del mismo asset comparten
carpeta; `[tools] build` no está confinado al proyecto, igual que antes de
esta entrega; al pasar una ficha de receta a piezas no se borran
`recipe.json` ni el guion publicados.

### Cierre de riesgos antes del merge (2026-11-18)

Resolución del product owner: conservar la extracción de `ops/publish.rs`,
integrar K009 primero y cerrar validación obsoleta y concurrencia.

- **K009** ya estaba en `origin/main` (PR #26 y #27) antes de crear esta
  rama, que sale de ese punto: K011 se construyó y se probó encima de K009.
  Las notas de este plan y de los informes que decían «K009 sigue sin unir»
  eran un error de `main`, que no volvió a mirarlo tras unirse K010.
- **Validación obsoleta**, hecha en `main` con test rojo primero: el informe
  anterior se borra al empezar a verificar; el informe guarda `verify_seeds`
  y `max_tris` y la puerta los compara.
- **Concurrencia**, hecha en `main` con test rojo primero: cerrojo por asset
  para generar, verificar y publicar.
- Comprobado con Godot de verdad: con una verificación en marcha, otra
  verificación, publicar y la vista previa se niegan; al terminar la
  primera, publicar funciona y el cerrojo no queda en disco.
- Sigue sin hacer, menor: un `validation.json` escrito a mano con los datos
  correctos pasa la puerta; `[tools] build` no está confinado al proyecto;
  al pasar una ficha de receta a piezas no se borra lo publicado antes. El
  camino de los assets de piezas conserva su informe anterior si una
  herramienta corta la verificación: no se ha tocado.

## Estimate vs actual

De `actuals.jsonl`. Como en K010, los encargos salen en torno a la mitad del
tiempo estimado.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|
| T1 | implementer | 1m/200k | 0,5m/165k | −50 % | ninguna |
| T2 | implementer | 2m/300k | 1,0m/206k | −50 % | 194 líneas de código en vez de 100: validar cada desplazamiento ocupa |
| T3 | implementer | 1m/200k | 0,9m/178k | −10 % | ninguna |
| T4 | implementer | 1m/200k | 0,4m/176k | −60 % | ninguna |
| T5 | implementer | 1m/200k | 0,3m/53k | −70 % | ninguna; los valores fijados salieron a la primera |
| T6 | implementer | 2m/300k | 0,9m/136k | −55 % | ninguna |
| T7 | implementer | 1m/200k | 1,0m/158k | 0 % | ninguna |
| T8 | implementer | 1m/200k | 0,9m/289k | −10 % | ninguna |
| T9 | implementer | 1m/200k | 0,5m/93k | −50 % | ninguna |
| T10 | implementer | 2m/300k | 1,1m/342k | −45 % | ninguna |
| T11 | implementer | 2m/300k | 0,9m/285k | −55 % | ninguna |
| T12 | implementer | 1m/200k | 0,9m/251k | −10 % | ninguna |
| T13 | implementer | 2m/300k | 0,7m/213k | −65 % | ninguna |
| T14 | implementer | 2m/300k | 1,5m/457k | −25 % | extrajo dos funciones de `publish.rs`, más de lo que el plan permitía |
| T16 | implementer | 1m/200k | 1,4m/511k | +40 % min, +156 % tok | tuvo que leer el fichero de tests entero y escribir la ficha en disco |
| T17 | implementer | 1m/200k | 0,7m/204k | −30 % | ninguna; un test salió mal por un error del encargo, no suyo |

No medido por el gancho: T15, que hizo `main`; el verificador; las tres
revisiones; y el trabajo de `main` (T18 a T21), con las correcciones.

## Outside this plan

Lo que la spec deja fuera: el piloto en Godot y regenerar lo publicado en un
Godot limpio, botones en la página, ampliar el vocabulario, que ASTRA lea lo
publicado, publicar GLB y limpiar los GLB de la vista previa.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
