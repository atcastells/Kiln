# Intent: K016b, la página respeta la confianza de K014

- **Originator**: la revisión de seguridad automática al subir K017 (PR #38),
  relayada por la sesión principal el 2026-11-22. La línea marcada viene del
  commit `16a71c4` de K016 (PR #37, abierto), no de K017.
- **Date**: 2026-11-22
- **Status**: approved by product owner (Aaron, 2026-11-22)
- **Slug**: k016-page-trust

## The problem, in the originator's words

Desde K016, la página sigue al proyecto que abra cualquier sesión MCP
(`Kiln::open` → `shared.show`), aunque la carpeta no sea de confianza. Las
herramientas MCP sí comprueban la confianza (`run_ctx`), pero `serve.rs` no la
mira en ningún sitio. Así, los POST de la página que lanzan Blender, Godot o
Claude pueden correr sobre un proyecto ajeno, y Godot ejecuta sus plugins.
K014 hizo la lista de confianza justo para evitarlo.

## Who it affects

Quien usa Kiln con Claude Code y la página a la vez: basta con que una sesión
MCP abra (por `project_open` o por las raíces del cliente) una carpeta que la
persona no ha revisado, y un clic en la página la ejecuta.

## What happens if we do nothing

K016 se une con un hueco que deshace K014: un `kiln.toml` ajeno elige qué
programa se lanza y un proyecto de Godot ajeno corre sus plugins, con un clic
en la página. Hay que tenerlo antes de unir el #37.

## How we will know it is solved

Una sesión MCP abre una carpeta sin confianza → la página la muestra → `POST
/api/build` (y el resto de los que lanzan programas) responde 403 con el
mensaje de `kiln trust`, y no se lanza nada. Tras `kiln trust --project`, la
misma página ya lanza.

## Open questions

- ¿La página sin confianza es solo lectura del todo, o deja editar como el MCP
  («se puede leer y editar») y solo bloquea lanzar programas? Propuesta en el
  spec: lo segundo, igual que el MCP.

---
Next stage: Design reads this file and writes `spec.md`.
