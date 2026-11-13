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
# tu juego con el kiln.toml de ejemplo (o copia examples/kiln.toml a la raíz del juego)
target/release/kiln serve --project ../mi-juego --config examples/kiln.toml   # http://127.0.0.1:4330
target/release/kiln check --project ../mi-juego --config examples/kiln.toml   # sale con 1 si hay errores
target/release/kiln index --project ../mi-juego --config examples/kiln.toml --out catalog.json
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
en su `kiln.toml`. `examples/kiln.toml` es un ejemplo de un juego inventado.

## Entrega 2: pedir un asset

En el grafo, escribe la petición («crear un ascensor») y pulsa Desglosar.
Kiln lanza Claude Code sin ventana (`claude -p`, con la skill
`skills/asset/SKILL.md`) en el proyecto; el agente lee el proyecto y el
catálogo y escribe la ficha en la carpeta `[assets] specs` de `kiln.toml`.
El grafo sigue lo que hace y, al terminar, despliega la ficha: petición,
asset, piezas, de dónde sale cada modelo (reutilizar, variante, buscar,
retocar), material de la paleta y validación. `examples/specs/hoist.toml` es
una ficha de ejemplo.

- Necesita `claude` en el PATH (o su ruta en `KILN_CLAUDE`) con sesión iniciada.
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
kiln build --project ../mi-juego --config examples/kiln.toml --asset hoist
kiln godot --project ../mi-juego --config examples/kiln.toml
```

## Lo que viene

- Entrega 5: el contexto del proyecto. Una pantalla para explicar el juego, separada en capas en un grafo, con un modo de preguntas y respuestas sugeridas que lo afina; el agente lo lee al desglosar.
- El primer asset de punta a punta y la app de escritorio (Tauri).
- Componer escenas con módulos de piezas (un pasillo de suelo, techo, muros y luces).
- Kiln como servidor MCP, para que cualquier Claude le pida catálogo o assets.
- El grafo pasará a Rete.js cuando los nodos se puedan editar.
