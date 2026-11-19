# Context: K013 canonical geometry v0

Lee esto antes de cada tarea. Lo escribe solo la sesión principal.

- Norma: ASTRA `docs/design/ast-esp-001-s1-geometria-canonica-v0.md` (S1) y `.sdlc/canonical-geometry/measurements.md` (M); guiones de medición en ASTRA `.sdlc/canonical-geometry/measure/` (README dice cómo correrlos). ASTRA = /Users/acastells/.t3/projects/astra.
- Rama `k012-asteroid-pilot` en Kiln y ASTRA. Sin escrituras de git.
- Verify de receta: `crates/kiln-cli/src/verify_recipe.rs` (GLB en `kiln/cache/generated/<id>/verify/`, leído con `kiln_core::mesh::read`, `f32`/`u32`). Publish: `crates/kiln-cli/src/ops/publish_recipe.rs` (no lanza Godot). Runner de Godot: `crates/kiln-cli/godot/generate.gd`.
- Helper de prueba: `verify_recipe::tests::sphere_glb(segments, flip_one)`; con 8 segmentos da 80 triángulos, 16 sin área.
- Archivo canónico (D3): bytes LE del hash S1 §6.2, cabecera de 32 bytes (format u32 = 0, class u32 = 0 radial, cuantos por metro u32 = 1000, center 3×i32 = 0, V u32, T u32), luego vértices 3×i32 y triángulos 3×u32. Tamaño exacto 32 + 12V + 12T. `geometry_sha256` = SHA-256 del archivo.
- Cuantizar: `round(f64(x) × 1000)`, mitades lejos de cero (`f64::round`). Recuento: `2 + S·S/2` vértices, `S·S` triángulos.
- Códigos de rechazo: los de M §8 (`I3_duplicate_position`, `I8_not_radial`, `I9_covers_a_direction_2_times`…).
- Cada agente corre solo sus tests (`cargo test -p <crate> <filtro>`), salida a archivo con `2>&1 | tee`; la batería entera la corre T14. Compilación incremental.
- Rojo primero. Si un test falla, se arregla el código, no el test. Menos de 100 líneas nuevas sin tests por tarea.
- Decisión 001 (T1): el GLB de `generate.gd` sale hacia fuera; la construcción no invierte el giro. Posiciones del GLB = arrays de Godot bit a bit; sin transformación de nodo.
- Unión: cada vértice canónico se numera por su primera aparición en el orden de POSITION; los triángulos, en el orden de índices del GLB y con su orden de vértices tal cual (sin rotar). El hash depende de ese orden (S1 §6.2 lo admite).
- `config.rs`: `GenerateSection.canonical: Option<CanonicalClass>` (`RadialV0`, "radial_v0"). `mesh::node_transform(path)` / `node_transform_bytes` → `Some("nodo N: clave")` si hay transformación; también en nodos con `skin` (se mantiene: la canónica es sin esqueleto). Identidad comparada exacta.
- Validador (T3): `topology::check_i1_i5`, una pasada completa por código en el orden de `geo.py`; elemento = primer vértice o triángulo canónico que falla (reglas en `work/T3-result.md`). Los índices son canónicos, no los del GLB que usa `build`.
- El repo no aplica rustfmt: no formatear archivos ajenos.
- `topology::check_i1_i7` encadena I1–I5 e I6–I7; `check_i6_i7` y `radial::check_i8_i9` entran en pánico o dan veredictos sin sentido si no se cumple lo anterior: llamar siempre a `canonical::validate`.
- I9 (T5): perturbación simbólica d = m + εb + ε²c (m = a+b+c del triángulo 0); conos abiertos sin tolerancia. Códigos `I9_covers_a_direction_<2..9>_times` y `_10_or_more_times`. El prototipo `geo.py` usa conos cerrados (det ≥ 0).
- Detalles de rechazo del módulo canonical en inglés.
- `canonical::validate` = I1–I7 y luego I8–I9. Fixture real en `crates/kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin` (asteroid_64 semilla 42, Godot 4.7.1 arm64), positivo y base de las mutaciones de la vía C.
- Decisión 002: se mantiene `generator_radius`; las recetas de cota ≤ 0 no llegan a la vía canónica en verify.
- `validation.json`, por semilla: `geometry_sha256` o `canonical {code, element, detail}`; caché `kiln/cache/generated/<id>/verify/<seed>.canonical.bin`. `asset.json` con la clave: `files` += `geometry/seed_<n>.bin`, `geometry: [{seed, file, geometry_sha256}]`, `kiln_version`, `godot_version`.
- `validation.json` con la clave lleva `recipe.godot_version` (una por receta). Test de la vía A/B: `KILN_CANONICAL_RECIPES=<ASTRA>/godot/assets/kiln cargo test -p kiln-cli --test godot_canonical -- --ignored`.
- T16 / I-1 (decisión 003): con la clave, `generate::check::canonical` da `canonical_odd_segments` (Error) antes de Godot si `sphere.segments` puede salir impar: impar fijo o cualquier rango con extremos distintos. Solo se llama desde `verify_recipe::run`; `kiln generate` y `spec::check` no lo usan.
- T16 / I-2: `publish_recipe::prune_geometry` borra de `geometry/` solo los `seed_<dígitos>.bin` que no están en `files` (sin seguir enlaces, nunca directorios). Sin la clave, un `geometry/` anterior se deja como está.
- T16 / nit 1: con la clave, un lote en el que ninguna semilla trae versión de Godot da `canonical_engine_missing` (Error) en verify; versiones mezcladas con ausentes siguen siendo `canonical_engine_mismatch`.
