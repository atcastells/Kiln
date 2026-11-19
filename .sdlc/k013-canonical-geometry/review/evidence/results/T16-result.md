---
task: k013-canonical-geometry/T16
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cd ~/Kiln && cargo test -p kiln-core generate && cargo test -p kiln-cli --bins verify_recipe && cargo test -p kiln-cli --bins publish_recipe && cargo clippy --workspace --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/generate/check.rs, crates/kiln-cli/src/verify_recipe.rs, crates/kiln-cli/src/ops/publish_recipe.rs, .sdlc/k013-canonical-geometry/decisions/003-odd-segments.md, .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [crates/kiln-core/src/generate/check.rs:118, crates/kiln-cli/src/verify_recipe.rs:179, crates/kiln-cli/src/verify_recipe.rs:239, crates/kiln-cli/src/ops/publish_recipe.rs:117, crates/kiln-cli/src/ops/publish_recipe.rs:148, .sdlc/k013-canonical-geometry/work/T16-red-core.log, .sdlc/k013-canonical-geometry/work/T16-red-verify.log, .sdlc/k013-canonical-geometry/work/T16-red-publish.log, .sdlc/k013-canonical-geometry/work/T16-core.log, .sdlc/k013-canonical-geometry/work/T16-verify.log, .sdlc/k013-canonical-geometry/work/T16-publish.log, .sdlc/k013-canonical-geometry/work/T16-clippy.log]
---

## Summary

- I-1: `generate::check::canonical` (check.rs:118) devuelve `canonical_odd_segments` (Error, en castellano). Se llama solo con la clave, en `verify_recipe::run` (verify_recipe.rs:179) y antes de la salida temprana, así que Godot no llega a lanzarse. Es más estricto de lo que el paréntesis «any end» deja leer: rechaza **cualquier rango con extremos distintos**, también `[8, 10]`. `draw` hace floor de `a + (b−a)·u` y saca 9: el rojo de verify lo muestra, con `canonical_count` en 6 semillas (T16-red-verify.log). Pasan un par fijo, `[n, n]` con n par o el parámetro ausente (por defecto 32). Si el valor ya tiene un problema de rango, se deja a `check`. Sin la clave no cambia nada (lo prueba el mismo test).
- I-2: `prune_geometry` (publish_recipe.rs:148) se llama tras escribir el lote nuevo y antes de `asset.json` (publish_recipe.rs:117). Borra solo los `seed_<dígitos>.bin` que no están en `files`. Nunca toca directorios. Un enlace se borra como enlace (el destino sobrevive, probado). Si el borrado falla, devuelve `Err` antes del contrato. Sin la clave, el `geometry/` viejo se queda: hay un comentario que lo dice.
- Nit 1: con la clave, si ninguna semilla trae versión, sale `canonical_engine_missing` (Error) y verify falla (verify_recipe.rs:239). Si unas semillas traen versión y otras no, sigue siendo `canonical_engine_mismatch`.
- Cadena: edité `plan.md` (nota de T6 y fila T16 en done) y `context.md` (3 líneas) por encargo expreso de la sesión principal, nada más. Escribí `decisions/003-odd-segments.md`.
- Verde: kiln-core generate 57/0, verify_recipe 25/0, publish_recipe 31/0; clippy `-D warnings` limpio. Hubo una ronda roja, cuatro fallos y todos por la razón esperada (logs `T16-red-*`). Batería entera no corrida (la corre la orquestación).

Learned: un rango entero `[a, b]` con a ≠ b siempre puede dar impar con `resolve::draw`; la regla útil es «par fijo», no «extremos pares».
Learned: los helpers `canonical_ready`/`two_seeds` de publish_recipe convierten en test un escenario de republicación en ~20 líneas.
Learned: la falsa de Godot (`generate::tests::FAKE`) se adapta con `replace` sobre su `echo`; vale para simular un motor sin versión.

## Detail

Tests nuevos (ningún test existente cambiado):
- `generate::check::tests::canonical_refuses_segments_that_can_be_odd`: 5 casos que rechaza y 5 que pasan. También comprueba que `check` sin clave no da el código.
- `verify_recipe::tests::canonical_refuses_odd_segments_before_godot`: con la clave, 1 hallazgo, sin semillas y sin `ran`. Sin la clave, 0 hallazgos.
- `verify_recipe::tests::canonical_refuses_a_batch_without_a_godot_version`: la falsa escribe su JSON sin `godot`. El hallazgo es único y de severidad error, y no queda `recipe.godot_version`.
- `ops::publish_recipe::tests::a_republish_removes_only_the_geometry_of_seeds_no_longer_verified`: publica {1,2} y luego {1}. Comprueba que `files` lista solo seed_1 y que seed_2.bin desaparece. Sobreviven `seed_2.bin.bak`, `notes.txt`, `seed_.bin` y `seed_x.bin`, y el destino del enlace `seed_3.bin` queda intacto.

El test de publish lleva `#[cfg(unix)]` porque crea un enlace simbólico, como `a_link_in_the_destination_is_replaced_not_followed`. Lo añadí después del verde y no cambia nada en macOS, así que no volví a correr nada. Ninguna tabla ni test enumera los códigos `canonical_*`: `crates/kiln-cli/tests/common/canonical.rs:60` solo forma su propio texto. Con `radial_v0` solo aparecen `config.rs` y los dos archivos de test tocados, así que ningún ejemplo de `examples/` usa la clave. La regeneración del fixture de T6 está en `crates/kiln-core/examples/canonical_fixture.rs`. En la nota del plan dejé el texto dictado tal cual. `git status` muestra todo K013 sin confirmar; de T16 son solo los archivos de `files:`.

No tocado (fuera del encargo): `kiln generate` (crates/kiln-cli/src/generate.rs:70) y `spec::check` (crates/kiln-core/src/spec.rs:579) no aplican la regla de impares. `generate` produce un modelo de prueba y no publica, y verify es la puerta. Las tres fichas de ASTRA usan segmentos pares fijos (32/64/128) y `tests/godot_canonical.rs` usa 32–256 pares: ninguna queda rechazada por la regla nueva.
