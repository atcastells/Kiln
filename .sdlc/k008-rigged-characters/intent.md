# Intent: personajes con esqueleto y animaciones a través de Kiln (T010-C1 de ASTRA, K008 en Kiln)

- **Originator**: Aaron Castells (PO de ASTRA V2)
- **Date**: 2026-11-18
- **Status**: approved by product owner (el encargo llegó como «Queda aprobada T010-C1»)
- **Slug**: k008-rigged-characters

## The problem, in the originator's words

Texto completo en `request.md`. Lo que toca a Kiln:

> Kiln — Validación de esqueletos, pesos y animaciones GLB.

> Fase 1 — Comprobación del pipeline. Utilizar un humanoide mínimo de prueba
> para verificar que Kiln conserva el esqueleto y las animaciones al ejecutar
> build → verify → publish. Si aparece un bloqueo, corregir únicamente esa
> capacidad.

> Prioridad inmediata: demostrar primero el pipeline de personajes animados
> en Kiln. Después producir el técnico definitivo de esta entrega, evitando
> invertir trabajo artístico en un formato cuya compatibilidad todavía no
> está comprobada.

Las fases 2–4 (modelar el técnico, integrarlo en Godot, verificar y capturar)
son trabajo del repositorio ASTRA y llevan allí su propia cadena.

## Who it affects

Quien produzca personajes para ASTRA (el originador y sus agentes). El
técnico es el primero; el encargo pide una fuente «reutilizable para futuras
variantes», así que el camino se volverá a usar.

## What happens if we do nothing

La única vía de un GLB a `godot/assets/kiln/` de ASTRA es
`build → verify → publish`. Hoy esa vía entrega un personaje sin esqueleto
ni animaciones y lo da por bueno (medido el 2026-11-18, ver `spec.md`):
el técnico no se puede publicar animado sin saltarse Kiln.

## How we will know it is solved

Un humanoide mínimo de prueba, con esqueleto, pesos y las animaciones
`idle` y `walk`, pasa `build → verify → publish` y el GLB publicado conserva
las tres cosas; Godot sin ventana lo instancia con `Skeleton3D` y
`AnimationPlayer` y las animaciones mueven huesos. Un GLB que las pierde
hace fallar `verify`.

## Open questions

- Variantes pintadas (`paint`) de un asset con esqueleto: el encargo habla de
  «futuras variantes» pero no las pide en esta entrega. Se dejan fuera.
- Bucle de las animaciones: Godot las importa sin bucle. ¿Lo fija quien
  integra (ASTRA) o Kiln al publicar? Se propone que lo fije ASTRA.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
