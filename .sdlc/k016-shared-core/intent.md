# Intent: K016, un núcleo de Kiln compartido

- **Originator**: Aaron, 2026-11-21, fase 2 del plan «Kiln como app standalone» (Claude Docs).
- **Approved**: Aaron pidió seguir con la fase 2 el 2026-11-21 («Seguimos»).

## Problem

Cada `kiln mcp` y cada `kiln serve` son procesos aparte. Un montaje o una
revisión lanzados desde Claude Code mueren al cerrar la conversación; la página
no ve lo que lanza el MCP y los dos pueden escribir lo mismo a la vez. La
revisión por MCP lanza un segundo Claude aparte, que gasta otra sesión.

## Outcome

Se lanza un montaje desde Claude Code, se cierra, y al volver el trabajo sigue
y la página lo muestra.

## Out of scope

`setup`, `config_*`, `preview`, `open_viewer`, `kiln-start` (fase 3); plugin e
instalador (fase 4); app de escritorio (fase 5). Las órdenes de la terminal
(`kiln build`...) siguen fuera del núcleo.
