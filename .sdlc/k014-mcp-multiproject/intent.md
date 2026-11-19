# Intent: K014, un solo MCP de Kiln para cualquier proyecto

- **Originator**: Aaron, 2026-11-19, fase 1 del plan «Kiln como app standalone» (Claude Docs).
- **Approved**: Aaron aprobó empezar la fase 1 el 2026-11-19.

## Problem

`kiln mcp --project DIR` ata cada registro MCP a un juego. Un principiante tiene
que registrar Kiln una vez por proyecto y no puede cambiar de juego dentro de
una conversación.

## Outcome

Con un solo registro de usuario en Claude Code
(`claude mcp add --scope user kiln -- kiln mcp`) se trabaja en dos juegos
distintos en la misma conversación.

## Out of scope

Núcleo compartido, trabajos en disco, bloqueo por proyecto (fase 2);
`setup`, `config_*`, `preview` (fase 3); plugin e instalador (fase 4).
