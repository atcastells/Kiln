# Encargo original (literal)

Texto pegado por el originador el 2026-11-18. `intent.md`, `spec.md` y
`plan.md` citan sus fases (1–4) desde aquí.

---

Queda aprobada T010-C1 — Primer personaje funcional de ASTRA V2.

El objetivo es sustituir el NPC primitivo del técnico por un personaje humanoide Neo-PSX, animado e integrado en la simulación existente.

## Alcance aprobado

| Área | Entregable |
|---|---|
| Kiln | Validación de esqueletos, pesos y animaciones GLB |
| Blender | Personaje técnico con fuente .blend editable |
| Animación | Reposo y caminar |
| Godot | Personaje integrado en la estación existente |
| Rust | Sin cambios de comportamiento ni autoridad |
| Validación | Kiln, Godot headless, capturas y regresiones |

## Orden de implementación

Fase 1 — Comprobación del pipeline. Utilizar un humanoide mínimo de prueba para verificar que Kiln conserva el esqueleto y las animaciones al ejecutar build → verify → publish. Si aparece un bloqueo, corregir únicamente esa capacidad.

Fase 2 — Producción artística. Modelar el técnico con silueta industrial Neo-PSX, ropa de mantenimiento y proporciones compatibles con la arquitectura de T010-A/B. Priorizar una fuente editable y reutilizable para futuras variantes.

Fase 3 — Integración. Asociar el personaje a un NPC existente. Godot reproducirá reposo o caminar según el movimiento observado, sin modificar la Utility AI ni inventar acciones.

Fase 4 — Verificación. Comprobar importación, reproducción de animaciones, desplazamiento, orientación, persistencia visual y ausencia de regresiones. Generar capturas reales en el taller y corredor.

## Criterio de cierre

T010-C1 estará completa cuando el técnico existente pueda desplazarse por la estación con un modelo humanoide reconocible y animaciones básicas, manteniendo intacta su autonomía.

La aprobación artística se realizará sobre capturas de Godot, no únicamente sobre el modelo aislado de Blender.

Quedan fuera los demás personajes, animaciones de trabajo complejas, expresiones faciales, combate y nuevos comportamientos NPC.

Prioridad inmediata: demostrar primero el pipeline de personajes animados en Kiln. Después producir el técnico definitivo de esta entrega, evitando invertir trabajo artístico en un formato cuya compatibilidad todavía no está comprobada.
