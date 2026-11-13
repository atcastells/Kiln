# Plan: Empezar un juego desde cero (entrega 9)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aarón, 2026-11-13)
- **Nothing is implemented until this file is approved.**

## Hecho significa

- En una carpeta vacía, `kiln serve --project CARPETA` abre el asistente; tras
  los cinco pasos con el agente de verdad, `kiln check` sale con 0 y el
  contexto tiene el intent fijado y una afirmación fijada por capa.
- Cerrar la página en el paso 3 y volver a abrirla sigue en el paso 3.
- Los proyectos que ya existen (`examples`) se abren igual que antes.
- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`
  en verde.

## Parar y preguntar si

- Las decisiones propuestas del spec no son las que querías.
- Recargar la configuración en el servidor obliga a rehacer `Ctx` más allá
  de un cerrojo.
- Un test que ya existe falla por algo ajeno al cambio.

## Tareas

Cada encargo cubre un punto y menos de 100 líneas nuevas.

| # | Tarea | Ficheros | Test que lo prueba | Requisito | Corre en | Depende | Est | Estado |
|---|-------|----------|--------------------|-----------|----------|---------|-----|--------|
| T1 | `Context.intent`, `guard_pinned` del párrafo y `missing(&layers)` | `kiln-core/src/context.rs` | Unitario: lee un contexto sin `[intent]`; el agente solo propone sobre un párrafo fijado; `missing` lista capa sin fijar, choque y párrafo sin fijar | R4, R5 | implementer | — | 2m/200k | done (cause: run on main, not implementer; also `settle` and `Intent.settled`, to know a wizard project from an old one) |
| T2 | `[project] kind`, por defecto `game`, y su aviso en `check` | `kiln-core/src/config.rs`, `kiln-core/src/check.rs` | Unitario: sin `kind` es `game`; `kind = "film"` da aviso | R2 | implementer | — | 1m/150k | done (cause: run on main) |
| T3 | `new_game::scaffold`: `kiln.toml`, paleta vacía y contexto con lo básico | `kiln-core/src/new_game.rs` (nuevo), `kiln-core/src/lib.rs` | Unitario en carpeta temporal: `open` del resultado sin errores; se niega si ya hay `kiln.toml`; licencias y triángulos llegan a `[rules]` | R2, R3, R6 | implementer | T1, T2 | 2m/250k | done (cause: run on main; basics saved as answered questions so refine does not ask them again) |
| T4 | Modo ordenar escribe el párrafo del intent | `skills/context/SKILL.md` | Lo prueba T7 con el agente de verdad | R4 | main | T1 | 1m/100k | done (cause: none) |
| T5 | Servidor en modo asistente y rutas `/api/new`, `/api/context/intent`, `/api/settle`; recarga tras crear | `kiln-cli/src/serve.rs`, `kiln-cli/src/agent.rs` | `curl` contra `kiln serve` en carpeta vacía: crea, escribe, asienta; sin `X-Kiln` se rechaza | R1, R3, R6 | main | T3 | 4m/1M | done (cause: wizard step comes back as open questions too, so reopening lands on step 4) |
| T6 | La página del asistente, cinco pasos | `kiln-cli/ui/index.html` | Capturas de cada paso en el navegador | R1, R5, R6 | main | T5 | 6m/2M | done (cause: steps 1-2 stay in the page until «Crear», so the basics can go into kiln.toml in one write; `[hidden]` lost to layout CSS) |
| T7 | README y verificación de punta a punta con un juego inventado, más dos flujos vecinos (abrir `examples`, ordenar el contexto de un proyecto existente) | `README.md` | Informe del verificador con capturas | todos | verifier | T4, T6 | 5m/1M | done (cause: run on main, not the verifier; one real agent run on an invented game, «Islote de Sal») |

## Calendario

| Tanda | Tareas (en paralelo) | La más larga | Después (main) |
|-------|----------------------|--------------|----------------|
| 1 | T1, T2 | 2m | batería entera |
| 2 | T3, T4 | 2m | batería entera |
| 3 | T5 | 4m | `curl` |
| 4 | T6 | 6m | capturas |
| 5 | T7 | 5m | leer el informe |

- **Reloj estimado**: 19 min, más las comprobaciones entre tandas
- **Suma secuencial**: 21 min
