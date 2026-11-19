---
task: k013-canonical-geometry/T10
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cd ~/Kiln && KILN_CANONICAL_RECIPES=~/.t3/projects/astra/godot/assets/kiln cargo test -p kiln-cli --test godot_canonical -- --ignored --nocapture 2>&1 | tee /tmp/claudio-test-k013-canonical-geometry-T10b.log"
persist: propose
files: [/Users/acastells/Kiln/crates/kiln-cli/tests/godot_canonical.rs, /Users/acastells/Kiln/crates/kiln-cli/tests/common/canonical.rs]
evidence: [crates/kiln-cli/tests/godot_canonical.rs:29-85, crates/kiln-cli/tests/godot_canonical.rs:87-112, crates/kiln-cli/tests/common/canonical.rs:25-74, /tmp/claudio-test-k013-canonical-geometry-T10b.log, /tmp/claudio-clippy-k013-T10b.log]
---

## Summary

- Vía A en verde: los 68 veredictos coinciden con M§8 (66 `ok` y 2 `I8_not_radial`), sin ajustar ningún caso. Una sola prueba `#[ignore]` recorre los 68 y hace el assert al final (`godot_canonical.rs:87-112`, log en /tmp/claudio-test-k013-canonical-geometry-T10b.log).
- Control con respuesta conocida: `pub_64_42` da `2713c0a9…d78a`, el mismo hash que el fixture de T1, y un assert lo comprueba. Los 48 `geometry_sha256` publicados son distintos entre sí; la tabla de abajo los recoge para T13.
- Tiempo de reloj: 5,0 s de prueba (6 hilos, 68 ejecuciones de Godot 4.7.1 arm64 sin ventana). Dos ejecuciones, antes y después de partir los helpers, dan tablas idénticas línea a línea.
- No hubo rojo previo, porque el código bajo prueba ya existía (T2–T8). Hacen de control el hash conocido y los dos negativos.
- Clippy (`-p kiln-cli --all-targets -D warnings`) volvió a comprobar `kiln-cli` y salió limpio: /tmp/claudio-clippy-k013-T10b.log. Durante la tarea, `publish_recipe.rs` (T9, en paralelo) dio avisos de compilación. No lo toqué.
- Helpers en `tests/common/canonical.rs`: el lanzador de `generate.gd`, `verdict` y `run_all`. Se enlaza con `#[path]`, así que `common/mod.rs` no cambia. T11 puede reutilizarlo.
- **Para T14:** hace falta `KILN_CANONICAL_RECIPES=<ASTRA>/godot/assets/kiln`; si falta, la prueba se detiene con un panic que lo explica. Kiln no lleva rutas de ASTRA y las recetas no se copian. Para ver la tabla, `--nocapture`.

## Detail

Cómo es la vía. `kiln-cli` solo tiene `[[bin]]`, así que `run_seed` no se puede llamar desde `tests/`. Por eso se reproduce la misma forma de trabajo: `include_str!` de `godot/generate.gd` y `godot/kiln_generator.gd`, el mismo `project.godot` que `SCRATCH_PROJECT` y un job `{id, glb, material, ops}`, con `material` tomado de cada semilla (en las 48 es `null`). Godot se lanza con `--headless --path <work> --script generate.gd -- job.json out.json`. El GLB pasa las mismas comprobaciones que `verify_recipe.rs::canonical`: `node_transform == None`, una sola malla, `build(positions, indices, segments)` y `validate`. Cada caso tiene su propio proyecto dentro de un `TempDir` que se borra al terminar, y ningún GLB queda en el repo.

Los casos derivados se toman de `asteroid_64` semilla 42 como en ASTRA `measure/dump.gd`, pero conservan `facet`. Cada uno cambia solo la esfera (radio y segmentos), el stretch y las amplitudes. Los dos negativos llevan un comentario que explica por qué saltan a propósito la regla estática `generator_radius`.

Aprendido:
- Learned: 68 ejecuciones de Godot sin ventana en 6 hilos tardan unos 4 s. Que cada caso tenga su proyecto evita carreras en `.godot/`.
- Learned: `scale_r10_s256` y `max_r10000_k5_s256` pasan a 1 mm sin desbordar, como dice M§4.
- Learned: como `kiln-cli` no tiene lib, las pruebas de integración solo llegan a su lógica por el binario o copiando la forma del job.

