---
task: k013-canonical-geometry/T3
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core canonical && cargo clippy -p kiln-core --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/canonical/topology.rs, crates/kiln-core/src/canonical/mod.rs]
evidence: [crates/kiln-core/src/canonical/topology.rs:31, crates/kiln-core/src/canonical/topology.rs:37, crates/kiln-core/src/canonical/topology.rs:53, crates/kiln-core/src/canonical/topology.rs:61, crates/kiln-core/src/canonical/topology.rs:71, crates/kiln-core/src/canonical/mod.rs:6, /tmp/claudio-test-k013-canonical-geometry-T3.log, /tmp/claudio-clippy-k013-canonical-geometry-T3.log]
---

## Summary

- `kiln_core::canonical::topology::check_i1_i5(&Geometry) -> Result<(), Rejection>` (topology.rs:31) y `COORD_LIMIT = 1 << 30`. 77 líneas sin pruebas en topology.rs (doc incluida) + 1 en mod.rs, por debajo de 100. Lo hice yo, sin delegar.
- Orden de pasadas, cada código una pasada completa, igual que `geo.py` `validate` (ASTRA measure/geo.py:139-148): `I1_too_small`, `I1_out_of_range`, `I2_index_out_of_range`, `I2_repeated_index`, `I3_unused_vertex`, `I3_duplicate_position`, `I4_open_edge`, `I4_edge_shared_by_more_than_two`, `I5_inconsistent_orientation`.
- Rojo primero: 9 negativos fallaron con el stub `Ok(())` (9 failed, 1 passed; el log solo guarda la pasada verde). Ahora `cargo test -p kiln-core canonical` pasa 25/25 (10 de T3). Clippy `-D warnings` limpio.
- `mod.rs`: solo `pub mod topology;` (mod.rs:6, tras `pub mod radial;` de T5, que entró después; no he corrido pruebas sobre radial.rs).
- Hueco del plan: la fila T3 no lista `I2_repeated_index`; el brief y `geo.py` sí. Implementado y probado.

Learned:
- Elemento: `I1_too_small` None; `I1_out_of_range` primer vértice; `I2_*` primer triángulo; `I3_unused_vertex` primer vértice sin usar; `I3_duplicate_position` el vértice posterior del par; `I4_*` primer triángulo con una arista usada 1 vez / más de 2; `I5` primer triángulo con una arista dirigida ya vista en uno anterior (no tiene por qué ser el invertido). T4 y el cargador de Rust (T-078) deberían copiar estas reglas.
- Los `HashMap` solo cuentan o buscan; el elemento sale de recorrer `Vec` en orden de índice, así que es determinista.
- `unsigned_abs()` evita el pánico de `i32::MIN.abs()`; probado con ±2^30, `i32::MIN`, `i32::MAX` (rechazo) y 2^30−1 (pasa).
- Un tetraedro menos un triángulo da `I1_too_small`: `I4_open_edge` se prueba con un octaedro (fixture en las pruebas, reutilizable en T4).
- Los índices de triángulo son los canónicos (tras `build`), no los de la entrada GLB que usa `build` para su `I2_index_out_of_range` (ver T2).
- El repo no aplica rustfmt (build.rs tampoco pasa `rustfmt --check`); no he formateado.

## Detail

Pruebas en topology.rs `mod tests`: aceptación de tetraedro y octaedro; una por código, con mutación del tetraedro (octaedro para `I4_open_edge`, elemento 4 = `[1,3,4]`, primer triángulo con arista del `[1,5,3]` quitado). `I2_index_out_of_range` también comprueba que gana a un `I2_repeated_index` en un triángulo anterior. Sin datos sensibles.
