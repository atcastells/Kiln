# Plan: K006 entrega B — puerta industrial piloto en ASTRA (K002-B)

- **Spec**: `spec.md`, R11–R14. **Estado**: pendiente de aprobación.
- **Dónde se trabaja**: en el repo de ASTRA. En Kiln solo cambian estos
  artefactos, salvo que aparezca un bloqueo real.

## Decisiones que necesitan tu visto bueno

| # | Decisión | Propuesta |
|---|---|---|
| PB1 | Rama de ASTRA | Rama nueva `k002b-door-pilot`. ASTRA está en `task-008-economic-circuit`, un commit por delante de su remoto y con tres ficheros sin confirmar: dime desde dónde la saco |
| PB2 | Dónde vive el modelo | Un guion de Blender en `assets/kits/industrial/src/door.py` genera `door_frame.blend` y `door_leaf.blend`. Se guardan el guion y los `.blend` |
| PB3 | Dónde se publica | `[publish] dir = "godot/assets/kiln"` |
| PB4 | Capturas | Con `scripts/godot_station.sh --shot`, que ya abre ventana. Pruebo a colocarla fuera de la pantalla; si así no dibuja, la dejo visible |

Parámetros de trabajo que diste, no canon de ASTRA: hueco de 1,4 × 2,2 m;
materiales lisos de una paleta industrial provisional, aparte de la de
interfaz; objetivo de 2.000 triángulos por asset, medido; `max_tris` de
`kiln.toml` se queda en 20.000; sin animación: la hoja se oculta al abrir,
como hoy.

## Done means

- `kiln check` en ASTRA sin errores y `scripts/kiln_check.sh` en verde.
- `build` → `verify` → `publish` de los dos assets, por CLI y por MCP, sin
  pasos a mano. Transcripción en `e2e-b.md`.
- `door.tscn` usa lo publicado. `cargo test --workspace` y
  `scripts/godot_station.sh` de ASTRA igual que su línea base.
  `git diff --stat -- crates protocol` vacío.
- Un cambio en el modelo se publica y sube la versión sin tocar `door.tscn`.
- Capturas reales: cerrada, abierta y restringida, antes y después del
  cambio, en `shots/`. La revisión visual es tuya.

## Stop and ask if

- Hace falta cambiar código de Kiln.
- La línea base de ASTRA ya está en rojo.
- Hay que tocar algo bajo `crates/` o `protocol/` de ASTRA.
- La puerta no cabe en 2.000 triángulos o en el hueco.
- Godot con ventana importa distinto que sin ella.

## Tasks

| # | Task | Files (ASTRA) | Test that proves it | Req | Runs on | Depends | Est | Status |
|---|------|---------------|---------------------|-----|---------|---------|-----|--------|
| B0 | Línea base antes de tocar: batería de Rust, `godot_station.sh`, `kiln_check.sh` | — | Resultados guardados | R13 | main | — | 5m/1M | todo |
| B1 | Configuración: `[anchors.doorway]` con el hueco, `[publish] dir`, paleta industrial provisional | `kiln.toml`, `kiln/materials.toml` | `kiln check` sin errores; `kiln_check.sh` igual que en B0 | R11 | main | B0 | 4m/1M | todo |
| B2 | Guion que modela marco y hoja y guarda los dos `.blend`. Origen en el suelo y en el centro del hueco, ancho en X | `assets/kits/industrial/src/door.py`, los dos `.blend` | El catálogo de Kiln lista las dos piezas con sus medidas; dos ejecuciones del guion dan el mismo hash estructural | R11 | implementer | B1 | 6m/2M | todo |
| B3 | Fichas `door_industrial_frame` (anclaje `doorway`, variante `restricted` que cambia solo una franja) y `door_industrial_leaf` | `kiln/specs/*.toml` | `kiln check` sin errores; el montaje cabe en el hueco y sus triángulos quedan medidos | R11 | implementer | B2 | 4m/1M | todo |
| B4 | Recorrido completo por CLI y por MCP | `godot/assets/kiln/`, `e2e-b.md` (Kiln) | Las tres comprobaciones pasan en los dos assets; lo publicado lleva `asset.json` versión 1 | R12 | main | B3 | 6m/2M | todo |
| B5 | La puerta en la escena: `mesh` contiene la hoja; hijos nuevos para el marco y su variante restringida. Colisión, `ray` y `set_state` no cambian de forma | `godot/station/door.tscn`, `door_view.gd`, un test nuevo en `godot/tests/` | Test rojo primero: abierta oculta la hoja y deja el marco; restringida cambia el marco. `reconcile_test.gd` sigue en verde | R13 | main | B4 | 8m/3M | todo |
| B6 | Modificación: un detalle del modelo cambia, se monta, verifica y publica | `door.py`, un `.blend` | La versión sube a 2 y `git diff -- godot/station/door.tscn` queda vacío | R14 | main | B5 | 5m/2M | todo |
| B7 | Capturas con ventana, antes y después de B6 | `shots/` (Kiln) | Seis PNG de más de 1 KB, mirados uno a uno | R14 | main | B5, B6 | 6m/2M | todo |
| B8 | Verificación independiente, revisiones y handoff final | `handoff.md` (Kiln) | Informe del verifier | todos | verifier | B7 | 6m/2M | todo |

Las capturas de «antes» se sacan al acabar B5 y las de «después» al acabar B6.

## Riesgos

- Importar en el proyecto de Godot de ASTRA ejecuta sus plugins de editor
  (`addons/`). Es código del propio repo.
- Una puerta hecha por guion puede quedar pobre: la revisión juzga cadena y
  aspecto por separado, y mejorar el `.blend` no exige tocar nada más.
- No sé todavía si `.gitignore` de ASTRA deja pasar los `.import` y los GLB
  publicados; lo miro en B4.
