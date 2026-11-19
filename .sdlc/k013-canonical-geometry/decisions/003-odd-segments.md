# Decisión 003: con `[generate] canonical`, `sphere.segments` debe ser par y fijo

- **Tarea**: K013/T16 (hallazgo I-1 de la revisión T15). Fecha: 2026-11-19. Decide: el desarrollador.
- **Decisión**: con `[generate] canonical` activo, la comprobación estática de verify rechaza, antes de lanzar Godot, una receta cuyo `sphere.segments` puede salir impar. Código `canonical_odd_segments` (`crates/kiln-core/src/generate/check.rs`, `canonical`, llamado desde `crates/kiln-cli/src/verify_recipe.rs`, `run`). Sin la clave no cambia nada. S1 no se toca.
- **Alcance de «puede salir impar»**: un valor fijo impar, o cualquier rango `[a, b]` con `a ≠ b`. `resolve::draw` toma `floor(a + (b − a)·u)`, así que un rango de extremos enteros distintos llega a un impar en un extremo o dentro (`[8, 10]` da 9). Solo pasan un par fijo o `[n, n]` con n par. Si falta el parámetro, vale el de por defecto (32).

## Por qué

El generador hace S/2 anillos, de modo que los recuentos de S1 §2.2 (`2 + S·S/2` vértices, `S·S` triángulos) solo se cumplen con S par. Con S impar, verify acababa en `canonical_count` en cada semilla tras lanzar Godot (`review/evidence/odd-segments.log`). Así se rechaza antes y con un mensaje claro.

## Alternativa rechazada

Generalizar la fórmula del recuento a S impar. Se descarta porque cambia S1, que está aprobada.

## Pendiente

Hecho: S1 §2.2 se anotó en ASTRA (`docs/design/ast-esp-001-s1-geometria-canonica-v0.md`, «Nota de implementación (K013, decisión 003)») el 2026-11-19.
