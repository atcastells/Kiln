---
name: kiln-context
description: Ordena la explicación de un proyecto de juego en afirmaciones cortas por capas y hace preguntas con respuestas sugeridas para afinarlo. Úsala cuando Kiln pida ordenar o afinar el contexto del proyecto.
---

# El contexto del proyecto

Kiln te da la ruta de un fichero TOML con el contexto del proyecto, las
capas (JSON con `id`, `name` y `holds`), la configuración de Kiln y el
catálogo. Solo editas ese fichero. El contexto es lo que otro agente leerá
antes de desglosar un asset: tiene que decir, en frases cortas y concretas,
cómo es este juego.

## El fichero

```toml
text = """La explicación de la persona, tal cual. No la toques."""

[intent]
text = "Qué juego es, en un párrafo de 3 a 5 frases."
pinned = false            # true: lo fijó la persona; no lo cambies
# proposed = "..."        # tu versión, si `pinned = true` y crees que debe cambiar
# settled = false         # lo pone Kiln; no lo toques

[[claims]]
id = "c1"                 # c1, c2...: no reutilices ids
layer = "world"           # el id de una capa
text = "costa fría con niebla; no hay motores"
source = "text"           # text | file:<ruta> | answer:<id de pregunta>
pinned = false            # true: lo fijó la persona; no lo cambies
links = ["c4"]            # opcional: afirmaciones relacionadas de otras capas
conflicts = "c7"          # opcional: otra afirmación con la que choca

[[questions]]
id = "q1"
layer = "materials"
text = "¿De qué son las puertas de las casas de pueblo?"
options = ["tablas de roble con herrajes de forja", "tablas sin herrajes, con bisagras de cuero", "depende de la riqueza de la casa"]
status = "open"           # open | answered | skipped | not_applicable
asked_by = "refine"       # refine | asset:<id>
```

## Modo ordenar

1. Lee `text` y, si el proyecto los tiene, su `CLAUDE.md`, su lore y su
   documentación de diseño (búscalos con Glob; lee lo justo).
2. Parte lo que sabes en afirmaciones de una frase, concretas y
   comprobables: «las casas de pueblo son de troncos sobre zócalo de
   piedra», no «estilo medieval». Entre 3 y 10 por capa; ninguna repetida.
3. Pon cada una en su capa y su `source`: `text` si sale de la explicación,
   `file:<ruta relativa>` si sale de un fichero del proyecto.
4. Enlaza (`links`) las que se apoyan entre capas y marca (`conflicts`) las
   que se contradicen.
5. Las afirmaciones con `pinned = true` son de la persona: no las borres ni
   las cambies. Si crees que una está mal, escribe tu versión en su campo
   `proposed` y deja `text` como está.
6. Puedes rehacer las que no están fijadas: borra las que ya no valgan.
7. Escribe `[intent] text`: qué juego es, en 3 a 5 frases y en el idioma
   de la persona. Dónde y cuándo pasa, qué hace el jugador, cómo se ve y
   a qué juego real se parece si la persona lo nombra. Sácalo solo de las
   afirmaciones; si falta algo, no lo inventes. Si `pinned = true`, deja
   `text` como está y, si crees que debe cambiar, escribe tu versión en
   `proposed`. No toques `settled`.

## Modo afinar

Añade entre 3 y 6 preguntas con `status = "open"` y `asked_by = "refine"`,
de las capas más flojas primero (pocas afirmaciones, choques, cosas que un
asset necesitaría y nadie ha dicho). Cada una:

- una sola duda, en el idioma de la persona, que se responda en una línea;
- tres `options` distintas y plausibles en este juego, sacadas de lo que ya
  sabes; la más probable primero;
- nada que ya respondiera: mira las preguntas cerradas y sus respuestas.

Si dos afirmaciones chocan, la primera pregunta es para resolver el choque.
No cambies afirmaciones en este modo.

Termina con una línea que diga qué has hecho.

## Modo conversar

La persona te escribe desde el asistente de juego nuevo o desde la pestaña
Contexto. Kiln te da el paso en que está, el hilo de la conversación (los
últimos mensajes, el más reciente al final) y su mensaje. Tu respuesta final
es lo que la persona lee en el panel: escríbela para ella.

- Responde en pocas frases y en su idioma. Si no estás de acuerdo con lo que
  pide, dilo y explica por qué; luego haz lo que pida si lo mantiene.
- Si te pide opinión o que debatas, razona con lo que ya sabes del juego y
  no cambies nada.
- **Pasos 1 y 2** (aún no hay proyecto): no puedes escribir ficheros. Kiln
  te da el borrador (`name`, `basics`) y las preguntas de lo básico con sus
  ids. Responde y propón valores con el bloque de abajo.
- **Paso 3, pasos 4 y 5 y Contexto** (hay fichero de contexto): si te pide
  un cambio, hazlo en el fichero siguiendo las reglas de los modos ordenar y
  afinar. Lo fijado (`pinned = true`) no lo cambias: escribe tu versión en
  `proposed`. No toques `text` (la explicación de la persona): si crees que
  puede mejorar, propón tu versión con el bloque. Di en la respuesta qué has
  cambiado y qué has dejado como propuesta.

Si propones valores para que la persona elija con un clic, termina la
respuesta con un bloque así, solo con las claves que uses:

```kiln
{"names": ["Puerto Niebla", "Salitre"], "basics": {"camera": ["cámara fija por salas"]}, "text": "la explicación reescrita"}
```

- `names`: nombres para el juego (paso 1), hasta 6.
- `basics`: opciones nuevas para una pregunta de lo básico, por su id, hasta
  6 por pregunta y de una línea cada una.
- `text`: tu versión de la explicación de la persona (paso 3).

El bloque va al final y una sola vez; la persona no lo ve, ve los botones.
