# Medición R7 / D5: `geometry_sha256` en arm64 nativo y en x86-64 EMULADO

Tarea T13 de K013, medida el 2026-11-19. **Todo lo que esta página llama «x86-64» es x86-64 EMULADO con Rosetta dentro de Docker Desktop, sobre un Mac arm64. No es un equipo x86-64 nativo.**

## Resultado

- `geometry_sha256`: **48 de 48 iguales** entre arm64 nativo y x86-64 emulado. Las 48 semillas publicadas son 16 de `asteroid_32`, 16 de `asteroid_64` y 16 de `asteroid_128`.
- GLB: **48 de 48 idénticos byte a byte**, con el mismo SHA-256 del archivo en los dos lados. Por tanto también coinciden bit a bit los `f32` de las posiciones, los índices y el chunk JSON, y no solo la geometría cuantizada a 1 mm.
- El lado arm64 reproduce línea a línea la tabla de `work/T10-result.md`: 48 de 48. `pub_64_42` = `2713c0a9…d78a`, igual que el fixture.
- Todas las ejecuciones terminaron con código de salida 0 y produjeron su GLB. Las 48 pasan `canonical::validate` en los dos lados.

## Entornos

| | arm64 (nativo) | x86-64 EMULADO |
|---|---|---|
| Máquina | MacBook Pro, Apple Silicon (kernel `RELEASE_ARM64_T6041`), macOS 27.0 (26A428) | El mismo Mac. Docker Desktop: Engine 29.9.2, VM `linuxkit` 7.0.14, `linux/arm64` |
| Contenedor | — | `debian:bookworm-slim`, `--platform linux/amd64`, id de imagen amd64 `sha256:c67e92c8…af018`. `uname -m` = `x86_64`, glibc 2.36-9+deb12u14 |
| Emulador | ninguno | **Rosetta**. `/run/rosetta/rosetta` aparece mapeado en `/proc/self/maps` de un proceso amd64 del contenedor, y `/proc/cpuinfo` dice `vendor_id: VirtualApple`. No es QEMU: en los mapas no hay `qemu`. Todo queda en `amd64-env.txt` |
| Bibliotecas de Godot | sistema de macOS | `ldd`: `libm.so.6`, `libc.so.6`, `librt`, `libpthread` y `libdl` de `/lib/x86_64-linux-gnu`, es decir, glibc |
| Banderas de CPU visibles | — | sse4_2, avx, avx2, fma, f16c, bmi1/2, entre otras (`/proc/cpuinfo` dentro del contenedor) |
| Godot | `/opt/homebrew/bin/godot` 4.7.1.stable.official.a13da4feb (macOS, arm64) | `Godot_v4.7.1-stable_linux.x86_64` 4.7.1.stable.official.a13da4feb, de la release oficial `godotengine/godot` `4.7.1-stable` en GitHub |
| Integridad del binario | Homebrew | SHA-512 del zip `4ccdab7a…945a2de9`, igual al de `SHA512-SUMS.txt` de esa misma release |
| Modo | `--headless`, sin ventana | `--headless`, sin ventana. Falta `libfontconfig.so.1` en la imagen: Godot lo avisa y sigue. No se instaló nada |
| Hash canónico | En el host, con `kiln_core::canonical` (`build` + `validate` + `to_bytes` + `geometry_sha256`) | El mismo binario del host, sobre los GLB copiados del contenedor (volumen montado) |

En `amd64-env.txt` hay dos salidas que confunden:
- Un primer `ls /run/rosetta` falla. La ruta no es visible como archivo dentro del contenedor, pero el traductor sí aparece mapeado en los procesos.
- Un primer `docker image inspect` sin `--platform` da `arm64`, porque muestra la variante por defecto del índice multiplataforma. Con `--platform linux/amd64` da `amd64`, y dentro del contenedor `uname -m` es `x86_64`.

## Cómo se midió

