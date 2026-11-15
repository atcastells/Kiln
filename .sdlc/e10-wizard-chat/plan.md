# Plan: Hablar con el agente en cada paso del asistente (entrega 10)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aarón, 2026-11-13)
- **Nothing is implemented until this file is approved.**

## Hecho significa

- En una carpeta vacía, `kiln serve --project CARPETA`:
  - paso 1: «dame tres nombres» → tres botones que rellenan el nombre;
  - paso 2: «otras opciones de cámara» → opciones nuevas en la pregunta de
    cámara; la carpeta sigue vacía hasta «Crear el proyecto»;
  - tras crear, `kiln/chat.toml` tiene esos mensajes;
  - paso 5: pedir un cambio sobre una afirmación fijada la deja igual, con
    `proposed`; sobre una no fijada, la cambia;
  - recargar la página enseña el hilo.
- En un proyecto asentado, la pestaña Contexto tiene el panel con el mismo
  hilo.
- El asistente sin usar el panel funciona igual que en la entrega 9, y
  `examples` se abre igual que antes.
- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`
  en verde.

## Parar y preguntar si

- El `result` del stream-json no trae el texto final de la respuesta en la
  versión de `claude` instalada.
- Pasar `jobs` a `handle_new` obliga a rehacer el bucle de `serve::run` más
  allá de un argumento.
- El agente no respeta el bloque `kiln` en la prueba real (más de 1 de cada
  3 turnos sin él cuando se le piden valores): habría que cambiar el
  formato antes de seguir.
- Un test que ya existe falla por algo ajeno al cambio.

## Tareas

Cada encargo cubre un punto y menos de 100 líneas nuevas.

| # | Tarea | Ficheros | Test que lo prueba | Requisito | Corre en | Depende | Est | Estado |
|---|-------|----------|--------------------|-----------|----------|---------|-----|--------|
| T1 | `chat.rs`: `Chat`, `Message`, `load`/`save`/`recent` | `kiln-core/src/chat.rs` (nuevo), `kiln-core/src/lib.rs` | Unitario: sin fichero da hilo vacío; guardar y leer da lo mismo; `recent(2)` de 5 da los dos últimos | R4, R5 | implementer | — | 1m/150k | done (cause: none; missing file only, other read errors fail) |
| T2 | `split_offer` y `Offer` con sus límites | `kiln-core/src/chat.rs` | Unitario: sin bloque, bloque bueno, dos bloques (gana el último), JSON roto, clave desconocida, 7 nombres → 6, valor de 300 caracteres descartado | R2 | implementer | T1 | 2m/200k | done (cause: 135 lines, the excess is tests) |
| T3 | `NewGame.chat` y `scaffold` escribe `kiln/chat.toml` | `kiln-core/src/new_game.rs` | Unitario en carpeta temporal: con 2 mensajes, `chat::load` los devuelve; sin `chat`, no hay fichero | R5 | implementer | T1 | 1m/150k | done (cause: none) |
| T4 | Modo conversar en la skill | `skills/context/SKILL.md` | Lo prueba T9 con el agente de verdad | R2, R3 | main | T2 | 1m/100k | done (cause: none) |
| T5 | `Job.reply`/`offer` y lectura del `result` del stream | `kiln-cli/src/agent.rs` | Unitario: una línea `result` de ejemplo da el texto; con bloque `kiln`, `reply` sin bloque y `offer` lleno | R1, R2 | implementer | T2 | 2m/200k | done (cause: dead-code clippy red until T6/T7 wired it) |
| T6 | `Task::Chat`: prompt con paso, borrador o contexto y hilo; herramientas sin escritura sin proyecto; guarda el turno en `chat.toml` | `kiln-cli/src/agent.rs` | Unitario del prompt (lleva los 20 últimos mensajes y el paso) y de los argumentos (sin proyecto no hay `Write`/`Edit`) | R1, R3, R4, R6 | implementer | T5, T4 | 3m/300k | done (cause: 170 lines, spawn duplicated to leave run() alone; unit tests only, end to end in T7) |
| T7 | Rutas: `handle_new` con `jobs` y `POST /api/chat` y `GET /api/jobs/<id>`; con proyecto, `POST`/`GET /api/chat`; `POST /api/new` lleva `chat` | `kiln-cli/src/serve.rs` | `curl` contra `kiln serve` en carpeta vacía con `KILN_CLAUDE` apuntando a un `claude` falso que escribe un `result` fijo: responde, propone y no escribe nada; sin `X-Kiln`, 403 | R1, R5, R6 | main | T3, T6 | 3m/500k | done (cause: the fake claude broke on macOS sh `echo` turning \n into newlines; printf fixed it) |
| T7b | R8, pedido durante la construcción: `Context::settled_part` y `draft_settled`, en el prompt del chat | `kiln-core/src/context.rs`, `kiln-cli/src/agent.rs` | Unitarios de los dos y del prompt; `curl` con el `claude` falso: el prompt lleva las fijadas y no la no fijada | R8 | main | T7 | 1m/150k | done (cause: basics were listed twice, as pinned claim and answered question; deduped) |
| T7c | R9, pedido durante la construcción: `claude()` quita las variables de la API | `kiln-cli/src/agent.rs`, `README.md` | Unitario: el `Command` de `claude()` las quita | R9 | main | — | 1m/50k | done (cause: none; Kiln never called the API, the CLI was already on claude.ai login) |
| T8 | Panel en el asistente: hilo, envío, log, propuestas por paso, repintar desde el paso 3 | `kiln-cli/ui/index.html` | Capturas de los pasos 1, 2 y 5 con el `claude` falso de T7 | R1, R2, R5 | main | T7 | 6m/2M | done (cause: step 5 did not show proposals on pinned claims, added Aceptar/Descartar per claim; local error messages kept out of the thread; checked with a scripted run in the browser, not screenshots of each step) |
| T9 | Panel en Contexto, README y verificación de punta a punta con el agente de verdad, más dos flujos vecinos (asistente sin panel, abrir `examples`) | `kiln-cli/ui/index.html`, `README.md` | Informe del verificador con capturas | R7, todos | verifier | T8 | 5m/1M | done (cause: verifier amber, browser part was covered on main with the fake claude; 5 real turns; fixed its NITs: /api/new thread now cleaned, offers kept numbered in the thread, a misplaced doc comment) |

## Calendario

| Tanda | Tareas (en paralelo) | La más larga | Después (main) |
|-------|----------------------|--------------|----------------|
| 1 | T1 | 1m | batería entera |
| 2 | T2, T3 | 2m | batería entera |
| 3 | T4, T5 | 2m | batería entera |
| 4 | T6 | 3m | batería entera |
| 5 | T7 | 3m | `curl` |
| 6 | T8 | 6m | capturas |
| 7 | T9 | 5m | leer el informe |

- **Reloj estimado**: 22 min, más las comprobaciones entre tandas
- **Suma secuencial**: 24 min
