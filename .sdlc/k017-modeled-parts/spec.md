# Spec: piezas modeladas por el MCP (`source.kind = "model"`)

- **Derived from**: `intent.md`
- **Status**: approved (con el plan, 2026-11-22)
- **Approved by**: Aaron Castells (product owner), 2026-11-22

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Una pieza de una ficha puede declarar `source = { kind = "model" }` y su forma en `[[parts.shapes]]`, con un vocabulario cerrado: `box`, `para` y `tube` | «una pieza nueva solo puede salir del catálogo… o de fuera»; «sin escribir guiones fuera de Kiln» | `spec::parse` acepta las tres formas; cualquier otra clave o forma da `spec_unreadable` o `model_shape` |
| R2 | `kiln build` construye la pieza en Blender desde sus formas, sin caja de relleno ni `todo` | «Kiln la monta como caja de relleno» | GLB montado: la pieza no está en la colección `todo`; triángulos, medidas y zonas son los que dan sus formas |
| R3 | Cada forma lleva una `zone`; las zonas son las ranuras de material de la pieza y se pintan con `materials` (o `material`) como cualquier pieza del catálogo | «cada material como zona que la ficha asigna a la paleta» | Una zona sin material en la ficha da error `model_zone` en `kiln check`; en el GLB cada zona es un material `part/zone` |
| R4 | UV en caja por metro, con la escala del proyecto (`[assets] uv_per_m`, 1.0 si falta) | «UV en caja a media vuelta por metro»; «Kiln no supone nada de un proyecto concreto» (CLAUDE.md) | Con `uv_per_m = 0.5`, una cara de 2 m de ancho ocupa 1,0 en U |
| R5 | Medidas y presupuesto se conocen antes de Blender: `size_m` sale de las formas; si la ficha lo da y no coincide (±1 mm), error `model_size`; los triángulos de las formas por sus copias se suman a la cuenta de `budget` de `kiln check` (aviso, como las piezas reutilizadas; `verify` ya da error sobre el GLB) | «cumple… escala… presupuesto de triángulos» | Tres casos en `kiln check`: sin `size_m`, con uno que no casa, con el presupuesto superado |
| R6 | Límites que impiden una ficha abusiva: 256 formas por pieza, `segments` de 3 a 32, coordenadas finitas y dentro de ±100 m, formas sin volumen rechazadas | Seguridad del servidor (CLAUDE.md) y OWASP | Una tabla de casos rotos da su código, sin pánico y sin lanzar Blender |
| R7 | El agente aprende el vocabulario por el MCP: el prompt `kiln-asset` y la petición del agente de la página llevan «Vocabulario de modelado», y la skill `asset` explica cuándo modelar | «Una sesión de Claude, solo con las herramientas del MCP de Kiln» | `mcp_prompts` y `agent` comprueban el texto; la skill tiene su sección con un ejemplo sin nombres de ASTRA |
| R8 | De principio a fin sobre una copia de ASTRA: una sesión `claude -p` con solo las herramientas de `kiln mcp` rehace `workbench` como pieza modelada, la monta, la verifica y la publica en la copia | «How we will know it is solved», puntos 1 y 3 | `kiln verify` en verde; capturas del original y del modelado con el mismo renderizador, lado a lado; triángulos dentro del presupuesto de la ficha, medidas ±1 cm, cada zona pintada con la paleta. (Antes decía «triángulos dentro de ±10 % del original» y «mismas zonas»; el product owner lo cambió el 2026-11-22, D2.) |
| R9 | Sin piezas `model`, nada cambia | Regresión | Las pruebas existentes pasan sin tocarlas; `examples/` monta y publica igual (mismos checks) |

## Design

**Ficha.** Coordenadas en metros y ejes de glTF (Y arriba, la cara de la
pieza hacia +Z), en el marco propio de la pieza. Kiln baja la base al origen
como con cualquier modelo, así que el desplazamiento absoluto no importa.

```toml
[[parts]]
id = "bench"
what = "encimera sobre armario y patas"
source = { kind = "model" }
materials = { Top = "worn_paint", Leg = "dark_steel" }

[[parts.shapes]]
zone = "Top"
box = [[-0.60, 0.86, -0.42], [0.60, 0.92, 0.42]]        # dos esquinas opuestas

[[parts.shapes]]
zone = "Leg"
para = { origin = [0.52, 0, 0.34], u = [0.06, 0, 0], v = [0, 0.86, 0], w = [0, 0, 0.06] }

[[parts.shapes]]
zone = "Leg"
tube = { from = [0, 0, 0], to = [0, 0.8, 0], radius = 0.03, segments = 6 }   # radius_to opcional: cono
```

