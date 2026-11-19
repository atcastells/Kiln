---
task: k013-canonical-geometry/T1
from: integrator
kind: result
result: done
data: measurement
cost:
regen: "W=~/Kiln/.sdlc/k013-canonical-geometry/work/T1-scratch; python3 $W/make_jobs.py $W ~/.t3/projects/astra; cd $W/proj; for k in sph8 sph64 ast42 ast42s; do godot --headless --path . --script generate.gd -- $W/jobs/$k.json $W/out/$k.out.json; godot --headless --path . --script dumpraw.gd -- $W/jobs/$k.json $W/out/$k.raw; done; python3 $W/compare.py $W; python3 $W/extra.py $W; python3 $W/helper_check.py $W"
persist: propose
files: [/Users/acastells/Kiln/.sdlc/k013-canonical-geometry/plan.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/intent.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/context.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/decisions/001-glb-orientation.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/work/T1-scratch/]
evidence: [decisions/001-glb-orientation.md, work/T1-scratch/compare.log, work/T1-scratch/extra.log, work/T1-scratch/helper_check.log, crates/kiln-cli/src/generate.rs:202-241, crates/kiln-core/src/mesh.rs:163-205, crates/kiln-cli/src/verify_recipe.rs:205-231]
---

## Summary

- Ediciones de la cadena hechas: en `plan.md`, Status aprobado con la rama; en `intent.md`, quitada «Requires product owner approval.»; `context.md` creado con el texto exacto.
- Decisión (`decisions/001-glb-orientation.md`): **T2 no invierte el giro**. El GLB sale hacia fuera: det > 0 en 64/64 y 4096/4096 triángulos con área, exacto en f32 y también en enteros tras cuantizar. Los arrays crudos salen hacia dentro (det < 0), como dicen S1 §5.2 y M§3 (`compare.log`).
- P1: las posiciones del GLB son iguales bit a bit a las de los arrays, y en el mismo orden, en los cuatro casos (sph8, sph64, ast42 con y sin facet). P3: ningún nodo trae translation, rotation, scale ni matrix. Se cumplen los supuestos de la spec, así que no hay parada.
- P4: tras cuantizar, unir y descartar salen 34/64 a 8 segmentos y 2050/4096 a 64, también en ast42 con y sin facet. El helper `sphere_glb` da el mismo signo y los mismos recuentos (`helper_check.log`).
- Permutación: en una malla indexada, el GLB escribe cada triángulo como (c,b,a) del crudo. Con facet, el exportador genera los índices `0,2,1,…` y queda (a,c,b).
- Hallazgo para T2/T11, no bloqueante: con y sin facet salen los mismos vértices, numerados por su primera aparición en POSITION, y los mismos triángulos orientados, pero cada triángulo empieza en otro vértice. Como S1 §6.2 no normaliza el orden, el hash sería distinto. Las 48 semillas de K012 llevan facet, así que entre ellas es coherente. T11 debe construir el negativo «orden de Godot» invirtiendo el GLB.
- Entradas: generador de ASTRA, con el mismo SHA que el `BUILDER` de Kiln; ops de la semilla 42 tomados de `asteroid_64/recipe.json`; esfera base de radio 1000 m (`measure/sphere.gd`). Godot 4.7.1 en arm64, sin ventana. No se corrió `kiln verify` en ASTRA.

## Detail

La tabla completa, la forma de los nodos y el comando de reproducción están en `decisions/001-glb-orientation.md`. El lector GLB en Python (`work/T1-scratch/compare.py`) reproduce `mesh::read`: offsets, stride y tipos de índice. Redondeo: mitades lejos de cero sobre `f64(f32)`. Determinantes con `Fraction` y enteros de Python, sin desbordamiento. `work/T1-measurement.md` es un borrador anterior de este artefacto; se puede borrar (el `rm` falló por un error del adaptador).
