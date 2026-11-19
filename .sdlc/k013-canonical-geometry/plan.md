# Plan: K013, Kiln produce y valida el artefacto de geometría canónica v0

- **Derived from**: `spec.md` (aprobada el 2026-11-18)
- **Status**: approved (2026-11-18). Rama: `k012-asteroid-pilot` en Kiln y ASTRA.
- **Mode**: orchestrate. Cada tarea va al `integrator`; él decide si la hace o la pasa a `implementer`.
- **Repos**: código en `~/Kiln`; paquetes republicados en ASTRA. Rama de trabajo: ver *Stop and ask*, punto 1.

## Done means

- R1–R10 de `spec.md` con su criterio de aceptación cumplido y su evidencia en `review/`.
- Kiln: `cargo test --workspace`, `cargo test --workspace -- --ignored` y `clippy -D warnings` en verde; ningún test existente cambiado.
- ASTRA: los tres paquetes `asteroid_{32,64,128}` republicados con `geometry/seed_<n>.bin`; `recipe.json` y `kiln_generator.gd` con los mismos bytes; fase `asteroid` de `scripts/godot_station.sh` en verde.
- Tabla de los 89 casos (vía, veredicto, código) y tabla de los 48 hashes arm64 frente a x86-64 emulado, o la razón escrita de por qué no se midió.
- Revisión de tres pasadas con veredicto o renuncia en *Review*.

## Stop and ask

1. **Rama.** `~/Kiln` y ASTRA están en `k012-asteroid-pilot`. Propuesta: seguir en esa rama en los dos, como la spec. Si se prefiere `k013-canonical-geometry`, la crea el desarrollador (`branch`/`checkout` solo si lo pide).
2. T1 mide que el GLB **no** conserva las posiciones `f32` de los arrays, o que los paquetes de K012 traen transformación de nodo: se para, porque cambia la spec.
3. Algún caso de la vía A o B da un veredicto distinto de M§8: se explica y se pregunta antes de seguir; no se ajusta el caso.
4. El consumidor de K012 en ASTRA falla con el paquete republicado: no se toca el consumidor sin preguntar.
5. Una tarea que pase de 100 líneas nuevas sin contar tests: se parte antes de lanzarla.

## Tasks

Estimaciones con la mediana real de este repo (`calibrate.sh`): `implementer` 0,9 min / 197 k; las del integrador, con margen por lo que hace él mismo.

