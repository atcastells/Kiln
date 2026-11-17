# Spec: Kiln como cadena de producción artística para ASTRA (K006)

- **Derived from**: `intent.md` (aprobado 2026-11-17), con `request.md` (el
  encargo literal: fases A–E, pruebas K2-xx) y `audit.md`
- **Status**: approved (2026-11-17), con D1–D10; D8 (rama de ASTRA) y D9 se concretan antes de la entrega B
- **Approved by**: Aaron Castells

La auditoría (`audit.md`) dice qué hay: Kiln ya monta en Blender, exporta GLB
y habla con Godot. Faltan siete cosas: saber qué herramientas hay, lanzarlas
con límites y errores legibles, admitir piezas con fuente `.blend`, validar
el GLB, comprobar que Godot lo importa, un contrato por asset (con hash y
anclajes) y un paso de publicar. Este spec añade solo eso.

## Decisiones que necesitan tu visto bueno

- **D1. La fuente editable es el `.blend` de cada pieza, más la ficha que las
  monta.** El `.blend` del montaje es un derivado: `build` lo regenera
  (`assemble.py:197`) y un retoque a mano dentro de él se pierde. Una puerta
  hecha solo con la ficha no es viable hoy: en Kiln una caja solo sale como
  sustituto de una pieza que falta y queda marcada como pendiente
  (`build.rs:227-243`), y el kit de ASTRA está vacío. Por eso las piezas de
  la puerta se modelan en `.blend` dentro del kit, y Kiln las exporta (R3).
  Los `.blend` del piloto los genero con un script de Blender que se guarda a
  su lado; ese script lo corre quien modela, no Kiln.
- **D2. Dos entregas.** *Entrega A* (solo Kiln): R1–R10, probada con
  `examples`. *Entrega B* (ASTRA, más lo que haga falta ajustar en Kiln):
  R11–R14, la puerta. Cada una con su `plan.md` y su PR.
- **D3. Validadores externos, opcionales y solo detectados.** Kiln no instala
  nada. Si `gltf_validator` (Khronos, Apache-2.0) está en el `PATH` o en
  `[tools]`, se usa; si no, consta como «ausente» y valen las comprobaciones
  propias (R5). glTF-Transform (MIT, pide Node) no se integra: hoy no
  resuelve ninguna carencia concreta. Solo se detecta y se informa.
- **D4. No se integra un puente MCP de Blender.** El encargo pide priorizar
  uno existente si supera las pruebas. *Corrección (2026-11-17, durante la
  entrega A)*: aprobé esta decisión diciendo que en esta máquina no había
  ninguno, y no es cierto: hay uno instalado como extensión de Claude Desktop
  (`ant.dir.gh.blender.blender-mcp`). No está conectado a Claude Code ni a
  Kiln y no lo he probado. La entrega A no lo necesita (Blender sin ventana
  cubre montar, exportar y verificar), pero la decisión queda **pendiente de
  que la confirmes** con el dato correcto.
- **D5. Medidas: la puerta se ajusta al hueco que ya existe**, 1,4 × 2,2 m en
  pared de 0,2 m (`module_view.gd:3-7`, `space.rs:23`). La pregunta q15 del
  contexto de ASTRA sigue abierta con opciones más estrechas (0,9 y 1,2 m);
  cambiar el hueco es cambiar Rust, y eso está fuera.
- **D6. Estados visuales: solo los que Rust tiene.** Abierta, cerrada y
  restringida. No se dibuja «bloqueada» ni «sin energía» («La presentación no
  crea estados operativos», `docs/art/decisiones/4.4-…:22`).
- **D7. Los tests que lanzan Blender o Godot siguen aparte** (`#[ignore]`,
  como K005). La orquestación los corre una vez por entrega.
- **D8. Cambios en ASTRA (entrega B).** ASTRA está en
  `task-008-economic-circuit` con cambios sin confirmar. Necesito que digas:
  (a) en qué rama trabajo, y (b) si lo de ASTRA lleva su propia cadena allí
  (`docs/tasks/`, `.sdlc/`) o basta con esta. Toco `kiln.toml`, `kiln/`,
  `assets/kits/`, `godot/station/door.*`, la carpeta de assets publicados y,
  para las capturas, un test de Godot y su entrada en `scripts/`. Nada bajo
  `crates/` ni `protocol/`.
