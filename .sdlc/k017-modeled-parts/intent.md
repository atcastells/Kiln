# Intent: generar por el MCP de Kiln assets de buena calidad para ASTRA

- **Originator**: Aaron Castells (rol de product owner)
- **Date**: 2026-11-22
- **Status**: approved by product owner, 2026-11-22 (con el plan); pregunta 5 resuelta: texturas fuera
- **Slug**: k017-modeled-parts

## The problem, in the originator's words

> mejora kiln, hasta poder generar usando el mcp assets de buena calidad,
> foco en el proyecto Astra en .t3/projects

## Lo que hay hoy (medido el 2026-11-22, no es la solución)

- Por el MCP, una pieza nueva solo puede salir del catálogo (`reuse`,
  `variant`, `retouch`, `asset`) o de fuera (`search`). Si no está en ninguna
  parte, Kiln la monta como caja de relleno (`crates/kiln-core/src/build.rs:246-252`).
  `retouch` se queda en un aviso («retocar: …»): nadie retoca.
- El único camino que crea forma nueva es `[generator]`, y solo sirve para
  mallas orgánicas de una pieza (asteroides).
- ASTRA tiene 55 fichas. Salvo los tres asteroides generados, sus piezas
  propias (5 kits) vienen de guiones de
  Blender escritos a mano en `assets/kits/*/src/`: 20 ficheros y 5.682 líneas,
  que se lanzan fuera de Kiln con `blender --background`. Las ayudas se copian
  entre guiones («The helpers are copied, not imported», `industrial/src/workshop.py:8`):
  `box_uv` está siete veces y `add_shape` cinco.
- Las convenciones de esos guiones (metros, Z arriba, la cara de una pieza de
  pie hacia −Y, cada material como zona que la ficha asigna a la paleta, UV
  en caja a media vuelta por metro) solo viven en los comentarios de ASTRA.
  Kiln no las conoce ni las comprueba.

## Who it affects

- Quien produce assets de ASTRA con Claude: cada pieza nueva pide escribir o
  copiar un guion de Blender, lanzarlo a mano y luego montarla con Kiln.
- Kiln como producto: lo que hace bien (montar, medir con Godot, validar,
  publicar) depende de que alguien haya modelado antes la pieza fuera.

## What happens if we do nothing

Por el MCP solo salen asteroides y montajes de piezas que ya existen. Cada
asset nuevo de ASTRA sigue costando un guion a mano, con sus ayudas
copiadas, y su calidad depende de quien lo escriba. No hay revisión visual de
la pieza antes de montarla.

## How we will know it is solved

- Una sesión de Claude, solo con las herramientas del MCP de Kiln y sobre una
  copia de ASTRA, produce de principio a fin un asset nuevo cuyas piezas no
  están en el catálogo, sin escribir guiones fuera de Kiln, y lo publica con
  `kiln verify` en verde.
- Ese asset no tiene cajas de relleno, cumple las convenciones de ASTRA
  (escala, orientación, zonas de material, UV, presupuesto de triángulos) y
  pasa la misma revisión visual que una pieza hecha a mano del mismo kit
  (capturas comparadas lado a lado).
- Una pieza existente de un kit de ASTRA (p. ej. `workbench`) se rehace por
  el MCP y queda a la altura de la original en la comparación.

## Respuestas del originador (2026-11-22)

1. **Calidad**: el nivel de los kits hechos a mano (neo-PSX, pocos
   triángulos, texturas pixeladas), juzgado con capturas lado a lado.
2. **Prueba**: rehacer `workbench` por el MCP y compararlo con el original.
3. **Rama**: encima de `k016-shared-core` (PR #37), antes de que se una.
4. **Guiones de ASTRA**: se quedan como están; solo cambia lo nuevo.

## Open questions

5. **Texturas**: los kits generan texturas propias (`textures.py`,
   `rock_texture.py`). Propuesta: fuera de esta entrega. Las piezas modeladas
   usan los materiales que ya tiene la paleta del proyecto, como `workbench`.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
