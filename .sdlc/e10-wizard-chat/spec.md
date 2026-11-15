# Spec: Hablar con el agente en cada paso del asistente (entrega 10)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aarón, 2026-11-13

## Cómo está hoy

- El agente se lanza con `agent::start(jobs, ctx, Task)` como un
  `claude -p` de un turno. Las tareas del contexto (`Sort`, `Refine`) editan
  `kiln/context.toml`, y después `guard_pinned` deshace lo que tocó en lo
  fijado y lo convierte en `proposed` (`kiln-cli/src/agent.rs`).
- Ninguna tarea guarda la conversación ni devuelve texto a la persona: el
  `Job` solo lleva `log` (lo que hace) y `spec` (el fichero que escribió).
- Sin `kiln.toml`, el servidor atiende con `handle_new`, que no tiene `Ctx`
  ni `jobs` (`kiln-cli/src/serve.rs`): ahí no se puede lanzar al agente.
- Los pasos 1 y 2 del asistente viven solo en la página hasta «Crear el
  proyecto», que manda `{ name, basics }` a `POST /api/new`.

## Decisiones (las del intent, ya respondidas)

1. Aplica y propone, con el mismo `guard_pinned` de siempre.
2. Pasos 1 y 2: el agente solo lee y devuelve propuestas; no escribe.
3. Un hilo para todo, guardado en `kiln/chat.toml` desde que existe el
   proyecto. Antes, en la página; «Crear el proyecto» lo manda y se guarda.
4. El mismo panel también en la pestaña Contexto, sobre el mismo hilo.

## Decisiones técnicas propuestas

5. **La memoria la lleva Kiln, no Claude Code.** Cada turno es un `claude -p`
   nuevo cuyo prompt lleva los últimos 20 mensajes del hilo. Así no dependo
   de `--resume` ni de dónde guarda Claude Code sus sesiones, y el hilo se
   puede leer y versionar.
6. **Las propuestas van en un bloque al final de la respuesta.** El agente
   termina, si propone valores, con un bloque ` ```kiln ` con JSON:
   `{"names": [...]}`, `{"basics": {"camera": [...]}}` o `{"text": "..."}`.
   Kiln lo separa del texto, valida sus claves y lo devuelve aparte; la
   página lo pinta como botones. Un bloque mal formado se ignora y se dice
   en el log.
7. **Lo que aplica, lo aplica en el fichero.** Desde el paso 3, el agente
   puede editar `kiln/context.toml` (afirmaciones, preguntas, `[intent]`),
   como en los modos ordenar y afinar. No toca `text` (la explicación de la
   persona): si quiere mejorarla, la propone con `{"text": ...}`.

## Requisitos

| # | Requisito | Viene de (intent) | Criterio de aceptación |
|---|-----------|-------------------|------------------------|
| R1 | Hay una conversación en cada paso del asistente | «en todos los pasos» | Los pasos 1 a 5 enseñan el panel; enviar un mensaje devuelve una respuesta del agente en el panel |
| R2 | Se pueden pedir mejoras y aparecen en el paso | «solicitar mejoras» | Paso 1: «dame nombres» → botones que rellenan el nombre. Paso 2: «otras opciones de cámara» → opciones nuevas clicables. Pasos 3-5: «hazlo más oscuro» → cambian las afirmaciones o el párrafo no fijados, o aparece una propuesta sobre lo fijado |
| R3 | Lo fijado no se pisa | «aplica y propone» | Tras un turno que pide cambiar una afirmación fijada, su `text` sigue igual y tiene `proposed` |
| R4 | El hilo es uno y sigue | «debatir» | El agente recibe los mensajes anteriores; el turno 2 puede referirse al 1 («la segunda opción») |
| R5 | El hilo no se pierde | decisión 3 | Tras «Crear el proyecto», `kiln/chat.toml` tiene los mensajes de los pasos 1 y 2; al recargar la página, el panel los enseña |
| R6 | Antes de crear, el agente no escribe | decisión 2 | En los pasos 1 y 2 se lanza sin `Write` ni `Edit`; la carpeta sigue vacía |
| R7 | También en la pestaña Contexto | decisión 4 | El panel aparece en Contexto con el mismo hilo; sus mensajes llevan `step = "contexto"` |
| R8 | El agente cuenta con lo ya fijado | Aarón, durante la construcción (2026-11-13): «si pido al LLM para algo, que tenga contexto de lo que ya hemos fijado» | El prompt de cada turno lleva un bloque «Ya fijado»: en los pasos 1 y 2, el nombre y lo básico elegidos en el borrador; desde el paso 3, el párrafo fijado, las afirmaciones fijadas por capa y las respuestas dadas (sin repetir las que ya son afirmaciones). Lo no fijado no entra en el bloque |
| R9 | Kiln usa la suscripción, no la API | Aarón, durante la construcción (2026-11-14): «la idea es que kiln no use la api de anthropic sino que use la subscripción» | `agent::claude()` lanza Claude Code sin `ANTHROPIC_API_KEY` ni `ANTHROPIC_AUTH_TOKEN`; vale para el chat, las tareas del agente y la revisión |

## Diseño

**El núcleo** (`kiln-core/src/chat.rs`, nuevo):

- `Chat { messages: Vec<Message> }`, `Message { who: "person" | "agent", step: String, text: String }`.
  `load(path)` (vacío si no existe), `save(path, &Chat)`, `recent(n)`.
- `split_offer(reply) -> (String, Option<Offer>)`: separa el último bloque
  ` ```kiln ` del texto. `Offer` admite solo `names: Vec<String>`,
  `basics: BTreeMap<String, Vec<String>>` y `text: String`, y como mucho 6
  valores por lista y 200 caracteres por valor; lo demás se descarta.
