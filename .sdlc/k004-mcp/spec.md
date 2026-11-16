# Spec: Kiln como servidor MCP (K004)

- **Derived from**: `intent.md`
- **Status**: approved (2026-11-16), con D1, D2 y D3
- **Approved by**: Aaron Castells

## Decisiones que este spec propone y necesitan tu visto bueno

Son las tres preguntas que el intent deja abiertas:

- **D1. El revisor visual sigue siendo de Kiln.** `review` es la única
  herramienta que lanza un `claude -p`: el revisor aparte, con Opus fijado
  y sin poder editar la ficha (`crates/kiln-cli/src/review.rs:124-229`).
  Existe para que el agente que coloca las piezas no se juzgue a sí
  mismo, y eso no cambia porque quien llame sea un cliente MCP. Además, el
  cliente puede ver las hojas de cada vuelta (R9).
- **D2. La conversación (`/api/chat`) queda fuera.** En un cliente MCP, la
  conversación ya es la del propio cliente.
- **D3. El cliente objetivo es Claude Code.** Es MCP estándar por stdio,
  así que cualquier cliente debería funcionar, pero solo se prueba con
  Claude Code.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | `kiln mcp --project DIR [--config FILE]` sirve MCP por stdio con `rmcp`, en una versión fija exacta. Por stdout solo sale el protocolo; el registro va a stderr | Respuestas: conexión, dependencias | Test que lanza el binario, hace `initialize` y `tools/list` y recibe JSON-RPC válido |
| R2 | La página y el MCP comparten la misma lógica. Cada acción de `serve.rs` pasa a una función de un módulo común (`ops`) que llaman los dos. `serve.rs` solo traduce HTTP y el MCP solo traduce herramientas | Alcance: todo lo que hace la página | Las rutas responden igual que antes (tests actuales en verde y una prueba de humo de cada ruta); ninguna acción está escrita dos veces |
| R3 | **Leer**: `status` (resumen del proyecto: piezas, fichas, hallazgos por gravedad, siguiente paso), `catalog` (con filtro y paginado), `check`, `tree` (un nodo y sus hijos), `spec_get`, `context_get`, `content` (conjuntos y elementos) | Alcance; «ve qué herramientas hay» | Cada herramienta, sobre `examples`, devuelve los mismos datos que la ruta equivalente |
| R4 | **Producción**, como trabajos: `build`, `paint` (con variante), `godot_measure`, `review` e `import`. Devuelven al momento el id del trabajo; `job` (id, desde qué línea, espera de hasta 30 s) da las líneas nuevas, el estado y el fichero resultante; `jobs` lista los trabajos | «lanzar, seguir y recoger sin quedarse bloqueado» | Un trabajo falso de 2 minutos se sigue con `job` sin que ninguna llamada pase de 30 s; al terminar, `job` da el fichero |
| R5 | **Fichas desde el agente que llama**: `spec_write` (id y TOML). Kiln lo parsea y lo pasa por `spec::check`; si hay errores no escribe nada y devuelve los hallazgos. Escribe solo en `[assets] specs`. `spec_layout` añade `at` y `turn` a las piezas sin tocar nada más | Respuestas: agente | Una ficha inválida no llega al disco y devuelve el hallazgo; `examples/specs/hoist.toml` reescrito por `spec_write` queda igual |
| R6 | **Contexto con dos voces.** `context_edit` es la persona: texto, párrafo de intención, afirmación, respuesta, y lo que escribe queda fijado, como en la página. `context_propose` es el agente: añade afirmaciones y preguntas y aplica `guard_pinned`, así que lo fijado nunca cambia y como mucho queda una propuesta | Respuestas: agente; garantías de hoy | Test: `context_propose` sobre una afirmación fijada deja la original y una propuesta |
| R7 | **Las skills como prompts MCP**: `kiln-asset`, `kiln-context` y `kiln-review` sirven el texto de `skills/*/SKILL.md`, adaptado para decir qué herramienta usar en lugar de qué fichero escribir | Respuestas: agente («el cliente sigue las skills») | `prompts/list` las da; seguir `kiln-asset` en Claude Code produce una ficha válida en `examples` (prueba manual) |
| R8 | **Materiales y zonas**: `bind`, `material_set`, `material_validate`, `zones_split`, con las mismas comprobaciones que la página (material en la paleta, pieza en el catálogo) | Alcance | Mismos errores que la ruta ante un material que no existe |
| R9 | **Ver el resultado**: `review_sheets` (asset, vuelta) devuelve las hojas como imágenes MCP y las medidas; los resultados de montar, pintar y revisar dan la ruta, relativa al proyecto | «ve el resultado de cada acción» | Tras una revisión, el cliente recibe las cuatro hojas como imagen |
| R10 | **Buscar fuera**: `search` (asset, pieza) e `import` (trabajo). Siguen las reglas de hoy: solo licencias de `[rules]`, la licencia se vuelve a leer en el sitio y se reescriben `LICENSE.md` y `CREDITS.md` | Alcance; garantías | Test: ninguna respuesta del MCP contiene el valor de `KILN_POLYPIZZA_KEY` ni de `KILN_SKETCHFAB_TOKEN` puestos en el entorno |
| R11 | **Juego nuevo**: en una carpeta sin `kiln.toml`, el MCP da `new_game_options` (pasos y respuestas sugeridas) y `new_game` (crea el proyecto). Con proyecto, `settle` asienta. El resto de herramientas dice qué falta, como hace la página | Alcance: el asistente | Test: carpeta vacía → `new_game` → `status` funciona |
| R12 | **Entradas validadas** en cada herramienta: ids planos (`spec::plain_id`), largos máximos (petición de 500 caracteres, como `/api/ask`), rutas confinadas a las raíces del proyecto (como `/files/`). Errores claros, en castellano y con qué hacer | Garantías; OWASP | Tests con `../`, ids con `/` y textos de 1 MB: rechazados sin tocar el disco |
| R13 | Cada herramienta lleva anotaciones MCP (`readOnlyHint`, `destructiveHint`, `openWorldHint` en `search` e `import`) para que el cliente pida permiso donde toca | Garantías | `tools/list` las lleva; test que fija la lista y sus anotaciones |
| R14 | Cobertura: una tabla en el README y un test garantizan que toda ruta `/api/*` tiene herramienta o está en «fuera de alcance» | «Puede hacer lo que hoy hace la página» | Test que falla si se añade una ruta sin herramienta ni excepción |
| R15 | README: entrega K004 (cómo registrar `kiln mcp` en Claude Code, herramientas, prompts y límites) y «Lo que viene» actualizado | — | README revisado |
| R16 | De punta a punta solo con MCP: en `examples`, pedir un asset siguiendo `kiln-asset`, colocar, montar y revisar, sin abrir la página | «How we will know»: hueco de medida | Prueba manual con Claude Code y Blender, con la transcripción guardada en `.sdlc/k004-mcp/` |

