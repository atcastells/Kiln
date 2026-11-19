---
task: k013-canonical-geometry/T13
from: integrator
kind: result
result: done
data: measurement
cost:
regen: "see .sdlc/k013-canonical-geometry/measurements-x86.md §Reproducir (make_jobs.py per side, host godot xargs run, docker run --platform linux/amd64 ... run_amd64.sh, cd work/T13-scratch/hasher && cargo build --release, t13-hasher per side, shasum -a 256 per GLB)"
persist: propose
files: [.sdlc/k013-canonical-geometry/measurements-x86.md, .sdlc/k013-canonical-geometry/work/T13-scratch/]
evidence: [.sdlc/k013-canonical-geometry/measurements-x86.md, work/T13-scratch/arm64-geom.txt, work/T13-scratch/amd64-geom.txt, work/T13-scratch/glb-sha.txt, work/T13-scratch/amd64-env.txt, work/T13-scratch/amd64-run.txt]
---

## Summary

- R7 medido, pero solo en **x86-64 EMULADO**. Los 48 `geometry_sha256` de las semillas publicadas son iguales en arm64 nativo y en x86-64 emulado: 48/48 (`diff` de `work/T13-scratch/arm64-geom.txt` y `amd64-geom.txt`). La tabla está en `measurements-x86.md`.
- Los 48 GLB son además idénticos byte a byte (`work/T13-scratch/glb-sha.txt`). La igualdad llega a los `f32`, no solo a la cuantización de 1 mm.
- Emulador: **Rosetta**, confirmado por `/run/rosetta/rosetta` mapeado en `/proc/self/maps` y por `vendor_id: VirtualApple` (`amd64-env.txt`). El entorno x86-64 es Docker Engine 29.9.2 con `debian:bookworm-slim` amd64 (glibc 2.36; `ldd` muestra el `libm.so.6` de glibc) y Godot oficial `linux.x86_64` 4.7.1, con SHA-512 verificado contra `SHA512-SUMS.txt` de la release.
- El lado arm64 reproduce los 48 de T10 línea a línea. Tiempo de reloj: 3 s en arm64 y 11 s emulado, 6 en paralelo en ambos.
- Qué prueba: el binario x86-64 oficial, con el libm de glibc, da los mismos bytes. Qué no prueba: una CPU x86-64 física (no he comprobado la fidelidad de Rosetta), otros libm, ni los builds de Windows o macOS Intel. Para cerrar R7 del todo falta un equipo nativo (D5): allí sirve sin cambios el mismo `docker run`.
- Sin cambios en código ni en tests del producto. El scratch ocupa 322 MB (binario de Godot, zip y dos árboles de trabajos); queda a decisión de la sesión principal borrarlo. Queda un duplicado de este artefacto, `work/T13-measurement.md`, que no pude borrar: se puede descartar.

Learned:
- Learned: en Docker Desktop, Rosetta se ve desde dentro de un contenedor amd64 en `/proc/self/maps` (`/run/rosetta/rosetta`). `ls /run/rosetta` y `binfmt_misc` no lo muestran.
- Learned: `docker image inspect` sin `--platform` informa de la variante por defecto (arm64) aunque se haya bajado la amd64.
- Learned: el binario `linux.x86_64` de Godot 4.7.1 corre `--headless` en `debian:bookworm-slim` sin paquetes extra. El aviso de que falta `libfontconfig.so.1` no bloquea.
- Learned: el GLB de `generate.gd` no incrusta la ruta ni el id del job, así que dos lados con rutas distintas dan archivos idénticos.
- Learned (causa sin determinar): en esta sesión, cuatro órdenes de shell que contenían `rm -f` no llegaron a ejecutarse («no active turn»). La corrida pasó sin borrar nada.

## Detail

- Los trabajos tienen la forma de `crates/kiln-cli/tests/common/canonical.rs:28-35`: `{id, glb, material, ops}`, con el `project.godot` de `SCRATCH_PROJECT` y `generate.gd` y `kiln_generator.gd` leídos del repo. Los genera `work/T13-scratch/scripts/make_jobs.py`, uno por lado.
- El hasher, `work/T13-scratch/hasher`, es un proyecto cargo con `[workspace]` propio y `kiln-core` por ruta. Hace `node_transform`, comprueba que hay una sola malla, y luego `build`, `validate`, `to_bytes` y `geometry_sha256`. Lo compilé con `CARGO_TARGET_DIR` en el scratchpad de la sesión, que no persiste.
- La corrida completa sobrescribió el GLB `pub_128_42` del humo: su mtime es 00:00:19, dentro de la corrida.
- Las 48 ejecuciones emuladas dieron `exit=0 glb=yes` (`amd64-run.txt`). Ningún fallo.
