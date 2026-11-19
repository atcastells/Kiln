---
task: k013-canonical-geometry/T8
from: integrator
kind: result
result: needs-decision
data: evidence
cost:
regen: "cargo test -p kiln-cli verify_recipe && cargo clippy -p kiln-cli --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-cli/src/verify_recipe.rs, .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [crates/kiln-cli/src/verify_recipe.rs:18, crates/kiln-cli/src/verify_recipe.rs:204, crates/kiln-cli/src/verify_recipe.rs:226, crates/kiln-cli/src/verify_recipe.rs:106, crates/kiln-cli/src/verify_recipe.rs:174, crates/kiln-core/src/generate/check.rs:132, /Users/acastells/.t3/projects/astra/docs/design/ast-esp-001-s1-geometria-canonica-v0.md:178, /tmp/claudio-test-k013-canonical-geometry-T8-red.log, /tmp/claudio-test-k013-canonical-geometry-T8.log, /tmp/claudio-clippy-k013-canonical-geometry-T8.log]
---

## Summary

- T8 está completo y T9 puede arrancar con la forma JSON de abajo. La decisión pendiente solo afecta al criterio de R3/T10 sobre las recetas negativas.
- T8 hecho por mí, sin delegar: solo `verify_recipe.rs`, unas 55 líneas de código y 4 tests. Con `[generate] canonical`, `run()` hace el paso canónico en cada semilla legible después de `judge`, haya o no hallazgos anteriores (:204). El orden es nodo → recuento → `build` → `validate` (`fn canonical`, :226). Si pasa, escribe `<seed>.canonical.bin` junto a `<seed>.glb` en `kiln/cache/generated/<id>/verify/`.
- Si falla, se añade un hallazgo Error con el código del rechazo y el mensaje «semilla N: …», y el informe queda inválido. En `validation.json`, cada semilla gana `geometry_sha256` si pasa o `canonical` si falla (:106). Sin la clave, no se relee el GLB y la entrada sale igual que hoy.
- Rojo primero: 4 FAILED y 16 ok (log `-red`). En verde, `cargo test -p kiln-cli verify_recipe` da 20/20 (log T8) y Clippy de kiln-cli con `-D warnings` no da avisos. No cambié ningún test existente ni `sphere_glb`. La traslación del nodo usa un ayudante nuevo, `with_node`.
- Tests: con la esfera buena de 8, las 16 semillas dan un hash de 64 hex y un `.bin` de 1208 B (32+12·34+12·64), cuyo SHA-256 es el del informe. Con la cara invertida salen `generated_flipped` y 16 `I5_inconsistent_orientation`, uno por semilla y sin ningún `.bin`. Con `translation` en el nodo salen 16 `canonical_node_transform`. Sin la clave, las semillas llevan las 8 claves de hoy y no hay `.bin`. Activando la clave en el mismo proyecto y quitando `geometry_sha256`, el `Value` del informe sale igual.
- **Decisión pendiente (leído, no ejecutado):** en `numeric_rules` (check.rs:132-151) la cota empieza en 1, `stretch` la multiplica y `noise` le resta. Las dos recetas negativas de S1 §4.2 (S1:178-179) caen por debajo de 0 en el primer `noise` (0,2 − 1,0 y 0,2 − 0,6). Las dos dan `generator_radius` como Error, y `run()` vuelve sin semillas (verify_recipe.rs:174; el test `a_static_error_never_launches_godot` lo confirma). Así que nunca dan `I8_not_radial` en verify, y eso contradice S1:182, «Esas recetas no se rechazan en bloque». Ver «Decision needed».

Learned: la forma que se añade a cada elemento de `recipe.seeds[]` en `validation.json` es una de estas dos, nunca las dos a la vez, y ninguna sin la clave: si pasa, `"geometry_sha256": "<64 hex en minúsculas>"`; si falla, `"canonical": {"code": "<str>", "element": <entero o null>, "detail": "<str en inglés>"}`.
Learned: el artefacto está en `kiln/cache/generated/<id>/verify/<seed>.canonical.bin` (`glb.with_extension("canonical.bin")`). `verify_dir` borra y recrea el directorio en cada verify, así que no quedan `.bin` viejos. T9 lo renombra a `geometry/seed_<n>.bin`.
Learned: los códigos nuevos de verify son `canonical_node_transform` y `canonical_mesh_count`. Si `node_transform` da `Err`, también es `canonical_node_transform`, con el detalle «cannot read the nodes: …». Un GLB ilegible sigue sin entrada de semilla y solo da `generated_unreadable`, como hoy. `SeedResult` tiene un campo nuevo, `canonical: Option<Result<String, Rejection>>`, y solo se construye en verify_recipe.rs.
Learned: `mesh::read` da una entrada por primitiva de triángulos, así que «una malla» significa una sola primitiva en todo el GLB.

## Decision needed

La receta negativa `0,2 − 3 × 1,0` (R3 y T10) se para en la comprobación estática `generator_radius` antes de generar.

| Opción | Qué cambia |
|---|---|
| A. Con la clave activa, `generator_radius` no corta la verificación: se genera y la vía canónica decide | Toca `run()` y la regla estática. Cambia lo que verifica una receta, más allá de T8 |
| B. Que R3 y T10 acepten `generator_radius` como rechazo de esa receta, y que el `I8_not_radial` de la receta se pruebe solo en la vía A con Godot, sin pasar por verify | Ningún cambio de código. Hay que ajustar el criterio en plan y spec |

No hay opción C. Que una receta pase la comprobación estática significa que su cota es mayor que 0, y S1 §4.2 dice que con cota positiva la receta ya es radial (con el ruido de valor absoluto ≤ 1). Por tanto, ninguna receta negativa puede pasar la comprobación estática.

La A es la que se ajusta a la letra de S1 («deciden I8 e I9 semilla a semilla»), pero cambia una regla de verify que hoy existe y necesita una tarea propia, con su decisión de alcance. La B es la que menos toca. Decide la sesión principal: no entra en el alcance de T8.

## Detail

- La cadena se editó como pedía el encargo. En `plan.md`, T6 queda `done (test de regeneración #[ignore] pendiente de decisión)`, y `context.md` gana la viñeta de `canonical::validate` y del fixture.
- Ninguna regla deduplica los hallazgos canónicos: cada mensaje lleva su semilla, así que no se repiten. La deduplicación de `glb_findings` en `verify()` no los toca.
- `segments_of` devuelve `u64`. Se pasa a `build` con `u32::try_from(..).unwrap_or(u32::MAX)`, y un valor fuera de rango acaba como `canonical_count`.
- No toqué `crates/kiln-core/tests/canonical_cases.rs` ni sus fixtures.
- No hay test de `canonical_mesh_count`: no lo pedía el encargo y `sphere_glb` da una sola primitiva. Se puede añadir con `with_node` y una segunda primitiva si se quiere.
