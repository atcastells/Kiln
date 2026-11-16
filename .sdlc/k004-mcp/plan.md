# Plan: Kiln como servidor MCP (K004)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (2026-11-16)
- **Nothing is implemented until this file is approved.**

## Done means

- `cargo test --workspace` y `cargo clippy --workspace --all-targets` en
  verde. La línea base de hoy es 156 tests en verde y 1 ignorado.
- Las rutas de la página responden igual que antes del cambio (T1, en
  verde antes y después de T10).
- `kiln mcp` da todas las herramientas y prompts del spec, con sus
  anotaciones (T19), y el test de cobertura de rutas está en verde (T20).
- El `verifier` ha pasado por el cambio y por dos flujos de al lado: la
  página y la línea de órdenes (T22).
- Hay un asset de punta a punta solo con MCP en Claude Code, con su
  transcripción guardada (T23).

## Stop and ask if

- Un test que hoy está en verde falla por algo ajeno a K004. Antes de
  diagnosticar, se mide sin el cambio.
- T1 descubre que una ruta hace hoy algo distinto de lo que dice el
  código. Se apunta y no se «arregla» de paso.
- `rmcp` exige el runtime multihilo de `tokio`, o una feature que arrastra
  red (HTTP, SSE).
- Un encargo pasa de 100 líneas nuevas: se parte antes de lanzarlo.
- Para sacar la lógica a `ops` habría que cambiar lo que responde la
  página.
- Para T23 faltan Blender o la sesión de `claude`.
- K003 empieza mientras tanto y toca `serve.rs`.

## Tasks

### Fase 0: red de seguridad y armazón

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Prueba de humo de las rutas: lanza `kiln serve` en un puerto libre sobre una copia temporal de `examples`. Comprueba que cada GET da 200; que cada POST sin `x-kiln` da 403; que los POST con datos inválidos dan hoy 400 con su mensaje; y el caso bueno de `bind`, `material`, `split`, `context/claim` y `settle` sobre la copia. Además, la utilidad común de proyecto temporal | `crates/kiln-cli/tests/routes.rs`, `crates/kiln-cli/tests/common/mod.rs` | La prueba en verde sobre el código de hoy | R2 | implementer | — | 3m/300k | done (12 tests; `/api/bind` escribe antes de validar, fijado tal cual) |
| T2 | Armazón de `ops`: `ops/mod.rs` con los ayudantes comunes (`spec_file`, `rel`, `check_id` con `spec::plain_id`, `check_len`, `confine` a las raíces del proyecto) y sus tests; ficheros vacíos de cada área; `mod ops;` en `main.rs` | `crates/kiln-cli/src/ops/mod.rs` y los vacíos `read.rs`, `production.rs`, `materials.rs`, `context.rs`, `specs.rs`, `sources.rs`, `new_game.rs`; `main.rs` | Tests de `check_id`, `check_len` y `confine` (rojos antes, con `../` y enlaces) | R12 | main | — | 4m/1.7M | done (cause: none; también se copia `make_ctx` como `ops::ctx`; sin rojo previo: lo sustituyó una mutación) |