- **D9. Capturas (K2-09).** Necesitan Godot con ventana, y el `CLAUDE.md` de
  ASTRA lo prohíbe sin autorización. La pido aquí para la entrega B.
- **D10. Paleta provisional.** ASTRA no tiene paleta 3D
  (`kiln/materials.toml` vacío; la dirección de arte la deja pendiente). La
  puerta usará pocos materiales lisos, marcados como provisionales, sin
  texturas (q16 abierta).

## Requirements

### Entrega A — Kiln

| # | Requirement | Traces to | Acceptance criterion |
|---|-------------|-----------|----------------------|
| R1 | **Herramientas.** Kiln detecta Blender, Godot, `gltf_validator`, `gltf-transform`, `claude` y `curl`: ruta y versión. Avisa si Blender es anterior a 5.0 o Godot anterior a 4.7, que son las versiones con las que Kiln se ha probado. Sale en `status` (MCP) y en `kiln tools` (CLI) | K2-01; Fase B | Test con binarios falsos: uno presente, uno ausente, uno viejo; los tres se informan bien. A mano: informa 5.2.3 y 4.7.1 |
| R2 | **Un lanzador.** Blender y Godot se lanzan desde una sola función: carpeta de trabajo propia, tiempo máximo (`[tools] timeout_s`, 600 por defecto) y un resultado con código de salida, duración, error resumido y ruta del registro completo. `production.rs`, `review.rs` y `content.rs` la usan | Fase B («registros estructurados», «tiempos máximos») | Test: un proceso falso que duerme se corta al vencer el plazo y el error lo dice; uno que falla deja el registro. Los tests de hoy y los dos ignorados siguen en verde |
| R3 | **Piezas con fuente `.blend`.** Un `.blend` dentro de un kit es una pieza: Kiln lo exporta a GLB en la caché con un script suyo y ajustes fijos, y anota el sha256 del `.blend`. Si el `.blend` cambia, la pieza y los assets que la usan constan como desfasados. Ningún cliente pasa scripts | K2-02; Fase B («conservar el `.blend`», «repetir una exportación») | Test ignorado: un `.blend` de fixture entra en el catálogo con sus medidas; tocarlo lo marca desfasado |
| R4 | **Leer la estructura del GLB.** `glb.rs` da además nodos (nombre, padre, transformación), extras, triángulos y materiales por malla, y un hash estructural | K2-03, K2-04 | Test sobre un GLB de fixture: nodos, extras y hash esperados |
| R5 | **Validar un GLB sin herramientas de fuera.** Hallazgos: no es un GLB válido; referencias rotas (búfer, imagen, material, nodo); nombres de nodo repetidos o vacíos; escala negativa o no uniforme en la raíz; material fuera de la paleta; triángulos sobre el presupuesto; medidas que no cuadran con la ficha; pieza pendiente (`kiln_todo`); anclaje declarado que falta | K2-04 | Un test por hallazgo, con GLB rotos a propósito |
| R6 | **Anclajes.** La ficha admite `[[anchors]]` (`id`, `class`, `at`, `turn`). El montaje los exporta como nodos vacíos `anchor_<id>`. `kiln.toml` declara las clases (`[anchors.<class>]`, con `fits` y, opcional, `opening_m`). `check` avisa de clase desconocida y de un asset que no cabe en el hueco de su clase | Fase D («puntos de montaje… reglas de compatibilidad») | Tests de `check`. Test ignorado: el GLB montado lleva el nodo en su sitio |
| R7 | **Verificar**: `kiln verify --asset ID` y herramienta MCP `verify` (trabajo). Corre R5, `gltf_validator` si está, y la importación en Godot: un proyecto temporal en la caché carga el GLB, lo instancia y devuelve nodos, caja y errores. Escribe `<build>/<id>/validation.json`. Un validador ausente no es un error. No es `check`: `check` es inmediato y no lanza nada; esto lanza Godot | K2-04, K2-05; «Validar el resultado» | Test ignorado con Godot: un GLB bueno pasa, uno truncado falla con el motivo. Sin `gltf_validator`: consta «ausente» y el resultado vale |
| R8 | **Contrato del asset.** `build` y `paint` escriben `<build>/<id>/asset.json`: id, función (de la ficha), fuente (ficha y piezas, con sha256 y procedencia), GLB y variantes (sha256 y hash estructural), medidas, ejes y origen, anclajes, materiales, licencia y atribución, versiones de las herramientas. `spec_get` (MCP) devuelve además el contrato y el último informe | Fase D; «Consultar el informe» | Test: los campos salen de la ficha y de `sources.json`. Test ignorado: dos montajes seguidos dan el mismo hash estructural |
| R9 | **Publicar**: `kiln publish --asset ID` y herramienta MCP `publish`. Solo si `validation.json` vale y corresponde al GLB actual (por hash). Copia GLB, variantes y `asset.json` a `[publish] dir`/`<id>/`, hace que Godot importe y juzga la importación por su resultado. La versión de producción sube solo si el contenido cambió | «Publicar una versión utilizable por ASTRA» | Tests: sin validación, o con el GLB cambiado después, no publica; publicar dos veces lo mismo no sube la versión |
| R10 | **Nada cambia para quien no lo usa.** Un proyecto sin `[anchors]`, `[publish]` ni `.blend` en sus kits da los mismos hallazgos y respuestas que hoy. El test de cobertura MCP y el README recogen `tools`, `verify` y `publish` | K2-07, K2-08 | Batería de hoy en verde sin tocar sus tests; `examples` da los mismos hallazgos antes y después |