1. `work/T13-scratch/scripts/make_jobs.py` crea una carpeta por semilla con la misma forma que `tests/common/canonical.rs`: `project.godot` igual que `SCRATCH_PROJECT`, `generate.gd` y `kiln_generator.gd` de `crates/kiln-cli/godot/`, y `job.json` con `{id, glb, material, ops}` tomado de cada semilla de `recipe.json`. Hay dos árboles: `jobs-arm64` (rutas del host) y `jobs-amd64` (rutas `/jobs`).
2. arm64: `godot --headless --path <caso> --script generate.gd -- job.json out.json` para los 48 casos, 6 en paralelo.
3. x86-64 emulado: un único `docker run --platform linux/amd64` monta el binario oficial (solo lectura) y `jobs-amd64`, y ejecuta `scripts/run_amd64.sh`: los mismos 48, 6 en paralelo, con `timeout 3500`.
4. En el host, `work/T13-scratch/hasher` (proyecto cargo desechable, fuera del workspace de Kiln, que depende de `kiln-core` por ruta) comprueba `node_transform == None` y que hay una sola malla, y aplica `build(positions, indices, segments)`, `validate`, `to_bytes` y `geometry_sha256`. `shasum -a 256` da el hash de cada GLB.

## Tiempos de reloj

| Lado | 48 semillas | Nota |
|---|---|---|
| arm64 nativo | 3 s | 6 en paralelo |
| x86-64 emulado | 11 s (dentro del contenedor y en el host) | 6 en paralelo. Una sola semilla `asteroid_128` en frío tarda 1,8 s |

Muy por debajo del tope de 60 minutos.

## Qué prueba y qué no

**Prueba:**
- El binario oficial `linux.x86_64` de Godot 4.7.1 genera, a partir de las mismas recetas, GLB idénticos byte a byte a los del binario macOS arm64, y por tanto el mismo `geometry_sha256`. Lo que se ejecuta es su código máquina x86-64 compilado, enlazado con el `libm.so.6` de glibc 2.36 (lo muestra `ldd`), no con el `libm` de Apple. Este punto no lo invalida la emulación: el código x86-64 y su biblioteca matemática son los que corren, traducidos. No he comprobado qué instrucciones de coma flotante usa ese binario.
- El código canónico de Kiln (hash en el host) da los mismos 48 valores que la vía A de T10.

**No prueba:**
- Lo que haría una CPU x86-64 física. Rosetta traduce a arm64. Si su traducción de alguna instrucción de coma flotante (FMA, redondeos, x87, denormales) se apartara de la de Intel o AMD, aquí no se vería. No he comprobado la fidelidad de Rosetta en coma flotante contra hardware real ni contra documentación.
- Otros `libm`: otra glibc, musl, el CRT de Windows o el `libm` de macOS en Intel.
- Los builds de Godot para Windows o para macOS x86_64, otras versiones de Godot y los builds compilados a mano.
- Que los 48 casos cubran todas las recetas posibles: son las semillas publicadas, no las derivadas de escala de T10.

**Si algún día hubiera diferido**, esta medida no habría permitido separar la causa: arquitectura, sistema o `libm`, o el emulador. Como el resultado es igualdad total, esa ambigüedad no se presenta.

**Para cerrar R7 del todo** falta repetirlo en un equipo x86-64 nativo (D5: «si tienes un equipo x86-64 real, se mide además allí»). Allí sirve sin cambios el mismo `docker run --platform linux/amd64` de «Reproducir», que en un host x86-64 corre nativo. `run_amd64.sh` y los `job.json` usan las rutas `/godot` y `/jobs`, así que sin Docker habría que regenerar los trabajos con otra raíz. Después, el mismo hasher sobre los GLB.

## Reproducir

```
S=~/Kiln/.sdlc/k013-canonical-geometry/work/T13-scratch
python3 -I $S/scripts/make_jobs.py ~/Kiln ~/.t3/projects/astra/godot/assets/kiln $S/jobs-arm64 $S/jobs-arm64
python3 -I $S/scripts/make_jobs.py ~/Kiln ~/.t3/projects/astra/godot/assets/kiln $S/jobs-amd64 /jobs
# arm64 nativo (host)
cd $S/jobs-arm64 && cut -d' ' -f1 cases.txt | xargs -P6 -I{} sh -c 'godot --headless --path "$1/{}" --script "$1/{}/generate.gd" -- "$1/{}/job.json" "$1/{}/out.json" > "$1/{}/godot.log" 2>&1' _ "$PWD"
# x86-64 emulado (contenedor)
docker run --rm --platform linux/amd64 -v $S/godot-x86:/godot:ro -v $S/jobs-amd64:/jobs -v $S/scripts:/scripts:ro debian:bookworm-slim timeout 3500 sh /scripts/run_amd64.sh
# hash canónico, por lado
(cd $S/hasher && cargo build --release)
t13-hasher $S/jobs-arm64 $S/jobs-arm64/cases.txt > $S/arm64-geom.txt
t13-hasher $S/jobs-amd64 $S/jobs-amd64/cases.txt > $S/amd64-geom.txt
diff $S/arm64-geom.txt $S/amd64-geom.txt
# hash del GLB entero
for n in $(cut -d' ' -f1 $S/jobs-arm64/cases.txt); do shasum -a 256 $S/jobs-arm64/$n/seed.glb $S/jobs-amd64/$n/seed.glb; done
```

