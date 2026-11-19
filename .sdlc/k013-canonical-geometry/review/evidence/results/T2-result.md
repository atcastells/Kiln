---
task: k013-canonical-geometry/T2
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core canonical && cargo clippy -p kiln-core --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/canonical/mod.rs, crates/kiln-core/src/canonical/build.rs, crates/kiln-core/src/lib.rs, .sdlc/k013-canonical-geometry/context.md, .sdlc/k013-canonical-geometry/plan.md]
evidence: [crates/kiln-core/src/canonical/mod.rs:21, crates/kiln-core/src/canonical/mod.rs:29, crates/kiln-core/src/canonical/mod.rs:42, crates/kiln-core/src/canonical/mod.rs:60, crates/kiln-core/src/canonical/mod.rs:87, crates/kiln-core/src/canonical/build.rs:10, crates/kiln-core/src/canonical/build.rs:22, /tmp/claudio-test-k013-canonical-geometry-T2.log, /tmp/claudio-clippy-k013-canonical-geometry-T2.log]
---

## Summary

- **Por encima del límite: 119 líneas de código sin pruebas, para un máximo de 100.** Todo lo que queda lo pide el brief: tipos, `quantize`, `build`, `to_bytes`, `from_bytes`, el hash y la protección contra un índice fuera de rango. La sesión principal decide si lo acepta o qué hay que separar. He quitado un `Display` que nadie usaba.
- Antes del código edité la cadena: añadí 3 líneas al final de `context.md` y puse T1 y T7 en `done` en `plan.md`. La tarea la hice yo, sin delegarla.
- `kiln_core::canonical` tiene `Geometry`, `Rejection { code, element, detail }` con `new`, las constantes v0, `quantize`, `build`, `to_bytes`, `from_bytes` y `geometry_sha256` (este reutiliza `asset::sha256_hex`).
- Las 11 pruebas nuevas fallaron primero (rojo por `todo!()`). Ahora `cargo test -p kiln-core canonical` pasa 15/15: las 11 nuevas y 4 de T7 y de hash que entran en el filtro. Clippy con `-D warnings` no da avisos.
- La prueba reproduce la esfera de `sphere_glb`. Con 8 segmentos salen 34 vértices y 64 triángulos (de 54 posiciones y 80 triángulos de entrada); con 64, 2050 y 4096. El giro no se invierte.
- Códigos nuevos que no están en M§8: `canonical_count`, `canonical_quantize`, `canonical_size` y `canonical_header`. `build` también da `I2_index_out_of_range`, pero su `element` es el índice del triángulo **en la entrada**, antes de descartar. Si T3 numera los triángulos canónicos, la numeración no coincide.

Learned:
- Ruta de la unión: cuantizar → `HashMap<[i32;3], u32>` por primera aparición → remapear índices; un `NaN` se rechaza con el índice del vértice.
- Ese `HashMap` solo hace de índice de búsqueda: el orden de los vértices sale de `Vec::push`, así que el hash no depende del orden de iteración del `HashMap`.
- `from_bytes` comprueba la cabecera antes que el tamaño (salvo si hay menos de 32 bytes) y calcula el tamaño en `u64`, así que V = `u32::MAX` da `canonical_size` sin desbordar.
- `from_bytes` no valida que los índices de triángulo estén en rango: eso lo hace I2 en T3.
- El repo no está formateado con `rustfmt` (`cargo fmt --check` ya da diferencias en `bindings.rs`), así que no he formateado.

## Detail

- Una ronda en rojo, una en verde, Clippy (1 aviso, `manual_is_multiple_of`, corregido) otra tras el arreglo y la última tras quitar `Display`: 4 ejecuciones de la prueba, el máximo.
- `quantize`: `(x as f64 * 1000.0).round()`, se rechaza si no es finito o queda fuera de i32. Pruebas: 0.0005 → 1, −0.0005 → −1, ±0.0004999 → 0, 2e6 → 2e9, ±3e6, NaN y ±inf → `canonical_quantize`.
- `build`: si el número de índices no es múltiplo de 3 → `canonical_count`. Si el recuento no cuadra, el detalle trae lo esperado y lo obtenido (prueba con 10 segmentos: 52 esperados, 34 obtenidos).
- Bytes: la prueba fija cabecera y desplazamientos de un tetraedro. Ida y vuelta igual. Se rechazan el truncado, 20 bytes, vacío, +12 bytes y V = `u32::MAX` (`canonical_size`), y un byte cambiado en cada palabra de la cabecera (`canonical_header`).
- Orden comprobado: `vertices[0]` = polo `[0,1000,0]`; `triangles[0]` = `[0,2,1]` (el primer triángulo que no se descarta, con su orden de vértices tal cual).
