# Decisión 001: orientación y posiciones del GLB que Kiln lee

- **Tarea**: K013/T1 (medición, sin código de producto). Fecha: 2026-11-18.
- **Decisión**: la construcción canónica (T2) **no invierte el giro**. El GLB que exporta `generate.gd` ya sale hacia fuera; se toman los triángulos del GLB en su orden de índices tal cual.
- **Supuestos de la spec que se confirman**: posiciones `f32` conservadas bit a bit y en orden; ningún nodo con transformación. No hay motivo para parar (plan, *Stop and ask* 2).

## Qué se midió

Godot 4.7.1-stable (official), macOS arm64, sin ventana. Proyecto desechable como el de `run_seed` (`crates/kiln-cli/src/generate.rs:202-241`): `generate.gd` de Kiln y `kiln_generator.gd` de ASTRA `godot/assets/kiln/asteroid_64/`. Ese guion y el `BUILDER` de Kiln (`crates/kiln-cli/godot/kiln_generator.gd`) tienen el mismo SHA-256 (`3c5ae7c6…a36f0`, igual que `script_sha256` del paquete).

Entradas (`job.json` con la forma de `generate.rs:223-225`):

| Nombre | Ops |
|---|---|
| `sph8` | `sphere` radio 1000 m, 8 segmentos, sin ruido (radio de ASTRA `measure/sphere.gd`) |
| `sph64` | ídem, 64 segmentos |
| `ast42` | Semilla 42 de `asteroid_64/recipe.json`, ops ya resueltos con `noise_seed`, **con `facet`** (lo que pasa verify) |
| `ast42s` | La misma sin `facet` (malla lisa indexada, como vuelca `measure/dump.gd`) |

Arrays crudos: `gen.build(job)` → `surface_get_arrays(0)` con las mismas entradas (`facet` incluido en `ast42`: sale sin índices). GLB leído con un lector en Python que reproduce `kiln_core::mesh::read` (`crates/kiln-core/src/mesh.rs:163-205`: offsets, stride, índices 5121/5123/5125, sin índices = 0..n).

## Resultados

| | sph8 | sph64 | ast42s | ast42 (facet) |
|---|---|---|---|---|
| V / índices GLB = crudo | 54 / 240 | 2210 / 12672 | 2210 / 12672 | 12672 / 12672 |
| Triángulos (= `tris` de `out.json`) | 80 | 4224 | 4224 | 4224 |
| 1. Posiciones bit a bit, en orden | sí | sí | sí | sí |
| 2. Permutación de cada triángulo GLB respecto al crudo | (c,b,a) en 80/80 | (c,b,a) 4224/4224 | (c,b,a) 4224/4224 | (a,c,b) en 4160; 64 de los polos ambiguas (dos vértices iguales) |
| 2. Giro GLB, det(a,b,c) exacto en f32 | + 64, 0 × 16 | + 4096, 0 × 128 | + 4096, 0 × 128 | + 4096, 0 × 128 |
| 2. Giro crudo | − 64, 0 × 16 | − 4096, 0 × 128 | − 4096, 0 × 128 | − 4096, 0 × 128 |
| 3. Transformación de nodo | ninguna | ninguna | ninguna | ninguna |
| 4. Tras cuantizar, unir y descartar: V / T | 34 / 64 (esperado 34 / 64) | 2050 / 4096 (2050 / 4096) | 2050 / 4096 | 2050 / 4096 |
| 4. Giro de lo que sobrevive, en enteros | + 64 | + 4096 | + 4096 | + 4096 |

- **Nodos** (los cuatro GLB): dos nodos, `{name, children:[1]}` raíz de la escena y `{name, mesh:0}`. Ni `translation`, `rotation`, `scale` ni `matrix` en ninguno. Una malla, una primitiva, `mode` 4; atributos POSITION, NORMAL, TANGENT, TEXCOORD_0.
- **Índices**: el GLB de la malla facetada trae índices generados por el exportador con el patrón `0,2,1, 3,5,4…`; el crudo no tiene. En las mallas indexadas el GLB escribe cada triángulo como (c,b,a) del crudo.
- **Crudo hacia dentro, GLB hacia fuera**: coincide con S1 §5.2 y M§3 (los arrays de Godot salen con el signo contrario); el exportador ya hace la inversión.

## Consecuencias para las tareas siguientes

- **T2**: sin inversión. Los recuentos de R1 se cumplen sobre el GLB real a 8 y 64 segmentos. Regla de unión usada en la medición: cada vértice canónico se numera por su primera aparición en el orden del array POSITION. Esa numeración entra en el hash, y de ella depende que con y sin `facet` salga el mismo orden de vértices.
- **Helper de pruebas** (`verify_recipe::tests::sphere_glb`, `crates/kiln-cli/src/verify_recipe.rs:205-231`, radio 1 m): replicado en `helper_check.py`, sale hacia fuera con el mismo signo que el GLB real (+64 con 16 sin área a 8 segmentos; +4096 con 128 sin área a 64) y da 34/64 y 2050/4096 tras cuantizar, unir y descartar. Sirve para las pruebas de T2 sin inversión.
- **T11 (vía B, «orden de triángulos de Godot» → `I8_not_radial`)**: como el GLB ya sale hacia fuera, ese negativo se construye invirtiendo el GLB, como prevé la spec (*Casos*, vía B). Hay que decirlo en T11.
- **Hash y facet** (sin decidir aquí): la misma geometría con y sin `facet` da los mismos vértices cuantizados en el mismo orden de primera aparición y los mismos triángulos orientados, pero cada triángulo empieza en otro vértice ((c,b,a) frente a (a,c,b), rotaciones uno del otro). S1 §6.2 no normaliza el orden, así que darían `geometry_sha256` distintos. Las 48 semillas de K012 llevan `facet`, así que entre ellas es coherente; el hash queda ligado a cómo el exportador de Godot 4.7.1 escribe los índices.

## Cómo se reproduce

Guiones y salidas en `review/evidence/t1/`: `proj/` (proyecto Godot con `generate.gd`, `kiln_generator.gd` y `dumpraw.gd`), `make_jobs.py` → `jobs/`, `out/` (no conservado: lo regeneran los dos bucles de abajo), `compare.py` → `compare.log`, `extra.py` → `extra.log`, `helper_check.py` → `helper_check.log`.

```bash
W=~/Kiln/.sdlc/k013-canonical-geometry/review/evidence/t1
python3 $W/make_jobs.py $W ~/.t3/projects/astra
cd $W/proj
for k in sph8 sph64 ast42 ast42s; do
  godot --headless --path . --script generate.gd -- $W/jobs/$k.json $W/out/$k.out.json
  godot --headless --path . --script dumpraw.gd -- $W/jobs/$k.json $W/out/$k.raw
done
python3 $W/compare.py $W; python3 $W/extra.py $W; python3 $W/helper_check.py $W
```