### Entrega B — la puerta

| # | Requirement | Traces to | Acceptance criterion |
|---|-------------|-----------|----------------------|
| R11 | **La puerta, en dos assets**: `door_industrial_frame` (marco) y `door_industrial_leaf` (hoja), con sus piezas `.blend` en `assets/kits/` de ASTRA. Juntos ocupan 1,4 × 2,2 m; ancho en X, grosor en Z, origen en el suelo y en el centro del hueco. Anclaje de clase `doorway` en el marco. El marco tiene la variante `restricted`: cambia solo una franja junto al lector, no toda la puerta («color de alerta como ornamento omnipresente» es lo que la dirección de arte pide evitar, `4.1-…:26`). Hasta 2.000 triángulos por asset. Los ids de asset no tienen relación con el id persistente de ninguna puerta | Fase E 1–3; Fase D | `kiln check` sin errores en ASTRA; medidas del GLB dentro del hueco |
| R12 | **Recorrido completo**: `build` → `verify` → `publish` deja los dos assets en el proyecto de Godot de ASTRA, sin pasos a mano, por CLI y por MCP | «How we will know», punto 1; K2-02, K2-05, K2-07 | Transcripción de los dos recorridos en `.sdlc/k006-art-pipeline/` |
| R13 | **La puerta en ASTRA.** `door.tscn` conserva la raíz `StaticBody3D`, `set_state(open, restricted)` y los hijos `mesh`, `shape` y `ray`. `mesh` pasa a contener la hoja publicada: se oculta al abrir, como hoy exige `reconcile_test.gd:21,31`. El marco es un hijo nuevo que no se oculta; su variante restringida, otro que se muestra en su lugar. La colisión no cambia. No cambia nada bajo `crates/` ni `protocol/` | K2-06 | `cargo test --workspace` y `scripts/godot_station.sh` de ASTRA igual que su línea base, medida antes de tocar; `git diff --stat -- crates protocol` vacío |
| R14 | **Modificación y capturas.** Un cambio en un `.blend` o en la ficha se monta, verifica y publica sin tocar `door.tscn`, y sube la versión. Capturas con ventana: cerrada, abierta y restringida, antes y después del cambio | K2-09; «Una modificación artística…» | Capturas en `.sdlc/k006-art-pipeline/shots/` y tu revisión |

## Design

- **Módulos nuevos en `kiln-core`**: `tools.rs` (detección y versiones),
  `asset.rs` (contrato y hashes), `validate.rs` (hallazgos de R5). `glb.rs`,
  `spec.rs`, `config.rs`, `catalog.rs`, `check.rs` y `build.rs` se amplían.
