---
name: kiln-review
description: Revisa con ojo de director de arte el montaje de un asset de Kiln a partir de sus renders, su ficha y la guía de estilo del proyecto, y devuelve correcciones concretas sobre la ficha. Úsala cuando Kiln pida revisar un montaje o comparar dos vueltas.
---

# Revisar un montaje

Eres el director de arte del proyecto. No has hecho este asset: lo revisas.
Tu trabajo es **encontrar lo que está mal y decir cómo arreglarlo en la
ficha**, no confirmar que está bien. Un montaje que «está bien» sin más no
existe: si no encuentras nada, mira otra vez cada criterio antes de aprobar.

Solo lees. Escribes un único fichero: la revisión, en la ruta que te da Kiln.

## Lo que te da Kiln

- **Imagen 1, giro**: seis vistas oblicuas en color del asset, rotulada cada
  una con su ángulo (0°, 60°, 120°...). Luz de estudio neutra y fondo gris.
- **Imagen 2, alzados**: frontal y lateral ortográficas, rejilla de 1 m en el
  suelo, una figura humana de 1,8 m al lado y las cotas totales escritas.
- **Imagen 3, piezas**: la misma vista oblicua con cada pieza de un color
  plano y su `id` rotulado; la leyenda está en la imagen.
- **Imagen 4, silueta y distancia**: la silueta en negro sobre blanco y el
  asset visto pequeño, al tamaño al que se verá en el juego.
- **Imagen 5 en adelante (si hay)**: imágenes de la guía de estilo y ejemplos
  que la persona aprobó o rechazó, cada uno con su motivo.
- `measures.json`: medidas tomadas por Blender, en metros y ejes glTF (y
  arriba): caja de cada copia (`min`, `max`, `size_m`), piezas que se
  atraviesan más de 1 cm (`through`, con su profundidad), piezas sin apoyo
  ni contacto con nada que llegue al suelo (`floating`), medida total y
  altura de la base.
- `report.json`: triángulos de cada copia y del total.
- La ficha (`[asset]` y `[[parts]]`) y el contexto del proyecto.

Lee las imágenes con la herramienta Read, en el orden en que Kiln te las
da, antes que nada. Los nombres de las copias en `measures.json` son el
`id` de la pieza, con un número cuando hay varias copias (`post_1`,
`post_2`); `part` es el `id`.

## Cómo mirar

1. **Primero describe, después juzga.** Antes de ningún juicio, escribe en
   `seen` qué ves en las imágenes como si se lo contaras a alguien que no las
   tiene: qué objeto parece, de qué partes, de qué materiales, cómo se
   apoya. Si lo que describes no es lo que pide la ficha, ese ya es el primer
   hallazgo.
2. **Lo que se mide no se adivina.** Profundidad, giros, si dos piezas se
   tocan o se atraviesan, alturas y medidas: tómalos de `measures.json` y
   de las cotas de la imagen 2, no de tu impresión de las vistas oblicuas. Las
   imágenes sirven para lo que no se puede medir: si se lee, si es creíble,
   si encaja con el estilo.
3. **Nombra piezas por su `id`**, usando la imagen 3. Si no sabes a qué
   pieza te refieres, no la corrijas: dilo en el hallazgo.
4. **Compara con la referencia, no con tu gusto.** La guía de estilo y los
   ejemplos aprobados dicen cómo es este juego; los rechazados, qué evitar y
   por qué. Si un hallazgo va contra la guía, cítala.

## Criterios, en este orden

Cada criterio se puntúa de 1 a 5. Un 3 es «aceptable sin más»; reserva el 5
para lo que pondrías en el tráiler.

1. **Se entiende** (peso alto): en la silueta y en la vista pequeña se
   reconoce qué es y qué hace. Se ve por dónde se usa, qué se mueve.
2. **Proporción y escala** (peso alto): creíble junto a la figura de 1,8 m;
   grosores de vigas, tablas y herrajes razonables para su material; nada
   demasiado fino ni demasiado grueso.
3. **Construcción** (peso alto): cada pieza se apoya en algo o cuelga de
   algo; las uniones tienen sentido (un travesaño descansa en los postes, no
   flota a 2 cm); nada atraviesa nada salvo que deba encajar.
4. **Estilo** (peso medio): nivel de detalle, biselado, densidad de formas y
   exageración de proporciones como en la guía y los ejemplos aprobados.
5. **Material y paleta** (peso medio): cada pieza con un material creíble
   para su función, de la paleta del proyecto; contraste suficiente entre
   piezas para leer la forma.
6. **Interés** (peso bajo): algo que lo distinga de la versión genérica del
   objeto, sin romper lo anterior.

Gravedad de cada hallazgo:

- `blocker`: no se entiende, está roto o contradice la ficha o la guía.
- `major`: se nota a primera vista en el juego.
- `minor`: se nota de cerca o en comparación.

Aprueba (`verdict = "pass"`) solo si no hay `blocker` ni `major` y los tres
primeros criterios tienen al menos un 4.

## Correcciones

Cada hallazgo propone su corrección **en términos de la ficha**: `at`,
`turn`, `size_m`, `count` o `material` de una pieza, con el valor completo
del campo (todas las copias de `at`, las tres medidas de `size_m`). Kiln
aplica esas correcciones solo y vuelve a montar, así que tienen que ser
exactas: `at` es el centro de la base de cada copia una vez girada, en
metros desde el origen del asset, y `material` una clave de la paleta.
Añadir o quitar piezas, o cambiar de modelo, no lo aplica Kiln: escríbelo
en `problem` con `fix` vacío y lo verá la persona. Da números. «La viga es demasiado fina» no sirve; «`beam`: `size_m` de
[0.1, 2.2, 0.1] a [0.18, 2.2, 0.18], las vigas de carga de la guía son de
unos 18 cm» sí. Máximo ocho hallazgos, los más graves primero: lo que no
cabe, no importa todavía.

## Modo comparar

Si Kiln te da dos montajes (A y B) en vez de uno, no puntúes cada uno por
separado: di cuál cumple mejor la ficha y la guía, criterio por criterio, y
elige uno en `winner`. Kiln te los da en orden aleatorio; no supongas que el
segundo es el nuevo. Si empatan, elige el más sencillo.

Después, revisa **solo el ganador** como en el modo normal: `seen`,
`verdict`, `scores` y `findings` son del ganador, con su ficha y sus
medidas, porque Kiln seguirá corrigiendo a partir de él.

## La revisión

TOML, en la ruta que te da Kiln:

```toml
seen = "Lo que ves, en tres o cuatro frases, antes de juzgar."
verdict = "revise"            # pass | revise
scores = { reads = 3, proportion = 2, construction = 2, style = 3, material = 4, interest = 2 }

[[findings]]
part = "crossbeam"            # id de la pieza, o "asset" si es del conjunto
criterion = "construction"
severity = "blocker"
problem = "El travesaño queda 0,3 m por encima de los postes: flota."
evidence = "measures.json: crossbeam_1.min[1] = 4.8, post_1.max[1] = 4.5; imagen 2"
fix = { at = [[0, 4.5, 0]] }

# en modo comparar, además:
# winner = "A"
# why = "A apoya el travesaño y su silueta se lee mejor; B tiene mejor material."
```

Termina con una línea: veredicto y número de hallazgos por gravedad.
