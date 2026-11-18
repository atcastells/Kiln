# Spec: assets generados en el juego a partir de una receta (K010)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aaron Castells, product owner (2026-11-18)

## Requirements

Cada uno sale de una línea del intent: «D-n» es la fila n de su tabla de
decisiones, «Señal» es su apartado «How we will know it is solved».

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Una ficha puede llevar una receta (`[generator]`) en lugar de piezas. Receta y piezas se excluyen. | D-1, D-3 | Una ficha con `[generator]` y sin `[[parts]]` se lee y no da `no_parts`; una con las dos cosas da el error `generator_and_parts`. Test de `kiln-core`. |
| R2 | La receta es una pila de operaciones de un vocabulario cerrado que Kiln conoce, definido en un único sitio. | D-4 | Una operación o un parámetro fuera del vocabulario da error en `kiln check` (`generator_op`, `generator_param`). Un test comprueba que cada operación y parámetro de la tabla aparece en el guion de Godot. |
| R3 | Cada parámetro es un valor fijo o un rango `[mín, máx]` dentro de los límites del vocabulario. | D-3 | Un valor fuera de límites o un rango invertido da `generator_range`. El peor caso de triángulos se calcula sin generar y, si pasa de `budget_tris`, da `generator_budget`. |
| R4 | Los rangos se sortean en Rust a partir de la semilla, con un algoritmo escrito en esta spec; Godot recibe valores ya resueltos. | D-5 | `resolve(receta, semilla)` es una función pura de `kiln-core` con vectores de prueba fijos; cambiar la semilla cambia al menos un valor sorteado. |
| R5 | Un guion de Godot que publica Kiln genera la malla a partir de los valores resueltos y la exporta como GLB a la caché. | D-1, D-5, D-7 | Con Godot 4.7: la misma receta y semilla dan dos GLB idénticos byte a byte (mismo sha256); otra semilla da otro sha256 y otras medidas. Test `--ignored`. Corregido en Build (2026-11-18): el hash estructural no mira la posición de los vértices, así que dos semillas de una receta lo comparten y no sirve de criterio. |
| R6 | La página pinta un control por parámetro, un campo de semilla y el botón Aplicar; Aplicar guarda la receta, genera y la vista 3D carga el GLB. | D-3, D-7, Señal | En `scripts/ui_test.sh`: la ficha de ejemplo muestra sus controles; Aplicar llama a la ruta y la vista carga el GLB devuelto. |
| R7 | Lo que hace Aplicar se puede hacer por MCP. | D-7 (regla R14 de K004, `tests/coverage.rs`) | `tests/coverage.rs` pasa con la ruta nueva ligada a su herramienta. |
| R8 | Al pedir un asset, el agente decide si se monta con piezas o se genera; si se genera, escribe la receta con operaciones y rangos del vocabulario. | D-3, Señal | Prueba de punta a punta anotada en `e2e.md`: «crear un asteroide» deja una ficha con receta que pasa `kiln check`. Gasta suscripción: se lanza con permiso del PO. |
| R9 | Montar, verificar, publicar, revisar y pintar rechazan un asset generado con un mensaje claro. | D-8 | Cada una devuelve «es un asset generado: aún no se monta ni se publica» sin lanzar Blender ni Godot. Tests de `kiln-cli`. |

## Design

### La receta en la ficha

```toml
[asset]
id = "asteroid"
name = "Asteroide"
budget_tris = 6000

[generator]
seed = 42                    # la semilla de la vista previa
material = "basalt"          # clave de la paleta

[[generator.ops]]
op = "sphere"
radius_m = [8, 40]           # rango: lo sortea la semilla
segments = 64                # valor fijo

[[generator.ops]]
op = "stretch"
x = [1.0, 1.8]

[[generator.ops]]
op = "noise"
amplitude = [0.2, 0.5]
size = 0.6

[[generator.ops]]
op = "noise"
amplitude = [0.05, 0.15]
size = 0.1

[[generator.ops]]
op = "facet"
```

### Primer vocabulario

Vive en una tabla de `kiln-core` (módulo nuevo `generate`): operación,
parámetros, tipo, unidad, límites y valor por defecto. De esa tabla salen las
comprobaciones, los controles de la página y el texto que recibe el agente.

| Operación | Parámetros | Notas |
|---|---|---|
| `sphere` | `radius_m` (0,1 a 10 000), `segments` (8 a 256, entero) | Obligatoria y primera. Triángulos = 2 · segmentos · (segmentos/2 + 1). |
| `stretch` | `x`, `y`, `z` (0,2 a 5; por defecto 1) | Escala por eje. |
| `noise` | `amplitude` (0 a 1, fracción del radio), `size` (0,01 a 4, tamaño del relieve respecto al radio) | Repetible; cada una lleva su propia semilla derivada. |
| `facet` | ninguno | Normales por cara. |

Cráteres, vetas y un segundo material no entran: no están probados.

### Sorteo (R4)

`resolve(receta, semilla) -> valores` en `kiln-core`, sin dependencias nuevas:

1. Para el parámetro `p` de la operación número `i`, la clave es el texto
   `"<semilla>/<i>/<p>"`.
2. `h` = FNV-1a de 64 bits de la clave, pasado por un paso de SplitMix64.
3. `u` = los 53 bits altos de `h` divididos por 2^53, en [0, 1).
4. Valor = `mín + (máx − mín) · u`; un entero se redondea hacia abajo. Un
   valor fijo no se sortea.