- **En `kiln-cli`**: `run.rs` (el lanzador de R2); `ops/production.rs` gana
  `verify` y `publish`; `mcp/production.rs`, sus herramientas; `main.rs`, los
  subcomandos `tools`, `verify` y `publish`. Scripts nuevos:
  `blender/export_piece.py` y `godot/import_check.gd`.
- **Qué se reutiliza**: el plan de montaje y `assemble.py` (solo añade los
  nodos de anclaje), los assets hechos de assets, las variantes, los trabajos
  (`job`, `jobs`), `sources.json` para la procedencia, `link_godot` para los
  materiales, la caché de GLB que ya usa `godot_measure`. `sha2` es la única
  dependencia nueva de Rust.
- **Hashes.** sha256 del fichero, para saber qué se publicó. Hash estructural
  (nodos por nombre, jerarquía, transformaciones redondeadas a 1e-4,
  triángulos por malla, materiales, anclajes), para decir que dos montajes
  son equivalentes. No prometo GLB idénticos byte a byte: no lo he medido.
- **Versión de producción.** Un entero en el `asset.json` publicado; sube
  cuando cambia el sha256 de lo publicado.
- **Godot en ASTRA.** `mesh` es un `Node3D` con la hoja instanciada dentro;
  `door_view.gd` deja de crear un material y solo cambia visibilidades.
  Godot no busca nodos dentro del GLB: cada GLB publicado es una pieza
  entera, y por eso la puerta son dos assets.
- **Seguridad.** Las herramientas MCP reciben solo ids planos
  (`spec::plain_id`). Blender y Godot corren únicamente scripts de Kiln.
  `[publish] dir` y los `.blend` se confinan al proyecto, como el contenido
  en K002. La importación de prueba ocurre en la caché, no en el juego.
- **Datos.** Kiln escribe en `[tools] build`, la caché y `[publish] dir`, y
  en ningún otro sitio del proyecto.

## Policy applied

- **Personal data**: ninguno. En la atribución solo salen autores de modelos
  públicos que su licencia obliga a acreditar, como hoy.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna nueva. Los registros guardan la salida de Blender
  y Godot, que no reciben claves.
- **Decisions about people / high risk**: no.

## Out of scope

- Lo que el encargo excluye: el kit de T010, NPC, personajes, animaciones
  complejas, editor visual, generación 3D por IA, cambios en Rust.
- Instalar herramientas; integrar glTF-Transform u optimizar GLB (D3).
- Un puente MCP de Blender (D4).
- Que Kiln ejecute scripts del proyecto o del cliente (D1).
- Animar la hoja: el estado abierto la oculta.
- Texturas, densidad de texel y paleta 3D definitiva (pendientes en la
  dirección de arte de ASTRA).
- Resolver q15 y el choque entre `max_tris = 20000` de `kiln.toml` de ASTRA
  y las 2.000 de su contexto: se anotan, no se tocan.
- Tablas de compatibilidad de los conectores del contenido de juego (K002 de
  Kiln): R6 cubre los anclajes de assets, no los `conn_*` de los módulos.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Una puerta modelada por script queda pobre en la revisión artística | K2-09 sale «no adecuada» aunque la cadena funcione | La revisión juzga cadena y aspecto por separado; mejorar el `.blend` a mano no exige tocar nada más (R14) |
| Dos montajes no dan el mismo hash estructural | K2-03 falla | El hash ordena por nombre y redondea; se mide en la entrega A antes de prometerlo |
| La exportación de un `.blend` hecho a mano trae cosas inesperadas (cámaras, modificadores, varias mallas) | Piezas mal medidas | Ajustes de exportación fijos; R5 avisa; la fixture del test incluye un `.blend` «sucio» |
| Tests de ASTRA que dependen de la puerta actual | Regresión en ASTRA | Línea base antes de tocar; se conservan nombres de nodo y `set_state` |
| ASTRA tiene cambios sin confirmar en otra rama | Mezclar trabajos | D8: no toco ASTRA hasta que digas la rama |
| Godot sin ventana importa distinto que con ventana | La verificación pasa y la captura falla | `publish` importa en el proyecto real y las capturas de R14 lo confirman |
| El lanzador nuevo cambia `build` o `review` | Regresión | Tests de hoy más los dos ignorados, antes y después de R2 |

---
Next stage: Build reads this file and writes `plan.md`.