| # | Tarea | Requisito | Archivos | Prueba que la demuestra | Runs | Depende | Est. | Status |
|---|-------|-----------|----------|-------------------------|------|---------|------|--------|
| T0 | Trasladar la cadena de ASTRA a `.sdlc/k013-canonical-geometry/` y escribir este plan | D1 | `.sdlc/` de los dos repos | Los tres archivos aquí; la carpeta de ASTRA, borrada | integrator | — | 3m/300k | done |
| T1 | Medir el GLB real frente a los arrays: `generate.gd` sobre la esfera base de 8 y 64 segmentos y `asteroid_64` semilla 42; comparar posiciones `f32`, orden de índices, sentido de giro respecto al centro y transformación de nodo. Decide si la construcción invierte el giro | Riesgo 1, R1, R10 | solo `work/` | Medición; decisión en `decisions/001-glb-orientation.md` | integrator | T0 | 20m/1.5M | done |
| T2 | `kiln-core::canonical`: tipos, cuantizar (S1 §5.5), unir enteros iguales, descartar triángulos que repiten vértice, recuento `2 + S·S/2` / `S·S`, inversión de giro si T1 la pide, serializar (D3, `class` radial = 0) y `geometry_sha256` | R1, R4 | `crates/kiln-core/src/canonical/{mod,build}.rs`, `lib.rs` | Esfera de 8 segmentos de `verify_recipe::sphere_glb` → 34 / 64; tamaño = 32 + 12V + 12T; hash = SHA-256 del archivo; mitades lejos de cero | implementer | T1 | 2m/250k | done |
| T3 | Validador I1–I5 con los códigos de M§8 y el elemento | R2 | `canonical/topology.rs` | Una prueba por código: `I1_too_small`, `I1_out_of_range`, `I2_index_out_of_range`, `I3_duplicate_position`, `I3_unused_vertex`, `I4_open_edge`, `I4_edge_shared_by_more_than_two`, `I5_inconsistent_orientation` | implementer | T2 | 2m/250k | done |
| T4 | Validador I6–I7: ciclo único por vértice, conexión y Euler | R2 | `canonical/topology.rs` | `I6_pinched_vertex`, `I7_not_connected`, `I7_not_a_sphere` | implementer | T3 | 2m/250k | done |
| T5 | Validador I8–I9 con `i128`: determinante respecto al centro y cobertura de direcciones | R2 | `canonical/radial.rs` | `I8_not_radial` (tetraedro descentrado, vértice a través del centro), `I9_covers_a_direction_2_times` (doble vuelta de M§8: 226 V, 448 T) | integrator | T2 | 15m/1.2M | done |
| T6 | `canonical::validate` (I1→I9, primer fallo) y la vía C: tetraedro y 15 mutaciones de un positivo real guardado como fixture | R2, R6 | `canonical/mod.rs`, `crates/kiln-core/tests/canonical_cases.rs`, un fixture `.bin` | 16 casos con el veredicto de M§8 | implementer | T4, T5 | 3m/300k | done (regeneración movida a examples/canonical_fixture.rs (decisión del desarrollador)) |
| T7 | Interruptor en `kiln.toml` (`[generate] canonical = "radial_v0"`; ausente = lo de hoy) y rechazo de GLB con transformación de nodo | R9, R10 | `crates/kiln-core/src/config.rs`, `mesh.rs` | Sin clave = desactivado; valor desconocido = error; GLB a mano con `translation` → código propio; tests existentes intactos | implementer | T0 | 2m/250k | done |
| T8 | Verify: con la opción activa, artefacto por semilla en la caché junto al GLB; `validation.json` gana `geometry_sha256` o el rechazo por semilla; corre aunque fallen comprobaciones anteriores | R1, R3 | `crates/kiln-cli/src/verify_recipe.rs` | Receta buena → 16 hashes; cara invertida → `I5_inconsistent_orientation` (decisión 002); sin opción → informe idéntico al de hoy | integrator | T6, T7 | 15m/1.2M | done |
| T9 | Publish: revalidar todos (I1–I9 y hash) antes de escribir nada; `geometry/seed_<n>.bin`; `asset.json` con `files`, `geometry_sha256` por semilla, versión de Kiln y de Godot; `version` sin cambio (D4) | R4, R5 | `crates/kiln-cli/src/ops/publish_recipe.rs` | Dos publicaciones, mismos bytes; artefacto alterado → fallo y paquete intacto; sin opción → los tres archivos de hoy | implementer | T8 | 3m/300k | done |
| T10 | Vía A con Godot (`#[ignore]`): 48 semillas de K012, 16 escaladas, cuerpo máximo y borde → `ok`; dos recetas negativas → `I8_not_radial`. Recetas de `measure/sphere.gd` y `extra.gd` de ASTRA (`.sdlc/canonical-geometry/measure/`) | R6 | `crates/kiln-cli/tests/godot_canonical.rs` | 68 veredictos iguales a M§8 | integrator | T8 | 30m/2M | done |
| T11 | Vía B con Godot (`#[ignore]`): GLB real directo al validador: sin preparar, facetada, retícula sin soldar, soldada bit a bit → `I3_duplicate_position`; giro de Godot → `I8_not_radial` | R6 | mismo archivo de T10, sección propia | 5 veredictos iguales a M§8 | implementer | T10 | 3m/300k | done (caso 3 desde la esfera base: en el GLB, la retícula de la semilla 42 es la misma entrada que el caso 1) |
| T12 | ASTRA: activar la opción en `kiln.toml`, `kiln verify` y `publish` de los tres paquetes, comprobar bytes de `recipe.json` y del guion, fase `asteroid` | R4, R8 | ASTRA `kiln.toml`, `godot/assets/kiln/asteroid_*/` | `git diff` limitado a `asset.json` y `geometry/`; fase `asteroid` en verde | integrator | T9 | 20m/1.5M | done |
| T13 | x86-64 emulado: Docker `--platform linux/amd64`, Godot 4.7.1 `linux.x86_64`, las 48 semillas; comparar hashes con arm64 | R7 | solo `.sdlc/k013-canonical-geometry/` | Tabla de 48 filas etiquetada «emulado», o la razón de no medir | integrator | T9 | 30m/1.5M | done |
| T14 | Verificación: batería entera de Kiln una vez (normal e `--ignored`), clippy, fase `asteroid` de ASTRA y dos flujos vecinos (publicar `examples/`; verify de una receta sin la opción) | Done means | — | Salidas en archivo y su ruta en el artefacto | verifier | T11, T12, T13 | 8m/1.3M | done (631/0/29, clippy limpio; base sin K013 562/27) |
| T15 | Revisión: tres `reviewer` (bugs, seguridad, conformidad con spec y plan) y consolidación | Done means | `review/` | Hallazgos con archivo:línea, o renuncia | integrator | T14 | 15m/2M | done |
| T16 | Correcciones de la revisión T15 (decisiones del desarrollador, 2026-11-19): I-1 `canonical_odd_segments` en la comprobación estática de verify con la clave (decisión 003); I-2 publish borra los `geometry/seed_<n>.bin` que ya no están en el lote; nit 1 `canonical_engine_missing` si ninguna semilla trae versión de Godot | Review | `generate/check.rs`, `verify_recipe.rs`, `ops/publish_recipe.rs`, `decisions/003-odd-segments.md` | `canonical_refuses_segments_that_can_be_odd`, `canonical_refuses_odd_segments_before_godot`, `canonical_refuses_a_batch_without_a_godot_version`, `a_republish_removes_only_the_geometry_of_seeds_no_longer_verified` | integrator | T15 | 10m/800k | done |
| T17 | Cierre: verificación final una vez (Kiln normal, `--ignored` y clippy; ASTRA republicar sin cambio de bytes, fase `asteroid`, `kiln_check.sh`), recuentos en `CLAUDE.md`, `handoff.md` (I-3, I-4), limpieza de `work/` (I-5) | Done means, Review | `CLAUDE.md`, `handoff.md`, `review/`, `plan.md` | `review/final-*.log` en verde: 635/0/29, 29/0 ignoradas, clippy limpio, sha y `git status` de ASTRA idénticos | integrator | T16 | 25m/2M | done |