Salidas en bruto: `T13-scratch/{arm64-geom.txt, amd64-geom.txt, glb-sha.txt, amd64-env.txt, amd64-run.txt, arm64-godot-version.txt}`.

## Tabla: `geometry_sha256` (48 semillas publicadas)

| paquete | semilla | arm64 nativo | x86-64 EMULADO (Rosetta) | ¿igual? |
|---|---|---|---|---|
| asteroid_32 | 42 | e8fe200d5c56fc60e5a993a80f991f2789a161b032969557b4e1ec9f53a25277 | e8fe200d5c56fc60e5a993a80f991f2789a161b032969557b4e1ec9f53a25277 | sí |
| asteroid_32 | 1227191235 | f4595b2ad019e50a24ff65f5c5c8fdae760a37306a9b13f7ba9c54abcda12fbc | f4595b2ad019e50a24ff65f5c5c8fdae760a37306a9b13f7ba9c54abcda12fbc | sí |
| asteroid_32 | 4096378883 | 6857cfe7e0c45e2ce2e4b87256c3313d8e536b0befb43ce4cc365410913fd55e | 6857cfe7e0c45e2ce2e4b87256c3313d8e536b0befb43ce4cc365410913fd55e | sí |
| asteroid_32 | 621036976 | 531f6340cee5009c8c8f9bfbe2a49a221e3281b35e06aebe19788bd950cf84fe | 531f6340cee5009c8c8f9bfbe2a49a221e3281b35e06aebe19788bd950cf84fe | sí |
| asteroid_32 | 1846092899 | a6fec5143aa28e43a0e84ab8ba610be53a2e638ea848b460c41d1a140d56a5d5 | a6fec5143aa28e43a0e84ab8ba610be53a2e638ea848b460c41d1a140d56a5d5 | sí |
| asteroid_32 | 1595708226 | 71a549d03cca4d9e4f8a6989ba798f60a57597b2b4b8fb4ad130208205a0ffb6 | 71a549d03cca4d9e4f8a6989ba798f60a57597b2b4b8fb4ad130208205a0ffb6 | sí |
| asteroid_32 | 3943211157 | e4c0c9de2daa2dd2f2e9578f9531b1f953b3dd405cf77b8bcd94b635d9703b8a | e4c0c9de2daa2dd2f2e9578f9531b1f953b3dd405cf77b8bcd94b635d9703b8a | sí |
| asteroid_32 | 1967550902 | 62def660732c5bc651d7d57b2ed7349bcdda9bb554ffb5b393f5f407e852c015 | 62def660732c5bc651d7d57b2ed7349bcdda9bb554ffb5b393f5f407e852c015 | sí |
| asteroid_32 | 167205829 | 7c869df700b3eb7ae1eaf457a2332c8ee09c6b3359d1d8331f11d4ecc8fed362 | 7c869df700b3eb7ae1eaf457a2332c8ee09c6b3359d1d8331f11d4ecc8fed362 | sí |
| asteroid_32 | 2394521605 | 666ba7129a6bb76b8650c5726fdae18dd001c95872e4d6264ed9578ac68d2f4d | 666ba7129a6bb76b8650c5726fdae18dd001c95872e4d6264ed9578ac68d2f4d | sí |
| asteroid_32 | 3890178997 | dd9af15ed75770880b517863a124f7bf6406fb81b4dce921fca1ef12cdf949fd | dd9af15ed75770880b517863a124f7bf6406fb81b4dce921fca1ef12cdf949fd | sí |
| asteroid_32 | 1319969106 | 13236d3f3b7532a8586f4ec83adf6399d60303618247d896e9d7c8ffb398aab5 | 13236d3f3b7532a8586f4ec83adf6399d60303618247d896e9d7c8ffb398aab5 | sí |
| asteroid_32 | 1859157860 | 5c2095ec8d5921490810b681e4c83593ca671eae93506051a45cc750aa48f375 | 5c2095ec8d5921490810b681e4c83593ca671eae93506051a45cc750aa48f375 | sí |
| asteroid_32 | 3665219471 | 1cf547f1826ed48f379cd07c5b111879f51bb3066b2deff58101e8965852b7ad | 1cf547f1826ed48f379cd07c5b111879f51bb3066b2deff58101e8965852b7ad | sí |
| asteroid_32 | 2142325897 | 015dd044ab1467ab39acb2d1041a777042ac439cb311fdb7e7e3f873a5431e77 | 015dd044ab1467ab39acb2d1041a777042ac439cb311fdb7e7e3f873a5431e77 | sí |
| asteroid_32 | 303757059 | 9efd1fcdb1c66f048a97d2b1cd4154257a48fef92c7e413a3b831df19bc5c5d2 | 9efd1fcdb1c66f048a97d2b1cd4154257a48fef92c7e413a3b831df19bc5c5d2 | sí |
| asteroid_64 | 42 | 2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a | 2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a | sí |
| asteroid_64 | 1227191235 | 4a0b556a06e3e23f5a62f9366a8b8aff9e11ebab4a8892a4d0bcfd1257141cec | 4a0b556a06e3e23f5a62f9366a8b8aff9e11ebab4a8892a4d0bcfd1257141cec | sí |
| asteroid_64 | 4096378883 | 449b8af226b4970835a29594316e2098fa36367cd3a1dc8d16b1cb09bae3c492 | 449b8af226b4970835a29594316e2098fa36367cd3a1dc8d16b1cb09bae3c492 | sí |
| asteroid_64 | 621036976 | de3be14769e10963c006709a11f59f1a2c5113a00f5132e45600e291c813b380 | de3be14769e10963c006709a11f59f1a2c5113a00f5132e45600e291c813b380 | sí |
| asteroid_64 | 1846092899 | a656bc925cc4e78c4f8e79ba0301eb981b4546d0b8df9cf65b858ad322933386 | a656bc925cc4e78c4f8e79ba0301eb981b4546d0b8df9cf65b858ad322933386 | sí |
| asteroid_64 | 1595708226 | 6586286249bfb3ce3b4e0327d21c3085c3c7599f99b2c967f00afb2a51bf678f | 6586286249bfb3ce3b4e0327d21c3085c3c7599f99b2c967f00afb2a51bf678f | sí |
| asteroid_64 | 3943211157 | c3bcc8f679173a5b665c7e05958948339be65959e2e3a01991d348f2fd76a005 | c3bcc8f679173a5b665c7e05958948339be65959e2e3a01991d348f2fd76a005 | sí |
| asteroid_64 | 1967550902 | de58846a4ca737100315843647b2cd35426a9ffe16f4a18b0a8bccbd255357a6 | de58846a4ca737100315843647b2cd35426a9ffe16f4a18b0a8bccbd255357a6 | sí |
| asteroid_64 | 167205829 | d3b0905a51d2e191a2c4e9805986f0ccf699cd40e0786e0657ce5b7e0fba2efd | d3b0905a51d2e191a2c4e9805986f0ccf699cd40e0786e0657ce5b7e0fba2efd | sí |
| asteroid_64 | 2394521605 | 164d6fc0eeb5b37a59341b0d201bba06f94f777055fb05997119c5aac595ad37 | 164d6fc0eeb5b37a59341b0d201bba06f94f777055fb05997119c5aac595ad37 | sí |
| asteroid_64 | 3890178997 | 0d5d258006c76b610972c798e12b1c835a673a136265e1d71d638fcce0ba489b | 0d5d258006c76b610972c798e12b1c835a673a136265e1d71d638fcce0ba489b | sí |
| asteroid_64 | 1319969106 | c1aab986fc75b896f4818b72993b6260ccb50ad7c0a314bef1788ee9080d9992 | c1aab986fc75b896f4818b72993b6260ccb50ad7c0a314bef1788ee9080d9992 | sí |
| asteroid_64 | 1859157860 | 8336f399c3d3d7aeea0eebcfa37aadf90e508417f7a3cfd83a8ca2e3c5020325 | 8336f399c3d3d7aeea0eebcfa37aadf90e508417f7a3cfd83a8ca2e3c5020325 | sí |
| asteroid_64 | 3665219471 | 986805572610c2ad21b3df241272720ea0113685fee474af5cc5470685bab7f5 | 986805572610c2ad21b3df241272720ea0113685fee474af5cc5470685bab7f5 | sí |
| asteroid_64 | 2142325897 | d8083d1f1c3dfe697fac2553367b42fd834c898e0512561f11117dfdca37190d | d8083d1f1c3dfe697fac2553367b42fd834c898e0512561f11117dfdca37190d | sí |
| asteroid_64 | 303757059 | 35ea7dfe6fd9aaceeb27235540f333a8cecc47014933662dbe8fc09640a4af38 | 35ea7dfe6fd9aaceeb27235540f333a8cecc47014933662dbe8fc09640a4af38 | sí |
| asteroid_128 | 42 | e25d73781f18361746f2fea48c641b83d429a64c959a15851c5e6fb5b5f16436 | e25d73781f18361746f2fea48c641b83d429a64c959a15851c5e6fb5b5f16436 | sí |
| asteroid_128 | 1227191235 | 19b94000ba3a5138ef710b2c1ab649b3c6c4899079d825e6d9009232631dd105 | 19b94000ba3a5138ef710b2c1ab649b3c6c4899079d825e6d9009232631dd105 | sí |
| asteroid_128 | 4096378883 | ba8ce461e2dd6ef039218f6d616c535ceda87258af3e8427ca61cd755af665d5 | ba8ce461e2dd6ef039218f6d616c535ceda87258af3e8427ca61cd755af665d5 | sí |
| asteroid_128 | 621036976 | db02348d2ac11fc7ce33ef8202140d19b01b816f7105823331ffe8290ffe4baf | db02348d2ac11fc7ce33ef8202140d19b01b816f7105823331ffe8290ffe4baf | sí |
| asteroid_128 | 1846092899 | adc84a63de8f084771c2f1e63986abe2ededed5208dcae4034c19137f4ceb948 | adc84a63de8f084771c2f1e63986abe2ededed5208dcae4034c19137f4ceb948 | sí |
| asteroid_128 | 1595708226 | f91c47beb0c16b89e815ff8337f4f62dda98a54c422d0acf315d37ee7032e665 | f91c47beb0c16b89e815ff8337f4f62dda98a54c422d0acf315d37ee7032e665 | sí |
| asteroid_128 | 3943211157 | 6ba858ee34894e963ed54b29c96962a800880e926abc7ab9db08e1b8585932d1 | 6ba858ee34894e963ed54b29c96962a800880e926abc7ab9db08e1b8585932d1 | sí |
| asteroid_128 | 1967550902 | 9f6a9894004aa9ff49b0fbd3ba2e3f400e1f6f9a29c1fee5607b8da743f243b0 | 9f6a9894004aa9ff49b0fbd3ba2e3f400e1f6f9a29c1fee5607b8da743f243b0 | sí |
| asteroid_128 | 167205829 | 8b1777b94086e7e5ba73087b075b48cf28ba7c3414ba30f00fcf7d4bf88585aa | 8b1777b94086e7e5ba73087b075b48cf28ba7c3414ba30f00fcf7d4bf88585aa | sí |
| asteroid_128 | 2394521605 | 634b919fc2180a12924f8ed1d973695a8204ea6aaff4a958d2f92c532dac5ab3 | 634b919fc2180a12924f8ed1d973695a8204ea6aaff4a958d2f92c532dac5ab3 | sí |
| asteroid_128 | 3890178997 | 1adef44dc3da59031b4f6fbf88fd8b2d761530c2f716478ec2eec159025149c9 | 1adef44dc3da59031b4f6fbf88fd8b2d761530c2f716478ec2eec159025149c9 | sí |
| asteroid_128 | 1319969106 | 5a4065df9706dc3a3207ed61e9b3e754fbeeeab1a4e5e4dbfa08d6ad640fde14 | 5a4065df9706dc3a3207ed61e9b3e754fbeeeab1a4e5e4dbfa08d6ad640fde14 | sí |
| asteroid_128 | 1859157860 | 1343afcc2c99e79f710dbe4fa00765725069c11db399953fd7d167ecf4f4c951 | 1343afcc2c99e79f710dbe4fa00765725069c11db399953fd7d167ecf4f4c951 | sí |
| asteroid_128 | 3665219471 | acafe504c66c24cadec4c9d38d33abbbaaea39635b2cc25535c3a76c11e2dc5a | acafe504c66c24cadec4c9d38d33abbbaaea39635b2cc25535c3a76c11e2dc5a | sí |
| asteroid_128 | 2142325897 | 5f32b3ea75d0d0a4dbd13d9cbec45e4c5799db70282709932088e5c73709c6f8 | 5f32b3ea75d0d0a4dbd13d9cbec45e4c5799db70282709932088e5c73709c6f8 | sí |
| asteroid_128 | 303757059 | 06a5b8792f0ea1afa05eabb589e44e47194309e8faef7c4ef81f9a5ef53331b1 | 06a5b8792f0ea1afa05eabb589e44e47194309e8faef7c4ef81f9a5ef53331b1 | sí |

