# Kiln

Motor local de assets para juegos en Godot: lee un proyecto por su `kiln.toml`,
monta el catálogo, desgrana peticiones en piezas y construye y verifica modelos
con Blender y Godot. Rust, sin dependencias fuera de Rust en el servidor.

## Comandos

```bash
cargo build --release      # binario target/release/kiln
cargo test --workspace     # 423 pasan, 0 fallan, 15 ignorados; ~14 s en caliente
cargo test --workspace -- --ignored   # los 15: piden blender y godot en el PATH
cargo clippy --workspace --all-targets -- -D warnings   # limpio, sin avisos
scripts/ui_test.sh         # UI en Chrome sin ventana; pide node >= 22 y Chrome
target/release/kiln serve --project examples   # http://127.0.0.1:4330
```

## Estructura

Todo es actual; no hay partes heredadas.

- `crates/kiln-core`: biblioteca (catálogo, comprobaciones, GLB, Godot,
  contenido, reglas de UI).
- `crates/kiln-cli`: binario `kiln` (servidor, MCP en `crates/kiln-cli/src/mcp`, agente,
  verificación) y la página web en `crates/kiln-cli/ui`.
- `skills/`: skills que Kiln pasa a `claude -p` (asset, context, review).
- `examples/`: Puerto Niebla, juego inventado que sirve de fixture.
- `.sdlc/`: una carpeta por entrega (`k0NN-*`, `eN-*`).

## Convenciones

- Código, identificadores y commits en inglés; textos de la UI en castellano.
- Kiln no supone nada de un proyecto concreto: reglas, carpetas y nombres van
  en su `kiln.toml`.
- Antes de proponer un commit: `cargo test --workspace` y el `clippy` de arriba.

## Seguridad

- El servidor solo escucha en `127.0.0.1`; solo la propia página lanza al agente.
- Kiln quita `ANTHROPIC_API_KEY` y `ANTHROPIC_AUTH_TOKEN` al lanzar `claude`.
- NUNCA commitear `.env*`, certificados ni keystores.

## Git

- `add`/`commit` solo cuando se pide, con confirmación.
- `push` de la rama de trabajo y abrir su PR (`gh pr create`) los hace Claude
  al cerrar una entrega. NUNCA `push --force` ni `push` directo a `main`.
- NUNCA `merge`, `rebase`, `reset`, `stash`, `tag`, `cherry-pick` (denegados
  en `.claude/settings.json`): unir un PR es del desarrollador.
- `checkout` y `branch` solo si el desarrollador lo pide. Rama base: `main`.

## Lo que Claude falla

Vacío por ahora. Cuando un error se repita dos veces, va aquí.

## Godot con ventana

Kiln solo lanza Godot sin ventana (`--headless`): importar, medir y verificar.
Si una tarea necesita Godot con ventana (capturas de un proyecto que usa lo
publicado, como ASTRA), no se abre en el escritorio: en macOS `--position`
fuera de pantalla no esconde la ventana.

Se corre en un contenedor Linux con pantalla virtual y Vulkan por software:

- Imagen con Godot 4.7.1 `linux.arm64`, `xvfb` y `mesa-vulkan-drivers` (más
  la cadena de compilación del proyecto, si arranca un servidor propio).
- Sobre una copia del proyecto sin su caché de importación (`.godot`) ni
  `target/`, para no pisar la caché ni los binarios del anfitrión.
- `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, una importación
  previa con `godot --headless --import` (la primera en frío puede fallar con
  las fuentes) y el guion de capturas bajo
  `xvfb-run -a -s "-screen 0 1280x800x24"`.
- «Antes» y «después» se sacan con el mismo renderizador.

En ASTRA ya está hecho: `scripts/godot_window.sh --shot DIR` (ver su
`CLAUDE.md`, sección «Godot con ventana»).
