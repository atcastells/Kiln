---
task: k013-canonical-geometry/T9
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-cli --bins verify_recipe && cargo test -p kiln-cli --bins publish_recipe && cargo clippy -p kiln-cli --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-cli/src/ops/publish_recipe.rs, crates/kiln-cli/src/verify_recipe.rs, .sdlc/k013-canonical-geometry/decisions/002-static-radius-rule.md, .sdlc/k013-canonical-geometry/spec.md, .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [crates/kiln-cli/src/verify_recipe.rs:227, crates/kiln-cli/src/verify_recipe.rs:130, crates/kiln-cli/src/verify_recipe.rs:714, crates/kiln-cli/src/verify_recipe.rs:605, crates/kiln-cli/src/ops/publish_recipe.rs:44, crates/kiln-cli/src/ops/publish_recipe.rs:95, crates/kiln-cli/src/ops/publish_recipe.rs:622, crates/kiln-cli/src/ops/publish_recipe.rs:41, crates/kiln-cli/src/ops/publish_recipe.rs:89, crates/kiln-cli/src/ops/publish_recipe.rs:115, crates/kiln-cli/src/ops/publish_recipe.rs:157, crates/kiln-cli/src/ops/publish_recipe.rs:230, crates/kiln-cli/godot/generate.gd:54, /tmp/claudio-test-k013-canonical-geometry-T9-fu-verify-red.log, /tmp/claudio-test-k013-canonical-geometry-T9-fu-verify.log, /tmp/claudio-test-k013-canonical-geometry-T9-fu-publish-red.log, /tmp/claudio-test-k013-canonical-geometry-T9-fu-publish.log, /tmp/claudio-clippy-k013-canonical-geometry-T9-fu.log, /tmp/claudio-test-k013-canonical-geometry-T9-red.log, /tmp/claudio-test-k013-canonical-geometry-T9.log]
---

## Summary

- **Pido ratificar 3 cambios en piezas de prueba de K013 (sin commitear; `git diff -U0` muestra que son líneas añadidas por K013, no de antes).** (1) `without_canonical_the_report_and_files_are_todays` (verify_recipe.rs:714) quita también `recipe.godot_version` antes de comparar `on == off`; si no, la igualdad no puede cumplirse. La comprobación «sin clave, el informe de hoy» queda intacta. (2) El helper `canonical_ready` (publish_recipe.rs:622) pone `godot_version` en su informe. (3) `with_node` pasa a usar un helper nuevo, `with_json` (verify_recipe.rs:605), y su comportamiento no cambia. No toqué ningún test anterior a K013.
- R4 está completo. Verify (verify_recipe.rs:227) recoge `run.out["godot"]` de cada semilla y, con la clave, escribe `recipe.godot_version` una vez (:130). Si las semillas no coinciden, da un Error `canonical_engine_mismatch` y no escribe el campo. Publish (publish_recipe.rs:44) se niega en castellano antes de escribir si falta el campo; si está, lo copia a `asset.json` (:95).
- Sin la clave, el informe y el contrato no cambian. Lo prueban `without_canonical_the_report_and_files_are_todays` y `without_canonical_the_contract_has_no_provenance_keys`.
- Añadí el test que faltaba de `canonical_mesh_count` (`canonical_refuses_two_meshes`). Ya pasaba en rojo: cubre código de T8.
- Rojo: verify, 3 FAILED y 20 ok; publish, 2 FAILED y 28 ok. Verde: verify 23/23 y publish 30/30 (`--bins`). Clippy de `kiln-cli --all-targets -D warnings`, sin avisos. No toqué T10.
- Cadena hecha: decisión 002 creada, R3 de spec, plan (T8 con su prueba nueva y T7–T9 en `done`) y context (+2 líneas).
- Encontrado, sin cambiar código: (a) si ninguna semilla devuelve una cadena `godot`, verify pasa y publish dice «vuelve a verificar», cosa que no lo arregla. El `generate.gd` real siempre la escribe. (b) context.md no menciona `recipe.godot_version` en `validation.json`: ver Learned.

Learned: `validation.json` con la clave lleva `recipe.godot_version` (cadena, una por receta) al nivel de `recipe`, no por semilla. Les sirve a T10 y T12.

## Detail

### T9 (primera pasada)

- Hice T9 en `publish_recipe.rs` sin delegar: unas 70 líneas de código y 7 tests.
- Puerta: en `gate` (:230), con `[generate] canonical`, una semilla sin `geometry_sha256` hace que publish se niegue.
- Revalidación: `fn geometry` (:157) corre antes de `create_dest` (lo llama en :41). Lee `<cache>/generated/<id>/verify/<seed>.canonical.bin` con `cache_dir`, no sigue enlaces, y hace `from_bytes` + `validate` y la comparación del SHA-256.
- Escritura: recipe.json, el guion, `geometry/seed_<n>.bin` con los bytes leídos (:115) y `asset.json` el último. El contrato se arma en :89. `version` no cambia (D4).