## Tabla: SHA-256 del GLB entero

Los dos lados dan el mismo valor en las 48 filas, así que se muestra una sola columna.

| paquete | semilla | SHA-256 del GLB (arm64 nativo = x86-64 EMULADO) | ¿igual? |
|---|---|---|---|
| asteroid_32 | 42 | 85e203c79f595ecf4d4bf5d92178a6f1423b610c1d7b22a3d20c3cea4e657f0c | sí |
| asteroid_32 | 1227191235 | c59cc267a8cd3e3a852a67c67f41ea7abe030b665dda29e32791b7ee400d6572 | sí |
| asteroid_32 | 4096378883 | 6d8ed3bbcda5c88a7ef38f4be4f07b933a33534e80ab3aaa7d54d6383b3c2f73 | sí |
| asteroid_32 | 621036976 | 13e9a2004eb8d9433e1383084a8ba1cf184c158abd222f5191e7183a9fdb51ea | sí |
| asteroid_32 | 1846092899 | 786f98a16493f874b0c622873aac13a3311b79dd021e2218f4f350962e4af506 | sí |
| asteroid_32 | 1595708226 | 8462ce3ca2dedfba7ef244003c4067e70bc33f9ff7e249e8b688d0f2ec318670 | sí |
| asteroid_32 | 3943211157 | be3dcb3f2096f63ac156c34b5a4c78e530c13403664f9d9586583d1fb49a3151 | sí |
| asteroid_32 | 1967550902 | 35edc591df2ee212f57898d4be7b89deaa9bd0e3bfbd928794df987aeac2b7a6 | sí |
| asteroid_32 | 167205829 | c3ebd7df3e4a29e2a4007048f6ceef7725fb4bd0d0850598313e10443ec5da75 | sí |
| asteroid_32 | 2394521605 | 53f63c9b2fc0a2572882ad105edad0606de10e6714f8e8c7340c24c44bfaefc7 | sí |
| asteroid_32 | 3890178997 | 251c06904207db41061821b91c6bb369ac681c29c843457533e6a819f10349a3 | sí |
| asteroid_32 | 1319969106 | 7daa0691ab8b25db616507b6a88012cb1da146ca669be7e8aeb88caf5822438e | sí |
| asteroid_32 | 1859157860 | cd7b59a1a842119b5ffd3a60d3ddddc5b67b0cb012087db5b617e60f53d7c4df | sí |
| asteroid_32 | 3665219471 | e5cb3830e76bde61e3721d3b3b30bc4caf5c312e642da2bc0514286e19f84d9a | sí |
| asteroid_32 | 2142325897 | b2e295ba5167aaaa52685b56ea1a81db2706267304708710ad3bf6f67ed5feac | sí |
| asteroid_32 | 303757059 | 1d0b3e3a4a98df648f3ac5fa467ef7bc73be4eea69f7d95d77d0afd11ddeea71 | sí |
| asteroid_64 | 42 | 0f5f7afc40bebf2edfc0e9f59c372b64043fcb5f8ffe2f01adc9dec8341a3162 | sí |
| asteroid_64 | 1227191235 | 1d6bc7bca9e1e28f9f36fa6a9cc4d811d353826a732605a6d36e65e66354d63d | sí |
| asteroid_64 | 4096378883 | 9d40e0903dbece5866b0b6743af38a97de946891b25239055f53271b771cd3ea | sí |
| asteroid_64 | 621036976 | 53f848790641486035a43b0a0083e5f37776e85cd84ba39da176201d7cd25ab9 | sí |
| asteroid_64 | 1846092899 | dcb3780097717a4ae5f2585cf3c6b45659bb07e21e8fdcf0f8a3e10332770d46 | sí |
| asteroid_64 | 1595708226 | 6efcbfc0e6cbf3b55a21d007eaf7ac054bf35c18b1bffb6a4203b9fe776055a1 | sí |
| asteroid_64 | 3943211157 | 84cd422f6a7de0ff15859a0099c0eb7c8f5cfef8d842cbe320e5f7ab2e73ef45 | sí |
| asteroid_64 | 1967550902 | d1b722b1a93729418b758b1c3f05e46d338c4a73650ce08283e7dd2cc102d591 | sí |
| asteroid_64 | 167205829 | 740d36a8aa1569ea6a0f43cbe7ec9f169494ea42bd88fc5b8049160f54fd57f8 | sí |
| asteroid_64 | 2394521605 | 602760b5f483b2c914011b8221bb05dbd075318b93168124a36446335dc30450 | sí |
| asteroid_64 | 3890178997 | 897290848d3e0fe4aef778a9e7581de5c460c25e418f7d424089982e9c781bc6 | sí |
| asteroid_64 | 1319969106 | 54ac131035b35fd61d344e7e2770478f1749c8056dc26f5fa4aace88c645bc09 | sí |
| asteroid_64 | 1859157860 | df6188fc01aa13f62e9499ff3353097042c081472c78a51c25c562ca17ac2127 | sí |
| asteroid_64 | 3665219471 | a2f0a48a26c9a53f05b5eb09583a74a844b3bd401bd1173f9cacdeb8f23d8498 | sí |
| asteroid_64 | 2142325897 | 00cd145816034517d00662b175508de813017430d40f8caa592485fd6dac4d3b | sí |
| asteroid_64 | 303757059 | 4e4f462b7fc84e6113d366b2afbdd6567f0c18cdf8f31943858158b3ae03beef | sí |
| asteroid_128 | 42 | 85157b7518edccd5a6e5e165342bc5f4a34753a89e8fb4e96d514b7df6279e32 | sí |
| asteroid_128 | 1227191235 | a57c9e343871a6dcb340baa907e5985b151efe47e141dcbb9b60d0259b684031 | sí |
| asteroid_128 | 4096378883 | c9c283e1540be85a9739c9b8791686777137dfc6c594af5711e70118b380827e | sí |
| asteroid_128 | 621036976 | a2819f8da2d00da95702977e61384ec2775c3685e26e3cc1d8c35840afaba155 | sí |
| asteroid_128 | 1846092899 | b07d9f09d2962fcc900009994aac255d151c8cb8a12f62414fe8e90dfc177759 | sí |
| asteroid_128 | 1595708226 | 6fc47b6ef55203c7991186766aa6881fa345cc34489706c53d35823d102c4dbb | sí |
| asteroid_128 | 3943211157 | faca7530a603cd319182b3727bdeea70255d6c3b9f17bab3785a99c9a47852ec | sí |
| asteroid_128 | 1967550902 | 0b1480d97b34b07bde9eff9a24aaa5d9343ffb3dfa3664ef498b7cd36290d3d3 | sí |
| asteroid_128 | 167205829 | 272e7326bed1558767a0e622906192f1bb6b67d94e087e85eadb426f05cffb40 | sí |
| asteroid_128 | 2394521605 | faed2ad9a61a455d01f456ec57018445472146e858ed6d0894aa7b03ba573cc5 | sí |
| asteroid_128 | 3890178997 | 6745e35110a8d3c921d991d1d44888c665c46aedb2784c91fb320707e751c0ee | sí |
| asteroid_128 | 1319969106 | 5a9fbe6fbe609358f5fad0bfeb69b996eb67ae5b19b7f66919c2edb56eabf6c2 | sí |
| asteroid_128 | 1859157860 | e9322d6254ff55188bd89b41b85ad08c8834bc97fae900843bc7ac54b5d8581c | sí |
| asteroid_128 | 3665219471 | a790f4def57ba93631871023c8929c6b55ec6d155aed8673cf771d75bfaebf46 | sí |
| asteroid_128 | 2142325897 | ebf2b5ff8794faa976c80396b0ff01e786ebd5b40bfa32ad3ed3390587d715d3 | sí |
| asteroid_128 | 303757059 | 1312c5a4a8350af423fb7efb01add89bbe7f0c376731e2d672a68c4b7b7dad8a | sí |
