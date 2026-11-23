# Spec: K018, el primer asset conversando

| # | Requisito | Dónde |
|---|-----------|-------|
| R1 | `setup` (MCP) y `kiln setup` (terminal) buscan Blender, Godot y Claude Code en el PATH y en los sitios de siempre (apps de `/Applications` y `~/Applications`, Steam, Homebrew, Snap, Flatpak, `~/.local/bin`), dicen versión, si sirve y el enlace de descarga del que falte, y guardan lo encontrado en `$KILN_HOME/tools.json`. | `kiln-core/src/tools.rs`, `ops/setup.rs` |
| R2 | Un `kiln.toml` que deja el nombre a secas (`blender`, `godot`) usa el programa guardado si ese nombre no está en el PATH; uno que nombra su programa no cambia. Claude, igual en `agent::claude`. | `Config::parse`, `tools::resolve` |
| R3 | Seguridad: por MCP solo se elige entre lo que Kiln encontró. Una ruta cualquiera solo la guarda la persona (`kiln setup --blender RUTA`). `config_set` no cambia programas. | `ops/setup.rs`, `ops/start.rs` |
| R4 | `example_create`: juego de ejemplo en una carpeta nueva o vacía, abierto y de confianza, con paleta de siete colores y un farol de formas listo para montar. | `ops/start.rs` |
| R5 | `config_get` / `config_set` para 7 ajustes esenciales; el resto del fichero y sus comentarios quedan igual; las carpetas, dentro del proyecto; el fichero debe seguir cargando. | `ops/start.rs` |
| R6 | `palette_add`: un color liso en la primera tabla de materiales. | `ops/start.rs` |
| R7 | `preview`: hoja PNG de 768 px con frente, perfil, planta y tres cuartos, dibujada en Rust (PNG y DEFLATE propios, sin dependencias), devuelta como imagen MCP y guardada junto al modelo. Sirve para montajes y para assets generados. | `kiln-core/src/preview.rs`, `png.rs` |
| R8 | `open_viewer`: abre la página del núcleo en `#/build/<id>` (o `#/spec/<id>` si es generado) y devuelve el enlace; `KILN_NO_BROWSER` lo evita en los tests. | `mcp/start.rs` |
| R9 | Prompt `kiln-start` (y `guide` con `start`): de las herramientas a publicar, en orden, sin `reviewer = "kiln"` (D1). | `skills/start/SKILL.md` |
| R10 | `status` da `assets` (montado, verificado, publicado) y el siguiente paso del recorrido. | `ops/read.rs`, `ops/start.rs` |
| R11 | Un juego nuevo trae `[publish] dir`: `assets/kiln` en un proyecto de Godot (con `godot_project = "."`), `publicado` si no. | `kiln-core/src/new_game.rs` |