### Fase 1: la lógica en `ops` (ficheros nuevos; `serve.rs` no se toca)

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T3 | Lectura: `status` (resumen y siguiente paso), `catalog` (filtro y página de 50 por defecto), `check`, `tree` (nodo e hijos), `spec_get`, `context_get`, `content` | `ops/read.rs` | Unit sobre `examples` y `examples/content`: los mismos datos que dan hoy `/api/catalog` y `/api/context`; la paginación corta y dice cuántas hay | R3 | implementer | T2 | 3m/300k | done (6 tests) |
| T4 | Producción: `build`, `paint`, `godot_measure`, `review` e `import` arrancan su trabajo con `agent::job`; `job_wait(id, since, max 30 s)` devuelve líneas nuevas, estado y fichero; `jobs` lista. `review_sheets(asset, round)` lee `<build>/<id>/review/` | `ops/production.rs` | Unit con un trabajo falso: `job_wait` vuelve antes de 30 s con líneas nuevas y con el fichero al terminar; `review_sheets` sobre una carpeta fabricada | R4, R9 | implementer | T2 | 3m/300k | done (3 tests) |
| T5 | Materiales: `bind`, `material_set`, `material_validate`, `zones_split`, la lógica de `serve.rs:287-362` sin cambiarla | `ops/materials.rs` | Unit: un material fuera de la paleta da el mismo error; una pieza fuera del catálogo también | R8 | implementer | T2 | 2m/250k | done (5 tests) |
| T6 | Contexto: `context_edit` (texto, intención, afirmación, respuesta), lo de `serve.rs:406-435`; `settle`; `context_propose` nuevo, que añade afirmaciones y preguntas como agente y aplica `guard_pinned` | `ops/context.rs` | **Rojo primero**: `context_propose` sobre una afirmación fijada deja la original y una propuesta; una pregunta nueva sale `open` con `asked_by` | R6 | implementer | T2 | 3m/300k | done (4 tests, rojo primero) |
| T7 | Fichas: `spec_write(id, toml)` parsea y pasa `spec::check`; si hay errores no escribe y los devuelve; escribe solo en `[assets] specs`. `spec_layout(id, parts)` cambia solo `at` y `turn` y pone `status = "layout"` | `ops/specs.rs` | **Rojo primero**: una ficha inválida no llega al disco; un id con `/` se rechaza; `hoist.toml` reescrito queda igual; `spec_layout` no toca otros campos | R5 | implementer | T2 | 3m/300k | done (5 tests; `turn` son tripletes como `at`; sin rojo previo: lo sustituyó una mutación) |
| T8 | Fuentes: `search(asset, part)` sobre `fetch::search`; nada más (`import` va en T4) | `ops/sources.rs` | Unit con una pieza que no existe: mismo error que `/api/search` | R10 | implementer | T2 | 1m/150k | done (2 tests) |
| T9 | Juego nuevo: `new_game_options` (lo de `GET /api/new`, los dos casos) y `new_game` (`new_game::scaffold`) | `ops/new_game.rs` | Unit: carpeta vacía → `new_game` → existe `kiln.toml`; con `kiln.toml`, error | R11 | implementer | T2 | 2m/200k | done (3 tests) |
| T10 | `serve.rs` llama a `ops` y se borra la lógica duplicada; las respuestas no cambian | `crates/kiln-cli/src/serve.rs` | T1 en verde sin tocarla; todos los tests en verde | R2 | main | T1, T3-T9 | 6m/2M | done (cause: none; −254/+73 líneas en `serve.rs`, mutación detectada por T1) |

### Fase 2: el servidor MCP

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T11 | `rmcp` en versión exacta (la más reciente con al menos dos semanas publicada) y `tokio` con features mínimas; `kiln mcp --project` con runtime de un hilo, stdio y registro a stderr; proyecto opcional (carpeta sin `kiln.toml`); un router vacío por área para que T12-T18 no compartan fichero; utilidad de test que lanza el binario y habla JSON-RPC | `Cargo.toml` (kiln-cli), `main.rs`, `mcp/mod.rs` y los vacíos de cada área, `tests/mcp.rs` | `initialize` y `tools/list` contra el binario; stdout solo lleva JSON-RPC | R1 | main | T10 | 6m/2M | done (cause: estudiar la API de rmcp 3.8.0 en local; 4 tests; release 6,3 MB) |
| T12 | Herramientas de lectura sobre `ops::read` | `mcp/read.rs` | Integración: `status` y `catalog` sobre `examples` | R3 | implementer | T11 | 2m/250k | done (7 tests) |
| T13 | Herramientas de producción y trabajos; `review_sheets` devuelve las hojas como imágenes MCP | `mcp/production.rs` | Integración: `build` sin asset da error claro; `job` de un id inexistente da error; `review_sheets` sobre una carpeta fabricada da cuatro imágenes | R4, R9 | implementer | T11 | 3m/300k | done (6 tests; base64 propio; descripción de `godot_measure` corregida por main) |
| T14 | `spec_write` y `spec_layout` | `mcp/specs.rs` | Integración: una ficha inválida devuelve hallazgos y no escribe | R5 | implementer | T11 | 2m/200k | done (4 tests) |
| T15 | `context_get`, `context_edit`, `context_propose` y `settle`, con descripciones que separan la voz de la persona y la del agente | `mcp/context.rs` | Integración: `context_propose` no cambia lo fijado | R6 | implementer | T11 | 2m/250k | done (6+2 tests; también `ops/context.rs`: `intent` y `conflicts` en `context_propose`) |
| T16 | Materiales, zonas y `search` | `mcp/materials.rs`, `mcp/sources.rs` | Integración: material fuera de la paleta, mismo error que la página | R8, R10 | implementer | T11 | 2m/250k | done (6 tests; `bind` por MCP valida antes de escribir) |
| T17 | `new_game_options` y `new_game`; sin proyecto, el resto de herramientas dice qué falta | `mcp/new_game.rs` | Integración: carpeta vacía → `new_game` → `status` funciona | R11 | implementer | T11 | 2m/200k | done (3 tests) |
| T18 | Prompts `kiln-asset`, `kiln-context` y `kiln-review`: el texto de `skills/*/SKILL.md` con una cabecera que traduce «escribe el fichero» por la herramienta que toca | `mcp/prompts.rs` | Integración: `prompts/list` y `prompts/get` de las tres | R7 | implementer | T11 | 2m/200k | done (4 tests) |

