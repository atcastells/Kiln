# Intent: Kiln comprueba la integridad de una malla derivada y avisa antes de montar

- **Originator**: Aaron Castells (rol de product owner)
- **Date**: 2026-11-20
- **Status**: forma elegida (B) por el product owner el 2026-11-20; quedan abiertas las preguntas 2 y 4. Spec y plan en borrador.
- **Slug**: derived-mesh-verify

## The problem, in the originator's words

Pregunta del desarrollador al cerrar la tarea EST-004 de ASTRA: qué había
tardado más de Kiln y si faltaba algo. Dos puntos de la respuesta de la
sesión principal, literales:

> Verificación de malla reutilizable: check.py ha crecido a medida para esta
> cavidad. Si Kiln ofreciera esas comprobaciones (cerrada, variedad,
> autointersección, correspondencia con cortadores) para una pieza derivada
> de otra, la siguiente excavación costaría mucho menos. Eso sería trabajo en
> Kiln, y en P1 decidiste presentarlo como dependencia antes de tocarlo.

> lo único que costó algo fue descubrir a mano lo que pide: las tres filas de
> material que faltaban, salieron como avisos en el primer verify y obligaron
> a repetirlo

Respuesta del desarrollador:

> Ok orquestra esas mejoras

Esta cadena cubre solo la parte de Kiln.

## Who it affects

- Quien publica con Kiln una pieza cuya malla sale de un guion propio (en
  ASTRA, la cavidad de EST-004 producida en Blender y publicada con
  `source = { kind = "reuse" }`). Hoy, una; la siguiente excavación de ASTRA
  sería la segunda.
- Quien monta un asset por primera vez con claves de paleta nuevas: lo
  descubre en `verify`, después del montaje.

## What happens if we do nothing

- Cada pieza derivada vuelve a escribir en su proyecto las comprobaciones
  genéricas de malla (cerrada, variedad, orientación, degenerados,
  autointersección, componentes). En EST-004 eso vive en un `check.py` de
  789 líneas, mezclado con las comprobaciones propias de la cavidad.
- Los avisos que se pueden saber antes de montar siguen saliendo después, y
  cuestan un montaje y una verificación más.

## How we will know it is solved

- Una ficha puede pedir a `kiln verify` las comprobaciones genéricas de
  integridad sobre su GLB, y una malla con un borde abierto, una cara girada,
  un triángulo degenerado, dos triángulos que se cruzan o una componente de
  más sale en rojo con su código y su elemento.
- La cavidad publicada de EST-004 pasa esas comprobaciones tal como está.
- El aviso de claves de paleta que faltan sale antes del primer montaje, en
  el camino que se recorre de verdad (no solo en un comando que no se lanzó).
- Lo propio de la cavidad (correspondencia con los cortadores, solera,
  transición, topología declarada) se queda en ASTRA.

## Open questions

1. **Qué mínima se hace.** Resuelta (Aaron Castells, 2026-11-20). Eligió
   «Integridad de malla en verify (Recomendada)», descrita así, literal:
   «Comprobaciones de integridad activables por ficha en `kiln verify`, con
   la topología esperada declarable (la cavidad tiene un asa y hoy no pasaría
   la regla de Kiln). Entre 250 y 350 líneas en 3 o 4 encargos. Sustituye
   las 10 comprobaciones y 5 mutaciones genéricas de check.py. Además, añadir
   `kiln check` al procedimiento de Astra, que no toca Kiln.» La forma A
   (avisos de la ficha en `build`) no se pidió: la cubre el paso
   `kiln check` en el procedimiento de ASTRA.
2. **Severidad.** Abierta. ¿Las comprobaciones de integridad pedidas por una
   ficha son error (bloquean `publish`) o aviso? *Recomendación:* error. La
   ficha las pide de forma expresa, y un aviso no habría parado ninguna de
   las mutaciones de check.py. No se añade una clave para bajarlas a aviso.
3. **Género declarado.** Resuelta por la elección de la pregunta 1: la ficha
   declara componentes y género (`spec.md`, R3).
4. **Escala.** Abierta. El aviso `scale` (lado mayor de 30 m por defecto) sale en
   `kiln check` para la cavidad de 315 m. ¿Se deja como está, o una ficha o
   un kit puede declarar su tamaño esperado? *Recomendación:* dejarlo como
   está en esta cadena. Es un aviso, no bloquea, y no forma parte de la forma
   elegida. Si molesta en el procedimiento con `kiln check`, se puede hacer
   después como cadena aparte (por ejemplo, `max_size_m` por ficha).
5. **Procedencia.** Descartada por ahora (2026-11-20): no se añade el campo
   `derived_from`.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
