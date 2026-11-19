---
task: k013-canonical-geometry/T6
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core canonical && cargo clippy -p kiln-core --all-targets -- -D warnings && cargo run -p kiln-core --example canonical_fixture -- .sdlc/k013-canonical-geometry/work/T1-scratch/out/ast42.glb /tmp/x.bin && cmp /tmp/x.bin crates/kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin"
persist: propose
files: [crates/kiln-core/src/canonical/mod.rs, crates/kiln-core/tests/canonical_cases.rs, crates/kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin, crates/kiln-core/tests/fixtures/canonical/README.md, crates/kiln-core/examples/canonical_fixture.rs, .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [crates/kiln-core/src/canonical/mod.rs:45, crates/kiln-core/tests/canonical_cases.rs:108, crates/kiln-core/examples/canonical_fixture.rs:8, /tmp/claudio-test-k013-canonical-geometry-T6-red.log, /tmp/claudio-test-k013-canonical-geometry-T6.log, /tmp/claudio-clippy-k013-canonical-geometry-T6.log, /tmp/claudio-test-k013-canonical-geometry-T6-example.log]
---

## Summary

- `canonical::validate` (mod.rs:45) corre `topology::check_i1_i7` y luego `radial::check_i8_i9`, y gana el primer fallo.
- La fixture real `tests/fixtures/canonical/asteroid_64_seed42.bin` pesa 73 784 B y tiene 2050 V y 4096 T. Sale del `ast42.glb` de T1 (con facet), y su `geometry_sha256` es `2713c0a9…f670d78a`. La regenera `cargo run -p kiln-core --example canonical_fixture -- <GLB> <OUT.bin>` (examples/canonical_fixture.rs), que lee el GLB, exige una sola malla, construye con `build(.., 64)`, valida, escribe e imprime el hash. El README lo explica.
- Comprobación del example (log `-example`): sobre ast42.glb sale exit 0 y el mismo hash, y `cmp` contra la fixture da bytes idénticos. Sin argumentos imprime el uso y sale con 2. Con `sph8.glb` rechaza con `canonical_count` y sale con 1.
- Rojo primero, con el stub `Ok(())`: 15 fallos (log `-red`). Ahora `cargo test -p kiln-core canonical` pasa 34/34 unitarias y 17/17 de integración, y con `--include-ignored` también 17/17, 0 ignoradas. Clippy de kiln-core con `-D warnings` no da avisos.
- Los 17 casos de la vía C (2 positivos y 15 negativos) dan el código de M§8, sin ninguna discrepancia (tabla en Detail). En los I8 y en la doble vuelta se comprueba además que `check_i1_i7` da Ok.
- El desarrollador quitó a mano el test `#[ignore]` de regeneración: `test-guard` me había bloqueado al intentarlo. Yo no he tocado `canonical_cases.rs` después ni `crates/kiln-cli`.
- Cadena: en `plan.md`, T4 y T5 pasan a `done`, y a `context.md` le añadí las 3 viñetas pedidas, porque el encargo lo pedía expresamente.

Learned: `test-guard` cuenta un `expect` como aserción y bloquea quitarlo aunque sea de un test añadido en la misma tarea. Un generador va mejor en `examples/` desde el principio.
Learned: las mutaciones I8 sobre la malla real son exactas sin buscar vértices. Voladizo: a' = b + c − a da det(a', b, c) = −det(a, b, c). Vértice a través del centro: a' = −a. Ninguna de las dos crea posiciones duplicadas en la semilla 42.
Learned: los elementos de las mutaciones reales siguen las reglas de T3: duplicado → el vértice posterior (1000), fuera de rango → 7, índice → 10, sin usar → 2050.
Learned: `canonical_cases.rs` es la primera prueba de integración de kiln-core y `canonical_fixture.rs` su primer example. Los dos usan solo la API pública.

## Detail

| Caso (M§8) | Vía C | Esperado | Obtenido |
|---|---|---|---|
| Tetraedro centrado | sintético | ok | ok |
| Semilla 42 asteroid_64 | fixture real | ok | ok |
| Un triángulo quitado | real, sin el último | `I4_open_edge` | `I4_open_edge` |
| Un triángulo invertido | real, tri 100 con swap(1,2) | `I5_inconsistent_orientation` | `I5_inconsistent_orientation` |
| Un triángulo repetido | real, tri 100 añadido | `I4_edge_shared_by_more_than_two` | `I4_edge_shared_by_more_than_two` |
| Dos vértices en el mismo punto | real, v500 = v1000 | `I3_duplicate_position` (elem 1000) | `I3_duplicate_position` |
| Coordenada fuera del límite | real, v7.y = 2^30 | `I1_out_of_range` (elem 7) | `I1_out_of_range` |
| Índice fuera de los vértices | real, tri 10 → V | `I2_index_out_of_range` (elem 10) | `I2_index_out_of_range` |
| Vértice sin usar | real, vértice añadido | `I3_unused_vertex` (elem 2050) | `I3_unused_vertex` |
| Dos vértices lejanos fundidos | sintético, uv_sphere(3,6) con polos fundidos | `I6_pinched_vertex` | `I6_pinched_vertex` |
| Segundo cuerpo dentro | sintético, octaedro + octaedro/10 | `I7_not_connected` | `I7_not_connected` |
| Tetraedro que no contiene el centro | sintético, +3000 en x | `I8_not_radial` | `I8_not_radial` |
| Voladizo | real, a' = b+c−a en el tri 100 | `I8_not_radial` | `I8_not_radial` |
| Vértice a través del centro | real, a' = −a | `I8_not_radial` | `I8_not_radial` |
| Toro | sintético, torus(3,4) | `I7_not_a_sphere` | `I7_not_a_sphere` |
| Vacía | sintético | `I1_too_small` | `I1_too_small` |
| Doble vuelta (226 V, 448 T) | sintético, port de `a7_extra.py` | `I9_covers_a_direction_2_times` | `I9_covers_a_direction_2_times` |

La lista son 2 positivos y 15 negativos, como pide la spec en la tabla *Casos*. Los builders sintéticos (tetra, octa, uv_sphere, torus, double_wrap) están copiados de las pruebas unitarias de topology.rs y radial.rs, así que el archivo es autónomo. La fixture solo tiene posiciones cuantizadas de un asteroide procedural, sin datos sensibles.

Seguimiento del test de regeneración: `test-guard` bloqueó dos Edit míos, primero el que hacía saltar el test sin la variable y después el que lo borraba (`removes assertions (1 -> 0)`). No busqué cómo saltármelo. El borrado y el `use` sin `geometry_sha256` los hizo el desarrollador.
