# Decisión 002: se mantiene la regla estática `generator_radius`

- **Tarea**: K013/T8–T9. Fecha: 2026-11-18. Decide: el desarrollador.
- **Decisión**: Kiln conserva su regla estática `generator_radius` (`crates/kiln-core/src/generate/check.rs:130-160`, `numeric_rules`). Esa regla rechaza antes de Godot cualquier receta cuya cota radial inferior llegue a ≤ 0.
- **Consecuencia**: en `kiln verify`, `generator_radius` rechaza las dos recetas negativas de S1 §4.2, que nunca llegan a la vía canónica. Su `I8_not_radial` se demuestra en la vía A (T10), con `generate.gd` corrido directamente.

## Por qué

S1 §4.2 («Esas recetas no se rechazan en bloque») describe mal a Kiln: la regla es anterior a K013. Una receta que pasa la comprobación estática tiene cota > 0, y por tanto ya es radial (S1 §4.2). Así que ninguna receta negativa puede llegar a I8/I9 a través de verify (leído en T8, no ejecutado con esas dos recetas: `review/evidence/results/T8-result.md`; el test `a_static_error_never_launches_godot` confirma el mecanismo con otra receta).

## Alternativa rechazada

Dejar que I8/I9 decidan semilla a semilla cuando `[generate] canonical` está activo, lo que obligaría a saltarse `generator_radius`. Se descarta porque afloja una comprobación que ya existe y queda fuera del alcance de K013.

## Pendiente

Hecho: S1 §4.2 se anotó en ASTRA (`docs/design/ast-esp-001-s1-geometria-canonica-v0.md`, «Corrección (K013, decisión 002)») el 2026-11-19.