## Design

- **Proceso.** `kiln mcp` arranca un runtime `tokio` de un solo hilo
  únicamente en ese subcomando; el resto de Kiln sigue síncrono. Las
  herramientas llaman a `ops` dentro de `spawn_blocking`. El servidor
  vive mientras el cliente lo tenga abierto.
- **Módulos.**
  - `crates/kiln-cli/src/ops/`: lo que hoy está dentro del `match` de
    `serve.rs:99-134` y `179-480`, separado por áreas: `read`, `production`,
    `materials`, `context`, `sources`, `new_game`. Cada función recibe
    tipos de Rust y devuelve `Result<Value, String>`.
  - `crates/kiln-cli/src/mcp/`: `mod.rs` (servidor y registro), un
    fichero de herramientas por área y `prompts.rs`.
  - `agent.rs` conserva los trabajos (`Jobs`, `job`). El MCP los usa
    igual que la página.
- **Trabajos.** La misma estructura `Job`, en memoria del proceso. `job`
  espera de verdad: duerme hasta que hay líneas nuevas o el trabajo
  termina, con 30 s como mucho. Si el cliente lo admite, se mandan
  además avisos de progreso MCP. Si el proceso se cierra, el trabajo en
  curso se pierde, igual que al cerrar `kiln serve`.
- **Qué no se mueve.** Con el MCP, ni pedir ficha, ni colocar, ni ordenar
  o afinar el contexto lanzan `claude -p`: los hace el cliente con R5,
  R6 y R7. En la página siguen como hoy.
- **Datos.** Todo se queda en el proyecto: `kiln.toml`, `[assets] specs`,
  `kiln/context.toml`, `kiln/bindings.toml`, `kiln/zones.toml`, build y
  caché. El MCP no abre ningún puerto. La red solo la usan `search` e
  `import`, por `curl`, como hoy.

## Policy applied

- **Personal data**: ninguno. Kiln maneja assets, contexto de juego y
  licencias; en los créditos solo salen los autores de modelos públicos
  que la licencia CC-BY obliga a acreditar (ya pasa hoy).
- **Sensitive business data**: ninguno de los que define el perfil
  (costes, márgenes, tarifas, stock).
- **Credentials**: las claves de Poly Pizza y Sketchfab, por variable de
  entorno o `~/.config/kiln/keys.toml`, como hoy. Nunca salen en una
  respuesta, un registro ni un prompt (test de R10). El cliente MCP no las
  ve.
- **Decisions about people / high risk**: no.

## Out of scope

- La conversación `/api/chat` (D2).
- Que Kiln lance su `claude -p` desde el MCP para pedir, colocar, ordenar
  o afinar. Solo lo hace el revisor (D1).
- MCP por HTTP o dentro de `kiln serve`, y que la página vea los trabajos
  lanzados por MCP.
- Probar con clientes que no sean Claude Code (D3).
- Cualquier cambio de la interfaz (eso es K003) y de lo que Kiln valida.
- `kiln serve` sigue sirviendo los mismos ficheros: `/files/` y
  `/vendor/three.js` son de la página y no tienen herramienta.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| K003 (aprobado, sin empezar) también reescribe `serve.rs` (R7 y R8 de K003) | Conflictos y trabajo rehecho | Hacer primero R2 de K004 (sacar `ops` de `serve.rs`). Así K003 trabaja sobre un `serve.rs` delgado; el orden se fija en `plan.md` |
| `rmcp` cambia a menudo y arrastra `tokio` | Compilación más lenta y binario más grande | Versión exacta fija y `tokio` con las features mínimas; se mide el binario antes y después |
| El agente que llama escribe fichas malas | Montajes que fallan | `spec_write` no escribe nada que no pase `spec::check` (R5) |
| El cliente usa `context_edit` (persona) para lo que es del agente, y así lo fija | Se pierde la protección de lo fijado | La descripción de la herramienta y el prompt `kiln-context` lo limitan a las palabras de la persona. El control real es el permiso del cliente: `context_edit` lleva `destructiveHint` |
| Una herramienta de lectura devuelve demasiado (5.000 piezas) | Llena el contexto del cliente | Paginado con 50 por defecto y un resumen en `status` |
| Mover la lógica a `ops` rompe la página | Regresión en lo que ya funciona | Tests actuales y una prueba de humo de cada ruta antes y después de R2 |

---
Next stage: Build reads this file and writes `plan.md`.
