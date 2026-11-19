# Spec: K014, MCP multiproyecto

- **Derived from**: `intent.md`.
- **Decision** (pendiente en el plan, tomada aquí): proyecto activo por sesión.
  Cada proceso `kiln mcp` es una sesión del cliente; las herramientas actúan
  sobre el proyecto activo. Un parámetro `project` en las 31 herramientas queda
  para la fase 2 si hace falta con varios clientes a la vez.

## Requirements

| # | Requisito | Aceptación |
|---|-----------|------------|
| R1 | Registro global de proyectos: nombre, ruta absoluta y última apertura. En `$KILN_HOME/projects.json`; sin `KILN_HOME`, `~/Library/Application Support/Kiln` en macOS y `$XDG_DATA_HOME/kiln` o `~/.local/share/kiln` en el resto. Más reciente primero, máximo 50. Un fichero roto no rompe el MCP. | tests unitarios de `projects` |
| R2 | `projects`: lista los recientes con si la carpeta existe, si tiene `kiln.toml`, si es proyecto Godot y cuál está activo. | `mcp_projects.rs` |
| R3 | `project_open {path}`: ruta absoluta a una carpeta existente. Con `kiln.toml` la abre; con `project.godot` y sin `kiln.toml` la deja activa y dice que se cree con `new_game`; sin ninguno de los dos, la rechaza. Se apunta en el registro. | `mcp_projects.rs` |
| R4 | `project_create {path, name, basics}`: crea la carpeta si falta (su padre debe existir), escribe el proyecto como `new_game`, lo deja activo y lo apunta. Falla si ya hay `kiln.toml`. | `mcp_projects.rs` |
| R5 | `status` dice qué carpeta está abierta (`folder`). Sin proyecto activo, las herramientas dicen que se use `projects`, `project_open` o `project_create`. | `mcp_projects.rs` |
| R6 | `kiln mcp` sin `--project`: abre la primera de estas que tenga `kiln.toml` o `project.godot`: las *roots* del cliente, la carpeta de trabajo, el último proyecto del registro (este solo con `kiln.toml`). Si no hay ninguna, arranca sin proyecto. Las roots solo se usan si nadie ha abierto otro proyecto antes. Con `--project` se comporta como hoy. | `mcp_projects.rs` |
| R7 | `content_manifest {set?}`: la orden `kiln content-manifest` como trabajo. | `mcp_production.rs` |
| R8 | `tests/coverage.rs` también comprueba que cada orden de la CLI tiene herramienta o exclusión con motivo. | `coverage.rs` |

## Security

Rutas de entrada: absolutas, longitud acotada, canonicalizadas, deben ser
carpeta. El registro solo guarda rutas que el usuario abrió. Nada escucha en red.
