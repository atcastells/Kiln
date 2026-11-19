---
task: k013-canonical-geometry/T4
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core canonical::topology && cargo clippy -p kiln-core --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/canonical/topology.rs, .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [crates/kiln-core/src/canonical/topology.rs:83, crates/kiln-core/src/canonical/topology.rs:135, /tmp/claudio-test-k013-canonical-geometry-T4-red.log, /tmp/claudio-test-k013-canonical-geometry-T4.log, /tmp/claudio-clippy-k013-canonical-geometry-T4.log]
---

## Summary

- `topology::check_i6_i7(&Geometry)` (topology.rs:83), que da por buenas I1–I5, y `check_i1_i7` (topology.rs:135), que encadena las dos. Orden igual al de `geo.py`: `I6_pinched_vertex`, `I7_not_connected`, `I7_not_a_sphere`. Unas 62 líneas nuevas sin contar pruebas, doc incluida: `#[cfg(test)]` pasa de la línea ~78 a la 140. Lo hice yo, sin delegar. No he tocado `mod.rs` ni `radial.rs`.
- Rojo primero: 3 fallos con el stub `Ok(())` (log `-red`). I1–I5 pasaban en todos los fixtures, así que el rojo venía de I6/I7. Ahora 14/14 en `canonical::topology`. Clippy `-D warnings` de kiln-core limpio.
- Cadena: en `plan.md`, T2 y T3 pasan a `done`; a `context.md` le añadí las dos viñetas que pidió la sesión principal. Mi rol no edita esos archivos; lo hice porque el encargo lo pedía expresamente.

Learned:
- Reglas de elemento: `I6_pinched_vertex` da el primer vértice cuyo abanico tiene más de un ciclo. `I7_not_connected` da el primer triángulo fuera de la pieza del triángulo 0. `I7_not_a_sphere` no da elemento y pone `V - E + F = … = χ` en el detalle. T6 y el cargador de Rust deberían copiarlas.
- El octaedro no sirve para I6: al fundir dos vértices opuestos comparten vecinos, y salta antes `I4_edge_shared_by_more_than_two`. Uso una esfera UV de 3 anillos × 6 y fundo sus dos polos (elemento 0).
- La conexión usa union-find sobre los vértices, igual que `geo.py`. Con I6 cumplida equivale a alcanzar triángulos por aristas, que es como el encargo define el elemento.
- `check_i6_i7` entra en pánico si I1–I5 no se cumplen (`fan[0]`, `next[&cur]`, `ts[0]`). T6 y R5 deben llamar a `check_i1_i7`, nunca a `check_i6_i7` suelta.
- χ se calcula en `i64`: en `usize`, V − E desborda.
- Con I4 e I5 cumplidas, el abanico de cada vértice es una permutación, así que `next[&cur]` no puede faltar. El recorrido para como mucho en `fan.len()` pasos.

## Detail

Pruebas nuevas en `mod tests` de topology.rs, todas a través de `check_i1_i7`:
- Positivos: tetraedro, octaedro y `uv_sphere(3, 6)`.
- I6: la misma esfera con el polo inferior fundido en el 0.
- `I7_not_connected`: octaedro más un octaedro a 1/10 con los índices desplazados 6; elemento `Some(8)`.
- `I7_not_a_sphere`: `torus(3, 4)`, rejilla cerrada con orientación coherente, χ = 0.

Las posiciones de los fixtures generados están en el eje x y son distintas, porque I1–I7 no miran la geometría. En esta pasada no salió ningún fallo de `radial.rs` (T5). Sin datos sensibles.