## Waves

| Ola | Tareas | Por qué juntas |
|-----|--------|----------------|
| 1 | T0 | Traslado de la cadena |
| 2 | T1, T7 | Independientes, archivos disjuntos |
| 3 | T2 | Necesita la decisión de giro de T1 |
| 4 | T3, T5 | Archivos disjuntos (`topology.rs`, `radial.rs`) |
| 5 | T4 | Mismo archivo que T3 |
| 6 | T6 | Une el validador |
| 7 | T8 | — |
| 8 | T9, T10 | Archivos disjuntos |
| 9 | T11, T12, T13 | Test de Kiln, ASTRA y medición: disjuntos |
| 10 | T14, luego T15 | — |

Camino crítico: T0 → T1 → T2 → T3 → T4 → T6 → T8 → T10 → T11 → T14 → T15, unas 2 h de reloj. Suma secuencial: unas 3 h.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| El exportador cambia posiciones o giro | Los veredictos de M§8 no se reproducen | T1 va primero y para si cambia la spec (*Stop and ask* 2) |
| I9 es la prueba menos trivial (cobertura de direcciones con enteros) | Falsos positivos o negativos | T5 en el integrador, con la doble vuelta de M§8 y los 66 positivos de T10 como contraste |
| Las pruebas con Godot tardan | Batería lenta | `#[ignore]`, como las 15 actuales |
| La emulación x86-64 no arranca o tarda horas | R7 sin medir | Se cierra como «sin medir» con la razón, sin bloquear el resto |
| Dos tareas tocan `topology.rs` | Conflicto | T3 y T4 en olas distintas |

## Estimate vs actual

Sin medir: al cierre (T17, 2026-11-19) no hay `actuals.jsonl` en esta cadena, así que no hay reales que comparar con las estimaciones de la tabla. No se rellena a mano.

## Review

T15 (2026-11-19). Tres `reviewer` en paralelo; consolidación del integrador. Evidencia en `review/evidence/` (`findings.md` lo escribe la sesión principal con el texto del artefacto de T15).

| Pasada | Veredicto |
|---|---|
| bugs | 2 Important: I-1, recuento con S impar (`canonical/build.rs:53-54`, hueco de S1 §2.2, pide decisión); I-2, `geometry/seed_<n>.bin` obsoletos tras republicar (`ops/publish_recipe.rs:108-123`). Los dos reproducidos (`review/evidence/*.log`) |
| security | Sin Important; fuzzing sin pánicos en `from_bytes`, `validate`, `build` y `node_transform_bytes` |
| conformance | R1–R10 cumplen. *Done means* abierto: I-3, evidencia de T14 no está en disco (631/0/29 comunicado, no verificado); I-4, falta la tabla consolidada de 89 casos; I-5, `work/T13-scratch/` de 322 MB sin ignorar |

---
Next stage: tras la aprobación del ingeniero, Build implementa ola a ola.
