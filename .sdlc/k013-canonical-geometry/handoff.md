# Handoff: K013, Kiln produce y valida la geometría canónica v0

- **Fecha**: 2026-11-19
- **Estado**: **hecho**. Con la clave `[generate] canonical = "radial_v0"`,
  `kiln verify` construye y valida (I1–I9, enteros exactos) la geometría
  canónica de cada semilla del GLB que Kiln lee de verdad, y `kiln publish`
  la escribe en el paquete con su `geometry_sha256` y su procedencia. Los
  89 casos de S1 §8 dan el veredicto de M§8. Los tres paquetes de K012 están
  republicados en ASTRA con su geometría, sin cambiar un byte de lo que K012
  publicó. K013 está confirmado y unido a `main` (Kiln PR #33, ASTRA
  PR #22) y `work/` ya está borrado (I-5, *Estado Git*).
- **Entorno**: un solo equipo. Apple Silicon arm64, macOS 27.0, Godot
  4.7.1-stable (official) sin ventana. La comparación con x86-64 es
  **emulada** (Rosetta en Docker, en el mismo Mac); no hay medida en un
  equipo x86-64 nativo.
- **Norma**: AST-ESP-001-S1 (ASTRA,
  `docs/design/ast-esp-001-s1-geometria-canonica-v0.md`, «S1») y
  `.sdlc/canonical-geometry/measurements.md` de ASTRA («M»). Cadena: `intent.md`,
  `spec.md`, `plan.md`, `context.md`, `decisions/`, `measurements-x86.md` y
  `review/` en esta carpeta.

## Verificación final (cierra I-3)

Corrida una vez cada una el 2026-11-19, con el árbol de trabajo de las dos
ramas tal como queda (todo K013 sin confirmar).

| Qué | Resultado | Evidencia |
|---|---|---|
| Kiln `cargo test --workspace` | **635 pasan, 0 fallan, 29 ignorados** | `review/final-test.log` |
| Kiln `KILN_CANONICAL_RECIPES=<ASTRA>/godot/assets/kiln cargo test --workspace -- --ignored` | **29 pasan, 0 fallan** (incluye las dos pruebas de `godot_canonical`) | `review/final-ignored.log` |
| Kiln `cargo clippy --workspace --all-targets -- -D warnings` | limpio, código 0 | `review/final-clippy.log` |
| ASTRA: `kiln verify` + `kiln publish` de `asteroid_{32,64,128}` con el binario recién compilado | los seis, código 0; «versión 1, sin cambios» en los tres | `review/final-astra-republish.log` |
| ASTRA: sha256 de los 60 archivos de `godot/assets/kiln/asteroid_*/` antes y después | idénticos | `review/final-astra-sha-before.txt`, `review/final-astra-sha-after.txt` |
| ASTRA: `git status --short` antes, tras republicar y tras las dos fases | idéntico | `review/final-astra-status-before.txt`, `review/final-astra-status-after.txt` |
| ASTRA: `scripts/godot_station.sh asteroid` | `OK: asteroid recipe test passed`, código 0, ningún `FAIL` | `review/final-astra-asteroid.log` |
| ASTRA: `scripts/kiln_check.sh` | `OK: kiln validated the astra modules`; 26 piezas, 0 errores, 23 avisos | `review/final-kiln-check.log` |
| `geometry_sha256` publicados frente a T10, a su archivo y a x86-64 emulado | 48/48 en las tres comparaciones | `review/final-hash-crosscheck.log` |
| Ningún test existente cambiado | las 13 líneas que quita el diff de Kiln están fuera de los módulos `tests` | `review/final-kiln-diff-tracked.txt` |

La batería de T14 dio 631/0/29; los 4 de más son los tests nuevos de T16.
Los 60 archivos de ASTRA son, por paquete, `asset.json`, las 18 entradas de
`files` y el `kiln_generator.gd.uid` que ya estaba desde K012.

## Criterios de aceptación

| ID | Criterio | Estado | Evidencia |
|---|---|---|---|
| R1 | Construcción canónica: cuantizar, unir, descartar, recuento `2 + S·S/2` / `S·S` | hecho | `canonical::build::tests::canonical_sphere_8_gives_34_and_64`, `canonical_sphere_64_gives_2050_and_4096`, `canonical_count_rejects_wrong_segments`, `canonical_quantize_halves_away_from_zero` en `review/final-test.log`; medida sobre el GLB real en `decisions/001-glb-orientation.md` |
| R2 | Validador I1–I9 con enteros exactos, primer fallo con código y elemento | hecho | pruebas `canonical::topology::tests::*` y `canonical::radial::tests::*` en `review/final-test.log`; los 22 negativos de la tabla de abajo |
| R3 | Verify falla si una semilla no da geometría válida; el paso corre aunque fallen otros | hecho | `verify_recipe::tests::canonical_runs_on_every_seed_after_a_flipped_face`, `canonical_writes_an_artifact_and_its_hash_per_seed` en `review/final-test.log`; las recetas de cota ≤ 0 las para antes `generator_radius` (decisión 002) |
| R4 | Publish escribe `geometry/seed_<n>.bin` y `geometry_sha256` + procedencia; dos veces, mismos bytes | hecho | `ops::publish_recipe::tests::canonical_publish_writes_the_geometry_and_its_hashes`, `canonical_publish_twice_gives_the_same_bytes`, `canonical_publish_copies_the_godot_version`; en ASTRA, republicar no cambia ningún byte (`review/final-astra-sha-*.txt`) |
| R5 | Publish revalida todo antes de escribir; si uno falla, nada se publica | hecho | `ops::publish_recipe::tests::a_tampered_artefact_refuses_and_leaves_the_package_as_it_was` |
| R6 | 67 positivos y 22 negativos con el veredicto de M§8 | hecho, 89/89 | tabla de abajo; `review/final-ignored.log` (vías A y B), `review/final-test.log` (vía C, 17 pruebas de `canonical_cases.rs`) |
| R7 | Mismo `geometry_sha256` en x86-64, o por qué no | medido **solo emulado**: 48/48 iguales | `measurements-x86.md`; `review/evidence/t13/` |
| R8 | K012 sin cambios: `recipe.json`, `kiln_generator.gd`, `script_sha256`, consumidor y pruebas | hecho | `review/final-astra-asteroid.log`; diff de ASTRA limitado a `kiln.toml`, los tres `asset.json` y `geometry/` (`review/final-astra-status-*.txt`); sha de `recipe.json` y guion en `review/evidence/results/T12-result.md` |
| R9 | Sin la clave, verify y publish hacen lo de hoy | hecho; «`examples/` publica igual que antes» repetido el 2026-11-19 con bytes idénticos (`review/examples-regression.log`) | `config::tests::canonical_geometry_is_off_without_the_key`, `verify_recipe::tests::without_canonical_the_report_and_files_are_todays`, `ops::publish_recipe::tests::without_canonical_the_contract_has_no_provenance_keys`; ningún test existente cambiado (`review/final-kiln-diff-tracked.txt`) |
| R10 | GLB con transformación de nodo → rechazo con código propio | hecho | `mesh::tests::detects_a_transform_on_the_mesh_node`, `verify_recipe::tests::canonical_refuses_a_node_transform` (`canonical_node_transform`) |

## Los 89 casos (cierra I-4)

Vía A: receta → `generate.gd` (Godot 4.7.1 arm64) → GLB → `build` →
`validate`. Vía B: GLB real directo al validador, sin `build`. Vía C:
artefacto sintético o mutación de la semilla 42 de `asteroid_64` (fixture
`crates/kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin`, que es el
caso 17). Cuenta: 66 + 1 = 67 positivos; 2 + 5 + 15 = 22 negativos.

Fuente de las filas: resultados de T10, T11 y T6
(`review/evidence/results/`). Contraste con las corridas finales: las dos
pruebas de `godot_canonical` (vías A y B) y las 17 de `canonical_cases`
(vía C) afirman cada veredicto y pasan en `review/final-ignored.log` y
`review/final-test.log`; esas corridas no imprimen la tabla (sin
`--nocapture`). Los 48 hashes de la última columna son los de los
`asset.json` publicados, iguales a T10 (`review/final-hash-crosscheck.log`).

| # | Vía | Caso | Esperado (M§8) | Obtenido | `geometry_sha256` (publicado) |
|---|---|---|---|---|---|
| 1 | A | `pub_32_42` | ok | ok | `e8fe200d5c56fc60e5a993a80f991f2789a161b032969557b4e1ec9f53a25277` |
| 2 | A | `pub_32_1227191235` | ok | ok | `f4595b2ad019e50a24ff65f5c5c8fdae760a37306a9b13f7ba9c54abcda12fbc` |
| 3 | A | `pub_32_4096378883` | ok | ok | `6857cfe7e0c45e2ce2e4b87256c3313d8e536b0befb43ce4cc365410913fd55e` |
| 4 | A | `pub_32_621036976` | ok | ok | `531f6340cee5009c8c8f9bfbe2a49a221e3281b35e06aebe19788bd950cf84fe` |
| 5 | A | `pub_32_1846092899` | ok | ok | `a6fec5143aa28e43a0e84ab8ba610be53a2e638ea848b460c41d1a140d56a5d5` |
| 6 | A | `pub_32_1595708226` | ok | ok | `71a549d03cca4d9e4f8a6989ba798f60a57597b2b4b8fb4ad130208205a0ffb6` |
| 7 | A | `pub_32_3943211157` | ok | ok | `e4c0c9de2daa2dd2f2e9578f9531b1f953b3dd405cf77b8bcd94b635d9703b8a` |
| 8 | A | `pub_32_1967550902` | ok | ok | `62def660732c5bc651d7d57b2ed7349bcdda9bb554ffb5b393f5f407e852c015` |
| 9 | A | `pub_32_167205829` | ok | ok | `7c869df700b3eb7ae1eaf457a2332c8ee09c6b3359d1d8331f11d4ecc8fed362` |
| 10 | A | `pub_32_2394521605` | ok | ok | `666ba7129a6bb76b8650c5726fdae18dd001c95872e4d6264ed9578ac68d2f4d` |
| 11 | A | `pub_32_3890178997` | ok | ok | `dd9af15ed75770880b517863a124f7bf6406fb81b4dce921fca1ef12cdf949fd` |
| 12 | A | `pub_32_1319969106` | ok | ok | `13236d3f3b7532a8586f4ec83adf6399d60303618247d896e9d7c8ffb398aab5` |
| 13 | A | `pub_32_1859157860` | ok | ok | `5c2095ec8d5921490810b681e4c83593ca671eae93506051a45cc750aa48f375` |
| 14 | A | `pub_32_3665219471` | ok | ok | `1cf547f1826ed48f379cd07c5b111879f51bb3066b2deff58101e8965852b7ad` |
| 15 | A | `pub_32_2142325897` | ok | ok | `015dd044ab1467ab39acb2d1041a777042ac439cb311fdb7e7e3f873a5431e77` |
| 16 | A | `pub_32_303757059` | ok | ok | `9efd1fcdb1c66f048a97d2b1cd4154257a48fef92c7e413a3b831df19bc5c5d2` |
| 17 | A | `pub_64_42` | ok | ok | `2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a` |
| 18 | A | `pub_64_1227191235` | ok | ok | `4a0b556a06e3e23f5a62f9366a8b8aff9e11ebab4a8892a4d0bcfd1257141cec` |
| 19 | A | `pub_64_4096378883` | ok | ok | `449b8af226b4970835a29594316e2098fa36367cd3a1dc8d16b1cb09bae3c492` |
| 20 | A | `pub_64_621036976` | ok | ok | `de3be14769e10963c006709a11f59f1a2c5113a00f5132e45600e291c813b380` |
| 21 | A | `pub_64_1846092899` | ok | ok | `a656bc925cc4e78c4f8e79ba0301eb981b4546d0b8df9cf65b858ad322933386` |
| 22 | A | `pub_64_1595708226` | ok | ok | `6586286249bfb3ce3b4e0327d21c3085c3c7599f99b2c967f00afb2a51bf678f` |
| 23 | A | `pub_64_3943211157` | ok | ok | `c3bcc8f679173a5b665c7e05958948339be65959e2e3a01991d348f2fd76a005` |
| 24 | A | `pub_64_1967550902` | ok | ok | `de58846a4ca737100315843647b2cd35426a9ffe16f4a18b0a8bccbd255357a6` |
| 25 | A | `pub_64_167205829` | ok | ok | `d3b0905a51d2e191a2c4e9805986f0ccf699cd40e0786e0657ce5b7e0fba2efd` |
| 26 | A | `pub_64_2394521605` | ok | ok | `164d6fc0eeb5b37a59341b0d201bba06f94f777055fb05997119c5aac595ad37` |
| 27 | A | `pub_64_3890178997` | ok | ok | `0d5d258006c76b610972c798e12b1c835a673a136265e1d71d638fcce0ba489b` |
| 28 | A | `pub_64_1319969106` | ok | ok | `c1aab986fc75b896f4818b72993b6260ccb50ad7c0a314bef1788ee9080d9992` |
| 29 | A | `pub_64_1859157860` | ok | ok | `8336f399c3d3d7aeea0eebcfa37aadf90e508417f7a3cfd83a8ca2e3c5020325` |
| 30 | A | `pub_64_3665219471` | ok | ok | `986805572610c2ad21b3df241272720ea0113685fee474af5cc5470685bab7f5` |
| 31 | A | `pub_64_2142325897` | ok | ok | `d8083d1f1c3dfe697fac2553367b42fd834c898e0512561f11117dfdca37190d` |
| 32 | A | `pub_64_303757059` | ok | ok | `35ea7dfe6fd9aaceeb27235540f333a8cecc47014933662dbe8fc09640a4af38` |
| 33 | A | `pub_128_42` | ok | ok | `e25d73781f18361746f2fea48c641b83d429a64c959a15851c5e6fb5b5f16436` |
| 34 | A | `pub_128_1227191235` | ok | ok | `19b94000ba3a5138ef710b2c1ab649b3c6c4899079d825e6d9009232631dd105` |
| 35 | A | `pub_128_4096378883` | ok | ok | `ba8ce461e2dd6ef039218f6d616c535ceda87258af3e8427ca61cd755af665d5` |
| 36 | A | `pub_128_621036976` | ok | ok | `db02348d2ac11fc7ce33ef8202140d19b01b816f7105823331ffe8290ffe4baf` |
| 37 | A | `pub_128_1846092899` | ok | ok | `adc84a63de8f084771c2f1e63986abe2ededed5208dcae4034c19137f4ceb948` |
| 38 | A | `pub_128_1595708226` | ok | ok | `f91c47beb0c16b89e815ff8337f4f62dda98a54c422d0acf315d37ee7032e665` |
| 39 | A | `pub_128_3943211157` | ok | ok | `6ba858ee34894e963ed54b29c96962a800880e926abc7ab9db08e1b8585932d1` |
| 40 | A | `pub_128_1967550902` | ok | ok | `9f6a9894004aa9ff49b0fbd3ba2e3f400e1f6f9a29c1fee5607b8da743f243b0` |
| 41 | A | `pub_128_167205829` | ok | ok | `8b1777b94086e7e5ba73087b075b48cf28ba7c3414ba30f00fcf7d4bf88585aa` |
| 42 | A | `pub_128_2394521605` | ok | ok | `634b919fc2180a12924f8ed1d973695a8204ea6aaff4a958d2f92c532dac5ab3` |
| 43 | A | `pub_128_3890178997` | ok | ok | `1adef44dc3da59031b4f6fbf88fd8b2d761530c2f716478ec2eec159025149c9` |
| 44 | A | `pub_128_1319969106` | ok | ok | `5a4065df9706dc3a3207ed61e9b3e754fbeeeab1a4e5e4dbfa08d6ad640fde14` |
| 45 | A | `pub_128_1859157860` | ok | ok | `1343afcc2c99e79f710dbe4fa00765725069c11db399953fd7d167ecf4f4c951` |
| 46 | A | `pub_128_3665219471` | ok | ok | `acafe504c66c24cadec4c9d38d33abbbaaea39635b2cc25535c3a76c11e2dc5a` |
| 47 | A | `pub_128_2142325897` | ok | ok | `5f32b3ea75d0d0a4dbd13d9cbec45e4c5799db70282709932088e5c73709c6f8` |
| 48 | A | `pub_128_303757059` | ok | ok | `06a5b8792f0ea1afa05eabb589e44e47194309e8faef7c4ef81f9a5ef53331b1` |
| 49 | A | `scale_r10_s32` | ok | ok | — |
| 50 | A | `scale_r10_s64` | ok | ok | — |
| 51 | A | `scale_r10_s128` | ok | ok | — |
| 52 | A | `scale_r10_s256` | ok | ok | — |
| 53 | A | `scale_r100_s32` | ok | ok | — |
| 54 | A | `scale_r100_s64` | ok | ok | — |
| 55 | A | `scale_r100_s128` | ok | ok | — |
| 56 | A | `scale_r100_s256` | ok | ok | — |
| 57 | A | `scale_r1000_s32` | ok | ok | — |
| 58 | A | `scale_r1000_s64` | ok | ok | — |
| 59 | A | `scale_r1000_s128` | ok | ok | — |
| 60 | A | `scale_r1000_s256` | ok | ok | — |
| 61 | A | `scale_r10000_s32` | ok | ok | — |
| 62 | A | `scale_r10000_s64` | ok | ok | — |
| 63 | A | `scale_r10000_s128` | ok | ok | — |
| 64 | A | `scale_r10000_s256` | ok | ok | — |
| 65 | A | `max_r10000_k5_s256` | ok | ok | — |
| 66 | A | `edge_y05_a045` | ok | ok | — |
| 67 | A | `neg_k02_a1x3` | `I8_not_radial` | `I8_not_radial` | — |
| 68 | A | `neg_y02_a06` | `I8_not_radial` | `I8_not_radial` | — |
| 69 | B | arrays suaves sin preparar (semilla 42, 64 seg., sin `facet`) | `I3_duplicate_position` | `I3_duplicate_position` | — |
| 70 | B | malla facetada sin compartir (semilla 42, con `facet`) | `I3_duplicate_position` | `I3_duplicate_position` | — |
| 71 | B | retícula sin soldar (esfera base r 1000 m, 64 seg.) [1] | `I3_duplicate_position` | `I3_duplicate_position` | — |
| 72 | B | esfera base soldada bit a bit (2210 → 2056 V) | `I3_duplicate_position` | `I3_duplicate_position` | — |
| 73 | B | orden de triángulos de Godot (semilla 42, giro invertido) [2] | `I8_not_radial` | `I8_not_radial` | — |
| 74 | C | Tetraedro centrado (sintético) | ok | ok | — |
| 75 | C | Un triángulo quitado (real, sin el último) | `I4_open_edge` | `I4_open_edge` | — |
| 76 | C | Un triángulo invertido (real, tri 100 con swap(1,2)) | `I5_inconsistent_orientation` | `I5_inconsistent_orientation` | — |
| 77 | C | Un triángulo repetido (real, tri 100 añadido) | `I4_edge_shared_by_more_than_two` | `I4_edge_shared_by_more_than_two` | — |
| 78 | C | Dos vértices en el mismo punto (real, v500 = v1000) | `I3_duplicate_position` (elem 1000) | `I3_duplicate_position` (elem 1000) | — |
| 79 | C | Coordenada fuera del límite (real, v7.y = 2^30) | `I1_out_of_range` (elem 7) | `I1_out_of_range` (elem 7) | — |
| 80 | C | Índice fuera de los vértices (real, tri 10 → V) | `I2_index_out_of_range` (elem 10) | `I2_index_out_of_range` (elem 10) | — |
| 81 | C | Vértice sin usar (real, vértice añadido) | `I3_unused_vertex` (elem 2050) | `I3_unused_vertex` (elem 2050) | — |
| 82 | C | Dos vértices lejanos fundidos (sintético, uv_sphere(3,6) con polos fundidos) | `I6_pinched_vertex` | `I6_pinched_vertex` | — |
| 83 | C | Segundo cuerpo dentro (sintético, octaedro + octaedro/10) | `I7_not_connected` | `I7_not_connected` | — |
| 84 | C | Tetraedro que no contiene el centro (sintético, +3000 en x) | `I8_not_radial` | `I8_not_radial` | — |
| 85 | C | Voladizo (real, a' = b+c−a en el tri 100) | `I8_not_radial` | `I8_not_radial` | — |
| 86 | C | Vértice a través del centro (real, a' = −a) | `I8_not_radial` | `I8_not_radial` | — |
| 87 | C | Toro (sintético, torus(3,4)) | `I7_not_a_sphere` | `I7_not_a_sphere` | — |
| 88 | C | Vacía (sintético) | `I1_too_small` | `I1_too_small` | — |
| 89 | C | Doble vuelta (226 V, 448 T) (sintético, port de `a7_extra.py`) | `I9_covers_a_direction_2_times` | `I9_covers_a_direction_2_times` | — |

- **[1] Desviación de origen, sin cambio de veredicto.** M construye la
  retícula sin soldar con la semilla 42; por GLB esa entrada es byte a byte
  la del caso 69 (el GLB sin `facet` conserva el orden de la retícula), así
  que el 71 sale de la esfera base. Si se prefiere el mismo origen que M, el
  71 duplica el 69.
- **[2]** El GLB ya sale hacia fuera (decisión 001); este negativo se
  construye invirtiendo cada triángulo de la semilla 42 tras un `build` que da
  `ok`.
- Los 18 positivos derivados (49–66) no tienen hash publicado: no son
  paquetes. Las pruebas de la vía A afirman además que `pub_64_42` da
  `2713c0a9…d78a`, el hash de la fixture.

## Qué cambia en Kiln

Rama `k012-asteroid-pilot`, unida a `main` (PR #33).

- **Nuevo, `kiln-core::canonical`** (`crates/kiln-core/src/canonical/`):
  `mod.rs` (tipos, bytes S1 §6.2, `geometry_sha256`, `validate` = I1–I7 y
  luego I8–I9), `build.rs` (cuantizar, unir por primera aparición, descartar,
  recuento), `topology.rs` (I1–I7), `radial.rs` (I8–I9 con `i128` y
  perturbación simbólica). Sin E/S ni dependencias nuevas.
- **Cambiado**: `kiln-core/src/config.rs` (`[generate] canonical`,
  `CanonicalClass::RadialV0`), `mesh.rs` (`node_transform`),
  `generate/check.rs` (`canonical`, impares), `lib.rs`;
  `kiln-cli/src/verify_recipe.rs` (artefacto por semilla en
  `kiln/cache/generated/<id>/verify/<seed>.canonical.bin`, `geometry_sha256`
  o rechazo en `validation.json`, `recipe.godot_version`) y
  `kiln-cli/src/ops/publish_recipe.rs` (revalidar todo, copiar
  `geometry/seed_<n>.bin`, `asset.json` el último, `prune_geometry`).
  812 líneas añadidas y 13 quitadas en los seis archivos, tests incluidos.
- **Pruebas nuevas**: `kiln-core/tests/canonical_cases.rs` (vía C, 17),
  fixture `kiln-core/tests/fixtures/canonical/asteroid_64_seed42.bin`
  (73 784 B) con su `README.md`, `kiln-core/examples/canonical_fixture.rs`
  (la regenera), `kiln-cli/tests/godot_canonical.rs` (vías A y B,
  `#[ignore]`) y `kiln-cli/tests/common/canonical.rs`.
- **Códigos nuevos**:
  - I1–I9 (M§8): `I1_too_small`, `I1_out_of_range`, `I2_index_out_of_range`,
    `I2_repeated_index`, `I3_duplicate_position`, `I3_unused_vertex`,
    `I4_open_edge`, `I4_edge_shared_by_more_than_two`,
    `I5_inconsistent_orientation`, `I6_pinched_vertex`, `I7_not_connected`,
    `I7_not_a_sphere`, `I8_not_radial`, `I9_covers_a_direction_<2..9>_times`
    y `I9_covers_a_direction_10_or_more_times`.
  - Construcción y archivo: `canonical_count`, `canonical_quantize`,
    `canonical_header`, `canonical_size`. Un valor desconocido de
    `[generate] canonical` es error de configuración al leer `kiln.toml`.
  - Verify: `canonical_node_transform` (R10), `canonical_mesh_count`,
    `canonical_odd_segments` (decisión 003), `canonical_engine_mismatch`,
    `canonical_engine_missing`.
- `CLAUDE.md`: recuentos de la batería y la orden de las ignoradas con
  `KILN_CANONICAL_RECIPES`.

## Qué cambia en ASTRA

Rama `k012-asteroid-pilot`, unida a `main` (PR #22).

- `kiln.toml`: sección nueva al final, `[generate] canonical = "radial_v0"`
  (+4 líneas). `verify_seeds` sigue por defecto: 16 semillas desde la 42.
- `godot/assets/kiln/asteroid_{32,64,128}/asset.json`: gana `files`
  (18 entradas: `recipe.json`, `kiln_generator.gd` y 16 `geometry/seed_<n>.bin`),
  `geometry` (16 × `{seed, file, geometry_sha256}`), `kiln_version` `"0.1.0"` y
  `godot_version` `"4.7.1-stable (official)"`. `version` sigue en 1 (D4); el
  resto de campos no cambia.
- 48 archivos nuevos `geometry/seed_<n>.bin`: 18 488 B (32 seg.), 73 784 B
  (64) y 294 968 B (128) cada uno, `32 + 12V + 12T`. Total 6 195 840 B
  (6,2 MB).
- `recipe.json`, `kiln_generator.gd` (`3c5ae7c6…ca36f0`), el consumidor
  `godot/procedural/` y `godot/tests/asteroid_*` sin cambios.

## Hashes: arm64 frente a x86-64 EMULADO

Detalle en `measurements-x86.md`; datos en `review/evidence/t13/`
(`arm64-geom.txt`, `amd64-geom.txt`, `glb-sha.txt`, `amd64-env.txt`,
`amd64-run.txt`, `scripts/`, `hasher/`).

- **48 de 48 `geometry_sha256` iguales**, y los 48 GLB idénticos byte a byte.
- Lado x86-64: binario oficial `Godot_v4.7.1-stable_linux.x86_64` (SHA-512
  comprobado), `debian:bookworm-slim` amd64 con glibc 2.36, en Docker Desktop
  con **Rosetta** (comprobado por `/proc/self/maps` y `vendor_id:
  VirtualApple`).
- **Prueba**: el código máquina x86-64 de Godot, con el `libm` de glibc, da los
  mismos bytes que el de macOS arm64.
- **No prueba**: una CPU x86-64 física (no se ha comprobado la fidelidad de
  Rosetta en coma flotante), otros `libm` (otra glibc, musl, Windows, macOS
  Intel), los builds de Godot para Windows o macOS x86_64, ni otras versiones
  de Godot.

## Decisiones

| # | Decisión | Quién |
|---|---|---|
| 001 | El GLB de `generate.gd` ya sale hacia fuera; la construcción no invierte el giro. Posiciones `f32` bit a bit, sin transformación de nodo. El hash depende del orden de índices del exportador | medición T1 |
| 002 | Se mantiene la regla estática `generator_radius`; las recetas de cota ≤ 0 no llegan a la vía canónica en verify. Su `I8_not_radial` se prueba en la vía A. Pendiente: corregir S1 §4.2 | desarrollador |
| 003 | Con la clave, `sphere.segments` debe ser par y fijo (`canonical_odd_segments` antes de Godot). Pendiente: anotarlo en S1 §2.2 | desarrollador |

Además, de la spec: D3 (archivo = bytes del hash, `class` radial = 0), D4
(republicar sin subir `version`), D5 (x86-64 emulado y declarado) y D6
(fixtures de los casos no publicados en la batería de Kiln).

## Revisión

T15, tres pasadas (fallos, seguridad, conformidad). Fuente: *Review* de
`plan.md` y `review/evidence/results/T16-result.md`; el artefacto completo
de T15 no se escribió en disco. `review/findings.md` lo reconstruye desde
esas fuentes y esta tabla.

| Hallazgo | Qué era | Cómo se cerró |
|---|---|---|
| I-1 | Con S impar, el recuento de S1 §2.2 no se cumple y verify acababa en `canonical_count` tras lanzar Godot (`review/evidence/odd-segments.log`) | Decisión 003, T16: `canonical_odd_segments` antes de Godot. Pruebas `canonical_refuses_segments_that_can_be_odd`, `canonical_refuses_odd_segments_before_godot` |
| I-2 | Republicar dejaba `geometry/seed_<n>.bin` de semillas que ya no estaban en el lote (`review/evidence/stale-geometry.log`) | T16: `prune_geometry`, solo `seed_<dígitos>.bin` fuera de `files`, sin seguir enlaces. Prueba `a_republish_removes_only_the_geometry_of_seeds_no_longer_verified` |
| I-3 | La evidencia de T14 (631/0/29) no estaba en disco | Esta tarea: `review/final-*.log` |
| I-4 | Faltaba la tabla consolidada de los 89 casos | Esta tarea: tabla de arriba |
| I-5 | `work/T13-scratch/` de 322 MB sin ignorar | **Cerrado.** Lo citado está copiado a `review/evidence/` y `work/` ya no está en disco (ver *Estado Git*) |
| nit 1 | Lote en el que ninguna semilla trae versión de Godot (la cadena solo registra el arreglo, no el comportamiento previo) | T16: `canonical_engine_missing`. Prueba `canonical_refuses_a_batch_without_a_godot_version` |
| seguridad | Sin hallazgos importantes; fuzzing sin pánicos en `from_bytes`, `validate`, `build` y `node_transform_bytes` | — |

Logs de T16 (rojo y verde): `review/evidence/t16/`. Otros nits de T15, si los
hubo, no constan en la cadena.

**Sin confirmar**:
- ~~El flujo vecino de T14 «publicar `examples/` igual que antes»~~
  Repetido el 2026-11-19, tras unir K013. Se usó el flujo de
  `crates/kiln-cli/tests/godot_verify.rs`: una copia de `examples/` con
  `[publish] dir`, sobre la que se corrieron build, verify, publish y un
  segundo publish. Los archivos publicados de `hoist` tienen el mismo sha256
  con el binario de `2651608` (antes de K013) y con el de `bd608df`. Los 37
  archivos de `examples/` y `git status` no cambian. `examples/kiln.toml` no
  declara `[publish]`, así que no se puede publicar en su sitio, ni con K013
  ni sin él. Evidencia: `review/examples-regression.log` y
  `review/examples-regression-{before,after}.txt`.
- Los 23 avisos de `kiln_check.sh` no se compararon con los de antes de
  activar la clave. Ninguno menciona `generate`, `canonical` ni `asteroid`.
- `kiln generate` y `spec::check` no aplican la regla de impares (fuera del
  encargo de T16; verify es la puerta).

## Lo que queda abierto

- **x86-64 nativo**: repetir T13 en un equipo x86-64 real. Sirve el mismo
  `docker run --platform linux/amd64` de `measurements-x86.md`, con
  `review/evidence/t13/scripts/` y `hasher/`.
- **Correcciones de S1**: hecho. §4.2 (decisión 002) y §2.2 (decisión 003)
  se anotaron en ASTRA (`docs/design/ast-esp-001-s1-geometria-canonica-v0.md`)
  el 2026-11-19.
- **Límites de R7**: los de *No prueba*; la medida cubre las 48 semillas
  publicadas, no las 18 derivadas de escala.
- **El hash depende de Godot**: `geometry_sha256` depende de cómo el
  exportador de Godot 4.7.1 escribe los índices. Decisión 001 deduce, sin
  haberlo medido, que con y sin `facet` la misma geometría daría hashes
  distintos (cada triángulo empieza en otro vértice). Otra versión de Godot
  puede cambiarlo; `asset.json` lleva `godot_version` para eso, y verify
  rechaza lotes con motores mezclados o sin versión.
- **Tarea 2 de S1 §10**, Rust de ASTRA: cargar el artefacto, I1–I9 al
  cargar, Q1–Q3 y si Q2 pide más de 128 bits.
- **Tarea 3 de S1 §10**, Godot: derivación identidad desde el artefacto.

## Estado Git

- **K013 confirmado y unido a `main`** en los dos repos: Kiln PR #33 y ASTRA
  PR #22 (unidos el 2026-11-19).
- **ASTRA**: siguen sin seguimiento y sin tocar
  `.sdlc/canonical-geometry/measure/__pycache__/` y un PDF ajeno de
  `.sdlc/first-character/`.
- **`work/` borrado.** Lo que la cadena citaba allí está en
  `review/evidence/`: `work/T13-scratch/*` → `review/evidence/t13/` (sin
  Godot, el zip ni los árboles de trabajos, que se regeneran);
  `work/T1-scratch/` → `review/evidence/t1/` (sin `out/`, que se regenera);
  `work/T16-*.log` → `review/evidence/t16/`; `work/T*-result.md` y
  `T*-measurement.md` → `review/evidence/results/`. `measurements-x86.md`,
  `decisions/001` y `002`, `context.md` y `plan.md` ya citan las rutas de
  `review/evidence/`.

## Siguiente paso, cuando se decida

Ninguno se inicia solo: medir en x86-64 nativo y, después, la tarea 2
(Rust de ASTRA).
