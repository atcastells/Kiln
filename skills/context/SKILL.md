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