### Fase 3: garantías, cobertura y cierre

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T19 | Garantías por MCP: lista de herramientas y anotaciones fijada; `../`, ids con `/` y textos de 1 MB rechazados sin tocar el disco; con `KILN_POLYPIZZA_KEY` y `KILN_SKETCHFAB_TOKEN` puestos, ninguna respuesta ni stderr los contiene | `tests/mcp_guards.rs` | La prueba en verde; en rojo si se quita una anotación | R10, R12, R13 | implementer | T12-T18 | 3m/300k | done (4 tests; todas las garantías aguantan) |
| T20 | Cobertura: lee las rutas `/api/*` de `serve.rs` y falla si alguna no tiene herramienta ni está en la lista de excepciones (`chat`, D2) | `tests/coverage.rs` | En verde; en rojo con una ruta inventada | R14 | implementer | T12-T18 | 2m/200k | done (5 tests; 22 rutas; `GET` y `POST /api/chat` excluidas, D2) |
| T21 | README: entrega K004 (registro en Claude Code con `claude mcp add`, tabla de herramientas, prompts, límites, D1-D3) y «Lo que viene» al día | `README.md` | Revisión | R15 | implementer | T12-T18 | 2m/200k | done (README «Entrega 11: Kiln por MCP») |
| T22 | Verificación: build, tests, Clippy, `kiln mcp` real sobre `examples`; flujos de al lado: la página (`kiln serve`) y la línea de órdenes (`check`, `index`) | — | Informe del verifier | todos | verifier | T19-T21 | 5m/1.3M | done (ámbar: nada que bloquee; 6 nits; lanzó `import` con id inventado contra el encargo) |
| T23 | De punta a punta con Claude Code y Blender, sobre una copia de `examples`: pedir un asset siguiendo `kiln-asset`, colocar, montar y revisar solo por MCP; se guarda la transcripción | `.sdlc/k004-mcp/e2e.md` | La transcripción y el GLB montado | R16 | main + tú | T22 | 15m/2M | done (Claude Code `-p`: ficha, colocación, montaje y 3 vueltas de revisión; dos fallos → T24 y K005, ver `e2e.md`) |
| T24 | `kiln-asset` sin argumento obligatorio: sin petición, el agente la pregunta (Claude Code parte el slash command por espacios) | `mcp/prompts.rs`, `tests/mcp_prompts.rs` | Rojo primero: `prompts/get` sin `request` pide preguntar | R7 | main | T23 | 3m/1M | done (cause: hallado en T23; `asset` quitado con tu permiso, `request` es el único argumento) |
| T25 | `job` agrupa las líneas: tras la primera novedad espera a 1 s de silencio (10 s como mucho) | `ops/production.rs` | Rojo primero: seis líneas seguidas llegan en una llamada | R4 | main | T23 | 3m/1M | done (cause: hallado en T23, unas 50 llamadas a `job` en una revisión) |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1 (implementer), T2 (main) | 4m | `git diff --stat` contra los ficheros |
| 2 | T3, T4, T5, T6, T7, T8, T9 | 3m | diff, leer cada diff, `cargo test` |
| 3 | T10 | 6m | T1 en verde |
| 4 | T11 | 6m | `tools/list` a mano |
| 5 | T12, T13, T14, T15, T16, T17, T18 | 3m | diff, `cargo test` |
| 6 | T19, T20, T21 | 3m | diff, `cargo test`, Clippy |
| 7 | T22 | 5m | leer el informe |
| 8 | T23 | 15m | transcripción |

- **Estimated wall-clock**: unos 45 min de agentes más las comprobaciones
  entre oleadas (unos 15 min), así que alrededor de 1 h.
- **Sequential sum**: 78 min.
- **Estimated tokens**: unos 13,5M.

## Order and dependencies

- T1 va primero porque es la red: fija qué responde hoy la página antes
  de mover nada.
- T2 crea todos los ficheros de `ops` para que la fase 1 vaya en paralelo
  sin tocar el mismo fichero. `serve.rs` solo lo toca T10, una vez.
- T11 hace lo mismo con `mcp/`: un router por área, que se suman en
  `mod.rs`.
- Respecto a K003: K004 vacía `serve.rs` (T10) antes de que K003 lo
  toque. Si K003 empieza antes, se para (ver «Stop and ask»).

## Test strategy

TDD para la lógica nueva: T6 (`context_propose`), T7 (`spec_write` y
`spec_layout`) y T2 (validación de entradas) empiezan con un test en
rojo. Lo que solo se mueve de sitio (T5, T8, T10) lo protegen T1 y los
tests de hoy. Si un test falla, se arregla el código, no el test.

