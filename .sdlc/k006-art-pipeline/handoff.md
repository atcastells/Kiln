# Handoff: K002 de ASTRA (K006 en Kiln)

- **Fecha**: 2026-11-17
- **Estado**: **parcial**. La entrega A (herramientas en Kiln) está hecha y
  fusionada. La entrega B (la puerta piloto en ASTRA) no se ha empezado.
- **El criterio de cierre del encargo no se cumple todavía**: pide «una
  puerta visual utilizable en ASTRA, coordinada por Kiln y validada en
  Godot», y esa puerta no existe.

## Qué pruebas del encargo están cubiertas

| ID | Prueba | Estado | Evidencia |
|---|---|---|---|
| K2-01 | Descubrimiento | hecha | `kiln tools`; tests `tools::*`, `mcp_read::status_lists_the_tools…` |
| K2-02 | Blender genera o modifica un asset | hecha en Kiln, no con la puerta | tests `blender_piece`, `blender_anchor`, `blender_contract` |
| K2-03 | Reproducibilidad | hecha | `blender_contract`: dos montajes, mismo hash estructural |
| K2-04 | Integridad | hecha | `validate::tests` (un test por hallazgo), `godot_verify` |
| K2-05 | Godot importa e instancia | hecha con `hoist` de `examples`, no con la puerta | `godot_verify`, `blender_anchor` |
| K2-06 | La puerta representa la entidad de ASTRA | **no hecha** | — |
| K2-07 | MCP/CLI | hecha | ver inventario abajo |
| K2-08 | Regresiones | hecha | batería de Kiln; `index` de `examples` idéntico; `scripts/kiln_check.sh` de ASTRA pasa con el binario de `27024f3` (4 escenas, 0 errores, 0 avisos) |
| K2-09 | Revisión artística con capturas | **no hecha** | Sin permiso para abrir Godot con ventana |

## Kiln

- **Rama y SHA**: `main` en `76521f8` (PR #21 fusionado, cabeza `c3c946f`).
- **Fuera de `main`**: el commit `27024f3` de la rama `k006-art-pipeline` se
  subió después de la fusión. Corrige que Kiln siguiera un enlace simbólico
  al escribir registros, `validation.json`, `asset.json` y `pieces.json`.
  PR #22, abierto y sin fusionar.
- **Pruebas** (sobre `27024f3`): `cargo test --workspace` 347 en verde, 7
  ignorados; Clippy limpio; los 7 ignorados pasan con Blender y Godot reales.

## Herramientas MCP (30)

Verificadas contra Blender, Godot y el validador reales, por MCP
(`e2e-a.md`):

| Herramienta | Qué hace |
|---|---|
| `tools` | Herramientas externas y versiones |
| `build` | Monta en Blender y escribe `asset.json` |
| `verify` | Comprobaciones propias, glTF Validator, importación en Godot |
| `publish` | Copia lo verificado a `[publish] dir` y hace que Godot importe |
| `job`, `jobs` | Siguen los trabajos |
| `spec_get` | Ficha, contrato y último informe |

El recorrido se repitió por MCP sobre `27024f3`, después de todas las
correcciones: mismo resultado, con 17 avisos de `kiln` en lugar de 11 (causa
sin mirar).

Verificadas por sus tests, que no lanzan herramientas externas: `status`,
`catalog`, `check`, `tree`, `content`, `spec_write`, `spec_layout`,
`context_get`, `context_propose`, `context_edit`, `settle`, `bind`,
`material_set`, `material_validate`, `zones_split`, `new_game_options`,
`new_game`, `review_sheets`.

Verificadas solo por CLI, no por MCP: `paint` (`kiln paint`, con variante) y
`godot_measure` (`kiln godot`, que ahora exporta también los `.blend`).

Sin verificar en este encargo: `review` (necesita `claude` y gasta
suscripción), `search` e `import` (red y claves).

## Integraciones

- **Reutilizadas**: Blender sin ventana (`assemble.py`, `paint.py`,
  `repaint.py`, `split.py`, `render.py`), Godot sin ventana (`measure.gd`,
  `content_manifest.gd`), los trabajos, las variantes, los assets hechos de
  assets, `sources.json`.
- **Añadidas**: detección de herramientas; lanzador único con plazo y
  registro; piezas con fuente `.blend` (`export_piece.py`); anclajes;
  lectura de nodos y hash estructural del GLB; validación propia; adaptador
  del glTF Validator; importación de prueba en Godot (`import_check.gd`);
  contrato `asset.json`; `verify`; `publish`.
- **No integradas**: glTF-Transform (solo se detecta); el puente MCP de
  Blender instalado en Claude Desktop (no probado; decisión D4 pendiente de
  reconfirmar).

## Dependencias

| Herramienta | Versión | Licencia | Nota |
|---|---|---|---|
| Blender | 5.2.3 LTS | GPL | Se lanza como proceso. Licencia de memoria |
| Godot | 4.7.1 stable | MIT | Licencia de memoria |
| Khronos glTF Validator | 2.0.0-dev.3.10 | Apache-2.0 (GitHub, consultado) | Opcional. En esta máquina, compilado desde la etiqueta del repositorio: el binario oficial es solo x86_64 |
| glTF-Transform | no instalado | MIT (GitHub, consultado) | Solo detectado |
| `sha2` (Rust) | 0.10 | — | Única dependencia nueva de Kiln; licencia sin consultar |

## Asset piloto

**No existe.** En ASTRA no hay ficha, `.blend` ni GLB de la puerta:
`kiln/specs/` y `assets/kits/` están vacíos y el repo no tiene ningún GLB.
El recorrido completo se ha probado con `hoist`, el ejemplo de Kiln:

- Verificación: `kiln` pasa (0 errores, 11 avisos), `gltf_validator` pasa,
  `godot_import` pasa.
- Los 11 avisos son piezas pendientes (cajas en lugar de modelos por buscar)
  y materiales sin clave de la paleta.

## Limitaciones

- En Kiln no hay piezas-caja: una caja solo sale como sustituto de una pieza
  que falta, y queda como aviso. Las piezas reales son GLB o `.blend` de un
  kit.
- Kiln monta piezas; no modela. Los `.blend` los hace quien modela.
- Sin animación: `moves` es una etiqueta en el GLB, no una animación.
- Sin soporte comprobado para esqueletos, pesos ni personajes: la validación
  no mira `skins` ni animaciones.
- Texturas: Kiln tiene paleta con conjuntos de texturas, pero ASTRA no tiene
  paleta 3D ni resolución decidida (q16 abierta).
- Iluminación y señalización son de Godot; Kiln no las produce.
- Un retoque a mano en el `.blend` del montaje se pierde al volver a montar.
- Importar en el proyecto de Godot del juego ejecuta sus plugins de editor.
- `[tools] build` y `[tools] cache` no están confinados al proyecto.
- `validation.json` no va firmado.
- La página web no tiene botones para verificar ni publicar.
- `verify` hace las tres comprobaciones sobre el GLB base; las variantes solo
  pasan la de fichero.

## Aplazado

- Entrega B: R11–R14 (ficha y piezas de la puerta, recorrido en ASTRA,
  `door.tscn`, modificación y capturas).
- Decisiones abiertas en ASTRA: q15 (medidas de puerta; Rust y el greybox
  fijan 1,4 × 2,2 m), q16 (texturas), paleta 3D, `max_tris` (20.000 en
  `kiln.toml`, 2.000 en el contexto).