Learned: forma de `asset.json` con la clave. Los campos de hoy no cambian, y se añaden `files` += `"geometry/seed_<n>.bin"` en orden numérico de semilla, `"geometry": [{"seed": <u64>, "file": "geometry/seed_<n>.bin", "geometry_sha256": "<64 hex>"}]` en ese mismo orden, `"kiln_version": "<CARGO_PKG_VERSION de kiln-cli, hoy 0.1.0>"` y `"godot_version"`. Sin la clave, el JSON sale igual que hoy.
Learned: para T12, `changed` no tiene en cuenta la geometría (D4). Republicar un paquete de K012 con la clave escribe `geometry/` y los campos nuevos de `asset.json`, pero dice «versión 1, sin cambios» (`changed: false`).
Learned: `Published.files` coincide siempre con `files` de `asset.json`.
Learned: códigos de rechazo de publish. Con la geometría, el mensaje sale como «la geometría canónica de la semilla N de «id» … : vuelve a verificar», donde «…» es «no está en la caché», «no se lee (…)», «no es válida (<código>)» o «no es la verificada». Además, «tiene una semilla que no es un número» si `seed` no es u64, y «no dice con qué versión de Godot se generó» si falta `godot_version`.

- Tests nuevos, todos con el fixture real `asteroid_64_seed42.bin` vía `include_bytes!`:
  - `canonical_publish_writes_the_geometry_and_its_hashes`: comprueba `files`, `geometry`, `kiln_version` y que el SHA-256 de cada archivo publicado coincide con el de `asset.json`.
  - `canonical_publish_twice_gives_the_same_bytes`: los 5 archivos con los mismos bytes y `version` 1 sin cambio.
  - `a_tampered_artefact_refuses_and_leaves_the_package_as_it_was`: con un paquete ya publicado, cambia un byte del `.bin` de la semilla 2. Publish se niega, y la lista de archivos del paquete con sus hashes queda igual que antes.
  - `an_artefact_that_is_not_the_verified_one_refuses`: el `.bin` está intacto pero el informe trae otro hash.
  - `an_invalid_artefact_with_the_verified_hash_refuses`: un triángulo con sus vértices en otro orden y su hash en el informe, de modo que solo `validate` puede rechazarlo (da «no es válida»).
  - `a_missing_artefact_refuses`.
  - `a_report_without_the_hash_of_one_seed_refuses`.
  - Los cuatro rechazos que parten sin paquete previo (otro hash, inválido, ausente y sin hash) comprueban que no se creó `godot/`.
- Si `geometry/` existe y no es un directorio real (por ejemplo, un enlace), se borra antes de crearlo, como hace `put` con los archivos.
- T10 (`tests/godot_canonical.rs`) no rompió la compilación. No lo toqué.
- Fuera de alcance, sin implementar:
  1. Si la tanda se reduce (menos `verify_seeds`), los `geometry/seed_<n>.bin` que sobran se quedan en el paquete, aunque `files` ya no los liste.
  2. Si se quita la clave y se vuelve a publicar, `geometry/` se queda en el paquete.
  En los dos casos, el contrato de `asset.json` es correcto.

### Follow-up (opción A elegida y decisión 002)

- **Toca ratificar: cambié dos piezas de prueba propias de K013 (sin commitear, de T8 y T9).** (1) En `without_canonical_the_report_and_files_are_todays` (verify_recipe.rs:714), una línea quita también `recipe.godot_version` del informe con la clave y comprueba que estaba. Sin ella, la igualdad `on == off` no se puede cumplir con `godot_version` en el informe. La parte «sin clave, el informe de hoy» no cambia. (2) El helper `canonical_ready` (publish_recipe.rs:622) pone `godot_version` en su informe. Ningún test anterior a K013 se tocó.
- Verify (verify_recipe.rs:227): guarda `run.out["godot"]` de cada semilla, también de las que dan un GLB ilegible. Si la clave está activa y todas coinciden, queda un `recipe.godot_version` (:130). Si difieren, da un Error `canonical_engine_mismatch` y no escribe el campo. Si no hay semillas, o no llega ninguna versión, tampoco hay campo, nunca `null`. Sin la clave, nada cambia.
- Publish (publish_recipe.rs:44): con la clave, después de revalidar la geometría y antes de escribir nada, se niega con «la verificación de «id» no dice con qué versión de Godot se generó: vuelve a verificar». Si no se niega, copia `godot_version` a `asset.json` (:95).
- Tests nuevos: `canonical_refuses_two_meshes` (cubre `canonical_mesh_count`, que ya pasaba en rojo: es cobertura de código de T8), `canonical_records_the_godot_version_of_the_batch`, `canonical_refuses_seeds_made_by_different_godots` (el falso responde `fake-<pid>`), `canonical_publish_copies_the_godot_version`, `a_report_without_the_godot_version_refuses` y `without_canonical_the_contract_has_no_provenance_keys`.
- Rojo: verify, 3 FAILED y 20 ok; publish, 2 FAILED y 28 ok. Verde: verify 23/23 y publish 30/30. Clippy de `kiln-cli --all-targets -D warnings`, sin avisos; compila también `tests/godot_canonical.rs` de T10, que no toqué.
- Corrí `--bins` en vez del filtro a secas, para no compilar ni filtrar los binarios de integración de T10 mientras cambian. Los tests de estos dos módulos viven en el binario `kiln`.
- Cadena: `decisions/002-static-radius-rule.md` (cita check.rs:130-160; `numeric_rules` empieza en :130 y el hallazgo está en :150-161). Además, criterio de R3 en `spec.md`, columna de prueba de T8 en `plan.md`, T8 y T9 `done` (T7 ya lo estaba) y las dos líneas añadidas a `context.md`.

Learned: el mensaje de `canonical_engine_mismatch` lista las versiones distintas en formato Debug (`[Some("…"), …]`). Si se quiere más legible, es un retoque de una línea.
