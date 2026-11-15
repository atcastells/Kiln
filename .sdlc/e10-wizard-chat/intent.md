# Intent: Hablar con el agente en cada paso del asistente (entrega 10)

- **Originator**: Aarón
- **Date**: 2026-11-13
- **Status**: approved
- **Slug**: e10-wizard-chat

## El problema, en palabras del originador

> en el wizard de crear juego quiero poder debatir i solicitar mejoras al LLM
> en todos los pasos

## A quién afecta

A quien empieza un juego nuevo con el asistente de la entrega 9. Hoy el
agente solo entra en dos momentos y siempre con una tarea cerrada: «Ordenar y
preguntar» (pasos 3 y 4) y «Otra ronda» o «Más preguntas» (pasos 4 y 5). Cada
llamada es un `claude -p` de un turno, sin memoria de las anteriores. En los
pasos 1 (nombre) y 2 (lo básico) no hay agente, y en ningún paso se le puede
decir «esto no me convence», «dame otras opciones» o «explícame por qué».

## Qué pasa si no se hace

La persona solo puede aceptar, descartar o reescribir a mano lo que propone
el agente. Si el párrafo de qué juego es o una afirmación no le encaja, no
puede discutirlo ni pedir una versión mejor, y el intent se asienta con
cosas que no ha pensado de verdad o que ha escrito sola.

## Cómo sabremos que está resuelto

- En cada uno de los cinco pasos hay un sitio para escribirle al agente y
  recibir su respuesta sin salir del paso.
- Se le puede pedir una mejora concreta («hazlo más oscuro», «propón otras
  tres opciones de cámara») y el cambio aparece en el paso, sin pisar lo que
  la persona fijó.
- La conversación sigue el hilo: el agente recuerda lo que se dijo antes en
  el asistente.

## Preguntas abiertas (respondidas por Aarón, 2026-11-13)

1. **Qué puede hacer el agente**: aplica y propone. Cambia lo no fijado; sobre
   lo fijado deja una propuesta que la persona acepta o descarta.
2. **Pasos 1 y 2**: conversa y propone valores (nombres, opciones de lo
   básico) que la persona elige con un clic; no escribe nada en disco antes
   de «Crear el proyecto».
3. **Hilo**: uno solo para todo el asistente, que sabe en qué paso está. Se
   guarda en `kiln/` en cuanto existe el proyecto; antes vive en la página.
4. **Después de asentar**: también en la pestaña Contexto.

---
Siguiente etapa: Design lee este fichero y escribe `spec.md`.
Requiere el visto bueno de Aarón.
