# Auditoría de Kiln para K006 (Fase A del encargo)

- **Fecha**: 2026-11-17
- **Kiln**: rama `main`, SHA `72efc12`, `kiln-cli` y `kiln-core` 0.1.0.
- **Máquina**: Blender 5.2.3 LTS, Godot 4.7.1 stable, ambos en el `PATH`.
- **Batería** (`cargo test --workspace`, hoy): 270 tests en verde, 0 fallos,
  2 ignorados.
- **Ignorados, corridos a mano hoy** (`cargo test -p kiln-cli -- --ignored`):
  `turn_reaches_copies_of_models_like_boxes` (Blender 5.2.3) y
  `godot_export_matches_fixture_and_catches_a_moved_door` (Godot 4.7.1), los
  dos en verde.

Estados: **probado** (hay un test que lo ejerce y hoy pasa), **sin verificar**
(el código existe, ningún test lo ejerce con la herramienta real) y
**ausente**.

## Matriz de capacidades

| Capacidad | Estado | Dónde | Nota |
|---|---|---|---|
| Montar una ficha en Blender sin ventana (`build`) | probado | `production.rs:26-113`, `blender/assemble.py`; test `blender_turn.rs` | Solo se prueba el giro de las copias |
| Conservar el `.blend` del montaje | sin verificar | `assemble.py:197`, `<build>/<id>/.kiln/<id>.blend` | Se regenera en cada `build`: un retoque a mano se pierde |
| Exportar GLB con extras (`kiln_part`, `kiln_kind`, `kiln_moves`, `kiln_only_in`) | sin verificar | `assemble.py:199-201` | `glb.rs` no lee nodos ni extras; solo el test de K005 lee nodos |
| Volver a vestir materiales sin montar (`paint`, variantes) | sin verificar | `blender/repaint.py`, `production.rs` | |
| Zonas de material (`zones_split`) | sin verificar | `blender/split.py` | |
| Hojas y medidas para el revisor (`review`) | sin verificar | `blender/render.py`, `review.rs` | Necesita `claude` |
| Montar en un Blender abierto, por un puente MCP de Blender (`KILN_PLAN`) | sin verificar | README «Entrega 4»; `assemble.py` | No hay ningún puente MCP de Blender configurado en esta máquina; no se ha probado |
| Una pieza que es una caja a propósito | ausente | `build.rs:227-243` | La caja solo sale como sustituto de una pieza que falta, y lleva `todo` |
| Detectar Blender y su versión | ausente | — | `[tools] blender` se usa a ciegas (`config.rs:138`) |
| Tiempo máximo y carpeta de trabajo de Blender/Godot | ausente | `production.rs:116-135` | `Command::output()` sin límite ni `current_dir` |
| Errores estructurados de Blender | ausente | `production.rs:129-135` | Seis últimas líneas de stderr unidas con `\|` |
| Un único lanzador de Blender | ausente | `production.rs:116`, `review.rs:63-81` | Duplicado |
| Godot mide y exporta piezas `.res/.tres/.tscn` (`godot_measure`) | sin verificar | `godot/measure.gd`, `godot.rs:48-128` | Éxito = existe el fichero |
| Godot lista ids de escenas (`content-manifest`) | probado | `godot/content_manifest.gd`; test ignorado `content` | Sin herramienta MCP |
| Primera importación del GLB en Godot | sin verificar | `production.rs:218-226` | Se ignora el código de salida; no se comprueba que importe bien |
| Vincular materiales en el `.import` | probado | `godot.rs:137`, tests `link_tests` | Solo el texto del `.import`, sin Godot |
| Leer un GLB: caja, triángulos, materiales, imágenes | probado | `glb.rs:9-17`; tests `glb::tests` | |
| Validar un GLB (estructura, referencias rotas) | ausente | — | Ni propio ni Khronos glTF Validator |
| Inspeccionar/optimizar (glTF-Transform) | ausente | — | No instalado |
| Hash de lo exportado | ausente | — | Solo un ETag con `DefaultHasher` (`serve.rs:563`) |
| Procedencia y licencia de piezas de fuera | probado | `sources.rs:332-345`, `license.rs`; tests `sources::tests`, `license::tests` | Por kit, no por asset montado |
| Manifiesto o versión por asset | ausente | — | Solo `report.json` (`size_m`, `tris`, `built_at`) y el estado `stale` |
| Origen del asset | sin verificar | `spec.rs:87`, `assemble.py:99` | Centro de la base, Y arriba; no configurable |
| Puntos de conexión / anclajes | ausente | README «Lo que viene» | Los marcadores `conn_*`/`anchor_*` existen solo en el contenido de juego (K002 de Kiln) |
| Reglas de compatibilidad entre conectores | ausente | README «Validar contenido → Pendiente» | |
| Validar contenido de juego desde un esquema | probado | `content/`, K001/K002 de Kiln | |
| Publicar un asset en el proyecto de juego | ausente | — | Solo entra si `[tools] build` apunta dentro del proyecto de Godot |
| Decir qué herramientas hay y cuáles faltan | ausente | `ops/read.rs:28-61` | `status` no las menciona |
| MCP: 27 herramientas y 3 prompts | probado | `mcp/*.rs`; tests `mcp_*.rs` | Los tests no lanzan Blender, Godot, `claude` ni `curl` |
| CLI: `index check mcp serve build paint review godot content-manifest` | probado en parte | `main.rs:92-183` | |

