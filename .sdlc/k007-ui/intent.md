# Intent: gestionar desde Kiln la creación de la UI de un juego (K007)

- **Originator**: Aaron Castells
- **Date**: 2026-11-17
- **Status**: approved by product owner (2026-11-17)
- **Slug**: k007-ui

## The problem, in the originator's words

> mientras, como podemos mejorar kiln para gestionar desde alli la creacion de la UI ?

Tras la propuesta en cinco entregas (catálogo de UI, comprobaciones de UI,
objetivo de UI en el contexto, revisor visual de UI y fichas de componente),
y el aviso de que las capturas abren ventana:

> Se puede abrir ventana, sigue

## Qué pasa hoy (comprobado el 2026-11-17)

- **Kiln no sabe nada de UI.** Su catálogo es de kits 3D, piezas y
  materiales (`crates/kiln-core/src/catalog.rs`). Sus comprobaciones son
  licencias, escala, coste y contenido RON (`check.rs`, `validate.rs`). Su
  revisor visual monta en Blender y saca hojas de un asset 3D (`review.rs`,
  README «Entrega 7»).
- **Kiln solo lanza Godot sin ventana** (`crates/kiln-cli/godot/measure.gd`,
  `import_check.gd`). Para capturas con ventana, el `CLAUDE.md` de Kiln manda
  usar un contenedor Linux con `xvfb` y Vulkan por software, y dice que en
  ASTRA está en `scripts/godot_window.sh`. Ese script **no existe** hoy en
  `~/.t3/projects/astra/scripts/`.
- **En ASTRA la UI se ha hecho a mano**, en tres pasos de su SDLC
  (`.sdlc/ui-theme`, `ui-style`, `hud-states`, commits `0d7444f`, `5850c15`
  y `164bd7d`):
  - tema único `godot/ui/astra_theme.tres` con variaciones encadenadas;
  - pruebas de valores y de maquetación en GDScript (`ui_theme_test`,
    `ui_layout_test`, `hud_states_test`);
  - capturas con `godot/tests/ui_shots.gd` y diferencia píxel a píxel con la
    iteración anterior;
  - hojas de revisión con criterios marcados (`marks-*.json` y `sheet.py` de
    la skill `ui-visual-review`), contraste WCAG medido a mano y glifos
    buscados en la tabla `cmap` de la fuente;
  - cada decisión de arte aprobada se guarda como revisión ART en
    `docs/art/decisiones/` y como afirmación `look` en `kiln/context.toml`.
- **No hay un «objetivo de UI» escrito.** El porcentaje de avance que se dio
  (9 de 15 criterios) sale de la lista de la hoja del paso 1, no aprobada como
  objetivo.

## Alcance

Entregas 1, 2 y 3 (decisión del 2026-11-17):

1. **Catálogo de UI**: Kiln lee el tema del juego (tokens, fuentes, variaciones con su herencia y dónde se usan).
2. **Comprobaciones de UI**: contraste, glifos en la fuente, tamaño mínimo, variaciones huérfanas o inexistentes, colores escritos fuera del tema, tokens fuera de la paleta aprobada.
3. **Objetivo de UI en el contexto**: criterios con estado y el porcentaje en `status`.

Fuera de esta tarea: el revisor visual de UI (4) y las fichas de componente (5).

**Requisito previo (decisión del 2026-11-17):** antes del diseño de K007, el PO aprueba en ASTRA un intent corto con la lista de criterios del objetivo de UI y la definición de cada uno. Ahora mismo son los 15 de la hoja del paso 1, sin aprobar. K007 carga esa lista ya fijada, así que el porcentaje es firme desde el primer día.

**Requisito previo cumplido (2026-11-17).** La lista está aprobada en ASTRA, en `.sdlc/ui-target/spec.md` (spec y plan aprobados por el PO):
- Son 15 criterios, C1–C15. Cada uno tiene su definición y quién lo marca: Kiln, una prueba del juego o el PO. Los que marca Kiln son C10 (paleta y fuente desde el tema), C12 (contraste), C13 (glifos) y C14 (tema coherente).
- El umbral de contraste es 4,5:1 para el texto normal y 3:1 para el texto grande y lo que no es texto. El texto con contorno se mide contra su contorno (D1).
- En el contexto de ASTRA (`kiln/context.toml`, capa `look`), cada criterio es una afirmación fijada: `c68` es el umbral y `c69`–`c83` son C1–C15. Cada texto empieza por su id de criterio.
- El estado de cada criterio no va en la afirmación. Dónde vive es una decisión del diseño de K007. La partida, 12 de 15, está en la spec de ASTRA.

