# Intent: Kiln como servidor MCP, para hacerlo todo desde MCP (K004)

- **Originator**: Aaron Castells
- **Date**: 2026-11-16
- **Status**: approved by product owner (2026-11-16)
- **Slug**: k004-mcp

## The problem, in the originator's words

> Vamos a trabajar en un MCP de Kiln para poder hacer todo desde mcp.

Ya estaba en «Lo que viene» del README (`README.md:689`): «Kiln como
servidor MCP, para que cualquier Claude le pida catálogo o assets». K003
lo deja fuera de su alcance (`.sdlc/k003-ui/spec.md:74`).

## Qué hay hoy (sobre `main`, 1f0b479)

Kiln se usa de dos maneras, y ninguna es MCP:

- **Línea de órdenes** (`crates/kiln-cli/src/main.rs:3-10`): `index`,
  `check`, `serve`, `build`, `paint`, `godot`, `review`,
  `content-manifest`.
- **Página local** (`kiln serve`, solo en `127.0.0.1`): unas 25 rutas
  `/api/*` en `crates/kiln-cli/src/serve.rs:104-470`, entre ellas
  catálogo, pedir ficha (`ask`), montar, revisar, Godot, buscar y traer de
  fuera (`search`, `import`), vincular ranuras y materiales, validar,
  partir en zonas, pintar, el asistente de juego nuevo (`new`, `settle`),
  la conversación (`chat`) y el contexto (`context/*`). Varias de ellas
  solo lanzan cosas si la petición trae la cabecera `x-kiln: 1`
  (`serve.rs:76-86`).

Lo que pesa para un MCP:

- **Trabajos largos.** Pedir ficha, montar y revisar tardan minutos: la
  página los sigue sondeando `/api/jobs/<id>`.
- **Agente dentro del agente.** Pedir ficha, colocar, ordenar el contexto,
  conversar y revisar lanzan `claude -p` con la suscripción
  (`crates/kiln-cli/src/agent.rs:298, 420`). Un cliente MCP ya es un
  agente.
- **Garantías que hoy da la página.** Solo la propia página puede lanzar
  al agente; las claves de Poly Pizza y Sketchfab nunca llegan ni a la
  página ni al agente; «Traer» baja ficheros y reescribe `LICENSE.md` y
  `CREDITS.md`; el agente no tiene terminal.
- **Sin dependencias fuera de Rust**, y el servidor HTTP está hecho a
  mano (`README.md`, entrega 1).
- No hay nada de MCP en el código; la única mención es un Blender abierto
  por MCP que ejecuta `assemble.py` (`README.md:115`).

## Who it affects

Quien prepara los assets y el contenido de un juego con Kiln y prefiere
hacerlo desde un cliente MCP (Claude Code u otro) en lugar de desde la
página. Hoy, el originador, en sus juegos (Puerto Niebla de ejemplo,
Astra).

## What happens if we do nothing

Para usar Kiln desde un agente hay que pasar por la página o por la línea
de órdenes, que solo cubre ocho órdenes: el contexto, las fichas, los
materiales, buscar y traer, y el asistente solo existen en la página.
[Hueco: el originador no ha dicho aún qué le cuesta hoy en concreto.]

## How we will know it is solved

Observables desde un cliente MCP, sin abrir la página:

- Se conecta a Kiln y ve qué herramientas hay para un proyecto.
- Puede hacer lo que hoy hace la página, dentro del alcance que se fije
  (ver preguntas abiertas), y ve el resultado de cada acción.
- Un trabajo largo (montar, revisar) se puede lanzar, seguir y recoger sin
  quedarse bloqueado.
- Las garantías de hoy siguen en pie: las claves no salen, lo que se baja
  respeta `[rules] licenses`, nada escucha fuera de la máquina.
- [Hueco: medida concreta, p. ej. «un asset de punta a punta solo con MCP».]

## Respuestas del originador (2026-11-16)

- **Alcance**: todo lo que hace la página: las ocho órdenes y las rutas
  `/api/*` (catálogo, check, árbol, fichas, montar, revisar, pintar,
  Godot, buscar y traer, materiales, zonas, contexto y el asistente de
  juego nuevo).
- **Agente**: el trabajo de agente lo hace el agente que llama. Kiln da
  las piezas (leer el proyecto, escribir y validar fichas y contexto) y
  el cliente MCP sigue las skills; Kiln no lanza su propio `claude -p`
  desde el MCP.
- **Conexión**: un subcomando `kiln mcp` por stdio.
- **Dependencias**: se acepta el SDK oficial de Rust, `rmcp` (trae
  `tokio`). Sigue siendo «sin dependencias fuera de Rust».

## Open questions

- **El revisor visual.** La revisión (`kiln review`) lanza un revisor
  aparte, con Opus fijado y sin poder editar la ficha, precisamente para
  que no se juzgue a sí mismo el agente que colocó las piezas. Con
  «el agente que llama», ¿el cliente MCP revisa sus propias hojas, o la
  revisión es la excepción y sigue lanzando el revisor de Kiln?
- **La conversación (`chat`)** existe para hablar con el agente desde la
  página; en un cliente MCP es la propia conversación. ¿Se deja fuera?
- **Cliente objetivo.** Claude Code seguro; ¿también Claude Desktop u
  otros?
- **Base.** K003 (UI) está aprobado y sin empezar (0 de 28 tareas), sin
  commitear en `.sdlc/k003-ui/`, y toca `serve.rs`; ¿va antes, después o
  en paralelo?

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