## MCP ↔ CLI

- Con equivalente en CLI: `check`, `catalog` (≈ `index`), `build`, `paint`,
  `godot_measure` (= `godot`), `review`.
- Solo MCP: `status`, `tree`, `spec_get`, `content`, `import`, `job`, `jobs`,
  `review_sheets`, `spec_write`, `spec_layout`, `context_get`,
  `context_propose`, `context_edit`, `settle`, `bind`, `material_set`,
  `material_validate`, `zones_split`, `search`, `new_game_options`, `new_game`.
- Solo CLI: `content-manifest`.

## Dependencias externas

| Herramienta | Versión aquí | Licencia | Hoy en Kiln |
|---|---|---|---|
| Blender | 5.2.3 LTS | GPL (se lanza como proceso, no se enlaza) | Usada |
| Godot | 4.7.1 stable | MIT | Usada |
| Khronos glTF Validator | no instalado | Apache-2.0 (GitHub `KhronosGroup/glTF-Validator`, consultado hoy) | No |
| glTF-Transform | no instalado | MIT (GitHub `donmccurdy/glTF-Transform`, consultado hoy) | No |

Las licencias de Blender y Godot las doy de memoria; no las he vuelto a leer
en la fuente.

## ASTRA (rama `task-008-economic-circuit`, con cambios sin confirmar)

- Una puerta es un enlace entre dos módulos con id persistente
  (`crates/astra_sim/src/layout.rs:21-30`). Estado: abierta o cerrada
  (`door.rs:17-19`), más `restricted` (credencial). No hay «bloqueada» ni «sin
  energía».
- Las puertas se generan con la semilla; no están en ficheros de contenido.
  El cliente recibe `DoorView {id, a_id, a_connector, b_id, b_connector, open,
  restricted}` (`astra.proto:116`).
- Godot: `godot/station/door.tscn` es un `StaticBody3D` con `mesh`
  (`BoxMesh` 1,4 × 2,2 × 0,15), `shape` y `ray`. `door_view.gd` tiñe y, al
  abrir, esconde la malla y apaga la colisión. `world_view.gd:210-221` la
  coloca en el marcador `conn_<conector>` del módulo y la gira 0 o 90°.
- `interact.gd:44-45` exige `StaticBody3D` con `set_state`;
  `tests/reconcile_test.gd:21,31` lee los hijos `mesh` y `shape`.
- Hueco de la pared: 1,4 × 2,2 m, pared de 0,2 m
  (`godot/station/module_view.gd:3-7`; 14 dm en `astra_sim/src/space.rs:23`).
- No hay ningún GLB en el repo. `kiln/materials.toml` y `kiln/specs/` están
  vacíos. `kiln.toml` no tiene `[tools] build` ni `[materials] godot`.
- `scripts/kiln_check.sh` usa `$HOME/Kiln/target/debug/kiln` sin fijar
  versión.

Lo de ASTRA viene de leer el código, no de ejecutarlo.
