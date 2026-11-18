# Plan: piloto de integración de asteroides procedurales en ASTRA (K012)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-18)
- **Nothing is implemented until this file is approved.**

El código va en ASTRA (`~/.t3/projects/astra`, rama nueva
`k012-asteroid-pilot` desde `origin/main`); la cadena, las capturas y el
handoff, en Kiln (esta rama).

## Done means

En ASTRA: tres paquetes publicados (`asteroid_32`, `asteroid_64`,
`asteroid_128`); `godot/procedural/recipe_package.gd` los abre y construye
cualquiera de sus semillas; la escena `tests/asteroid_procedural.tscn` se
abre y enseña el asteroide con sus datos; la fase `asteroid` de
`scripts/godot_station.sh` pasa sin servidor; `--asteroid-shots DIR` deja
capturas y `cost.json`. Todo se mide en este Mac. `git diff origin/main --stat` no toca Rust, el
protocolo ni `station/`. Las regresiones de ASTRA y de Kiln pasan. En Kiln:
capturas de cada tanda, métricas y `handoff.md` con los diez criterios
K12 y el registro frente a AST-EST. Los dos PR, unidos.

## Stop and ask if

- Una regresión de ASTRA que ya existía falla por algo ajeno a este cambio.
- La copia de ASTRA tiene cambios sin confirmar de otra sesión cuando haya
  que cambiar de rama.
- Hace falta tocar `crates/`, `godot/proto/`, `godot/net/`, `godot/station/`
  o el código de Kiln.
- Una semilla construida en ASTRA no da los triángulos o las medidas del
  paquete dentro de la tolerancia.
- La misma semilla da dos hashes distintos en este Mac.
- Alguna ficha deja de verificar con el Kiln de `origin/main`.

Abrir ventanas de Godot en el escritorio está autorizado fuera del horario
laboral (de 17:00 a 08:00). En horario laboral las ventanas van en Docker,
con `scripts/godot_window.sh`; si una tanda con ventana cae en ese horario,
se para y se pregunta.

## Tasks

Los encargos de ASTRA pueden lanzar Godot sin ventana para sus propias
pruebas (`godot --headless --path godot -s tests/…`), sin servidor. Las
ventanas las abre `main`.

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Rama en ASTRA, las tres fichas, `kiln check`, `verify` y `publish` con el Kiln de `origin/main`, e importar una vez para que Godot cree el `.uid` | ASTRA: `kiln/specs/asteroid_{32,64,128}.toml`, `godot/assets/kiln/asteroid_{32,64,128}/` | `check` sin errores de esas fichas; tres informes válidos; tres ficheros por paquete y ningún GLB. Captura: las tres resoluciones | R1 | main | — | 6m/3M | pending |
| T2 | `open(id)`: localizar el paquete y las ocho negativas, cada una con su mensaje | ASTRA: `godot/procedural/recipe_package.gd` (nuevo), `godot/tests/asteroid_recipe_test.gd` (nuevo) | Un caso por negativa con paquetes estropeados en `user://`; los tres paquetes buenos abren; sin `SCRIPT ERROR` | R2, R5 | implementer | T1 | 2m/300k | pending |
| T3 | `build(package, seed)`: semilla de la tanda, forma de la entrada comprobada antes de llamar al guion, malla, medidas, vértices, milisegundos y hash | ASTRA: `godot/procedural/recipe_package.gd`, `godot/tests/asteroid_recipe_test.gd` | Las 16 semillas de los tres paquetes: triángulos exactos y medidas a menos de 1 en 100 000; semilla fuera de la tanda y entrada mal formada dan error; dos construcciones, mismo hash; tres semillas, tres hashes | R2, R3, R4, R5 | implementer | T2 | 2m/300k | pending |
| T4 | La escena: asteroide, cámara orbital, luz, barra de 100 m, rótulo de datos, parámetros `--asset` y `--seed`, error en pantalla | ASTRA: `godot/tests/asteroid_procedural.tscn` (nuevo), `godot/tests/asteroid_procedural.gd` (nuevo) | Sin ventana: la escena carga, el rótulo trae medidas, triángulos, vértices y tiempo; con un paquete que no existe trae el error y no hay `SCRIPT ERROR`. Captura con ventana: la escena | R6 | implementer | T3 | 3m/400k | pending |
| T5 | Fase `asteroid` en la regresión, entre las de por defecto, sin servidor; incluye la prueba de la escena | ASTRA: `scripts/godot_station.sh`, `godot/tests/asteroid_recipe_test.gd` | `scripts/godot_station.sh asteroid` termina con su línea OK | R9 | implementer | T4 | 1m/200k | pending |
| T6 | Guion de capturas: por resolución, tres semillas desde tres ángulos, bien encuadradas y con la barra de escala visible | ASTRA: `godot/tests/asteroid_shots.gd` (nuevo) | Con ventana (lo corre `main`): 27 PNG de más de 1 KB. Captura: una hoja por resolución | R7 | implementer | T3 | 2m/300k | pending |
| T7 | Métricas en `cost.json`: generación, triángulos y vértices, memoria, coste de dibujo con y sin asteroide, 200 reconstrucciones, equipo y renderizador | ASTRA: `godot/tests/asteroid_shots.gd` | `cost.json` trae cada bloque para las tres resoluciones | R7 | implementer | T6 | 2m/300k | pending |
| T8 | Modo `--asteroid-shots DIR` en el lanzador, sin servidor, como `--crew-shots` | ASTRA: `scripts/godot_station.sh` | El modo comprueba las PNG y el `cost.json` y termina con su línea OK | R7 | implementer | T5, T7 | 1m/200k | pending |
| T9 | Correr las capturas y métricas con ventana (Metal) y guardarlas en Kiln | Kiln: `.sdlc/k012-asteroid-pilot/shots/`, `metrics/cost.json` | Las capturas y el `cost.json` existen y se han mirado una a una | R7, R11 | main | T8 | 5m/3M | pending |
| T11 | Regresiones: fases por defecto de ASTRA, `cargo test` y `clippy` en ASTRA y en Kiln; el resultado pasa al verificador | — | Todo verde; el verificador mira el piloto, la estación de siempre (`basic`) y un asset de piezas publicado antes | R8, R9 | main + verifier | T8 | 12m/4M | pending |
| T12 | Handoff: los diez criterios K12 con su evidencia, métricas, límites y el registro frente a AST-EST §3 y §8 | Kiln: `.sdlc/k012-asteroid-pilot/handoff.md` | Cada criterio cita su prueba, su captura o su cifra | R10, R11 | main | T9, T11 | 6m/3M | pending |
| T13 | Orquestación: encargos, lectura de cada diff, `context.md`, una captura por tanda | Kiln: `.sdlc/k012-asteroid-pilot/context.md` | — | R11 | main | — | 20m/8M | pending |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1 | 6m | captura: las tres resoluciones publicadas |
| 2 | T2 | 2m | leer el diff |
| 3 | T3 | 2m | captura: tres semillas construidas por el consumidor |
| 4 | T4, T6 | 3m | captura: la escena con ventana |
| 5 | T5, T7 | 2m | correr la fase `asteroid` |
| 6 | T8 | 1m | — |
| 7 | T9 | 5m | capturas definitivas y métricas |
| 8 | T11 | 12m | T12 |

