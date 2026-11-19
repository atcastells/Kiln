# Hallazgos de la revisión T15 y su corrección (K013)

**Reconstrucción.** El artefacto original de T15 no se escribió en disco.
Este archivo lo reconstruye el 2026-11-19 solo desde tres fuentes:

- el apartado *Review* de `plan.md`;
- `review/evidence/results/T16-result.md`;
- la tabla *Revisión* de `handoff.md`.

Lo que no consta en esas fuentes no se ha añadido. En concreto, otros nits
de T15, si los hubo, no constan en la cadena: aquí solo figura el nit 1.

## Revisión T15 (2026-11-19)

Tres `reviewer` en paralelo (fallos, seguridad, conformidad con spec y plan) y
consolidación del integrador.

| Pasada | Veredicto |
|---|---|
| bugs | 2 Important: I-1 e I-2, los dos reproducidos (`review/evidence/*.log`) |
| security | Sin Important; fuzzing sin pánicos en `from_bytes`, `validate`, `build` y `node_transform_bytes` |
| conformance | R1–R10 cumplen. *Done means* abierto: I-3, I-4 e I-5 |

## Hallazgos y cierre

| Hallazgo | Qué era | Cómo se cerró | Estado |
|---|---|---|---|
| I-1 | Con S impar no se cumple el recuento de S1 §2.2 (`canonical/build.rs:53-54`): verify acababa en `canonical_count` en cada semilla tras lanzar Godot (`review/evidence/odd-segments.log`) | Decisión 003. T16: `canonical_odd_segments` en la comprobación estática de verify con la clave, antes de Godot. Rechaza un impar fijo y cualquier rango con extremos distintos. Pruebas `canonical_refuses_segments_that_can_be_odd` y `canonical_refuses_odd_segments_before_godot` | cerrado |
| I-2 | Republicar dejaba `geometry/seed_<n>.bin` de semillas que ya no estaban en el lote (`ops/publish_recipe.rs:108-123`; `review/evidence/stale-geometry.log`) | T16: `prune_geometry` borra solo los `seed_<dígitos>.bin` que no están en `files`, sin tocar directorios ni seguir enlaces. Prueba `a_republish_removes_only_the_geometry_of_seeds_no_longer_verified` | cerrado |
| I-3 | La evidencia de T14 (631/0/29) no estaba en disco: comunicada, no verificada | T17: verificación final en `review/final-*.log` (635/0/29; los 4 de más son los tests nuevos de T16) | cerrado |
| I-4 | Faltaba la tabla consolidada de los 89 casos | T17: tabla *Los 89 casos* de `handoff.md` | cerrado |
| I-5 | `work/T13-scratch/` de 322 MB sin ignorar | Lo citado se copió a `review/evidence/` en T17. `work/` ya no está en disco (comprobado con `ls` el 2026-11-19: «No such file or directory») | cerrado |
| nit 1 | Lote en el que ninguna semilla trae versión de Godot (la cadena solo registra el arreglo, no el comportamiento previo) | T16: `canonical_engine_missing` (error); si unas semillas traen versión y otras no, sigue siendo `canonical_engine_mismatch`. Prueba `canonical_refuses_a_batch_without_a_godot_version` | cerrado |
| seguridad | Sin hallazgos importantes | — | — |

## Evidencia

- Reproducción de I-1 e I-2: `review/evidence/odd-segments.log`,
  `review/evidence/stale-geometry.log` (y sus guiones `scratch_odd.rs`,
  `scratch_stale_geometry.rs.txt`).
- T16, rojo y verde: `review/evidence/t16/` y
  `review/evidence/results/T16-result.md`.
- Verificación final: `review/final-*.log`.

## Fuera del encargo de T16

`kiln generate` y `spec::check` no aplican la regla de impares; verify es la
puerta (`review/evidence/results/T16-result.md`, *Detail*).
