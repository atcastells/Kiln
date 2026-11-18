# Spec: la UI de un juego en Kiln (K007)

- **Derived from**: `intent.md`, aprobado por el PO el 2026-11-17.
- **Status**: approved (2026-11-17)
- **Approved by**: product owner (Aaron Castells, 2026-11-17). No hace falta el responsable de políticas (ver *Policy applied*).

## Enmiendas tras la aprobación

1. **Fondos translúcidos: el peor de negro y blanco (PO, 2026-11-17, en T4).** Al reproducir la tabla de ASTRA salió que se había medido a mano como si los fondos fueran opacos. Un fondo o un contorno con alfa se compone sobre negro y sobre blanco, y vale la cifra más baja. La tabla de ASTRA se corrigió con una enmienda en su spec (`.sdlc/ui-target/spec.md`), y R2 se cumple contra la tabla corregida. Donde el Design dice «compuesto con su alfa» o «sobre negro», vale esta regla.
2. **Tras la revisión (2026-11-17).** Lo que añaden o precisan las tres pasadas y el verifier (ver `review.md`):
   - **Rutas confinadas:** el tema, `target`, los globs y las fuentes `res://` se resuelven dentro de la raíz del proyecto, como hace `asset.rs`. Los mensajes no citan contenido de ficheros.
   - **Dos códigos fuera de R7:** `ui-theme` es siempre error (tema ilegible o fuera del proyecto) y `ui-files` es siempre aviso (patrón fuera del proyecto, glob vacío, enlace que sale de la raíz). Una fuente ilegible sigue el nivel de `glyphs`. Son errores de configuración, no reglas, y por eso no tienen nivel.
   - **R5:** se añade `add_theme_stylebox_override` a la lista, y se mira solo el código, no el contenido de las cadenas.
   - **R4:** una entrada del tema sin `base_type` no es variación y nunca es huérfana. La lista de clases de Godot crece a 39.
   - **R6:** un criterio `kiln:*` no cumple si hay un `ui-theme` o un `ui-files`, porque la regla no pudo medir. Con la regla en `off` queda pendiente, con un problema. Una regla, un `marker` o un `status` desconocidos lo dejan pendiente, con un problema.
   - **R1:** `catalog what=ui` añade `uses` (`fichero:línea`), `fonts` y `colors`. `kiln index` no incluye la UI: el catálogo de UI es solo por MCP. Es una desviación de R1 («y `kiln index`») aceptada para no cambiar el formato de `index`.
   - **R8:** las pruebas de `ui/` usan colores neutros. Fuera de esta tarea ya había nombres de ASTRA en `crates/` (`content/parity.rs`, `content/schema.rs`, `kiln-cli/src/content.rs`), de K002; quedan como estaban.
   - **El escáner** sigue las cadenas de varias líneas, y `theme_type_variation` solo cuenta cuando el lado derecho empieza por una cadena literal.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Kiln lee el tema de UI del juego sin Godot | Alcance 1, «Catálogo de UI» | `catalog what=ui` (MCP) y `kiln index` dan los tokens de color, las fuentes y las variaciones con su `base_type` y dónde se usa cada una. Probado con un tema de ejemplo en `examples/` y con `astra_theme.tres` |
| R2 | Comprobación de contraste | Alcance 2; señal «14,4:1 del enunciado» | Para cada variación con `font_color`, Kiln mide el contraste WCAG contra su fondo (D2) y avisa por debajo del umbral de `kiln.toml`. Sobre ASTRA en `acc541f` reproduce la tabla *Contraste de partida* de `.sdlc/ui-target/spec.md` de ASTRA, con ±0,1 |
| R3 | Comprobación de glifos | Alcance 2; «✓ y × presentes, ⚠ ausente» | Kiln lee la tabla `cmap` de cada fuente del tema y avisa de cada carácter de un texto de UI que no esté. Sobre ASTRA: ningún aviso. Con un «⚠» de prueba en un texto, un aviso |
| R4 | Comprobación de tema coherente | Alcance 2; «hallazgo si se reintroduce `HudTime`» | Avisa de una variación usada que no existe y de una definida que nadie usa (ni como base de otra). Sobre ASTRA: ningún aviso. Con `HudTime` añadida al tema y sin uso, un aviso |
| R5 | Comprobación de estilo fuera del tema | Alcance 2, «colores escritos fuera del tema» | En los scripts de UI que declara `kiln.toml` (no en los de 3D), avisa de `Color(`, `Color.html`, `add_theme_color_override`, `add_theme_font_size_override` y `StyleBoxFlat.new`. Sobre ASTRA: ningún aviso |
| R6 | Objetivo de UI con porcentaje | Alcance 3; «`status` da un porcentaje con criterios que aprueba el PO» | `status` da `ui_target: {met, total, percent, open}`. Kiln marca solo los criterios de tipo Kiln, a partir de las comprobaciones de R2–R5. Los demás criterios toman su estado de D1. Sobre ASTRA: 12 de 15, y `open` = C4, C6 y C15 |
| R7 | Gravedad configurable, aviso por defecto | Riesgos, «Gravedad» | Cada regla tiene `level = "warn" | "error" | "off"` en `kiln.toml`, y por defecto es `warn`. Un `error` hace que `kiln check` salga con 1 |
| R8 | Kiln sigue siendo genérico | Riesgos, «Kiln es genérico» | Ningún nombre ni valor de ASTRA en `crates/`. Las pruebas usan un tema de ejemplo en `examples/`. Un `kiln.toml` sin `[ui]` funciona igual que antes |
| R9 | Uso por MCP y CLI; la página solo enseña hallazgos | «Who it affects»; decisión «MCP y CLI primero» | Los hallazgos de UI salen en `check` (MCP y CLI) y en la vista de comprobaciones que ya existe. No hay vista nueva |

