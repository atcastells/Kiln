---
task: k013-canonical-geometry/T1
from: integrator
kind: result
result: done
data: measurement
cost:
regen: "W=~/Kiln/.sdlc/k013-canonical-geometry/work/T1-scratch; cd $W/proj; for k in sph8 sph64 ast42 ast42s; do godot --headless --path . --script generate.gd -- $W/jobs/$k.json $W/out/$k.out.json; godot --headless --path . --script dumpraw.gd -- $W/jobs/$k.json $W/out/$k.raw; done; python3 $W/compare.py $W"
persist: propose
files: [/Users/acastells/Kiln/.sdlc/k013-canonical-geometry/plan.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/intent.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/context.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/decisions/001-glb-orientation.md, /Users/acastells/Kiln/.sdlc/k013-canonical-geometry/work/T1-scratch/]
evidence: [decisions/001-glb-orientation.md, work/T1-scratch/compare.log, work/T1-scratch/extra.log, crates/kiln-cli/src/generate.rs:202-241, crates/kiln-core/src/mesh.rs:163-205]
---

## Summary

- Ediciones de cadena hechas: `plan.md` Status aprobado con rama; `intent.md` sin «Requires product owner approval.»; `context.md` creado con el texto exacto.
- Decisión (`decisions/001-glb-orientation.md`): **T2 no invierte el giro**. GLB hacia fuera (det > 0 en 64/64 y 4096/4096 triángulos con área, exacto en f32 y en enteros tras cuantizar); arrays crudos hacia dentro (det < 0), como S1 §5.2 y M§3 (`compare.log`).
- P1: posiciones del GLB bit a bit iguales a los arrays, en orden, en los cuatro casos (sph8, sph64, ast42 con facet, ast42 sin facet). P3: ningún nodo con translation/rotation/scale/matrix (dos nodos: raíz con hijo, hijo con malla). Se cumplen los supuestos de la spec: no hay parada.
- P4: tras cuantizar, unir y descartar: 34/64 (S=8) y 2050/4096 (S=64, también ast42 con y sin facet), como se esperaba.
- Permutación: en una malla indexada el GLB escribe cada triángulo como (c,b,a) del crudo; con facet, el exportador genera índices `0,2,1,…` y queda (a,c,b).
- Hallazgo para T2/T11, no bloqueante: con y sin facet se obtienen los mismos vértices y triángulos orientados, pero cada triángulo empieza en otro vértice. Como S1 §6.2 no normaliza el orden, el hash sería distinto. Las 48 semillas de K012 llevan facet, así que entre ellas es coherente. T11 debe construir el negativo «orden de Godot» invirtiendo el GLB.
- Entradas: generador de ASTRA con el mismo SHA que el `BUILDER` de Kiln; ops de la semilla 42 tomados de `asteroid_64/recipe.json`; radio de la esfera base 1000 m (`measure/sphere.gd`). Godot 4.7.1, arm64, sin ventana. No se ha corrido `kiln verify` en ASTRA.

## Detail

Tabla completa, forma de los nodos y comando de reproducción en `decisions/001-glb-orientation.md`. Lector GLB en Python (`work/T1-scratch/compare.py`) que reproduce `mesh::read` (offsets, stride, tipos de índice). Redondeo: mitades lejos de cero sobre `f64(f32)`; determinantes con `Fraction` y enteros de Python, sin desbordamiento.
