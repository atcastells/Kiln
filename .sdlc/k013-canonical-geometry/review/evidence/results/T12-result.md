---
task: k013-canonical-geometry/T12
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cd <ASTRA> && for a in asteroid_32 asteroid_64 asteroid_128; do ~/Kiln/target/debug/kiln verify --project . --asset $a && ~/Kiln/target/debug/kiln publish --project . --asset $a; done && scripts/godot_station.sh asteroid && scripts/kiln_check.sh"
persist: propose
files: [<ASTRA>/kiln.toml, <ASTRA>/godot/assets/kiln/asteroid_{32,64,128}/asset.json, <ASTRA>/godot/assets/kiln/asteroid_{32,64,128}/geometry/seed_<n>.bin (48 nuevos), .sdlc/k013-canonical-geometry/plan.md, .sdlc/k013-canonical-geometry/context.md]
evidence: [/tmp/claudio-T12-verify-asteroid_32.log, /tmp/claudio-T12-verify-asteroid_64.log, /tmp/claudio-T12-verify-asteroid_128.log, /tmp/claudio-T12-publish-asteroid_32.log, /tmp/claudio-T12-publish-asteroid_64.log, /tmp/claudio-T12-publish-asteroid_128.log, /tmp/claudio-T12-godot-asteroid.log, /tmp/claudio-T12-kiln-check.log]
---

## Summary

- Se cumplen R8 y R4. Los seis archivos `recipe.json` y `kiln_generator.gd` tienen los mismos bytes que antes (`diff` de los sha256, vacío). En cada paquete, `version` sigue en 1 y publish lo confirma: «versión 1, sin cambios», como dice D4.
- En `git diff --stat` solo salen `kiln.toml` (+4) y los tres `asset.json` (+102/-1 cada uno). La línea quitada es solo la coma tras `"kiln_generator.gd"`. Archivos sin seguimiento nuevos: 48 `geometry/seed_<n>.bin`, 16 por paquete. No apareció ningún `.uid`, `.import` ni otro archivo.
- Lo que añade `asset.json`: `files` (18 entradas, todas existen), `geometry` (16 entradas), `kiln_version` "0.1.0" y `godot_version` "4.7.1-stable (official)". Los campos de antes no cambian.
- Cada `geometry_sha256` coincide con el SHA-256 de su archivo (48 de 48). El `seed_42.bin` de asteroid_64 es idéntico byte a byte (`cmp`) al fixture de T1, con sha 2713c0a98ff8c024e3874428503dc803848931eb0b05b36fc2abfa88f670d78a.
- Tamaños: cada `.bin` mide 18 488 / 73 784 / 294 968 B (32 + 12V + 12T). La carpeta `geometry/` ocupa 320 K, 1,2 M y 4,6 M (`du`); en total, unos 6,1 MB.
- Verify: los tres salen con código 0, 16 semillas con hash, ningún rechazo canónico y `recipe.godot_version` presente. Publish: los tres salen con código 0.
- Fase `asteroid` (sin ventana): `OK: asteroid recipe test passed`, código 0 (/tmp/claudio-T12-godot-asteroid.log). `kiln_check.sh`: «OK: kiln validated the astra modules», 0 errores y 23 avisos, ninguno de generate, canonical ni asteroid (/tmp/claudio-T12-kiln-check.log).
- No toqué `godot/procedural/` ni `godot/tests/asteroid_*`, ni hice escrituras de git. Cadena: T10 y T11 están en `done` en plan.md y context.md tiene una línea más.

Learned: la clave `[generate] canonical = "radial_v0"` cabe en ASTRA como sección nueva al final de `kiln.toml`. ASTRA no tenía `[generate]`, así que `verify_seeds` toma su valor por defecto: 16 semillas, que empiezan en la 42.
Learned: el consumidor de K012 (`recipe_package.gd`) acepta el `asset.json` con los campos y archivos nuevos sin cambiar nada. Así se confirma el riesgo bajo que preveía la spec.
Learned: el hash que imprime la fase `asteroid` («seed42 hash=2bace…») es el de la malla de Godot, no `geometry_sha256`, así que no hay que compararlos.

## Detail

Antes y después (sha256; las dos columnas coinciden):

| archivo | sha256 |
|---|---|
| asteroid_32/recipe.json | b7fed2e1eb9af98d37673d0fe60b851b5c077c8e58830f58f7f68a11d23cb193 |
| asteroid_64/recipe.json | 74ece734d2a3c54e4372146c10acaa751e59f98feda9bd253048b97aeedf6517 |
| asteroid_128/recipe.json | c8e5e9b16433719f8616bb37fe156c1cebaa737e939543763ea3e0637582ed0c |
| asteroid_{32,64,128}/kiln_generator.gd | 3c5ae7c6e5781fe452bf2468ed085e9ad997907d283656cc6f1e55b068ca36f0 |

- `git status --short` previo: solo los dos archivos sin seguimiento que ya existían (`.sdlc/canonical-geometry/measure/__pycache__/`, un PDF de `.sdlc/first-character/`). Después de la fase de Godot y de kiln_check, son los mismos.
- Diff stat:
  ```
  godot/assets/kiln/asteroid_128/asset.json | 102 +++++-
  godot/assets/kiln/asteroid_32/asset.json  | 102 +++++-
  godot/assets/kiln/asteroid_64/asset.json  | 102 +++++-
  kiln.toml                                 |   4 ++
  4 files changed, 307 insertions(+), 3 deletions(-)
  ```
- Peor caso de verify (triángulos del GLB, no canónicos): 1088, 4224 y 16640.
- Las líneas `message [...]` del log de Godot son los casos negativos que `asteroid_recipe_test.gd:402` imprime a propósito. No son fallos: en el log no hay ningún FAIL ni ERROR.
- Sin verificar: no medí si los 23 avisos de kiln_check son los mismos que antes del cambio. Para hacerlo habría que revertir `kiln.toml`, y no lo hice. Ninguno menciona la clave nueva.
- Binario de Kiln: `cargo build -p kiln-cli`, compilación incremental del árbol de trabajo actual, que incluye T9–T11 sin commitear.