## Design

### Configuración (`kiln.toml`)

```toml
[ui]
theme = "godot/ui/astra_theme.tres"
scripts = ["godot/station/hud.gd", "godot/station/*_panel.gd"]   # scripts de UI (R4, R5)
scenes = ["godot/station/station.tscn"]                              # escenas con UI (R3, R4)
target = "kiln/ui_target.toml"                                       # D1

[ui.contrast]
normal = 4.5
large = 3.0
large_from_px = 18          # a partir de este tamaño, el texto cuenta como grande
level = "warn"

[ui.glyphs]
level = "warn"
[ui.theme_coherent]
level = "warn"
[ui.theme_only]
level = "warn"
```

Los valores de ASTRA (4,5 / 3 y los niveles) van en el `kiln.toml` de ASTRA.

### Lectura del tema (R1)

Kiln lee el `.tres` como texto, sin Godot: `[ext_resource]` (fuentes), `[sub_resource type="StyleBoxFlat"]` (`bg_color`, `border_color`, márgenes) y `[resource]` (`<Variación>/<tipo>/<nombre> = valor`, `<Variación>/base_type`). Una variación hereda de su `base_type` cuando este es otra variación del tema. Kiln no ejecuta Godot para esto; un `.tres` binario (`.res`) da un hallazgo y no se lee.

### Fondos para el contraste (R2, D2)

- **Texto con contorno:** si la variación tiene `font_outline_color` y `outline_size` mayor que 0, propios o heredados, se mide contra el contorno, compuesto con su alfa sobre negro.
- **Resto:** parejas declaradas en `kiln.toml`. El fondo es una caja de estilo del tema, y se toma su `bg_color` compuesto con su alfa:

```toml
[[ui.contrast.pairs]]
fg = "PanelBody"                 # variación con font_color
bg = "InfoPanel/panel"           # <variación>/<caja de estilo>

[[ui.contrast.pairs]]
fg = "HudTimeBar/fill"           # lo que no es texto: color de una caja frente a otra
bg = "HudTimeBar/background"
kind = "non_text"                # umbral «large» (3:1)
```

- **Sin fondo:** una variación con `font_color` que no tiene contorno ni pareja da un hallazgo `info` («sin fondo declarado») y no cuenta para el criterio.
- **Cifras:** se dan con un decimal, igual que en las hojas de revisión.

### Glifos (R3)

- **Qué fuentes se leen:** las de `[ext_resource type="FontFile"]` del tema, con un lector propio de la tabla `cmap` (formatos 4 y 12) en `kiln-core`, sin dependencias nuevas.
- **Qué textos se miran:** las cadenas literales de los scripts de UI y las propiedades `text` de las escenas.
- **Cuándo avisa:** un carácter que falta en alguna de las fuentes da un aviso con el fichero y la línea.
- **Lo que no ve:** el texto que viene del servidor no se mira, porque no está en el código. Va en *Out of scope*.

### Uso de variaciones (R4)

Una variación cuenta como usada si aparece en `theme_type_variation` dentro de los scripts y escenas declarados, o si es `base_type` de otra variación usada. Las variaciones de un tipo base de Godot, como `Label` o `Button`, no cuentan como huérfanas.

### Objetivo de UI (R6)

- **De dónde sale la lista:** de las afirmaciones fijadas del contexto cuyo texto empieza por un id de criterio (`C1 ·`…). Ninguna se escribe de nuevo en Kiln.
- **El fichero `target`:** dice qué comprobación marca cada criterio de tipo Kiln, y guarda el estado de los demás según D1.
- **Lo que da `status`:** cuántos cumplen, sobre el total, y la lista de los que no cumplen o siguen pendientes.

### Componentes

- `kiln-core`: `ui.rs` (tema, lectura de `cmap`, contraste, uso de variaciones), `config.rs` (`[ui]`) y `check.rs` (las cuatro reglas nuevas).
- `kiln-cli`: `ops/read.rs` (`status` con `ui_target`, `catalog what=ui`).

## Decisiones del PO

| # | Decisión | Respuesta |
|---|----------|-----------|
| D1 | Dónde se guarda el estado de los criterios que no marca Kiln | **`kiln/ui_target.toml`** (PO, 2026-11-17): fichero del juego, versionado, una fila por criterio con `marker` (`kiln:<regla>`, `test` o `po`), `status` (`met`, `unmet`, `pending`) y `evidence` (hoja o prueba). Lo escribe el agente cuando el PO confirma; Kiln solo lo lee. Sin herramienta MCP nueva de escritura. Para los `kiln:<regla>`, el estado lo calcula Kiln y el del fichero se ignora |
| D2 | Contra qué fondo mide Kiln cada texto | **Contorno automático más parejas declaradas** (PO, 2026-11-17). Ver *Fondos para el contraste* |

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.

## Out of scope

- El revisor visual de UI y las fichas de componente (entregas 4 y 5).
- Una vista nueva en la página.
- Los textos que llegan del servidor del juego, que no están en el código.
- Escribir en el juego: Kiln solo lee el tema y los scripts.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| El formato `.tres` cambia entre versiones de Godot | Lectura rota | Pruebas con el tema de ejemplo y con el de ASTRA (Godot 4.7.1); un `.tres` que no se entiende da un hallazgo y no rompe la carga |
| Cadenas construidas en tiempo de ejecución (`"%s te pregunta…"`) | Glifos sin mirar | Se miran los literales; lo dinámico queda fuera y se dice en el hallazgo |
| Los contrastes de la spec de ASTRA se midieron a mano | La señal de cierre falla por un error de la medida, no de Kiln | Si una cifra no coincide, se revisa primero la medida a mano |

---
Next stage: Build reads this file and writes `plan.md`.
