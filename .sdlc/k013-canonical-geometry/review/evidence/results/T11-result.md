---
task: k013-canonical-geometry/T11
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cd ~/Kiln && KILN_CANONICAL_RECIPES=~/.t3/projects/astra/godot/assets/kiln cargo test -p kiln-cli --test godot_canonical -- --ignored --nocapture 2>&1 | tee /tmp/claudio-test-k013-canonical-geometry-T11.log"
persist: propose
files: [/Users/acastells/Kiln/crates/kiln-cli/tests/godot_canonical.rs, /Users/acastells/Kiln/crates/kiln-cli/tests/common/canonical.rs]
evidence: [crates/kiln-cli/tests/godot_canonical.rs:116-209, crates/kiln-cli/tests/common/canonical.rs:26-48, /tmp/claudio-test-k013-canonical-geometry-T11.log, /tmp/claudio-clippy-k013-T11.log]
---

## Summary

- Vía B en verde: los 5 veredictos coinciden con M§8 y no se ajustó ningún caso. La prueba es `#[ignore]` (`godot_canonical.rs:157`) y está en su propia sección (`:116`). Hay 3 GLB reales de `generate.gd` y `validate` sin `build`; solo se cuantiza cada posición como su propio vértice y los índices se dejan tal cual (`direct`, `:120`).
- El caso 5 se construye invirtiendo el giro, como manda la decisión 001: el GLB ya sale hacia fuera. Se toma la semilla 42 con `facet`, se pasa por el `build` normal (un assert comprueba que da `ok`) y se invierte cada triángulo, lo que da `I8_not_radial`. Así lo dice el comentario del caso.
- Hay dos controles con respuesta conocida. La soldadura bit a bit da 2210 → 2056 vértices, como en M§2, y un assert lo comprueba. La vía A sigue igual tras partir el helper: 66 `ok`, 2 `I8_not_radial` y `pub_64_42` = `2713c0a9…d78a` (log).
- No hubo rojo previo, porque el código bajo prueba ya existía (T2–T8). Hacen de control los asserts anteriores.
- Clippy `-p kiln-cli --all-targets -D warnings` sale limpio: /tmp/claudio-clippy-k013-T11.log.
- **Para revisar (no bloquea)**: el caso 3 sale de otro GLB que en M (ver tabla).

| caso | GLB de origen | esperado (M§8) | obtenido |
|---|---|---|---|
| 1 arrays suaves sin preparar | asteroid_64 semilla 42, 64 seg., sin `facet` | I3_duplicate_position | I3_duplicate_position |
| 2 facetada sin compartir | asteroid_64 semilla 42, 64 seg., con `facet` | I3_duplicate_position | I3_duplicate_position |
| 3 retícula sin soldar | `sphere` r 1000 m, 64 seg., sin ruido ni `facet` | I3_duplicate_position | I3_duplicate_position |
| 4 esfera base soldada bit a bit | mismo GLB que el 3, soldado por `to_bits` | I3_duplicate_position | I3_duplicate_position |
| 5 orden de Godot | semilla 42 con `facet`, `build`, giro invertido | I8_not_radial | I8_not_radial |

## Detail

- **Por qué el caso 3 sale de otro GLB.** M lo construye con la semilla 42 en orden de retícula (`measure/a5_validator.py:25`). Por GLB, eso es byte a byte el caso 1: según la decisión 001, el GLB sin `facet` conserva las posiciones en el orden de la retícula. Por eso el 3 usa la esfera base (radio de `measure/sphere.gd`). Si la sesión principal prefiere el mismo origen que M, el 3 duplica el 1.
- **Caso 4.** Se unen solo las posiciones `f32` bit a bit idénticas, numeradas por su primera aparición, y se descartan los triángulos que repiten un vértice. Después se cuantiza. No se invierte el giro, a diferencia de `a5_validator.py:26` (que trabaja sobre arrays crudos hacia dentro).
- **Helper.** `verdict` se partió en `export` (Godot, comprobación de nodo y de malla única; devuelve posiciones e índices) y lo que ya había detrás. La vía A no cambia de comportamiento.
- Tiempo: 5,15 s las dos pruebas juntas (las 3 exportaciones de la vía B son secuenciales).

Aprendido:
- Learned: por GLB, «arrays suaves sin preparar» y «retícula sin soldar» de M§8 son la misma entrada si salen de la misma semilla. Hace falta otro GLB para que sean dos casos.
- Learned: soldar bit a bit la esfera base exportada reproduce M§2 sobre el GLB (2210 → 2056), así que el exportador conserva los ceros con signo de los polos.
- Learned: invertir todos los triángulos de una geometría válida pasa I1–I7 y cae justo en I8, sin tocar antes I5.
