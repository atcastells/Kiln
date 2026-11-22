# K017 de punta a punta (R8): `workbench` rehecho por el MCP

- **Fecha**: 2026-11-22. Kiln en la rama `k017-modeled-parts` (sin commit), `target/release/kiln`.
- **Dónde**: una copia de ASTRA en el scratchpad de la sesión, sin `target/`,
  `.godot` ni `.git`, con `[assets] uv_per_m = 0.5`. ASTRA no se tocó.
- **Cliente**: `claude -p` con `--strict-mcp-config`, `--tools ""` y
  `--allowedTools "mcp__kiln__*"`: solo las herramientas de `kiln mcp`, sin
  ficheros ni shell. `KILN_CORE=off` y un `KILN_HOME` propio, para no tocar el
  núcleo del desarrollador.
- **Encargo**: describe el banco igual que la ficha original («encimera…,
  armario con dos cajones ocre, bastidor…, balda, tornillo, maletín»), con la
  medida y el presupuesto (300 tri). Prohíbe usar piezas del catálogo.
- **Referencia**: la ficha `workbench` de ASTRA, montada en la copia con la
  misma versión de Kiln y renderizada con el mismo `render.py`.

## Resultado

| | Original (guion a mano) | Pasada A (sin `guide`) | Pasada B (con `guide`) |
|---|---|---|---|
| Formas | 10 cajas | 22 cajas | 21 cajas + 2 `para` |
| Triángulos (presupuesto 300) | 120 | 264 | 276 |
| Medida (m) | 1,20 × 1,06 × 0,88 | 1,20 × 1,06 × 0,88 | 1,20 × 1,06 × 0,88 |
| Zonas | Top, Frame, Leg, Shelf, Cabinet, Drawer, Vice, Case | Top, Frame, Cabinet, Drawer, Handle, Vice, Case | Top, Frame, Cabinet, Drawer, Handle, Shelf, Vice, Case |
| Cajas de relleno / `todo` | — | 0 | 0 |
| `kiln verify` | — | kiln, gltf_validator, godot_import: pasan | los tres pasan |
| `publish` | — | hecho | hecho |
| Turnos / tiempo / coste | — | 20 / 98 s / 0,93 $ | 28 / 156 s / 1,19 $ |
| Errores de herramienta | — | 4 fichas rechazadas: sacó el formato de los mensajes | 0 |
| Revisión | — | 1 vuelta, aprobada | 2 vueltas: un `major` (mordaza móvil sujeta solo por el husillo), corregido |

UV: en el GLB de la pasada A, las 264 caras dan 0,5 UV por metro, con una
desviación máxima de 7·10⁻⁸ (Blender, importando el GLB). R4 queda probado
sobre un asset real.

Capturas: `shots/side-by-side.png` muestra el original y las dos pasadas a 0° y
60°, con el mismo renderizador. `shots/original-elevations.png` y
`shots/model-b-elevations.png` son las vistas de frente y de lado.

## Frente a R8, tal como está escrito

- **Cumple.** `kiln verify` en verde y publicado, solo con el MCP, sin cajas
  de relleno. Las medidas son las pedidas (±0 cm). A ojo de la sesión
  principal, el resultado está al nivel del kit, con más detalle: tiradores,
  mordazas, husillo y asa. Lo juzga el desarrollador con la captura.
- **No cumple: triángulos dentro de ±10 % del original.** Salen 264 y 276
  frente a 120. Los dos caben en el presupuesto de 300 que pide la ficha. El
  criterio comparaba con el original y no con el presupuesto; el agente gastó
  el margen en detalle.
- **No cumple del todo: mismas zonas.** El encargo no da los nombres de zona
  del original. A no tiene `Leg` ni `Shelf` (patas y balda van en `Frame`), y
  añade `Handle`. B solo cambia `Leg` por `Frame` y añade `Handle`. Los
  materiales de la paleta son los mismos.

## Lo que la prueba cambió en Kiln

- **T9b, herramienta `guide`.** `claude -p` no pasa los prompts MCP al
  modelo. En la pasada A, el vocabulario de modelado solo llegó por los
  mensajes de error, y `para` y `tube` no se usaron. Con `guide`, la pasada B
  leyó las guías de fichas y de revisión, y no tuvo ningún error.

## Lo que la prueba encontró y no se arregla aquí

- **Las medidas de apoyo de la revisión van por pieza.** Una pieza `model` es
  una sola malla, así que una forma que flota dentro de ella no aparece en
  `measures.json`. En B, la sesión encontró una separación de 1 cm leyendo las
  coordenadas, no las medidas. Propuesta para otra entrega: medir apoyo e
  intersección por forma en las piezas `model`.
- **`review_submit` corrige piezas enteras, no formas.** Para arreglar una
  forma hay que reescribir la ficha con `spec_write`. Funciona, pero cuesta
  turnos.
- **En las hojas, `ind_steel_dark` sale gris medio** por la luz, y la
  encimera y el armario se parecen. Les pasa igual al original y a las dos
  pasadas.
