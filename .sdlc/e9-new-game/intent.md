# Intent: Empezar un juego desde cero (entrega 9)

- **Originator**: Aarón
- **Date**: 2026-11-13
- **Status**: approved
- **Slug**: e9-new-game

## El problema, en palabras del originador

> Quiero poder empezar un proyecto de juego desde 0 en kiln. De momento solo
> se pueden iniciar proyectos tipo juego. Debe haber un wizard sencillo pero
> que de un inicio con un intent asentado

## A quién afecta

A quien empieza un juego nuevo con Kiln. Hoy `kiln serve` falla si la
carpeta no tiene `kiln.toml`, y ese fichero hay que escribirlo a mano
copiando el ejemplo. El contexto empieza vacío y nada obliga a dejar claro
qué juego es antes de pedir el primer asset.

## Qué pasa si no se hace

El primer asset se desglosa sin saber qué juego es: el agente decide época,
estilo y escala por su cuenta, y las fichas salen sin un rumbo común.

## Cómo sabremos que está resuelto

- Desde una carpeta vacía, en pocos minutos y sin escribir TOML, hay un
  proyecto que `kiln check` acepta y que `kiln serve` abre.
- El proyecto nace con un intent asentado: un párrafo que dice qué juego es,
  que la persona ha leído y fijado, y al menos una afirmación fijada en cada
  capa del contexto.
- El único tipo de proyecto que se puede elegir es «juego».

## Preguntas abiertas

1. ¿Dónde vive el intent del juego: un campo nuevo del contexto
   (`kiln/context.toml`) o un fichero aparte (`kiln/intent.md`)?
2. ¿El asistente crea también el proyecto de Godot (`project.godot` y
   carpetas), o solo lo de Kiln?
3. ¿Se entra por la página (`kiln serve` en una carpeta sin `kiln.toml`),
   por la consola (`kiln new`) o por las dos?

---
Siguiente etapa: Design lee este fichero y escribe `spec.md`.
Requiere el visto bueno de Aarón.
