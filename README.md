# Kiln

Motor local de assets para proyectos de juego. Describes un asset y un grafo
de nodos lo desgrana en piezas, reutiliza lo que el juego ya tiene y liga cada
modelo a la paleta de materiales del proyecto. Trabaja con Godot, Blender y
Rust.

Diseño: [Motor de creación de assets](https://claude.ai/artifact/Au2dgBc6AtrwVevdstwL2z) ·
[Proyectos parecidos](https://claude.ai/artifact/AUiJFz821qrR9Z5pKgvWne) ·
[maqueta del grafo](https://claude.ai/artifact/QrShQzVqXJ1VoVwzvn4q5f).

## Entrega 1: el grafo de lo que ya hay

Kiln lee un proyecto descrito por su `kiln.toml` y monta el catálogo: cada
kit, cada pieza con su medida, triángulos, ranuras de material y licencia, y
los materiales de la paleta. Pasa las comprobaciones que fallan sin que nadie
lo vea (licencias, crédito CC-BY, escala, base en el suelo, coste) y lo
enseña como grafo en el navegador.

```bash
cargo build --release
# tu juego con su kiln.toml en la raíz; para probar sin juego: --project examples
target/release/kiln serve --project ../mi-juego   # http://127.0.0.1:4330
target/release/kiln check --project ../mi-juego   # sale con 1 si hay errores
target/release/kiln index --project ../mi-juego --out catalog.json
# --config FICHERO usa otro kiln.toml sin tocar el juego
```

Sin dependencias fuera de Rust; el servidor solo escucha en `127.0.0.1`.

## `kiln.toml`

```toml
[project]
name = "Puerto Niebla"

[assets]
kits = ["godot/kit"]           # cada subcarpeta es un kit: modelos, JSON de manifiesto, LICENSE.md

[materials]
tables = ["data/buildings/materials.toml"]   # tablas TOML con filas que llevan `id`
textures = ["godot/textures"]                # texturas `<clave>_albedo.<ext>`

[rules]
licenses = ["CC0", "CC-BY"]
max_tris = 20000
require_use = false            # true: cada asset debe nombrar el sistema del juego que lo usa
```

Kiln no supone nada de un proyecto concreto: sus reglas, sus carpetas, sus
niveles y sus convenciones de nombres (`albedo_suffix`, `map_suffixes`) van
en su `kiln.toml`. `examples/kiln.toml` es un ejemplo completo de un juego
inventado, Puerto Niebla.

## Entrega 2: pedir un asset

En el grafo, escribe la petición («crear un ascensor») y pulsa Desglosar.
Kiln lanza Claude Code sin ventana (`claude -p`, con la skill
`skills/asset/SKILL.md`) en el proyecto; el agente lee el proyecto y el
catálogo y escribe la ficha en la carpeta `[assets] specs` de `kiln.toml`.
El grafo sigue lo que hace y, al terminar, despliega la ficha: petición,
asset, piezas, de dónde sale cada modelo (reutilizar, variante, buscar,
retocar), material de la paleta y validación. `examples/specs/hoist.toml` es
una ficha de ejemplo para Puerto Niebla.

- Necesita `claude` en el PATH (o su ruta en `KILN_CLAUDE`) con sesión iniciada.
- Usa tu suscripción de Claude, con la sesión de `claude` (`claude auth status`
  dice `authMethod: "claude.ai"`). Kiln quita `ANTHROPIC_API_KEY` y
  `ANTHROPIC_AUTH_TOKEN` al lanzarlo, para que una clave del entorno no
  pase a cobrar por la API. Cada mensaje gasta del límite de uso de la suscripción.
- El agente solo puede leer y escribir ficheros (sin terminal) y no baja nada:
  las piezas «buscar» esperan tu permiso.
- Solo la propia página puede lanzar al agente: la petición lleva una cabecera
  propia y el servidor solo escucha en `127.0.0.1`.

## Entrega 3: el árbol del proyecto

Kiln se abre en el juego y se baja por niveles con doble clic (Juego ›
Lugar › Edificio › Sala › Asset); la miga de pan sube. Cada nodo dice
cuántos de los assets que tiene debajo ya tienen modelo y lista lo que
falta; desde un asset que falta, «Pedir ficha» lanza al agente con la
petición y su sitio en el árbol. Los niveles se declaran en `kiln.toml`
(`[[levels]]`) leyendo los datos que el juego ya tiene; el ejemplo lee
barrios, edificios y salas de tres ficheros de datos.

```toml
[[levels]]
level = "room"                                  # el nivel de estos nodos
files = ["data/buildings/rooms.toml"]           # globs; cada fichero es un nodo...
table = "rooms"                                 # ...o cada tabla de este array
id = "use"                                      # la clave con su id
children = [{ key = "fixtures", field = "id", level = "asset" }]
```

Un asset es una hoja: tiene modelo si el catálogo tiene una pieza con su
nombre, ficha si hay una ficha con su id, y si no, falta.

## Entrega 4: producción con Blender y Godot

La IA decide y Kiln ejecuta. En la ficha, «Colocar las piezas» lanza al
agente solo para el paso de montaje: añade a cada pieza dónde va cada copia
(`at`, el centro de su base en metros, Y arriba) y cómo gira (`turn`).
«Montar en Blender» convierte la ficha en un plan (`plan.json`) y lanza
Blender sin ventana con `crates/kiln-cli/blender/assemble.py`:

- importa las piezas del catálogo; «reutilizar» guarda su medida, «variante»
  y «retocar» se estiran a `size_m`;
- pone una caja de su medida en las que hay que buscar o no tienen glTF;
- viste cada pieza con su material de la paleta (textura de color o color de
  la tabla, `[materials] color_keys`) y cada ranura con su vínculo;
- escribe `<build>/<id>/<id>.glb`, el `.blend` y `report.json` con los
  triángulos y lo que queda por terminar a mano. Cada objeto lleva en sus
  extras su pieza, si se mueve y en qué estados sale, para Godot.

Hasta K005, `turn` solo giraba las cajas: las piezas con modelo (y las que
son otro asset) salían sin girar. Los montajes hechos antes con `turn` en
esas piezas hay que rehacerlos (`kiln build`). El test que monta con Blender
de verdad se ejecuta a mano (necesita Blender en el `PATH`):
`cargo test -p kiln-cli -- --ignored turn`.

El mismo script sirve en un Blender abierto (por MCP) con `KILN_PLAN`.
Sin ficha colocada, Kiln pone las piezas en fila para que se puedan ver.

**Vista 3D.** Cada pieza con glTF, cada pieza de una ficha y el montaje se
ven en 3D en el panel (three.js 0.187 incluido en el binario, sin red).

**Ranuras y paleta.** En el panel de una pieza, cada ranura de material se
vincula a una clave de la paleta; se guarda en `[materials] bindings`
(`kiln/bindings.toml`) y Blender lo usa al montar.

**Godot.** Las piezas `.res`, `.tres` y `.tscn` solo las abre Godot:
`kiln godot` (o «Medir con Godot» en el panel del proyecto) lanza Godot sin
ventana con `crates/kiln-cli/godot/measure.gd`, que mide cada una y la
exporta a GLB en la caché; desde ahí Blender y la vista 3D las usan.

```toml
[tools]
blender = "blender"            # o un Python con el módulo bpy
godot = "godot"
godot_project = "godot"        # la carpeta con project.godot
build = "kiln/build"
cache = "kiln/cache"
```

```bash
kiln build --project ../mi-juego --asset hoist
kiln godot --project ../mi-juego
```

## Entrega 5: el contexto del proyecto

Lo que el agente sabe del juego antes de desglosar un asset. En la pestaña
**Contexto** explicas el juego con tus palabras; «Guardar y ordenar» lanza
al agente (skill `skills/context/SKILL.md`), que lo parte en afirmaciones
cortas, cada una en su capa (Mundo, Estética, Escala, Materiales, Juego,
Técnica), con su origen (tu texto o un fichero del proyecto), enlaces entre
capas y choques marcados. El grafo va del juego a las capas, de cada capa a
sus afirmaciones y a sus preguntas abiertas.

- **Afinar**: el agente deja una ronda de preguntas con tres respuestas
  sugeridas; respondes con un clic, con tus palabras, «Saltar» o «No
  aplica». Cada respuesta es una afirmación nueva de su capa.
- **Fijar**: lo que escribes, editas o respondes queda fijado. El agente no
  lo cambia: como mucho deja su versión como propuesta, que aceptas o
  descartas (Kiln lo garantiza al terminar cada trabajo del agente).
- **Al pedir un asset**, el agente lee el contexto primero; si le falta un
  dato, decide lo más probable, lo dice en la ficha y deja la pregunta en
  el contexto con el asset que la pidió.

Todo vive en `kiln/context.toml` (texto plano, se versiona con el juego).
Las capas se cambian en `kiln.toml`:

```toml
[context]
file = "kiln/context.toml"
layers = [{ id = "sound", name = "Sonido", holds = "ambiente, efectos, música" }]   # sustituye a las seis
```

`examples/context.toml` es el contexto de Puerto Niebla.

## Entrega 6: buscar assets fuera

Una pieza que la ficha marca como «buscar» se busca en Poly Haven, Poly
Pizza y Sketchfab desde su panel. **Buscar** pregunta a los tres a la vez y
enseña una rejilla con miniatura, autor, licencia, triángulos (en aviso si
pasan del presupuesto) y enlace a la página. Solo salen las licencias de
`[rules] licenses`. **Traer** vuelve a leer la licencia en el sitio (nunca
se fía de la página), baja el modelo a `<kits>/<sitio>/<pieza>/`, apunta su
origen en `sources.json` del kit, reescribe su `LICENSE.md` y
`kiln/CREDITS.md` (lo que el juego tiene que acreditar), y la pieza de la
ficha pasa a «reutilizar» (si ya tiene la medida) o a «variante» (se escala
al montar).

| Sitio | Licencias | Clave |
| --- | --- | --- |
| Poly Haven | CC0 | ninguna |
| Poly Pizza | CC0, CC-BY | gratuita, `KILN_POLYPIZZA_KEY` |
| Sketchfab | las de `[rules]` | buscar sin clave; bajar con tu token, `KILN_SKETCHFAB_TOKEN` |

Las claves van en variables de entorno o en `~/.config/kiln/keys.toml`
(`polypizza = "…"`, `sketchfab = "…"`), nunca en `kiln.toml`; ni la página
ni el agente las ven. La red pasa por `curl`.

```toml
[sources]
sites = ["polyhaven", "polypizza", "sketchfab"]
kits = "godot/kits"        # por defecto, el primero de [assets] kits
resolution = "1k"          # texturas de Poly Haven
credits = "kiln/CREDITS.md"
```

## Entrega 7: el revisor visual

El agente que coloca las piezas nunca ve el resultado; el revisor sí.
**Montar y revisar**, en el panel de un asset (o `kiln review --asset ID`),
hace hasta tres vueltas:

1. Monta en Blender y saca cuatro hojas, siempre con las mismas cámaras, la
   misma luz y el mismo fondo: **giro** (seis vistas a 30° de altura, cada
   60°), **alzados** (frente y lado en ortográfica, rejilla de 1 m, figura
   de 1,80 m y cotas), **piezas** (cada una de un color con su `id`) y
   **silueta** (en negro, y el asset al tamaño al que se verá en el juego).
2. Blender mide lo que un modelo de visión no ve bien: la caja de cada
   copia, las piezas que se atraviesan y las que no se apoyan en nada
   (`measures.json`).
3. Un revisor aparte (`claude -p` con la skill `skills/review/SKILL.md`,
   Opus fijado y sin poder editar la ficha) describe lo que ve, puntúa seis
   criterios y devuelve hallazgos con su corrección en términos de la
   ficha (`at`, `turn`, `size_m`, `count`, `material`).
4. Kiln aplica esas correcciones y vuelve a montar. Desde la segunda vuelta
   el revisor compara la nueva con la mejor hasta entonces, en orden
   aleatorio, y se queda con la que gana: el resultado nunca empeora.

Al terminar quedan en su sitio la ficha y el montaje de la mejor vuelta. El
panel enseña las hojas de cada vuelta, sus puntuaciones, lo que se aplicó y
lo que queda para ti (añadir piezas, cambiar de modelo). Todo se guarda en
`<build>/<id>/review/`.

```toml
[agents]
review_model = "opus"   # el modelo del revisor
review_rounds = 3       # vueltas como mucho
```

Las hojas se sacan con Cycles por defecto, que funciona sin pantalla;
`KILN_RENDER_ENGINE=BLENDER_EEVEE` (Blender 5) es más rápido en un Mac con pantalla.

## Entrega 8: materiales por zona

Una **zona** es cada ranura de material de una pieza, con el nombre que le
da el modelo. La ficha elige un material de la paleta por zona, y las zonas
que no nombra se quedan con el material del modelo (así una pieza traída de
fuera conserva sus texturas):

```toml
[[parts]]
id = "load"
materials = { Wood = "pine" }       # la zona Iron sigue como venía
source = { kind = "reuse", from = "harbor/crate" }

[[variants]]                        # el mismo montaje con otros materiales
id = "weathered"
materials = { "post/Wood" = "oak", "load/Iron" = "rust" }
```

Para cada zona manda, por este orden: `materials` de la ficha, `material`
de la pieza (todas sus zonas), el vínculo del catálogo y el propio modelo.

En el GLB montado cada material se llama `pieza/zona`, y por eso cambiar un
material no obliga a montar otra vez:

- **En la vista 3D**: el panel del montaje tiene un desplegable por zona. El
  cambio se ve al momento y se guarda en la ficha.
- **En el GLB**: **Aplicar materiales** (`kiln paint --asset ID`) abre el
  montaje, vuelve a vestir cada zona y exporta. No recoloca nada.
- **En Godot**: si el montaje queda dentro del proyecto de Godot y
  `kiln.toml` dice dónde está el material de cada clave, Kiln escribe los
  vínculos en el `.import` del GLB. Probado con Godot 4.7.

```toml
[materials]
godot = "res://materials/{key}.tres"

[tools]
godot_project = "godot"
build = "godot/built"
```

**Variantes de color**: se eligen en el mismo panel. Cada una sale como
`<id>--<variante>.glb` con `kiln paint --asset ID --variant V`.

**Validar**, junto a una pieza del panel, pasa sus materiales de la ficha
al catálogo (`kiln/bindings.toml`): quedan como valor por defecto de esa
pieza para cualquier otro asset.

**Partir en zonas** una pieza que solo trae una ranura se elige en el panel
de la pieza y se guarda en `kiln/zones.toml`:

- **por trozos sueltos**: cada parte de la malla que no toca a otra es una
  zona (`part_1`, `part_2`…);
- **por colores de la textura**: los triángulos se agrupan por el color que
  tienen debajo (`color_1`, `color_2`…).

Las zonas se numeran de la que más caras tiene a la que menos, y salen al
montar.

## Entrega 9: empezar un juego desde cero

`kiln serve` en una carpeta sin `kiln.toml` abre un asistente de cinco pasos
en vez de fallar:

1. **Proyecto**: el nombre. El tipo es «Juego», el único por ahora
   (`[project] kind = "game"`; `check` avisa de cualquier otro).
2. **Lo básico**: cámara, estilo, altura de una persona, licencias y
   triángulos por pieza, con respuestas sugeridas. «Crear el proyecto»
   escribe `kiln.toml` (licencias y triángulos van a `[rules]`), una paleta
   vacía y `kiln/context.toml` con cada respuesta fijada en su capa.
3. **Tu juego**: lo explicas con tus palabras; el agente lo ordena en capas,
   escribe el párrafo de qué juego es (`[intent]` del contexto) y te hace
   una ronda de preguntas.
4. **Preguntas**: las respondes con un clic o con tus palabras.
5. **Asentar**: fijas el párrafo y al menos una afirmación por capa, sin
   choques abiertos. Hasta entonces el botón dice qué falta. Al asentar se
   abre el grafo de siempre.

Cada paso queda en disco: si cierras la página, el asistente sigue donde
estaba. Los proyectos que ya tenían `kiln.toml` se abren como antes.

```bash
mkdir ../mi-juego && target/release/kiln serve --project ../mi-juego
```

## Entrega 10: hablar con el agente en el asistente

Cada paso del asistente tiene al lado un panel «Hablar con el agente», y la
pestaña Contexto lo tiene en su ficha. Es un solo hilo: el agente recuerda
los últimos 20 mensajes y sabe en qué paso estás.

- Puedes debatir («¿encaja una cámara cenital con esto?») o pedir mejoras
  («dame tres nombres», «otras opciones de cámara», «hazlo más oscuro»).
- Cada mensaje lleva lo que ya has fijado: el nombre y lo básico elegidos,
  y desde el paso 3 el párrafo fijado, las afirmaciones fijadas y tus
  respuestas. El agente cuenta con ello y no lo contradice; si cree que algo
  debe cambiar, lo dice.
- Pasos 1 y 2: aún no hay proyecto, así que el agente solo lee y propone.
  Sus nombres y opciones salen como botones; la carpeta sigue vacía hasta
  «Crear el proyecto».
- Desde el paso 3 cambia el contexto (`kiln/context.toml`). Lo que fijaste
  no lo toca: deja una propuesta con Aceptar y Descartar. Tu explicación
  tampoco: si la mejora, sale «Usar este texto».
- El hilo se guarda en `chat.toml`, junto al contexto. Los mensajes de los
  pasos 1 y 2 se guardan al crear el proyecto.

## Entrega 10: assets hechos de assets

Una ficha puede usar otra ficha como pieza, con la fuente `asset`. Así se
hacen cosas grandes por niveles, cada uno con su ficha, su montaje y su
revisión: el muelle de carga usa dos grúas, la grúa usa sus piezas.

```toml
[[parts]]
id = "old_hoist"
what = "grúa de carga curtida por el salitre"
source = { kind = "asset", from = "hoist", variant = "weathered" }
at = [[2.2, 0.2, 0]]
```

- Cada asset es una caja cerrada: llega montado, con su medida y sus
  materiales. Para cambiarle los colores se elige una de sus variantes.
- `kiln build --asset cargo_quay` monta antes, de abajo arriba, los assets
  que usa y que no estén montados o estén por rehacer.
- Un asset montado queda «por rehacer» (`stale` en `builds`) si alguno de los
  que usa se montó después que él.
- Kiln avisa si una ficha usa una que no existe o una variante que no tiene,
  si una ficha acaba dentro de sí misma, y si los triángulos de lo que usa
  pasan de `budget_tris`.
- En el GLB, los materiales guardan el camino entero:
  `old_hoist/post/Wood`.
- En el grafo de la ficha, doble clic en la fuente de un asset entra en su
  ficha, y su panel tiene «Entrar en su ficha».
- Arriba, la miga de pan enseña el camino por el que has bajado
  (`Frente del puerto › Muelle de carga › Grúa`); cada tramo sube.
- El panel del asset dice de qué assets está hecho, dónde se usa y cuántas
  veces, y si está por rehacer.
- El buscador de arriba también encuentra assets, con su camino; Intro abre
  el primero.

El ejemplo está en `examples/specs/cargo_quay.toml`.

## Entrega 11: Kiln por MCP

`kiln mcp [--project DIR] [--config FICHERO]` sirve Kiln por MCP sobre stdio,
para Claude Code. El agente que llama hace el trabajo de agente (desglosar
una petición en ficha, colocar, ordenar el contexto) siguiendo los prompts;
Kiln comprueba y escribe. No escucha en ninguna red.

Se registra una vez, sin `--project`, y sirve cualquier juego (K014):

```bash
cargo build --release
claude mcp add --scope user kiln -- /ruta/a/kiln mcp
```

Al arrancar abre el primero que encuentre de: las *roots* del cliente, la
carpeta desde la que se abrió Claude Code, el último proyecto usado. Cuenta
como proyecto una carpeta con `kiln.toml` o con `project.godot`. Dentro de la
conversación se cambia de juego con `project_open` o se empieza uno con
`project_create`; `projects` lista los recientes. El registro de proyectos
está en `~/Library/Application Support/Kiln/projects.json` en macOS
(`$XDG_DATA_HOME/kiln` en Linux; `KILN_HOME` lo cambia). Con `--project` se
comporta como antes: ese juego desde el principio.

Son 35 herramientas. Empieza por `status`, que dice qué carpeta está abierta.

| Grupo | Herramientas |
|---|---|
| Leer | `status` resumen y siguiente paso; `tree` árbol, un nivel cada vez; `catalog` kits, piezas, materiales, fichas o la UI del juego (`what=ui`); `check` hallazgos; `content` conjuntos de contenido; `spec_get` una ficha; `review_sheets` hojas de la revisión como imágenes |
| Fichas | `spec_write` escribe una ficha si pasa la comprobación; `spec_layout` solo coloca piezas (`at`, `turn`) |
| Contexto | `context_get` lee; `context_propose` la voz del agente; `context_edit` la voz de la persona |
| Producción y trabajos | `build` monta con Blender; `generate` genera con Godot un asset de receta; `paint` aplica materiales; `godot_measure` mide lo que solo Godot abre y exporta las piezas `.blend`; `review` revisa el montaje; `verify` comprueba el GLB montado; `publish` lo lleva al juego; `tools` dice qué herramientas externas hay y su versión; `job` y `jobs` siguen los trabajos |
| Materiales y zonas | `material_set` material de una zona; `material_validate` lo pasa al catálogo; `bind` vincula una ranura a la paleta; `zones_split` parte una pieza en zonas |
| Fuera | `search` busca candidatos (no escribe); `import` trae un modelo con su licencia |
| Proyectos | `projects` los recientes y cuál está abierto; `project_open` abre otro por su ruta; `project_create` empieza uno en una carpeta nueva o en una de Godot sin `kiln.toml` |
| Juego nuevo | `new_game_options` el paso de arranque; `new_game` crea el juego en la carpeta abierta; `settle` lo da por asentado si no falta nada |
| Contenido | `content_manifest` Godot escribe el manifiesto de escenas (la orden `kiln content-manifest`) |

Tres prompts: `kiln-asset` (de la petición a la ficha), `kiln-context`
(ordenar o afinar el contexto) y `kiln-review` (explicar los hallazgos de la
revisión).

### Lo que hace la página, por MCP

| Página | MCP |
|---|---|
| `GET /api/catalog`, `/api/context` | `catalog` (y `status`), `context_get` |
| `POST /api/ask` (pedir ficha, colocar) | prompt `kiln-asset` + `spec_write`, `spec_layout` |
| `POST /api/context/sort`, `/refine` | prompt `kiln-context` + `context_propose` |
| `POST /api/context/text`, `intent`, `answer`, `claim` | `context_edit` |
| `POST /api/build`, `generate`, `paint`, `godot`, `review` | `build`, `generate`, `paint`, `godot_measure`, `review` (+ `review_sheets`) |
| `POST /api/search`, `import` | `search`, `import` |
| `POST /api/bind`, `material`, `validate`, `split` | `bind`, `material_set`, `material_validate`, `zones_split` |
| `GET` y `POST /api/new`, `POST /api/settle` | `new_game_options`, `new_game`, `settle` |
| `GET /api/jobs`, `/api/jobs/<id>` | `jobs`, `job` |
| `GET` y `POST /api/chat` | no hay (D2) |

`tests/coverage.rs` falla si la página gana una ruta o la CLI una orden sin su
herramienta.

### Un trabajo largo

`build`, `paint`, `godot_measure`, `review`, `verify`, `publish` e `import` tardan: devuelven
un trabajo. `job` espera hasta 30 s a que haya líneas nuevas o termine, y
cuando llegan sigue juntándolas hasta un segundo de silencio; se vuelve a
llamar con `since` = el `next` anterior hasta que el estado deje de ser
`running`. `jobs` los lista, con la carpeta del proyecto en que se lanzó cada
uno: cambiar de proyecto no para los trabajos en curso.

`kiln-asset` se puede lanzar sin argumentos (`/mcp__kiln__kiln-asset`): el
agente pregunta qué asset quieres. Claude Code parte el slash command por
espacios, así que una petición de varias palabras va en el mensaje siguiente.

`review` cambia la ficha con lo que corrige el revisor y deja cada vuelta en
`<build>/<id>/review/round-N/`.

### Las dos voces del contexto

`context_propose` es el agente: lo que fijó la persona no se toca, queda como
propuesta pendiente. `context_edit` es la persona: solo para lo que dice o
elige con sus palabras, y lo que registra queda fijado.

### Límites

- `review` sigue lanzando el revisor propio de Kiln, aparte (D1): usa la
  sesión de `claude` y gasta uso de la suscripción.
- No hay chat por MCP (D2): el agente que llama ya es el agente.
- Probado solo con Claude Code (D3).
- Los trabajos viven en el proceso: cerrarlo pierde el que esté en curso.
- `kiln serve` y `kiln mcp` abiertos a la vez sobre el mismo juego pueden
  escribir los mismos ficheros, sin bloqueo. La página no ve los trabajos
  lanzados por MCP.
- Las claves de Poly Pizza y Sketchfab nunca salen de Kiln.

## Entrega 12: de la fuente al juego (K006)

Kiln lleva un asset desde su fuente editable hasta el proyecto del juego:
monta, verifica y publica. Cada paso deja un fichero que el siguiente lee.

```bash
kiln tools   --project ../mi-juego                 # qué herramientas hay y en qué versión
kiln build   --project ../mi-juego --asset door    # monta y escribe asset.json
kiln verify  --project ../mi-juego --asset door    # comprueba el GLB y escribe validation.json
kiln publish --project ../mi-juego --asset door    # lo copia al juego, si la verificación pasó
```

Por MCP son `tools`, `build`, `verify` y `publish`; `spec_get` devuelve el
contrato y el último informe. `tools` ejecuta los programas de `[tools]` con
`--version`, por eso no es de solo lectura; `status` no ejecuta ninguno.

**Herramientas.** `kiln tools` ejecuta cada una con `--version`: Blender (probado
desde la 5.0), Godot (desde la 4.7), el glTF Validator de Khronos,
glTF-Transform, `claude` y `curl`. `status` dice solo si están, sin
ejecutarlas: es una lectura, y las rutas de `[tools]` las pone el proyecto.
Kiln no instala nada. glTF-Transform se informa pero no se usa.

```toml
[tools]
timeout_s = 600                     # plazo de cada ejecución de Blender o Godot
gltf_validator = "gltf_validator"   # opcional: si no está, consta como ausente
```

Blender y Godot se lanzan desde un solo sitio (`crates/kiln-cli/src/run.rs`):
en la carpeta de trabajo del asset, con ese plazo, y dejando el registro
completo en `<build>/<id>/.kiln/logs/`. Blender corre con
`--disable-autoexec`.

**Piezas con fuente `.blend`.** Un `.blend` dentro de un kit es una pieza.
Kiln lo exporta a GLB en la caché (solo las mallas, con los modificadores
aplicados, sin ejecutar los scripts que lleve dentro; si tiene una armadura,
también sale, ver la entrega 14) y anota su sha256. Lo
exportan `kiln build`, para las piezas que usa el asset, y `kiln godot`, para
todas. Si el `.blend` cambia, la pieza sale con el aviso `blend_stale` hasta
la siguiente exportación. Si junto al `.blend` hay un `.glb` con el mismo
nombre, vale el `.glb`.

**Anclajes.** Una ficha dice por dónde se une el asset a otra cosa:

```toml
# en la ficha
[[anchors]]
id = "mount"
class = "doorway"
at = [0, 0, 0]          # metros desde el origen del asset, Y arriba
turn = [0, 0, 0]        # grados

# en kiln.toml
[anchors.doorway]
fits = ["wall_gap"]     # clases con las que encaja
opening_m = [1.4, 2.2]  # ancho y alto del hueco donde debe caber

[anchors.wall_gap]
```

El montaje exporta cada anclaje como un nodo vacío `anchor_<id>`. `kiln check`
da error si la clase no está declarada (`anchor_class`) o si el asset montado
no cabe en el hueco de su clase (`anchor_opening`).

**El contrato.** `build` y `paint` escriben `<build>/<id>/asset.json`: el id,
para qué sirve (de la ficha), la ficha y cada pieza con su sha256, licencia y
crédito, cada GLB con su sha256 y su hash estructural, medidas, ejes y
origen, anclajes, materiales y las versiones de las herramientas. El hash
estructural sale de los nodos, su jerarquía, sus transformaciones (a 0,1 mm),
los triángulos y los materiales: dos montajes con el mismo hash estructural
son equivalentes aunque los ficheros no fueran idénticos byte a byte.

**Verificar.** `kiln verify` hace tres comprobaciones y escribe
`validation.json`:

| Comprobación | Qué mira |
|---|---|
| `kiln` | Que sea un GLB válido y sin referencias rotas; nombres de nodo vacíos o repetidos; escala negativa o desigual en la raíz; triángulos sobre el presupuesto; medidas distintas de las que dio Blender; materiales sin clave de la paleta; piezas pendientes; anclajes que faltan o están fuera de sitio |
| `gltf_validator` | El informe del validador de Khronos. Si no está instalado consta como ausente y no cuenta como fallo |
| `godot_import` | Godot importa el GLB en un proyecto de prueba de la caché, lo instancia y devuelve sus nodos |

**Publicar.** `kiln publish` solo publica si la última verificación pasó y
corresponde al GLB de ahora (por sha256). Copia el GLB, sus variantes y el
contrato a la carpeta de `[publish]`, con la versión de producción, que sube
cuando cambia lo publicado. Si esa carpeta está dentro del proyecto de Godot,
Godot la importa y Kiln comprueba que cada GLB tiene su `.import`; el
contrato con la versión nueva se escribe solo después. Si Godot no importa,
los GLB quedan copiados pero la versión publicada no cambia.

Los avisos no impiden publicar: un asset con piezas pendientes (cajas en lugar
de un modelo) o con materiales fuera de la paleta se verifica y se publica, y
los avisos constan en el informe. Las variantes de color pasan la comprobación
del fichero, no las tres; todas quedan atadas a la verificación por su sha256,
así que volver a pintar una variante obliga a verificar otra vez.

```toml
[publish]
dir = "godot/assets"    # dentro del proyecto; cada asset va a <dir>/<id>/
```

Los tests que lanzan Blender o Godot de verdad se ejecutan a mano, con las
herramientas en el `PATH`: `cargo test -p kiln-cli -- --ignored`.

Límites: importar en el proyecto de Godot del juego (al publicar, y al
montar si `[materials] godot` vincula materiales) abre ese proyecto, y Godot
ejecuta los plugins de editor y los scripts `@tool` que tenga activados: hazlo
solo con proyectos de confianza. La verificación no tiene ese problema: importa
en un proyecto de prueba vacío. `[tools] build` y `[tools] cache` los fija
`kiln.toml` y Kiln escribe ahí sin comprobar que queden dentro del proyecto,
como antes de esta entrega. Si una herramienta vence su plazo, Kiln la corta y
sigue, pero un proceso que ella hubiera lanzado puede seguir vivo. Un `.blend`
que no sea un fichero del propio proyecto (un enlace que sale fuera) no se
abre: sale con el aviso `blend_outside`. Kiln no ejecuta scripts del proyecto
ni del cliente; un retoque a
mano en el `.blend` del montaje se pierde al volver a montar (la fuente son
los `.blend` de las piezas y la ficha); no optimiza GLB; la página web no
tiene botones para verificar ni publicar.

## Validar contenido

Opcional y por proyecto. Kiln no sabe nada de ningún juego: cada proyecto
declara en su `kiln.toml` qué tipos de contenido tiene y cómo se
relacionan, y Kiln comprueba sus ficheros RON contra ese esquema, contra el
catálogo de assets y contra las escenas de Godot. Kiln prepara y valida
contenido; no es el motor del juego ni toca sus guardados. Un `kiln.toml`
sin `[content]` funciona igual que antes.

Cada **conjunto** (`[content.<conjunto>]`) agrupa unos ficheros de
definiciones y los tipos que contienen:

```toml
[tools]
godot_project = "godot"

[content.world]
definitions = ["data/content/**/*.ron"]
manifest = "kiln/content_manifest.json"
greybox = false
id_meta = "astra_id"

[content.world.kinds.Sector]
id = "id"
scene = "scene"

[content.world.kinds.Door]
id = "id"
refs = { connects = "Sector" }
distinct = ["connects"]
node_meta = { connects = "astra_connects" }

[content.world.kinds.Object]
id = "id"
asset = "asset"

[content.world.kinds.Installation]
id = "id"
refs = { sector = "Sector", objects = "Object" }
```

```ron
[
    Sector(id: "sector:warehouse", scene: "res://astra/warehouse.tscn"),
    Sector(id: "sector:yard", scene: "res://astra/yard.tscn"),
    Door(id: "door:warehouse_yard", connects: ("sector:warehouse", "sector:yard")),
    Object(id: "object:crate", asset: "harbor/crate"),   // clave de pieza del catálogo
    Installation(id: "installation:loading_bay", sector: "sector:warehouse", objects: ["object:crate"]),
]
```

Es la fixture `examples/content/`. Los IDs son estáticos de contenido, no
los que el juego asigne en ejecución.

### Referencia del esquema

Una clave que no esté en esta lista es un error (`content_schema`), nunca
se ignora, y el conjunto afectado no se valida hasta que se arregle.

**Conjunto** (`[content.<conjunto>]`)

| Clave | Qué es |
|-------|--------|
| `definitions` | Obligatoria. Globs desde la raíz del proyecto con los ficheros RON. No puede estar vacía. |
| `manifest` | Fichero JSON (dentro del proyecto) donde `kiln content-manifest` deja lo que Godot ve en las escenas. Sin ella no se comparan escenas. |
| `greybox` | `true`: un asset que falta en el catálogo es aviso, no error. Por defecto `false`. |
| `id_meta` | Metadato de los nodos de Godot que lleva su ID. Por defecto `content_id`. |
| `root` | Si el fichero es un registro `( ... )` y los elementos son una lista de ese campo, su nombre. Exige `default_kind`. Sin ella el fichero es una lista. |
| `default_kind` | Tipo de los elementos sin nombre (`( ... )` en vez de `Tipo( ... )`). Debe estar declarado. |
| `kinds` | Los tipos, `[content.<conjunto>.kinds.<Tipo>]`. |

**Tipo**

| Clave | Qué comprueba |
|-------|---------------|
| `id` | Obligatoria. Campo de texto con el ID del elemento. IDs repetidos: `content_duplicate_id`. |
| `id_prefix` | El ID en la escena es prefijo + valor del campo. Las referencias aceptan el valor con o sin prefijo. |
| `scene` | Campo con una ruta `res://` relativa y sin `..` que debe existir en `[tools] godot_project`, y el nodo con el ID debe estar en esa escena. |
| `asset` | Campo con una clave de pieza (`kit/pieza`) que debe estar en el catálogo. |
| `refs` | `campo = "Tipo"`. Cada valor del campo (suelto, tupla o lista) debe ser el ID de un elemento de ese tipo. |
| `distinct` | Campos cuyos valores (tupla o lista) deben ser todos distintos. |
| `node_meta` | `campo = "metadato"`. El campo debe valer lo mismo que ese metadato del nodo, según el manifiesto. |
| `markers` | Sublistas contra nodos hijos del nodo de la escena (ver abajo). |
| `fields` | Reglas por campo (ver abajo). |

**Marcador** (`markers = [ { ... } ]`): una lista de elementos que se
corresponde con hijos directos del nodo.

| Clave | Qué es |
|-------|--------|
| `field` | Campo del elemento con la lista. Si falta, se toma como lista vacía. |
| `name` | Campo de cada elemento de la lista con su nombre. |
| `node` | Nombre del nodo hijo esperado; debe contener `{name}`. |
| `meta` | Campos del elemento que deben valer lo mismo que el metadato del hijo con ese nombre. |

Un hijo que encaja con el patrón `node` y no corresponde a ningún elemento
de la lista es `content_marker_extra`.

**Regla de campo** (`fields = [ { ... } ]`)

| Clave | Qué comprueba |
|-------|---------------|
| `name` | Obligatoria. El campo. |
| `required` | `true`: el campo debe estar. Un campo ausente sin `required` no se comprueba. |
| `type` | `string`, `int`, `float`, `bool`, `ident` (palabra sin comillas), `list`, `tuple` o `struct`. Un `int` vale como `float`. |
| `one_of` | Lista cerrada de valores admitidos, para texto o `ident`. No puede estar vacía. |
| `min`, `max` | Rango admitido, para números. `min` no puede ser mayor que `max`. |
| `within` | Opcional. Nombre de un campo lista del elemento: la regla se aplica al campo `name` de cada ítem (una estructura) de esa lista, con el mismo significado de `required`, `type`, `min`/`max` y `one_of`. El hallazgo nombra la lista y el ítem (su campo `name` o, si no lo tiene, su posición). Una lista ausente no tiene ítems; un campo que no es lista o un ítem que no es estructura es un `content_field_type`. |

Por ejemplo, para exigir que cada conector tenga un `side` conocido:

```toml
fields = [
    { name = "side", within = "connectors", required = true, type = "ident", one_of = ["North", "East", "South", "West"] },
    { name = "offset", within = "connectors", type = "int", min = 0 },
]
```

Con `side: Up` en el conector `east`, el hallazgo es `content_field_value` y
dice «`connectors[east].side` vale «Up»...».

`definitions` y `manifest` son rutas del proyecto: una ruta absoluta o con
un componente `..` es un problema `content_schema` del conjunto.

### Un segundo ejemplo: una biblioteca en un solo fichero

Un registro con una lista (`root`), elementos sin nombre (`default_kind`),
prefijo de ID y marcadores:

```toml
[tools]
godot_project = "godot"

[content.library]
definitions = ["data/modules.ron"]
manifest = "kiln/library_manifest.json"
root = "modules"            # el fichero es un registro y sus elementos son esta lista
default_kind = "Module"     # tipo de los elementos sin nombre

[content.library.kinds.Module]
id = "key"
id_prefix = "module:"       # en la escena el ID es «module:<key>»
scene = "scene"
markers = [
    { field = "connectors", name = "name", node = "conn_{name}", meta = ["side", "offset"] },
]
fields = [
    { name = "kind", required = true, one_of = ["Access", "Workshop"] },
    { name = "size", type = "tuple" },
]
```

```ron
(
    modules: [
        (
            key: "access",
            kind: Access,
            size: (3, 2),
            connectors: [(name: "east", side: East, offset: 0)],
            scene: "res://modules/access.tscn",
        ),
    ],
)
```

La escena `access.tscn` tendría un nodo raíz con el metadato
`content_id = "module:access"` y un hijo `conn_east` con los metadatos
`side` y `offset`.

### Los dos comandos

```bash
kiln content-manifest --project examples/content   # Godot sin ventana lee las escenas
kiln check --project examples/content              # valida RON, catálogo, escenas y manifiesto
```

Salida de los dos sobre la fixture:

```
$ kiln content-manifest --project examples/content
world: Godot lee 2 escenas (godot)
world: manifiesto en <ruta absoluta>/examples/content/kiln/content_manifest.json
$ kiln check --project examples/content
2 piezas, 0 errores, 0 avisos
```

Y con una definición rota (una puerta que conecta con un sector que no
existe, y una instalación cuyo `sector` es un objeto):

```
ERROR  world:data/content/world.ron#door:warehouse_yard content_unknown_ref  `connects` hace referencia a «sector:nowhere», que no existe
ERROR  world:data/content/world.ron#door:warehouse_yard content_manifest_mismatch `connects` distinto: definición [sector:warehouse, sector:nowhere], manifiesto ["sector:warehouse","sector:yard"]
ERROR  world:data/content/world.ron#installation:loading_bay content_wrong_kind   `sector`: «object:crate» debería ser Sector y es Object
2 piezas, 3 errores, 0 avisos
```

Si hay algún error, `kiln check` sale con código distinto de cero. Los
hallazgos salen también en `kiln serve`, en la pestaña **Contenido**: cada
conjunto con sus tipos, sus elementos (tipo, ID, fichero y línea) y los
hallazgos de cada uno. La pestaña no aparece si el proyecto no declara
ningún conjunto.

`content-manifest` necesita `[tools] godot_project` y Godot 4
(`[tools] godot`). Escribe el manifiesto de cada conjunto que declara
`manifest` (o solo el de `--set NOMBRE`). Lee las escenas sin instanciarlas,
así que no se ejecutan los scripts de los nodos, pero Godot sí carga los
recursos que la escena enlaza, y con ellos sus scripts. Como `kiln godot`,
úsalo solo en proyectos de confianza. El manifiesto es un fichero técnico
generado que se versiona con el proyecto: la fuente del contenido sigue
siendo el RON.

### Manifiesto de escenas (versión 2)

```json
{
  "version": 2,
  "set": "world",
  "scenes": [
    {
      "scene": "res://astra/warehouse.tscn",
      "nodes": [
        {
          "id": "sector:warehouse",
          "meta": {},
          "children": [ { "name": "DoorToYard", "meta": { "astra_connects": ["sector:warehouse", "sector:yard"], "astra_id": "door:warehouse_yard" } } ]
        }
      ]
    }
  ]
}
```

Un nodo entra en la lista si tiene el metadato `id_meta`; ese valor es su
`id` y se quita de `meta`. `children` son sus hijos directos con sus
metadatos. Una versión distinta de 2, o un `set` que no es el del conjunto,
es `content_manifest_syntax`. Una escena que Godot no carga sale con un
campo `error`.

### El RON que lee Kiln

Kiln lee su propio subconjunto de RON, porque el crate `ron` pierde los
nombres de las estructuras, las palabras sin comillas y el orden de los
campos.

Admite: listas `[a, b]`, tuplas `(a, b)`, estructuras con nombre
`Tipo(campo: valor)` o sin él `(campo: valor)`, cadenas con las escapadas
`\"`, `\\`, `\n` y `\t`, enteros y decimales (con `_` y signo), `true` y
`false`, palabras sin comillas (`East`), comas finales y comentarios `//` y
`/* */`. El anidamiento máximo es 64. Se ignora una marca BOM UTF-8 al
principio del fichero.

Son errores de lectura un campo repetido dentro de una misma estructura
(en el segundo) y un entero fuera del rango de `i64`.

No admite: mapas `{ }`, `Some(...)` (`None` se lee como una palabra sin
comillas), estructuras con nombre y valores posicionales `Tipo(1, 2)`,
números hexadecimales, cadenas crudas `r"..."`, caracteres `'a'`, otros
escapes (`\u{..}`, `\r`) ni atributos `#![enable(...)]`. Un
error de lectura es un hallazgo `content_syntax` con línea y columna, nunca
una caída.

### Hallazgos

Salen en `kiln check` y en `kiln serve` como los demás, con el asunto
`<conjunto>:<fichero>#<id>` (o `<conjunto>:<fichero>` si no es de un
elemento, y `<conjunto>` si es de todo el conjunto).

| code | Cuándo |
|------|--------|
| `content_schema` | El esquema del `kiln.toml` no vale: clave desconocida, tipo sin declarar, `root` sin `default_kind`, `min` mayor que `max`... |
| `content_no_definitions` | Aviso: los globs de `definitions` no encuentran ningún fichero. |
| `content_syntax` | El RON no se lee, o no tiene la forma que pide `root` (con línea y columna). |
| `content_unknown_kind` | Un elemento con un tipo que el esquema no declara, o sin nombre y sin `default_kind`. |
| `content_missing_id` | Un elemento sin el campo `id` del tipo, o que no es de texto. |
| `content_duplicate_id` | Un ID repetido. |
| `content_unknown_ref` | Una referencia (`refs`) nombra un ID que no existe, o un valor que no es texto. |
| `content_wrong_kind` | La referencia existe pero es de otro tipo. |
| `content_not_distinct` | Un campo de `distinct` repite un valor. |
| `content_asset_missing` | El asset no es una pieza del catálogo (aviso con `greybox = true`). |
| `content_scene_missing` | La escena `res://` no es válida, no existe en el proyecto Godot o falta `[tools] godot_project`. |
| `content_field_missing` | Falta un campo con `required = true`. |
| `content_field_type` | El campo no es del `type` que pide la regla. |
| `content_field_range` | Un número fuera de `min`/`max`. |
| `content_field_value` | Un valor que no está en `one_of`. |
| `content_manifest_missing` | Aviso: falta el manifiesto; hay que ejecutar `kiln content-manifest`. |
| `content_manifest_syntax` | El JSON del manifiesto no se lee, o su versión o su conjunto no son los esperados. |
| `content_manifest_mismatch` | El manifiesto y el RON no coinciden: el nodo falta o está en otra escena, un metadato (`node_meta`) cambia, o hay un nodo sin definición. |
| `content_marker_missing` | Falta el nodo hijo que pide un elemento de un marcador. |
| `content_marker_mismatch` | El hijo existe pero un metadato (`meta`) no vale lo mismo que el campo. |
| `content_marker_extra` | Hay un nodo hijo con el patrón del marcador que no corresponde a ningún elemento. |

El test que exporta con Godot de verdad se ejecuta a mano (necesita
Godot 4 en el `PATH`): `cargo test -p kiln-cli -- --ignored content`.

### Pendiente

Las tablas de compatibilidad entre conectores no están implementadas:
ningún contenido usa todavía clases de conector. Hoy solo se comprueba que
el `side` de cada conector esté en una lista cerrada (`within` + `one_of`).

## Interfaz

La página que abre `kiln serve` es una aplicación de módulos ES sin paso de construcción: no hay Node, ni bundler, ni dependencias que instalar para usarla. Todo está en `crates/kiln-cli/ui/app/` y el binario lo lleva dentro.

**Cómo está organizada**

- `main.js` arranca: crea el almacén, el cliente de la API, el router y el marco, y lo monta.
- `store.js` es el almacén: un mapa de claves (`catalog`, `context`, `route`, `selection`, `jobs`…) con suscripción por clave. Las vistas leen de él y no se hablan entre sí.
- `api.js` es el cliente: lecturas con `If-None-Match` (el servidor contesta 304 si nada cambió), peticiones idénticas en vuelo compartidas, errores en castellano.
- `router.js` convierte `#/vista/arg1/arg2?clave=valor` en una ruta y al revés. Funciones puras, con pruebas.
- `shell.js` es el marco: barra superior, navegación, zona principal, panel de detalle, buscador, paleta de comandos y avisos.
- `views/*.js` son las vistas (inicio, contexto, árbol, contenido, catálogo, materiales, ficha, montaje, trabajos, asistente). Cada una exporta `mount(contenedor, ctx)` y devuelve su función de limpieza. `ui/` tiene los componentes (lista virtual, avisos, diálogo, paleta).
- `html.js` es la plantilla `html` que escapa todo lo que interpola; los números en texto salen con puntos de millar (`5.000`), aunque tengan cuatro cifras.
- Los módulos se sirven bajo `/ui/<ruta>` desde `UI_FILES`, la lista de `crates/kiln-cli/src/serve.rs`.

**Añadir una vista** (tres pasos)

1. Crea `ui/app/views/<nombre>.js` con `export function mount(el, ctx) { ...; return () => limpieza; }`.
2. Regístrala en `views/registry.js` (`<nombre>: () => import("./<nombre>.js")`) y añade el nombre a `VIEWS` en `router.js`; si va en el menú, una entrada en `navModel` de `nav.js`.
3. Añade el fichero a `UI_FILES` en `serve.rs` (una línea con `include_str!`), y una comprobación en `tests/app_views*.mjs`.

**Desarrollar sin recompilar**: `kiln serve --project DIR --ui-dir crates/kiln-cli/ui` lee los módulos del disco en cada petición; basta con recargar la página.

**Rutas**: `/` es la interfaz (`/app` sigue sirviendo la misma página para que los enlaces antiguos funcionen). `/classic` es la interfaz anterior, con un aviso de que se retirará en la próxima entrega; la interfaz nueva enlaza a ella desde el menú ("Interfaz anterior").

**Teclado**: `Ctrl K` (`Cmd K` en Mac) abre la paleta (vistas, acciones, piezas, kits y materiales; incluye "Pedir un asset…"); `/` va al buscador; en las listas, flechas, `Inicio`/`Fin` y `Intro` para elegir; `Esc` cierra la paleta, el diálogo o el panel de detalle y devuelve el foco a donde estaba. Hay un enlace "Saltar al contenido" y, a 900 px, el panel de detalle se superpone y se cierra con `Esc`.

**Pruebas**

- `scripts/ui_test.sh` corre las pruebas unitarias (`node --test`, incluida la del contraste de color AA de los dos temas) y un recorrido en Chrome sin ventana por `examples`, `examples/content`, un proyecto sintético y una carpeta vacía (el asistente), contra `/` y contra `/classic`. Incluye el recorrido solo con teclado, la revisión de roles y etiquetas y las pruebas a 900 px. Necesita Node 22 y Chrome (`CHROME=ruta` para otro). Termina con `OK: ui walkthrough passed`.
- `scripts/ui_fixture.sh DIR` genera el proyecto sintético (5.000 piezas, 2.000 hallazgos) para probar a mano.
- `scripts/ui_test.sh --update-baseline` guarda en `crates/kiln-cli/ui/tests/baseline.json` las comprobaciones y las medidas; después, una comprobación que estaba en verde y deja de estarlo es una regresión.
- **Presupuesto** (falla el script): primera vista menos de 300 ms, filtro del catálogo menos de 50 ms, menos de 80 nodos de fila con 5.000 piezas. Los tiempos admiten una tolerancia del 25 % (`TOLERANCE` en `walkthrough.mjs`) porque Chrome sin ventana en una máquina ocupada es ruidoso; los nodos no tienen tolerancia.

**Medidas** (proyecto sintético, mediana de 3 ejecuciones, Chrome sin ventana). "Anterior, primera medida" es la línea base de la interfaz antigua tal como se guardó al empezar; "anterior, ahora" es la misma interfaz medida hoy, cuando el servidor ya guarda el catálogo en memoria.

| medida | anterior, primera medida | anterior, ahora | nueva |
|---|---:|---:|---:|
| primera vista (pintada), ms | 379 | 70 | 68 |
| cambiar a Catálogo, ms | 111 | 117 | unos 45-60 (varía entre ejecuciones: 44 a 62) |
| pulsación en el filtro del catálogo, ms | 74 | 65 | 1,3 |
| pulsación en el buscador global, ms | 74 | 65 | 122 (incluye 120 de espera) |
| nodos DOM, vista Catálogo | 8.158 | 8.160 | 413 |
| nodos DOM, vista Inicio | 123 | 125 | 183 |
| filas en el DOM, Catálogo | - | - | 24 |
| `/api/catalog` en frío (lectura forzada), ms | 330 | 396 | 396 |
| `/api/catalog` con caché (304), ms | 330 | 23 | 22 |
| memoria de JS tras recorrer las vistas, MB | 3,7 | 3,7 | 4,8 |
| con 20.000 piezas (medida aparte, una ejecución): Catálogo abre en 49 ms con 24 filas y 413 nodos; pulsación en el filtro, 1,6 ms; `/api/catalog` en frío 1,5 s (9,3 MB) y 87 ms revalidado; memoria de JS, 21 MB | - | - | ver la fila |

La primera vista de la interfaz antigua también mejoró (379 a unos 70 ms) porque el servidor ahora guarda el catálogo en memoria; la columna "en frío" fuerza una lectura completa del proyecto con la marca explícita, ya que las comprobaciones previas dejan la caché caliente.

"Releer" fuerza una lectura nueva del disco con la cabecera explícita `X-Kiln-Fresh: 1`; `Cache-Control: no-cache` (recarga forzada del navegador, DevTools con la caché desactivada) no vacía la caché del servidor.

La mejora está en lo que crece con el proyecto: el catálogo pinta solo las filas visibles, así que el filtro y el cambio de vista dejan de depender de las 5.000 piezas. Inicio tiene más nodos y la memoria es algo mayor porque la interfaz nueva dibuja más información.

## Entrega 13: la UI del juego

Si `kiln.toml` tiene un bloque `[ui]`, Kiln lee el tema de Godot (`.tres`) y los
scripts de interfaz como texto, sin abrir Godot, y comprueba que la UI se lee
y es coherente. Sin `[ui]` todo funciona como antes.

- `catalog what=ui` (MCP) da las variaciones del tema con su `base_type`, la
  cadena de herencia, el color, el tamaño de letra y dónde se usan
  (`uses`: `fichero:línea`), además de las fuentes (`fonts`) y los colores
  (`colors`) del tema.
- Cuatro reglas, cada una con su código de hallazgo y su nivel
  `level = "warn" | "error" | "off"` (por defecto `warn`; un `error` hace que
  `kiln check` salga con 1):

| Regla | Código | Qué mira |
|---|---|---|
| `contrast` | `ui-contrast`, `ui-contrast-unbacked` | contraste WCAG de cada variación con `font_color` contra su fondo |
| `glyphs` | `ui-glyph` | cada carácter de un texto literal existe en la tabla `cmap` de las fuentes del tema |
| `theme_coherent` | `ui-variation-missing`, `ui-variation-unused` | una variación usada que no existe, o definida y sin uso (ni como base de otra) |
| `theme_only` | `ui-style-outside-theme` | `Color(`, `Color.html`, `add_theme_color_override`, `add_theme_font_size_override`, `add_theme_stylebox_override` y `StyleBoxFlat.new` en el código de los scripts de UI (no dentro de cadenas) |

Una entrada del tema sin `base_type` (los estilos de un tipo de Godot como
`Button/styles/normal`, o `default_font`) no es una variación y nunca sale como
huérfana.

Dos códigos más no dependen de ningún nivel:

- `ui-theme` (error): el tema no se puede leer (un `.res` binario, por
  ejemplo) o está fuera del proyecto. Es lo único que se informa de la UI.
- `ui-files` (aviso): un patrón o una fuente fuera del proyecto, un glob que no
  encuentra ningún fichero o un enlace que sale de la raíz. Una fuente que no
  existe o no se lee sigue el nivel de `glyphs`.

Todas las rutas (tema, `target`, globs y fuentes `res://`) se confinan a la raíz
del proyecto, y los mensajes nunca citan el contenido de un fichero.

```toml
[ui]
theme = "godot/ui/theme.tres"
scripts = ["godot/ui/*.gd"]          # scripts de UI: variaciones usadas y estilo fuera del tema
scenes = ["godot/ui/*.tscn"]         # escenas con UI (opcional)
target = "ui_target.toml"            # estado de los criterios (opcional)

[ui.contrast]
normal = 4.5
large = 3.0
large_from_px = 18
level = "warn"

[[ui.contrast.pairs]]
fg = "HarborBody"                    # variación con font_color
bg = "HarborPanel/panel"             # <variación>/<caja de estilo>

[ui.glyphs]
level = "warn"
```

`examples/kiln.toml` lo muestra completo con Puerto Niebla (`examples/godot/ui/`).

**Contra qué fondo se mide.** Si la variación tiene `font_outline_color` y
`outline_size` mayor que 0 (propios o heredados), se mide contra el contorno,
que también puede ser translúcido. El resto necesita una pareja `[[ui.contrast.pairs]]`; el fondo es el
`bg_color` de una caja de estilo del tema. Si el fondo es translúcido se
compone sobre negro y sobre blanco y gana la razón más baja. Una variación con
`font_color` sin contorno ni pareja da un hallazgo `info` («sin fondo
declarado») y no cuenta para el objetivo. Una pareja con `kind = "non_text"`
mide dos cajas entre sí con el umbral `large`. Las cifras se truncan a un
decimal, así que 4,46 sale como 4,4 y no aparenta cumplir 4,5.

**El objetivo de UI (`ui_target.toml`).** Los criterios son afirmaciones
fijadas del contexto cuyo texto empieza por `Cn ·` (por ejemplo «C1 · …»); no se
escriben de nuevo en Kiln. El fichero `target` lo escribe el agente cuando el
PO confirma, y Kiln solo lo lee. Una fila por criterio:

```toml
[[criteria]]
id = "C1"
marker = "kiln:contrast"     # po | test | kiln:<regla>

[[criteria]]
id = "C3"
marker = "po"
status = "pending"           # met | unmet | pending
evidence = "hoja de revisión o prueba"
```

Para `kiln:<regla>` el estado lo calcula Kiln y el `status` del fichero se
ignora:
- cumple si la regla no da avisos ni errores;
- no cumple si los da, o si hay un `ui-theme` o un `ui-files`, porque entonces
  la regla no pudo medir;
- queda pendiente, con un problema, si la regla está en `off`.

Para `po` y `test` manda el `status` del fichero. Una regla, un `marker` o un
`status` desconocidos dejan el criterio pendiente y añaden un problema. `status` (MCP) da `ui_target: {met, total, percent, open, problems}`,
con `open` la lista de los que no cumplen o siguen pendientes y `problems` las
filas sin afirmación o las afirmaciones sin fila.

**Límites.** El texto que llega del servidor del juego no se mira, porque no
está en el código; solo se miran los literales de los scripts y las propiedades
`text` de las escenas, no las cadenas construidas en tiempo de ejecución. No hay
vista nueva en la página: los hallazgos salen en `check` (CLI y MCP) y en la vista
de comprobaciones que ya existe.

## Entrega 14: personajes con esqueleto (K008)

Una pieza `.blend` con armadura pasa por `build`, `verify` y `publish` con su
esqueleto, sus pesos y sus acciones. La ficha declara qué animaciones espera:

```toml
[asset]
id = "crew_member"
name = "Tripulante"
animations = ["idle", "walk"]     # nombres de las acciones del .blend

[[parts]]
id = "body"
what = "personaje con esqueleto"
size_m = [0.62, 1.67, 0.22]
source = { kind = "reuse", from = "crew/humanoid" }
at = [[0, 0, 0]]
```

- **Exportar la pieza.** Si el `.blend` tiene una armadura, sale con las
  mallas, las pieles y todas sus acciones. Sin armadura, como siempre.
- **Montar.** La pieza con esqueleto se coloca una vez, tal como viene: sin
  hornear, unir, recentrar ni escalar. Su origen es el de la fuente (por
  convenio, los pies en el origen); `at` y `turn` mueven la raíz. Los materiales se visten igual
  que en cualquier pieza.
  El tamaño que informa el montaje es el de la pose de reposo, como las
  cotas del GLB, empiece donde empiece la primera animación (K009).
- **Verificar.** Con `animations` declaradas, falla si el GLB no tiene
  esqueleto (`rig_no_skeleton`) o le falta una (`rig_animation_missing`), y
  Godot lo comprueba por su lado (`godot_no_skeleton`,
  `godot_animation_missing`); una animación que no mueve ningún hueso es un
  aviso (`godot_animation_static`). Siempre, con o sin declaración, un
  esqueleto o una animación mal formados son error: `skin_no_joints_attr`,
  `skin_no_weights_attr`, `animation_empty` y los índices rotos
  (`glb_broken_ref`). La normalización de los pesos la mira el glTF Validator.
- **Contrato.** `asset.json` lleva `outputs[].rig`, con el número de huesos y
  cada animación con su duración. El hash estructural cubre pieles y
  animaciones; el de un GLB sin ellas no cambia.

Límites: una pieza con esqueleto no admite varias copias, cambio de tamaño ni
división por zonas: el montaje se detiene. La ficha no sabe qué pieza trae el
esqueleto, así que con `animations` declaradas marca `rig_count` en cualquier
pieza con `count` mayor que 1, también una estática. Las variantes de color
de un asset con esqueleto conservan piel y animaciones en una prueba a mano,
pero no tienen test. Kiln no fija
el bucle de las animaciones: Godot las importa sin bucle y lo pone quien
integra. Sin formas clave, cinemática inversa ni cambio de esqueleto. La raíz
toma el nombre de la pieza en el montaje; si una malla se llamaba igual,
pasa a llamarse `<pieza>_mesh`. Con `animations` sin declarar no se comprueba
que estén: decláralas. Guarda el kit del personaje fuera del proyecto de
Godot (por ejemplo `kits_src/`, añadido a `[assets] kits`): con un `.blend`
dentro de `godot/`, Godot intenta importarlo al publicar y `publish` falla.
Kiln lee como mucho 1024 huesos por piel y 128 caracteres por nombre.

## Entrega 15: assets generados a partir de una receta (K010)

Hay assets cuya forma debe cambiar en cada ejemplar que hace el juego: una
roca, un asteroide. Su ficha no lleva piezas sino una **receta**: una pila de
operaciones de un vocabulario cerrado, con cada parámetro como valor fijo o
como rango que la semilla sortea.

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
op = "facet"
```

| Operación | Parámetros | Notas |
|---|---|---|
| `sphere` | `radius_m` (0,1 a 10 000), `segments` (8 a 256, entero) | Obligatoria y primera. Triángulos = 2 · segmentos · (segmentos/2 + 1) |
| `stretch` | `x`, `y`, `z` (0,2 a 5; por defecto 1) | Escala por eje |
| `noise` | `amplitude` (0 a 1, fracción del radio), `size` (0,01 a 4, tamaño del relieve respecto al radio) | Se puede repetir |
| `facet` | ninguno | Normales por cara |

El vocabulario vive en una sola tabla (`kiln-core`, `generate::vocab`): de
ella salen las comprobaciones, los controles de la página y el texto que
recibe el agente. Al pedir un asset, el agente decide si se monta con piezas
o se genera.

**Generar.** `kiln generate --asset ID [--seed N]`, la herramienta MCP
`generate` o el botón Aplicar de la ficha. Kiln sortea los rangos, lanza su
guion en Godot sin ventana sobre un proyecto vacío de la caché y deja
`<cache>/generated/<id>/<semilla>.glb` y `generated.json` (receta, semilla,
valores sorteados, medidas, triángulos, sha256 y versiones). En la página,
la ficha muestra un control por parámetro y la semilla; Aplicar guarda la
receta, genera y carga la malla en la vista 3D. Al guardar solo se cambia la
tabla `[generator]` de la ficha; lo demás (anclajes, variantes, claves que
Kiln no conoce) se queda, aunque los comentarios del fichero se pierden. Al
abrir una ficha ya generada con esa semilla, la vista muestra su GLB.

**El sorteo** se hace en Rust, no en Godot, para que otro programa pueda
repetirlo. Para el parámetro `p` de la operación número `i` (desde 0): la
clave es el texto `"<semilla>/<i>/<p>"`; `h` es su FNV-1a de 64 bits pasado
por un paso de SplitMix64; `u` son los 53 bits altos de `h` entre 2^53; el
valor es `mín + (máx − mín) · u`, hacia abajo si es entero. Cada `noise`
recibe además una semilla de ruido: los 32 bits bajos de `h` para la clave
`"<semilla>/<i>/noise"`.

`kiln check` da error si la receta usa una operación o un parámetro fuera del
vocabulario (`generator_op`, `generator_param`), un valor fuera de límites
(`generator_range`), si no empieza por una esfera (`generator_order`), si el
peor caso de triángulos pasa de `budget_tris` (`generator_budget`) o si lleva
receta y piezas a la vez (`generator_and_parts`). Un material fuera de la
paleta es un aviso (`generator_material`), como en las piezas.

Límites: un asset generado no se monta, pinta ni revisa; esas órdenes lo
dicen y no hacen nada (verificarlo y publicarlo, en la entrega 16). La misma
receta y semilla
dan el mismo GLB byte a byte en la misma máquina y versión de Godot; entre
máquinas no está comprobado. `generated.json` es el de la última semilla
generada, y cada semilla deja su GLB en la caché sin que nada los borre. La
carpeta de trabajo de Godot (`<cache>/generated/<id>/.project`) se vacía en
cada ejecución. Dos trabajos a la vez sobre el mismo asset no se pisan: el
segundo se niega (entrega 16). No hay cráteres, varios materiales ni
colisión.

## Entrega 16: verificar y publicar una receta (K011)

Un asset generado no tiene un GLB que dar por bueno: lo que se da por buena
es la receta, y para eso hay que mirar más de una semilla.

**Reglas sin generar.** `kiln check` sigue la receta en orden con una cota
del radio que empieza en 1: cada `stretch` la multiplica por su factor menor
y cada `noise` le resta su amplitud, siempre con el peor extremo de un
rango. Si llega a cero, la malla puede cruzarse consigo misma y es un error
(`generator_radius`). Si las amplitudes de ruido suman más de 0,5, aviso
(`generator_amplitude`): suele salir con picos.

**Verificar.** `kiln verify --asset ID` genera una tanda fija de semillas (la
de la ficha y las derivadas de ella, siempre las mismas), lee cada GLB y
mide la malla:

| Hallazgo | Qué mira | |
|---|---|---|
| `generated_flipped` | caras que miran hacia dentro | error |
| `generated_degenerate` | triángulos sin área, aparte de los dos por segmento de los polos | error |
| `generated_tris` | triángulos distintos de los de la esfera, o por encima de `budget_tris` o de `[rules] max_tris` | error |
| `generated_unreadable` | un GLB que Kiln no lee | error |
| `generated_steep` | fracción de caras a más de 60° de la dirección del radio, con el estirado deshecho, sobre el umbral | aviso |

Las comprobaciones propias del GLB pasan por todas las semillas; el
validador de Khronos y la importación en Godot, por la de la ficha. El
informe es el `validation.json` de siempre con un bloque `recipe`: el hash
de la receta, la versión y el hash del generador, lo medido en cada semilla
y el peor caso.

```toml
[generate]
verify_seeds = 16     # semillas de la tanda, de 1 a 64
steep_max = 0.10      # fracción de caras empinadas a partir de la que avisa
```

**Publicar.** `kiln publish --asset ID` solo publica si la última
verificación pasó y corresponde a la receta y al generador de ahora. Deja en
`[publish] dir/<id>/`:

- `recipe.json`: la receta y, por cada semilla verificada, sus operaciones
  con los valores ya sorteados, sus triángulos y sus medidas;
- `kiln_generator.gd`: el guion que construye la malla, el mismo con el que
  Kiln generó las de la verificación;
- `asset.json`: el contrato, con la versión, que sube cuando cambia la
  receta o el guion.

No se publica ningún GLB y Godot no importa nada. Un juego genera una de las
semillas así:

```gdscript
var recipe = JSON.parse_string(FileAccess.get_file_as_string("res://assets/kiln/asteroid/recipe.json"))
var entry = recipe["seeds"][0]
var made = load("res://assets/kiln/asteroid/kiln_generator.gd").build({"material": entry["material"], "ops": entry["ops"]})
# made["mesh"] es un ArrayMesh; made["error"] trae el motivo si algo falla
```

Las órdenes son las mismas de un asset de piezas, también por MCP (`verify`,
`publish`). `build`, `paint` y `review` siguen sin valer para un asset
generado.

Límites: la cota del radio es prudente y rechaza alguna receta que saldría
bien. El umbral de caras empinadas sale de once recetas medidas. Las
semillas publicadas son las de la tanda: para otra semilla, el juego tiene
que repetir el sorteo de la entrega 15. La semilla número `k` de la tanda
(desde 1) son los 32 bits bajos de `mix("<semilla>/verify/<k>")`; si
repite una anterior, se prueba con `"<semilla>/verify/<k>/<j>"` para `j`
desde 1. La malla no lleva colisión.

**Un informe viejo no vale.** Verificar borra primero el informe anterior,
así que una tanda que una herramienta corta a medias no deja ninguno en pie
y publicar pide verificar de nuevo. Además de la receta y el generador,
publicar comprueba que `[generate] verify_seeds` y `[rules] max_tris` son
los de cuando se verificó.

**Una cosa cada vez por asset.** Generar, verificar y publicar un asset
generado toman un cerrojo (`<cache>/generated/<id>/.lock`); mientras uno
está en marcha, los otros se niegan y lo dicen. Si un proceso muere sin
soltarlo, el siguiente lo toma cuando lleva sin dar señales más que el
plazo de una herramienta (`[tools] timeout_s`) y un minuto. Entre
máquinas o versiones de Godot no está comprobado que la misma semilla dé la
misma malla.

## Lo que viene

- El primer asset de punta a punta y la app de escritorio (Tauri).
- Assets hechos de assets, lo que falta: la vista 3D con el padre en gris, el árbol que sigue por las piezas-asset, el agente que pide un nivel cada vez y los anclajes.
- El grafo pasará a Rete.js cuando los nodos se puedan editar.
