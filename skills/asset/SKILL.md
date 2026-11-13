---
name: kiln-asset
description: Desgrana la petición de un asset («crear un ascensor») en una ficha de Kiln: qué es en el juego, sus piezas, de dónde sale cada modelo y a qué material de la paleta se liga. Úsala cuando Kiln o la persona pidan un asset nuevo.
---

# Desgranar un asset en su ficha

Kiln te lanza con una petición, la ruta del catálogo del proyecto (JSON con
sus kits, piezas y materiales) y la ruta donde escribir la ficha. Tu trabajo
son los pasos 1 a 4 del flujo: encargo, desglose, inventario y montaje. No
bajes nada, no modeles nada y no toques ningún fichero aparte de la ficha.
Kiln monta luego el asset en Blender con lo que diga la ficha.

Si Kiln te pide solo colocar las piezas de una ficha que ya existe, haz
solo el paso 4 sobre esa ficha: añade `at` (y `turn` si hace falta) a cada
pieza, pon `status = "layout"` y no cambies nada más.

## 1. Encargo: qué es en este juego

- Lee la configuración de Kiln cuya ruta te da la petición (si no te da ninguna, `kiln.toml` en la raíz) y, si el proyecto los tiene, sus `CLAUDE.md`, su lore y sus
  principios. La petición se traduce al mundo de ese juego: en un juego
  medieval «un ascensor» sería un montacargas de torno; en uno de ciencia
  ficción, una cabina.
- Respeta las reglas del proyecto: las de `[rules]` en la configuración de
  Kiln y las que escriba su documentación. Si `require_use` es `true`, busca
  qué sistema del juego usaría el asset (`used_by`); si no encuentras
  ninguno, deja `used_by` vacío y dilo en `notes`.
- Decide sus estados visibles (`states`): abajo, arriba, cargado, abierto...
- Pon un presupuesto de triángulos razonable para su tamaño (`budget_tris`).

## 2. Desglose: piezas

Tres preguntas, en este orden:

1. Cuánto mide el objeto entero (metros reales).
2. Qué piezas lleva y cuántas de cada una (`count`).
3. Qué hace cada una: si se mueve (`moves`: `rotate_x`, `translate_y`...) y
   en qué estados aparece (`only_in`).

Pocas piezas y con sentido: lo que un artista modelaría por separado porque
se mueve, cambia de material o se reutiliza. Entre 3 y 10 suele bastar.

## 3. Inventario: de dónde sale cada pieza

Abre el catálogo y, para cada pieza, elige una fuente (`source.kind`):

- `reuse`: una pieza del catálogo sirve tal cual (`from` = su clave exacta,
  `kit/pieza`).
- `variant`: una pieza del catálogo con otra medida o material (`from` y
  `why`).
- `search`: no hay nada; se buscará CC0 o CC-BY fuera (`query` en inglés,
  dos o tres palabras).
- `retouch`: una pieza del catálogo como base para retocar en Blender
  (`from` y `why`).

Prefiere siempre reutilizar. Usa claves del catálogo tal como están, sin
inventarlas. Cada pieza que no sea `reuse` lleva `material` con una clave de
la paleta (`materials[].key` del catálogo); si no hay ninguna adecuada, pon
la clave que propones y explícalo en `notes`.

## 4. Montaje: dónde va cada pieza

Para cada pieza, una posición por copia (`at`, tantas como `count`), en
metros, con Y hacia arriba. El origen del asset es el centro de su planta a
ras de suelo. Cada `at` es el centro de la **base** de la copia tal como
queda montada: una pieza que apoya en el suelo lleva `y = 0`; una que
cuelga con su parte baja a 3 m lleva `y = 3`. `turn` gira cada copia en
grados sobre X, Y y Z; `size_m` se mide en los ejes de la pieza antes de
girarla (una viga de pie de 4,5 m tumbada como travesaño lleva
`turn = [[0, 0, 90]]`, y su base pasa a ser su costado). Kiln coloca cada
copia ya girada con el centro de su base en `at`, así que basta con los
tamaños para casar las piezas: un travesaño encima de dos postes de 4,5 m va
en `y = 4.5`.

Coloca el asset en su estado de reposo (el primero de `states`). Las
piezas que solo salen en otro estado (`only_in`) van donde estarían en él.
Pon `status = "layout"` al terminar.

## La ficha

TOML, en la ruta que te da Kiln, con este formato:

```toml
[asset]
id = "hoist"                       # inglés, snake_case; también es el nombre del fichero
name = "Montacargas de torno"      # en el idioma de la persona
asked = "crear un ascensor"        # la petición literal
used_by = ["works", "granary"]
where = "granero y obras"
states = ["down", "up", "loaded"]
budget_tris = 6000
status = "layout"
notes = "Tu lectura de la petición y tus dudas, en dos o tres frases."

[[parts]]
id = "frame"
what = "bastidor de dos postes y travesaño"
size_m = [2.2, 3.5, 0.3]          # x, y (alto), z
material = "log"
source = { kind = "variant", from = "village/beam", why = "viga del kit, más larga" }
at = [[0, 0, 0]]                  # centro de la base de cada copia (una por `count`)
turn = [[0, 0, 0]]                # opcional: grados sobre X, Y, Z
```

Escribe la ficha una sola vez, completa. Termina con una línea que resuma
cuántas piezas reutilizas y cuántas hay que buscar.
