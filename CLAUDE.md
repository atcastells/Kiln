# Kiln

## Godot con ventana

Kiln solo lanza Godot sin ventana (`--headless`): importar, medir y verificar.
Si una tarea necesita Godot con ventana (capturas de un proyecto que usa lo
publicado, como ASTRA), no se abre en el escritorio: en macOS `--position`
fuera de pantalla no esconde la ventana.

Se corre en un contenedor Linux con pantalla virtual y Vulkan por software:

- Imagen con Godot 4.7.1 `linux.arm64`, `xvfb` y `mesa-vulkan-drivers` (más
  la cadena de compilación del proyecto, si arranca un servidor propio).
- Sobre una copia del proyecto sin `godot/.godot` ni `target/`, para no pisar
  la caché de importación ni los binarios del anfitrión.
- `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, una importación
  previa con `godot --headless --import` (la primera en frío puede fallar con
  las fuentes) y el guion de capturas bajo
  `xvfb-run -a -s "-screen 0 1280x800x24"`.
- «Antes» y «después» se sacan con el mismo renderizador.

En ASTRA ya está hecho: `scripts/godot_window.sh --shot DIR` (ver su
`CLAUDE.md`, sección «Godot con ventana»).
