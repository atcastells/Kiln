# Review: K018, el primer asset conversando

## Verificado

- `cargo test --workspace`: 709 pasan, 0 fallan, 31 ignorados (antes 694).
  Nuevos: búsqueda de programas y `resolve` (`kiln-core/src/tools.rs`),
  PNG y DEFLATE con un inflador de prueba (`png.rs`), nodos, colores y hoja
  (`preview.rs`), `setup` que se niega a un programa que no encontró
  (`ops/setup.rs`), ajustes que guardan los comentarios, paleta y ejemplo
  (`ops/start.rs`), el recorrido entero por MCP sin Blender
  (`tests/mcp_start.rs`), el prompt `kiln-start` en orden y sin el revisor
  propio (`tests/mcp_prompts.rs`), y la superficie de herramientas fijada
  (`tests/mcp_guards.rs`, 44 herramientas y 4 prompts).
- `cargo clippy --workspace --all-targets -- -D warnings`: limpio.
- `preview` sobre un farol de formas: `shots/preview-farol.png` (13 KB).
- Claude Code en la terminal, de `/mcp__kiln__kiln-start` a dos assets
  montados y vistos: `e2e-claude-code.md`.

## Seguridad

- Ningún camino del MCP hace que Kiln ejecute un programa que elija el agente:
  `setup` solo guarda lo que encontró en el PATH o en los sitios de siempre;
  `config_set` no toca `[tools] blender` ni `godot`; `publish.dir` y
  `tools.godot_project` deben quedar dentro del proyecto. Lo demás lo decide
  la persona en su terminal (`kiln setup --blender RUTA`, `kiln trust`).
- `preview` lee un GLB que puede venir de fuera: reutiliza la lectura
  defensiva de `mesh.rs` (límite de vértices, accessors comprobados) y
  recorre los nodos una vez cada uno, así que un ciclo no cuelga.

## Pendiente

- No probado con Blender y Godot reales ni en la app de escritorio y la
  extensión de VS Code de Claude Code: en el contenedor no se pudieron
  instalar. Es el siguiente paso, en el Mac.
- Tras actualizar `kiln`, el núcleo viejo sigue sirviendo hasta `kiln stop`
  (ya avisa en stderr, pero un principiante no lo ve). Para la fase 4.
- `preview` dibuja colores planos: sin texturas, ni variantes, ni esqueleto
  en pose.