- **Estimated wall-clock**: algo más de una hora
- **Sequential sum**: 62 min
- **Estimated tokens**: unos 23 M (2 M de encargos, el resto `main`)

## Order and dependencies

T1 va primero porque todo lo demás lee los paquetes publicados. T2 y T3
escriben los mismos dos ficheros y van en fila. T4 y T6 tocan ficheros
distintos y solo necesitan el consumidor. T5 y T8 tocan el lanzador y van
en tandas distintas. T9, T11 y T12 las hace `main`: son las que abren
ventanas y juntan la evidencia.

## Test strategy

TDD: test rojo antes de implementar. Si un test falla, se arregla el código.

- **Unit**: no hay lógica en Rust.
- **Integration**: `asteroid_recipe_test.gd` sin ventana ni servidor
  (consumidor, negativas, comparación con el paquete, hashes y carga de la
  escena), como fase de la regresión.
- **Manual verification**: `main` mira cada captura. El verificador recorre
  el piloto y dos flujos vecinos: la estación de siempre (fase `basic`) y
  que un asset de piezas ya publicado se sigue viendo. Antes de diagnosticar
  un rojo de la regresión de ASTRA se mide la línea base sin el cambio.

## Verification commands

```
# ASTRA
scripts/godot_station.sh asteroid
scripts/godot_station.sh
scripts/godot_station.sh --asteroid-shots DIR
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
# Kiln
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Implementation risks

- `scripts/godot_station.sh` compila el servidor antes de cualquier fase:
  la fase nueva no lo usa, pero paga esa compilación.
- Una copia de ASTRA sin caché de importación falla con las fuentes del
  tema hasta importar una vez; la escena usa ese tema.
- Los paquetes se publican con el binario de Kiln de `origin/main`; si se
  publican con otro, el hash del guion del contrato puede no ser el del
  repo.
- La copia de ASTRA está hoy en `task-010d1-npc-activity` con un PDF ajeno
  sin seguimiento: se cambia de rama sin tocarlo.
- Las pruebas de paquetes rotos escriben en `user://`: deben limpiar lo que
  dejan.

## Review

Filled in Deploy. A plan is not closed while this section is empty.

## Estimate vs actual

Filled at Build close.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

Lo que la spec deja fuera: excavación, cráteres, colisiones, interiores,
naves, varios asteroides, LOD; sortear semillas en el juego; comparar la
malla entre este Mac y Linux (Docker es para el horario laboral, no una
medida de este piloto); una clave de
roca en la paleta; enlazar la escena desde el juego. La forma base sin
polos, si las capturas la piden, sería una entrega de Kiln aparte.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