- **Unit**: cada módulo de `ops` contra `examples` y proyectos temporales.
- **Integración**: `tests/routes.rs` (la página por HTTP), `tests/mcp.rs`
  y `tests/mcp_guards.rs` (el binario por stdio), `tests/coverage.rs`.
- **Manual verification**: el verifier lanza `kiln mcp` y `kiln serve`
  reales. T23, con Claude Code y Blender.

## Verification commands

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Cada implementer corre solo sus tests (`cargo test -p kiln-cli <filtro>`).
La batería entera la corre main al cerrar cada oleada.

## Implementation risks

- **Mover la lógica a `ops`**: `serve.rs` mezcla validación y respuesta
  (por ejemplo, `bind` comprueba `piece` después de intentar escribir,
  `serve.rs:287-297`). T5 conserva ese orden; si parece un fallo, se
  apunta y se pregunta.
- **Dos procesos sobre los mismos ficheros**: `kiln serve` y `kiln mcp`
  abiertos a la vez pueden escribir `context.toml` o una ficha al mismo
  tiempo. Hoy pasa lo mismo con dos pestañas; no se añade bloqueo. Va
  documentado en el README.
- **`rmcp` y su API de macros** cambia entre versiones; por eso T11 lo
  hace main y fija la versión exacta.
- **Tamaño del binario y tiempo de compilación** con `tokio`: se miden
  antes y después en T11.

## Estimate vs actual

Se rellena al cerrar Build desde `.sdlc/k004-mcp/actuals.jsonl`.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|
| T1 | implementer | 3m/300k | 1,4m/379k | −53 % / +26 % | ninguna |
| T2 | main | 4m/1.7M | sin medir | — | no se pasó a «in progress» antes de empezar |
| T3 | implementer | 3m/300k | 13,7m/197k | +357 % / −34 % | siete compilaciones desde cero en paralelo (copias aisladas, cada una con su `target`) |
| T4 | implementer | 3m/300k | 1m/243k | −67 % / −19 % | ninguna |
| T5 | implementer | 2m/250k | 13,4m/197k | +570 % / −21 % | la misma espera de compilación que T3 |
| T6 | implementer | 3m/300k | 0,8m/168k | −73 % / −44 % | ninguna |
| T7 | implementer | 3m/300k | 1,2m/306k | −60 % / +2 % | ninguna |
| T8 | implementer | 1m/150k | 0,5m/104k | −50 % / −31 % | ninguna |
| T9 | implementer | 2m/200k | 12,8m/106k | +540 % / −47 % | la misma espera de compilación que T3 |
| T10 | main | 6m/2M | 1,1m/1.2M | −82 % / −42 % | un solo script de sustitución; T1 ya lo protegía |
| T11 | main | 6m/2M | 4,7m/5.4M | −22 % / +172 % | estudiar la API de `rmcp` 3.8.0 en el código local y probar el patrón de parámetros |
| T12 | implementer | 2m/250k | 1,1m/328k | −45 % / +31 % | ninguna |
| T13 | implementer | 3m/300k | 1,9m/624k | −37 % / +108 % | base64 propio; un hook bloqueó una escritura por la palabra del nombre del fichero |
| T14 | implementer | 2m/200k | 0,7m/167k | −65 % / −17 % | ninguna |
| T15 | implementer | 3m/300k | 4,3m/586k | +43 % / +95 % | ampliación de `ops/context.rs` (`intent`, `conflicts`) que faltaba en T6 |
| T16 | implementer | 2m/250k | 0,9m/169k | −55 % / −32 % | ninguna |
| T17 | implementer | 2m/200k | 0,6m/78k | −70 % / −61 % | ninguna |
| T18 | implementer | 2m/200k | 0,9m/148k | −55 % / −26 % | ninguna |
| T19 | implementer | 3m/300k | 1m/188k | −67 % / −37 % | ninguna |
| T20 | implementer | 2m/200k | 1,3m/214k | −35 % / +7 % | ninguna |
| T21 | implementer | 2m/200k | 0,8m/136k | −60 % / −32 % | ninguna |
| T22 | verifier | 5m/1.3M | 12,7m/2.3M | +154 % / +81 % | recorrió el servidor real, la página y la línea de órdenes, más allá del encargo |

Para calibrar: con la caché de compilación caliente, los implementers
tardan la mitad de lo estimado. Cada copia aislada paga una compilación
desde cero; la próxima vez, compartir una caché de dependencias o sumar
unos 10 min por oleada.

## Outside this plan

- La conversación por MCP (D2), MCP por HTTP, otros clientes (D3).
- Que la página vea los trabajos lanzados por MCP.
- Bloqueo entre `kiln serve` y `kiln mcp` abiertos a la vez.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