Triángulos: `box` y `para` 12; `tube` `4·segments − 4` (sin tapas no hay
opción en esta entrega).

**Rust (`kiln-core`).**
- `model.rs` nuevo: `Shape` (enum de las tres formas con su `zone`),
  validación (R6), `tris()`, `bounds()`. Una sola tabla de formas y límites,
  como `generate/vocab.rs`, que también da el texto del vocabulario (R7).
- `spec.rs`: `"model"` en `KINDS`; `Part.shapes`; `shapes` solo con `model` y
  `model` siempre con `shapes`.
- Comprobaciones de la ficha, en `spec::check`: `model_shape`, `model_zone`,
  `model_size`, y la suma de triángulos a `budget`.
- `build.rs`: el `Item` de una pieza `model` lleva `shapes`, `model = None`,
  `fit = false`, `todo = None` y `size_m` de `bounds()`. `Plan` lleva
  `uv_per_m`.
- `config.rs`: `[assets] uv_per_m` (opcional, > 0).

**Blender (`crates/kiln-cli/blender/assemble.py`).** En `build()`, si el item
trae `shapes`: un bmesh con una ranura de material por zona (nombrada como la
zona, para que `zone_of` y `dress` la pinten igual que una pieza del catálogo),
cada forma convertida con `to_blender`, normales recalculadas, UV en caja con
`uv_per_m`, y `base_to_origin`. Es la misma lógica que `box_uv`/`add_shape` de
los kits de ASTRA, escrita una vez en Kiln.

**MCP.** Sin herramienta nueva: el agente escribe la ficha con las que ya
existen y monta con `build`. Cambia el texto del prompt `kiln-asset`
(`mcp/prompts.rs`) y de la petición de `agent.rs`, más `skills/asset/SKILL.md`
y el README. La revisión visual usa `review`/`review_sheets`/`review_submit`
de K016.

**Desviación (T9b), aceptada por el product owner el 2026-11-22 («Acepto las dos», D1):** se añade
una herramienta, `guide` (`asset`, `review`, `context`), que devuelve el
texto de los prompts. La prueba de punta a punta mostró que `claude -p` no
pasa los prompts MCP al modelo, así que el vocabulario solo llegaba por los
errores. Traza a R7.

**Límites añadidos en la revisión (R6):** zona de hasta 32 caracteres; tope
duro de 200.000 triángulos por pieza modelada con sus copias (`count`);
`bounds()` no mide formas con problemas; y `kiln build` rechaza, antes de
Blender, lo que `kiln check` rechazaría (`build::refuse`).

**Lo que se mueve.** Solo datos de la ficha (números y nombres de zona) en
`plan.json`. Blender no ejecuta código del agente: `assemble.py` es de Kiln.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno (geometría y nombres de material de un
  juego).
- **Credentials**: ninguna. La prueba de punta a punta lanza `claude -p`; Kiln
  ya quita `ANTHROPIC_API_KEY` y `ANTHROPIC_AUTH_TOKEN` al lanzarlo.
- **Decisions about people / high risk**: no.

## Out of scope

- Migrar los guiones de `assets/kits/*/src` de ASTRA (respuesta 4).
- Texturas nuevas (pregunta 5, propuesta): las piezas usan la paleta existente.
- Biseles, booleanas, tapas abiertas, curvas y deformaciones (`raked` de
  `guards.py`): `para` cubre las rampas de los kits; lo demás, si hace falta,
  en otra entrega.
- Cambios en el repo de ASTRA: la prueba va sobre una copia. Poner
  `uv_per_m = 0.5` en el `kiln.toml` de ASTRA y usar piezas modeladas allí lo
  decide el desarrollador después.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| K016 cambia antes de unirse y esta rama choca | Rebase a mano | Los ficheros de K017 apenas se cruzan con los de K016 (solo `mcp/prompts.rs`, en el texto); se anota en el PR |
| El agente modela mal (proporciones, piezas flotando) aunque el vocabulario funcione | No llega a «nivel de los kits» | R8 compara con el original; si falla, se anota qué faltó en la skill o el vocabulario y se itera el texto antes de tocar código |
| Ejes: los kits de ASTRA escriben en ejes de Blender (Z arriba, frente −Y) y la ficha en ejes de glTF | Piezas giradas o tumbadas | Prueba de Blender que monta una forma asimétrica y mide su caja en el GLB; la skill lo dice con un ejemplo |
| `claude -p` de la prueba gasta tokens y puede no converger | Coste y tiempo | Una sola pasada, con tope de turnos; si no converge, se informa con su registro |

---
Next stage: Build reads this file and writes `plan.md`.