## Who it affects

- **El agente que hace la UI de un juego** (en ASTRA, las tareas de UI de su SDLC): consulta el catálogo, las comprobaciones y el objetivo por MCP (`catalog`, `check`, `status`) en lugar de medir a mano.
- **El ingeniero y el PO**, por `kiln check` y por los hallazgos que la página ya enseña. En esta tarea la página no gana una vista nueva.

## What happens if we do nothing

Cada tarea de UI de un juego sigue midiendo a mano lo que Kiln podría comprobar:
- en ASTRA, el contraste WCAG de cada hoja se calculó a mano;
- los glifos «✓» y «×» se buscaron en la tabla `cmap` de Plex;
- las variaciones `HudTime` y `HudTimeWarn` se quitaron a mano cuando dejaron de usarse;
- el avance hacia el objetivo de UI fue una estimación sin lista aprobada.

Un error de cualquiera de estas medidas solo lo ve una persona o un revisor.

## How we will know it is solved

Kiln reproduce sobre ASTRA (commit `164bd7d`) lo que allí se midió a mano, y se comprueba en esta misma tarea:
- los contrastes de las hojas de `.sdlc/hud-states/` (por ejemplo, 14,4:1 del enunciado sobre la caja #111D26);
- «✓» y «×» presentes en IBM Plex Mono, y «⚠» ausente;
- un hallazgo si se reintroduce una variación huérfana como `HudTime`;
- `status` da un porcentaje de avance con criterios que aprueba el PO.

## Riesgos

- **Kiln es genérico.** Nada de ASTRA en el código: umbrales, criterios, rutas del tema y niveles de cada regla van en `kiln.toml` y en el contexto del juego (README, `kiln.toml`: «Kiln no supone nada de un proyecto concreto»).
- **El tema no dice sobre qué fondo va cada texto.** El contraste necesita parejas de texto y fondo que el `.tres` no declara; habrá que declararlas o deducirlas. Se decide en el diseño.
- **Leer `.tres` de Godot.** Hay que leerlo sin Godot (texto) o con Godot sin ventana, que es lo que Kiln ya hace en `measure.gd`. Se decide en el diseño.
- **Gravedad:** cada regla de UI tiene su nivel en `kiln.toml`; por defecto avisa y no rompe `kiln check`.

## Open questions

- Ninguna que bloquee el diseño. El requisito previo es la lista de criterios aprobada en ASTRA.

## Decisions

| Date | Question | Answer | Who |
|------|----------|--------|-----|
| 2026-11-17 | ¿Se pueden abrir ventanas de Godot? | Sí («Se puede abrir ventana, sigue») | Aaron Castells |
| 2026-11-17 | ¿Qué entregas entran? | 1 (catálogo), 2 (comprobaciones) y 3 (objetivo con %); 4 y 5 fuera | Aaron Castells |
| 2026-11-17 | ¿Desde dónde se usa? | MCP y CLI primero; la página solo enseña los hallazgos, sin vista nueva | Aaron Castells |
| 2026-11-17 | ¿Qué señal dice que está resuelto? | Reproducir sobre ASTRA (164bd7d) las medidas hechas a mano, y `status` con porcentaje sobre criterios aprobados por el PO | Aaron Castells |
| 2026-11-17 | ¿Cómo entra la lista de criterios de ASTRA? | El PO la aprueba antes en un intent de ASTRA; K007 la carga fijada | Aaron Castells |
| 2026-11-17 | ¿Qué gravedad tienen los hallazgos de UI? | Configurable por regla en kiln.toml; aviso por defecto | Aaron Castells |
| 2026-11-17 | ¿Se aprueba el intent? | Aprobado; rama `k007-ui` creada desde `k006-art-pipeline` (K006 no está en main y es la versión que usa ASTRA) | Aaron Castells |
| 2026-11-17 | ¿Está aprobada la lista de ASTRA? | Sí: `.sdlc/ui-target/spec.md` de ASTRA, afirmaciones c68–c83 | Aaron Castells |

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
