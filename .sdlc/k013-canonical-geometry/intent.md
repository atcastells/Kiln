# Intent: Kiln produce y valida el artefacto de geometría canónica v0

- **Originator**: Aaron Castells (rol de product owner)
- **Date**: 2026-11-18
- **Status**: approved by product owner (2026-11-18). Las seis preguntas abiertas pasan a `spec.md` como propuestas a aprobar.
- **Slug**: k013-canonical-geometry (escrito en ASTRA como `canonical-artifact-kiln`; movido aquí por la decisión D1 de `spec.md`)

## The problem, in the originator's words

Texto literal del encargo, al retomar AST-ESP-001-S1.

> Siguiente trabajo: el incremento de implementación del apartado 10 de la
> especificación, que son tres tareas en este orden (Kiln produce y valida el
> artefacto; Rust carga, valida y consulta; Godot deriva la malla). Empieza
> solo por la primera, la de Kiln, con su propia cadena SDLC: escribe el
> intent y párate para que lo apruebe. No implementes nada sin plan.md
> aprobado.
>
> Límites que la especificación deja abiertos y que la tarea de Kiln debe
> cerrar:
> - Las mediciones se hicieron sobre los arrays del generador, no sobre el
>   GLB que Kiln lee: hay que repetir los casos sobre el GLB real.
> - La igualdad entre equipos solo está medida en arm64.
> - La garantía de distancia (Q2) puede exigir más de 128 bits; es asunto de
>   la tarea de Rust, no de esta.

Y el texto de la tarea en la especificación aprobada
(AST-ESP-001-S1 (ASTRA, `docs/design/ast-esp-001-s1-geometria-canonica-v0.md`), §10.1):

> **Kiln: producir y validar el artefacto.** Al publicar una receta del
> vocabulario de esfera: cuantización, unión de enteros iguales, recuento
> esperado, I1–I9, hash y campos del manifiesto. Pruebas: los casos del
> apartado 8, leídos del GLB real.

## Who it affects

- Quien implemente después las tareas 2 (Rust) y 3 (Godot) del §10: consumen
  el artefacto y el `geometry_sha256` que esta tarea publica.
- Quien publica paquetes de asteroide con Kiln: una receta cuya geometría no
  pase I1–I9 deja de poder publicarse con artefacto canónico.
- El product owner, que aprueba el contenedor de archivo si esta tarea lo fija.

## What happens if we do nothing

La especificación queda aprobada sin ningún artefacto que la cumpla. La tarea
de Rust no tiene qué cargar, la de Godot no tiene de qué derivar, y los dos
límites de medición (GLB real, igualdad fuera de arm64) siguen abiertos.

## How we will know it is solved

- Kiln, al publicar un paquete del vocabulario de esfera, escribe un artefacto
  con el contenido del §2.3 y añade al manifiesto el `geometry_sha256` (§6.2)
  y la procedencia del §6.3.
- El validador de Kiln comprueba I1–I9 con enteros y rechaza con código y
  elemento (§3.3); no hay aceptación parcial ni reparación.
- Los 67 casos positivos y los 22 negativos del §8 dan el veredicto medido en
  `measurements.md` §8, ahora leyendo el GLB que Kiln lee de verdad.
- Queda medido si el mismo paquete da el mismo `geometry_sha256` fuera de
  arm64, o dicho por escrito que no se pudo medir y por qué.
- K012 no cambia: sus tres paquetes, su consumidor, sus pruebas y el
  `script_sha256` del guion publicado siguen iguales (§9.1).

## Out of scope

- Rust (carga, I1–I9 al cargar, Q1–Q3) y la cuestión de los 128 bits de Q2.
- Godot (derivación identidad).
- `space.rs`, `WorldState`, `FORMAT_VERSION` y el protocolo.
- La clase `general`, las regiones y la revisión de T-063 (§9.2).

## Open questions

1. **Dónde vive el código y su cadena.** El cambio es en el repo de Kiln
   (`~/Kiln`), pero esta cadena está en ASTRA, como la de K012. Propuesta:
   cadena aquí, código en Kiln, y los paquetes republicados en ASTRA.
2. **Identificador de la tarea.** ¿K013, siguiendo a K012? No está asignado.
3. **Contenedor del archivo.** La especificación fija los campos y los bytes
   del hash, no el formato (§2.3). Sin formato no hay archivo: ¿lo fija esta
   tarea, con tu aprobación en el `spec.md`?
4. **Se republican los tres paquetes de K012 con artefacto**, o el artefacto
   se estrena en un paquete nuevo. §9.1 dice que nada de lo publicado se
   reescribe y que el artefacto es «un archivo nuevo en el paquete».
5. **Igualdad fuera de arm64.** Medirla exige un equipo o contenedor x86-64.
   ¿Hay uno disponible, o vale la emulación (lenta, y no es el mismo caso)?
6. **Los 19 casos positivos que no son paquetes publicados** (escalados,
   cuerpo máximo, borde, tetraedro) y los negativos construidos a mano: ¿se
   generan como GLB de prueba en la batería de Kiln, sin publicarse en ASTRA?

Respondidas en `spec.md`, apartado *Decisiones* (D1–D6), aprobado el 2026-11-18.

---
Next stage: Design reads this file and writes `spec.md`.