5. Cada `noise` recibe además una semilla de ruido de 32 bits, los bits bajos
   de `h` con la clave `"<semilla>/<i>/noise"`.

Está escrito así para que otro programa (ASTRA) pueda repetirlo.

### Generar (R5, R7, R9)

- Guion `crates/kiln-cli/godot/generate.gd`, incluido en el binario como los
  demás. Lee un JSON con los valores resueltos y la ruta de salida; construye
  la esfera, aplica cada operación en orden y exporta con `GLTFDocument`.
- Corre con `run::godot` en un proyecto vacío de la caché, igual que la
  verificación (`verify.rs`, `scratch_dir`), no en el proyecto del juego.
- Salida: `<cache>/generated/<id>/<semilla>.glb` y un `generated.json` al lado
  con receta, semilla, valores resueltos, medidas, triángulos, hash
  estructural y versión de Godot y del generador.
- Operación `ops::production::generate(asset, seed)`, como trabajo de la cola.
  Ruta `POST /api/generate` con la cabecera propia de la página y herramienta
  MCP `generate`; fila nueva en la tabla de `tests/coverage.rs`.
- Orden de línea `kiln generate --asset ID [--seed N]`.

### Página (R6)

- En la vista de la ficha (`views/spec.js`), un asset con receta muestra en
  vez de la lista de piezas un bloque por operación con sus parámetros: un
  control simple para un valor fijo y uno doble para un rango, con los
  límites del vocabulario.
- Aplicar envía la receta y la semilla; el servidor valida, escribe la ficha,
  genera y devuelve la ruta del GLB, que carga `views/view3d.js` por `/files/`.
- Los textos, en castellano.

### Agente (R8)

`skills/asset/SKILL.md` gana un paso previo: si la forma del asset debe
variar en cada ejemplar (rocas, asteroides), escribe una receta en vez de
piezas. El vocabulario no se copia en la skill: Kiln lo añade al mensaje del
agente desde la tabla.

### Medido antes de escribir esto (2026-11-18, este Mac, Godot 4.7.1)

Prueba suelta fuera del repo: esfera de 64 segmentos estirada con dos capas
de `FastNoiseLite`, exportada sin ventana en un proyecto vacío.

- 0,13 s por ejecución en caliente (0,27 s la primera), 4 224 triángulos,
  GLB de 133 kB. Sin la cola de trabajos ni la carga en la página.
- Tres ejecuciones con la misma semilla: el mismo sha256. Otra semilla: otro.

### Corregido en Build y en la revisión (2026-11-18)

- R5: el criterio pasa a sha256 idéntico (ver la tabla).
- Un material fuera de la paleta es un aviso (`generator_material`), no un
  error, como en las piezas: el agente real propuso uno y generar se negaba.
- Aplicar no reescribe la ficha entera: solo cambia su tabla `[generator]`.
  Reescribirla desde la página perdía variantes y claves desconocidas.
- La carpeta de trabajo de Godot se vacía en cada ejecución y las salidas se
  borran antes de generar: sin eso, un fichero dejado ahí se ejecutaba y un
  GLB viejo pasaba por nuevo.
- Montar rechaza también un asset de piezas que lleva dentro uno generado.
- El catálogo dice qué GLB hay ya generado para la semilla de cada receta, y
  la ficha lo muestra al abrirla.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.
- **Entrada externa**: la receta llega por HTTP y por MCP. Se valida contra el
  vocabulario antes de escribir o generar; el id pasa por `ops::check_id`; la
  salida solo se escribe bajo la caché, que debe quedar dentro del proyecto.
  Godot ejecuta solo el guion de Kiln, en un proyecto vacío.

## Out of scope

- Publicar la receta y el generador en la carpeta del juego, y verificar sin
  GLB (D-8).
- La envolvente para `astra_worldgen` y la roca vista desde dentro (D-6).
- Vista en vivo al arrastrar y generador en JavaScript (D-7).
- Galería o barrido de semillas.
- Cráteres, vetas, varios materiales y detalle modelado en Blender.
- Colisión de la malla generada.
- Que el desglose funcione con otra petición distinta del asteroide.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| La misma semilla da otra malla en otra máquina o versión de Godot | La vista de Kiln no coincide con el juego | Solo se promete misma máquina y versión; `generated.json` anota ambas. Hoy solo es visual: lo que importa lo sortea Rust. |
| El sorteo de Kiln no coincide con el generador de números de ASTRA | ASTRA no reproduce los valores | El algoritmo queda escrito con vectores de prueba; adaptarlo es de la entrega de ASTRA. |
| El vocabulario es demasiado corto para servir de estándar | Asteroides poco creíbles | Añadir una operación es una fila de la tabla más su rama en el guion; el test de R2 vigila que no se separen. |
| El agente sigue desgranando un asteroide en piezas | La señal del intent no se cumple | El paso nuevo de la skill va antes del desglose; la prueba de punta a punta lo comprueba. |
| La caché no está entre las carpetas que sirve `/files/` | La vista no carga el GLB | Comprobarlo al planificar; si falta, se añade solo `<cache>/generated`. |
| Costuras o polos de la esfera al deformar | Grietas visibles | El desplazamiento depende solo de la posición; se mira en la prueba con Godot. |

---
Next stage: Build reads this file and writes `plan.md`.