- `NewGame` acepta `chat: Vec<Message>` y `scaffold` lo escribe en
  `kiln/chat.toml`.

**La skill** (`skills/context/SKILL.md`): un modo conversar. Responde en
pocas frases y en el idioma de la persona, sabiendo el paso. Aplica lo que se
pide en el contexto si el paso lo permite (lo fijado, en `proposed`) y, si
propone valores, termina con el bloque `kiln`. Si no está de acuerdo, lo dice
y explica por qué.

**El agente** (`kiln-cli/src/agent.rs`):

- `Job` gana `reply: Option<String>` y `offer: Option<Offer>`.
- `Task::Chat { step, message, history, draft }`. `draft` es el
  `{ name, basics }` de la página en los pasos 1 y 2. El prompt lleva el
  paso, el borrador o la ruta del contexto, las capas, lo básico con sus
  opciones y el hilo.
- Sin proyecto: `--allowedTools Read Glob Grep`, sin `--add-dir` de
  `kiln/`. Con proyecto: los de siempre, más `guard_pinned` al acabar.
- Lee el `result` del stream-json, aplica `split_offer` y, si hay proyecto,
  añade a `kiln/chat.toml` el mensaje de la persona y la respuesta.

**El servidor** (`kiln-cli/src/serve.rs`):

- `handle_new` recibe `jobs`. `POST /api/chat` (con `history` y `draft` en
  el cuerpo) y `GET /api/jobs/<id>` también en modo asistente.
- Con proyecto: `POST /api/chat` (`step`, `message`; el hilo lo lee del
  disco) y `GET /api/chat`.
- Todas detrás de `X-Kiln`, como las demás.

**La página** (`kiln-cli/ui/index.html`):

- Un panel «Hablar con el agente» al lado de la tarjeta (debajo en
  pantallas de menos de 1000 px): el hilo, un campo y «Enviar». Mientras el
  agente trabaja, enseña su log y bloquea el envío.
- Las propuestas de la respuesta se pintan como botones según el paso:
  nombres → rellenan el nombre; opciones de lo básico → se añaden a esa
  pregunta y se pueden elegir; `text` → «Usar este texto» en el paso 3.
- Desde el paso 3, al acabar el turno se relee el contexto y se repinta el
  paso.
- En Contexto, el mismo panel en la ficha lateral, con `step = "contexto"`.

## Fuera de esta entrega

- Conversar sobre un asset concreto (la ficha, el montaje).
- Respuestas que aparecen palabra a palabra: llegan enteras al acabar el
  turno, con el log de lo que hace mientras tanto.
- Resumir hilos largos: se mandan los últimos 20 mensajes y nada más.
