# Spec: Empezar un juego desde cero (entrega 9)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aarón, 2026-11-13

## Cómo está hoy

- `kiln serve` y los demás comandos piden un `kiln.toml` en la carpeta; sin
  él fallan con «no se puede leer kiln.toml».
- El contexto (`kiln/context.toml`) nace vacío. La persona escribe su texto,
  el agente lo ordena en afirmaciones por capa y abre preguntas. Nada dice
  cuándo el contexto es suficiente para empezar a pedir assets.
- `[project]` tiene `name` y `engine`, pero no tiene tipo.

## Decisiones propuestas (las preguntas abiertas del intent)

1. **El intent vive en el contexto**: un bloque `[intent]` en
   `kiln/context.toml`, con un párrafo y si está fijado. Así el agente de
   assets lo lee sin cambiar nada, porque ya lee el contexto antes de
   desglosar.
2. **Solo lo de Kiln**: el asistente no crea el proyecto de Godot. Deja
   `[project] engine = "godot"` y el proyecto de Godot se enlaza después en
   `[tools] godot_project`.
3. **Se entra por la página**: `kiln serve --project CARPETA` con una
   carpeta sin `kiln.toml` abre el asistente en vez de fallar. Sin comando
   `kiln new` de momento.

## Requisitos

| # | Requisito | Viene de (intent) | Criterio de aceptación |
|---|-----------|-------------------|------------------------|
| R1 | `kiln serve` en una carpeta sin `kiln.toml` abre el asistente | «empezar un proyecto de juego desde 0» | La página enseña el paso 1 en vez de un error |
| R2 | Solo se puede crear un proyecto de tipo juego | «solo se pueden iniciar proyectos tipo juego» | `[project] kind = "game"`; el asistente enseña el tipo fijo; otro `kind` da un aviso en `check` |
| R3 | El asistente crea un proyecto válido sin escribir TOML | «wizard sencillo» | Tras el paso 1, `kiln check` sale con 0 y `kiln serve` lo abre como cualquier proyecto |
| R4 | El proyecto nace con un intent: un párrafo de qué juego es | «un intent asentado» | `[intent] text` en el contexto, escrito por el agente a partir de lo que dijo la persona |
| R5 | Asentado tiene una regla comprobable | «asentado» | Párrafo fijado, una afirmación fijada al menos por capa y ningún choque abierto; la página dice qué falta |
| R6 | Nada se pierde si se cierra a medias | — | Cada paso escribe en disco; al volver a abrir, el asistente sigue donde se quedó |

## Diseño

**Los pasos, como los ve la persona.** Una columna centrada, un paso cada
vez, con «Atrás» y «Seguir». Se parece al asistente de proyecto nuevo de
Godot: pocos campos y valores por defecto razonables.

1. **Proyecto.** Nombre del juego. Tipo: «Juego», marcado y sin más
   opciones. Motor: Godot 4. «Crear» escribe `kiln.toml` y
   `kiln/context.toml` en la carpeta.
2. **Tu juego.** Un cuadro de texto con cuatro pistas: de qué va, dónde y
   cuándo pasa, qué hace el jugador, cómo se ve. Va al `text` del contexto.
3. **Lo básico.** Cinco preguntas fijas con respuestas sugeridas (un clic,
   o tus palabras): cámara, estilo visual, altura de una persona adulta,
   licencias de los assets, triángulos por pieza. Cada respuesta es una
   afirmación fijada de su capa; licencias y triángulos van también a
   `[rules]` del `kiln.toml`.
4. **Ordenar y afinar.** Lanza al agente del contexto (modo ordenar, el que
   ya existe) y después una ronda de preguntas. El agente escribe además el
   párrafo del intent. Las preguntas se responden con las mismas tarjetas de
   la pestaña Contexto.
5. **Asentar.** Enseña el párrafo (editable) y una línea por capa con sus
   afirmaciones fijadas. «Asentar» solo se activa cuando se cumple R5; si
   no, dice qué falta («Escala no tiene nada fijado»). Al asentar se abre el
   grafo de siempre.

**El núcleo** (`kiln-core`):

- `context.rs`: `Context.intent: Intent { text, pinned, proposed }`, con
  `#[serde(default)]` para leer los contextos que ya existen.
  `guard_pinned` también protege el párrafo fijado (el agente solo
  propone). `Context::missing(&layers) -> Vec<String>` devuelve lo que falta
  para asentar; vacío es asentado.
- `new_game.rs` (nuevo): `NewGame { name, basics }` y `scaffold(dir, &NewGame)`.
  Escribe `kiln.toml` (con `kind = "game"`, carpetas por defecto
  `assets/kits`, `kiln/specs`, una paleta vacía `kiln/materials.toml`) y
  `kiln/context.toml` con las afirmaciones de lo básico. Se niega si ya hay
  `kiln.toml`.
- `config.rs`: `ProjectSection.kind`, por defecto `"game"`; `check` avisa
  si es otro.

**La skill del contexto**: el modo ordenar escribe también `[intent] text`,
un párrafo de 3 a 5 frases sacado solo de las afirmaciones; si está fijado,
deja su versión en `proposed`.

**El servidor**: arranca sin `kiln.toml` en modo asistente. Rutas nuevas
`GET /api/new` (en qué paso está), `POST /api/new` (paso 1),
`POST /api/context/intent` (editar, aceptar, fijar) y `POST /api/settle`.
Tras el paso 1 recarga la configuración sin reiniciar. Todas con la misma
cabecera `X-Kiln` que ya protege las demás.

## Fuera de esta entrega

- Otros tipos de proyecto.
- Crear el proyecto de Godot.
- Abrir un proyecto existente sin `kiln.toml` leyendo lo que ya tiene (lore,
  `CLAUDE.md`): el modo ordenar ya lo lee si está, pero el asistente no lo
  busca aparte.
