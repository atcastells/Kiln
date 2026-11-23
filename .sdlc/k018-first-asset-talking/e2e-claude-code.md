# E2E: el recorrido en Claude Code (terminal), 2026-11-23

Claude Code 2.1.330 en el contenedor de desarrollo (Linux), modelo Sonnet,
`kiln mcp` registrado con `--mcp-config` y su propio `KILN_HOME`. Solo las
herramientas de Kiln permitidas. La persona escribe; el agente decide solo.
En el contenedor no hay Blender ni Godot: «Blender» es un doble de pruebas
(`kiln setup --blender`, como haría la persona) que escribe siempre un farol.

1. `/mcp__kiln__kiln-start`. Claude Code lista los cuatro prompts de Kiln como
   comandos. El agente llama a `setup`, explica en una frase qué hace Blender
   y qué hace Godot, da los dos enlaces y espera.
2. «Ya instalé Blender. Godot lo instalo luego. Prefiero un ejemplo, ponlo en
   …/juegos/Mi primer juego». `setup` lo encuentra; `example_create`;
   `spec_get` farol; `build`; `job` (hecho); `preview`. Describe lo que se ve
   (2,92 m, 100 triángulos, base de piedra, poste, cristal, tejadillo), ofrece
   `open_viewer` y pregunta qué cambiar. Recuerda que Godot hará falta para
   verificar y publicar.
3. «Hazme el farol con el cristal azul y luego un banco de madera sencillo».
   `catalog` materials, `guide` asset, `palette_add` (cristal_azul),
   `spec_write` farol y banco (sin hallazgos), dos `build`, dos `preview`. El
   agente compara la imagen con su ficha y avisa de que el montaje no cambió:
   era el doble de pruebas. `preview` le da lo bastante para juzgar.

Hallazgos que cambiaron el código:

- Con Godot ausente el agente esperaba a los dos; ahora `setup` dice que
  Godot puede instalarse después (solo hace falta para verificar y publicar).
- En la terminal la imagen no se ve: el recorrido pide dar siempre la ruta de
  `image`.

Falta en un Mac con Blender y Godot reales, y en la app de escritorio y la
extensión de VS Code (ver `review.md`).

# E2E: el recorrido en el Mac con Blender y Godot reales, 2026-11-23

macOS, rama en un worktree aparte, `KILN_HOME` temporal, Claude Code 2.1.330
sin interfaz (`claude -p` con `--resume`), solo herramientas de Kiln.

- `kiln setup`, sin rutas: Blender 5.2.3 y Godot 4.7.1 (Homebrew), Claude
  2.1.330 (`~/.local/bin`).
- Turno 1, `/mcp__kiln__kiln-start` (15 s): `setup`, `status`; ofrece el
  ejemplo o su juego.
- Turno 2, ejemplo (21 s): `example_create`, `spec_get`, `build` (92
  triángulos en Blender), `preview`. El agente ve en la planta que el
  tejadillo de 4 lados está girado 45° respecto al cristal.
- Turno 3, «verifícalo y publícalo» (20 s): `config_get`, `verify` (Kiln,
  glTF Validator y Godot pasan), `publish` a `publicado/farol` (versión 1).
- Turno 4, un banco de madera (40 s): `guide` asset, `catalog`, `spec_write`
  sin hallazgos, `build` (132 triángulos), `preview`: 1,6 x 0,86 x 0,44 m,
  listones en madera clara y laterales oscuros.

Unos 96 s de agente de punta a punta, del comando al farol publicado y un
segundo asset montado.

Cambiado tras esta prueba: el tejadillo del farol de ejemplo pasa a 8 lados,
para que no se vea girado respecto al cristal.
