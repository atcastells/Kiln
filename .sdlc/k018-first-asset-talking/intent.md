# Intent: K018, el primer asset conversando

- **Originator**: Aaron, 2026-11-23, fase 3 del plan «Kiln como app standalone» (Claude Docs).
- **Approved**: Aaron pidió arrancar la fase 3 el 2026-11-23 («si»).

## Problem

Kiln ya sirve cualquier proyecto por MCP (fase 1) desde un núcleo compartido
(fase 2), pero un principiante aún se atasca antes del primer asset: Blender y
Godot instalados como apps no están en el PATH y hay que escribir su ruta en
cada `kiln.toml`; un juego nuevo no sabe dónde publicar; no hay forma de ver
el asset dentro de la conversación; y nadie le dice por dónde empezar.

## Outcome

Alguien que no ha visto Kiln publica su primer asset solo hablando con Claude
Code: `/mcp__kiln__kiln-start`, y el agente hace el resto.

## Out of scope

Plugin de Claude Code e instalador (fase 4); app de escritorio (fase 5).
