---
task: k013-canonical-geometry/T13
from: integrator
kind: result
result: done
data: measurement
cost:
regen: "see .sdlc/k013-canonical-geometry/measurements-x86.md §Reproducir (make_jobs.py x2, docker run --platform linux/amd64 ... run_amd64.sh, cd work/T13-scratch/hasher && cargo build --release, t13-hasher per side, shasum -a 256 per GLB)"
persist: propose
files: [.sdlc/k013-canonical-geometry/measurements-x86.md, .sdlc/k013-canonical-geometry/work/T13-scratch/]
evidence: [.sdlc/k013-canonical-geometry/measurements-x86.md, work/T13-scratch/arm64-geom.txt, work/T13-scratch/amd64-geom.txt, work/T13-scratch/glb-sha.txt, work/T13-scratch/amd64-env.txt, work/T13-scratch/amd64-run.txt]
---

## Summary

- R7 medido, pero solo en **x86-64 EMULADO**: los 48 `geometry_sha256` de las semillas publicadas son iguales en arm64 nativo y en x86-64 emulado (48/48). Fuentes: `work/T13-scratch/arm64-geom.txt` y `amd64-geom.txt`, comparados con `diff`; la tabla está en `measurements-x86.md`.
- Los 48 GLB son además idénticos byte a byte (`work/T13-scratch/glb-sha.txt`). La igualdad llega a los `f32`, no solo a la cuantización de 1 mm.
- Emulador: **Rosetta**, confirmado por `/run/rosetta/rosetta` mapeado en `/proc/self/maps` y por `vendor_id: VirtualApple` (`amd64-env.txt`). Los demás datos del entorno: Docker Desktop 29.9.2, `debian:bookworm-slim` amd64 con glibc 2.36, y Godot oficial `linux.x86_64` 4.7.1 con SHA-512 verificado contra `SHA512-SUMS.txt` de la release.
- El lado arm64 reproduce los 48 de T10 línea a línea. Tiempo de reloj: 3 s en arm64 y 11 s emulado, con 6 en paralelo en ambos.
- Qué prueba: el binario x86-64 oficial y el libm de glibc dan los mismos bytes. Qué no prueba: una CPU x86-64 física (fidelidad de Rosetta), otros libm y los builds de Windows o de macOS Intel. Para cerrar R7 del todo falta un equipo nativo (D5); la receta del archivo sirve tal cual allí.
- Sin cambios en código ni en tests del producto. El scratch ocupa 322 MB: binario de Godot, zip y dos árboles de trabajos. Conviene que la sesión principal decida si se borra; `.sdlc/*/work` está fuera de git según la plantilla.

Learned:
- Learned: en Docker Desktop, Rosetta se ve desde dentro del contenedor amd64 en `/proc/self/maps` (`/run/rosetta/rosetta`). `binfmt_misc` y la ruta `/run/rosetta` no son visibles.
- Learned: el binario linux.x86_64 de Godot 4.7.1 corre `--headless` en `debian:bookworm-slim` sin paquetes extra. Avisa de que falta `libfontconfig.so.1`, pero no bloquea.
- Learned: el GLB de `generate.gd` no incrusta la ruta ni el id del job. Dos lados con rutas distintas dan archivos idénticos.
- Learned: en esta sesión, las órdenes de shell con `rm` o con `timeout` del host no llegaron a ejecutarse («no active turn»). Lo resolví sin borrar nada y moviendo `timeout` dentro del contenedor.

## Detail

- Los trabajos tienen la misma forma que `crates/kiln-cli/tests/common/canonical.rs:28-35`: `{id, glb, material, ops}`, con el `project.godot` de `SCRATCH_PROJECT` y `generate.gd` y `kiln_generator.gd` leídos del repo. Los genera `work/T13-scratch/scripts/make_jobs.py`, uno por lado.
- El hasher, `work/T13-scratch/hasher`, es un proyecto cargo con `[workspace]` propio y `kiln-core` por ruta. Comprueba `node_transform`, que hay una sola malla, y aplica `build`, `validate`, `to_bytes` y `geometry_sha256`. Lo compilé con `CARGO_TARGET_DIR` en el scratchpad de la sesión, que no persiste.
- El GLB `pub_128_42` del humo se sobrescribió en la corrida completa. Lo confirma el mtime 00:00:19, que cae dentro de la corrida y no en el humo de las 23:59.
- No hubo ningún fallo: las 48 ejecuciones dieron `exit=0 glb=yes` (`amd64-run.txt`).