| case | expected (M§8) | got | geometry_sha256 |
|---|---|---|---|
| pub_32_42 | ok | ok | e8fe200d5c56fc60e5a993a80f991f2789a161b032969557b4e1ec9f53a25277 |
| pub_32_1227191235 | ok | ok | f4595b2ad019e50a24ff65f5c5c8fdae760a37306a9b13f7ba9c54abcda12fbc |
| pub_32_4096378883 | ok | ok | 6857cfe7e0c45e2ce2e4b87256c3313d8e536b0befb43ce4cc365410913fd55e |
| pub_32_621036976 | ok | ok | 531f6340cee5009c8c8f9bfbe2a49a221e3281b35e06aebe19788bd950cf84fe |
| pub_32_1846092899 | ok | ok | a6fec5143aa28e43a0e84ab8ba610be53a2e638ea848b460c41d1a140d56a5d5 |
| pub_32_1595708226 | ok | ok | 71a549d03cca4d9e4f8a6989ba798f60a57597b2b4b8fb4ad130208205a0ffb6 |
| pub_32_3943211157 | ok | ok | e4c0c9de2daa2dd2f2e9578f9531b1f953b3dd405cf77b8bcd94b635d9703b8a |
| pub_32_1967550902 | ok | ok | 62def660732c5bc651d7d57b2ed7349bcdda9bb554ffb5b393f5f407e852c015 |
| pub_32_167205829 | ok | ok | 7c869df700b3eb7ae1eaf457a2332c8ee09c6b3359d1d8331f11d4ecc8fed362 |
| pub_32_2394521605 | ok | ok | 666ba7129a6bb76b8650c5726fdae18dd001c95872e4d6264ed9578ac68d2f4d |
| pub_32_3890178997 | ok | ok | dd9af15ed75770880b517863a124f7bf6406fb81b4dce921fca1ef12cdf949fd |
| pub_32_1319969106 | ok | ok | 13236d3f3b7532a8586f4ec83adf6399d60303618247d896e9d7c8ffb398aab5 |
| pub_32_1859157860 | ok | ok | 5c2095ec8d5921490810b681e4c83593ca671eae93506051a45cc750aa48f375 |
| pub_32_3665219471 | ok | ok | 1cf547f1826ed48f379cd07c5b111879f51bb3066b2deff58101e8965852b7ad |
| pub_32_2142325897 | ok | ok | 015dd044ab1467ab39acb2d1041a777042ac439cb311fdb7e7e3f873a5431e77 |
| pub_32_303757059 | ok | ok | 9efd1fcdb1c66f048a97d2b1cd4154257a48fef92c7e413a3b831df19bc5c5d2 |
| pub_64_42 | ok | ok | 2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a |
| pub_64_1227191235 | ok | ok | 4a0b556a06e3e23f5a62f9366a8b8aff9e11ebab4a8892a4d0bcfd1257141cec |
| pub_64_4096378883 | ok | ok | 449b8af226b4970835a29594316e2098fa36367cd3a1dc8d16b1cb09bae3c492 |
| pub_64_621036976 | ok | ok | de3be14769e10963c006709a11f59f1a2c5113a00f5132e45600e291c813b380 |
| pub_64_1846092899 | ok | ok | a656bc925cc4e78c4f8e79ba0301eb981b4546d0b8df9cf65b858ad322933386 |
| pub_64_1595708226 | ok | ok | 6586286249bfb3ce3b4e0327d21c3085c3c7599f99b2c967f00afb2a51bf678f |
| pub_64_3943211157 | ok | ok | c3bcc8f679173a5b665c7e05958948339be65959e2e3a01991d348f2fd76a005 |
| pub_64_1967550902 | ok | ok | de58846a4ca737100315843647b2cd35426a9ffe16f4a18b0a8bccbd255357a6 |
| pub_64_167205829 | ok | ok | d3b0905a51d2e191a2c4e9805986f0ccf699cd40e0786e0657ce5b7e0fba2efd |
| pub_64_2394521605 | ok | ok | 164d6fc0eeb5b37a59341b0d201bba06f94f777055fb05997119c5aac595ad37 |
| pub_64_3890178997 | ok | ok | 0d5d258006c76b610972c798e12b1c835a673a136265e1d71d638fcce0ba489b |
| pub_64_1319969106 | ok | ok | c1aab986fc75b896f4818b72993b6260ccb50ad7c0a314bef1788ee9080d9992 |
| pub_64_1859157860 | ok | ok | 8336f399c3d3d7aeea0eebcfa37aadf90e508417f7a3cfd83a8ca2e3c5020325 |
| pub_64_3665219471 | ok | ok | 986805572610c2ad21b3df241272720ea0113685fee474af5cc5470685bab7f5 |
| pub_64_2142325897 | ok | ok | d8083d1f1c3dfe697fac2553367b42fd834c898e0512561f11117dfdca37190d |
| pub_64_303757059 | ok | ok | 35ea7dfe6fd9aaceeb27235540f333a8cecc47014933662dbe8fc09640a4af38 |
| pub_128_42 | ok | ok | e25d73781f18361746f2fea48c641b83d429a64c959a15851c5e6fb5b5f16436 |
| pub_128_1227191235 | ok | ok | 19b94000ba3a5138ef710b2c1ab649b3c6c4899079d825e6d9009232631dd105 |
| pub_128_4096378883 | ok | ok | ba8ce461e2dd6ef039218f6d616c535ceda87258af3e8427ca61cd755af665d5 |
| pub_128_621036976 | ok | ok | db02348d2ac11fc7ce33ef8202140d19b01b816f7105823331ffe8290ffe4baf |
| pub_128_1846092899 | ok | ok | adc84a63de8f084771c2f1e63986abe2ededed5208dcae4034c19137f4ceb948 |
| pub_128_1595708226 | ok | ok | f91c47beb0c16b89e815ff8337f4f62dda98a54c422d0acf315d37ee7032e665 |
| pub_128_3943211157 | ok | ok | 6ba858ee34894e963ed54b29c96962a800880e926abc7ab9db08e1b8585932d1 |
| pub_128_1967550902 | ok | ok | 9f6a9894004aa9ff49b0fbd3ba2e3f400e1f6f9a29c1fee5607b8da743f243b0 |
| pub_128_167205829 | ok | ok | 8b1777b94086e7e5ba73087b075b48cf28ba7c3414ba30f00fcf7d4bf88585aa |
| pub_128_2394521605 | ok | ok | 634b919fc2180a12924f8ed1d973695a8204ea6aaff4a958d2f92c532dac5ab3 |
| pub_128_3890178997 | ok | ok | 1adef44dc3da59031b4f6fbf88fd8b2d761530c2f716478ec2eec159025149c9 |
| pub_128_1319969106 | ok | ok | 5a4065df9706dc3a3207ed61e9b3e754fbeeeab1a4e5e4dbfa08d6ad640fde14 |
| pub_128_1859157860 | ok | ok | 1343afcc2c99e79f710dbe4fa00765725069c11db399953fd7d167ecf4f4c951 |
| pub_128_3665219471 | ok | ok | acafe504c66c24cadec4c9d38d33abbbaaea39635b2cc25535c3a76c11e2dc5a |
| pub_128_2142325897 | ok | ok | 5f32b3ea75d0d0a4dbd13d9cbec45e4c5799db70282709932088e5c73709c6f8 |
| pub_128_303757059 | ok | ok | 06a5b8792f0ea1afa05eabb589e44e47194309e8faef7c4ef81f9a5ef53331b1 |
| scale_r10_s32 | ok | ok |  |
| scale_r10_s64 | ok | ok |  |
| scale_r10_s128 | ok | ok |  |
| scale_r10_s256 | ok | ok |  |
| scale_r100_s32 | ok | ok |  |
| scale_r100_s64 | ok | ok |  |
| scale_r100_s128 | ok | ok |  |
| scale_r100_s256 | ok | ok |  |
| scale_r1000_s32 | ok | ok |  |
| scale_r1000_s64 | ok | ok |  |
| scale_r1000_s128 | ok | ok |  |
| scale_r1000_s256 | ok | ok |  |
| scale_r10000_s32 | ok | ok |  |
| scale_r10000_s64 | ok | ok |  |
| scale_r10000_s128 | ok | ok |  |
| scale_r10000_s256 | ok | ok |  |
| max_r10000_k5_s256 | ok | ok |  |
| edge_y05_a045 | ok | ok |  |
| neg_k02_a1x3 | I8_not_radial | I8_not_radial |  |
| neg_y02_a06 | I8_not_radial | I8_not_radial |  |
