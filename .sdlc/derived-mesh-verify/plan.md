# Plan: integridad de malla en `kiln verify`, activable por ficha

- **Derived from**: `spec.md` (borrador)
- **Status**: approved by Aaron Castells, 2026-11-20 («Apruebo»), con las elecciones propuestas por la sesión principal: severidad error (bloquea `publish`); aviso de escala fuera de la cadena; `normals = "outward"` por defecto; autointersección dentro, y fuera a otra cadena si se dispara de coste. Rama `k015-derived-mesh-verify` desde `main` actualizado. T12 espera a la entrega de EST-004 y no decide qué se retira de `check.py`.
- **Nothing is implemented until this file is approved.**
- **Mode**: orchestrate. Cada fila de Kiln va a un `implementer` o al `integrator`.
- **Repos**: código en `~/Kiln`; la última fila, en ASTRA.

## Rama de partida

- **`claude/project-thread-ckdldl` ya está entregada.**
  - Su PR #35 (K014) se unió a `main` el 2026-11-19 con la cabeza `305ec28`
    (`gh pr view 35`).
  - GitHub da `main...claude/project-thread-ckdldl` ahead 0, behind 1: solo le
    falta el merge `f81505d`.
  - El `main`/`origin/main` local está desfasado (`822bd76`), porque no se ha
    hecho `fetch`.
  - No queda trabajo en curso en la rama: `git status` limpio, salvo esta cadena
    sin seguimiento.
- **Propuesta:** una rama nueva desde `main` actualizado, por ejemplo
  `k015-derived-mesh-verify`, con la convención `k0NN-*` de `CLAUDE.md`. La crea
  el desarrollador (`fetch`, `checkout`/`branch` solo si lo pide). No se mezcla
  con nada.
- **`target/debug/kiln` puede no coincidir con HEAD.** El binario es del 19-11 a
  las 23:18; el último commit, `305ec28`, es de las 23:19. T1 compila antes de
  medir. ASTRA usa ese binario (su `CLAUDE.md`, Commands): la fila de ASTRA
  recompila tras unir.

## Done means

- R1–R6 de `spec.md` con su prueba en verde. R7, en la fila de ASTRA.
- `cargo test --workspace` con lo de la línea base más las pruebas nuevas, 0
  fallos y ninguna prueba existente cambiada. Clippy `-D warnings`, limpio.
- La prueba `#[ignore]` de la cavidad real (R6) en verde, con su log en `review/`.
- Revisión de tres pasadas con veredicto en *Review*.

## Stop and ask if

- La autointersección (T6 + T7) no cabe en dos filas de menos de 100 líneas, o da
  rojo sobre la cavidad real: se para con una decisión (sacarla a otra cadena o
  ampliarla).
- La cavidad real da un rojo distinto del de autointersección (R6). No se ajusta
  la regla: se mide y se pregunta.
- Una prueba de K013 (`canonical_*`) cambia o falla.
- Hace falta rechazar claves desconocidas en `Spec`, o tocar `mesh::read` para
  algo más que contar las primitivas que salta.
- La pregunta 2 (severidad) se responde «aviso»: cambia T8 y la spec.
- La tarea necesita abrir ventanas o aplicaciones en la pantalla del
  desarrollador.

## Tasks

Estimaciones de `calibrate.sh ~/Kiln`. `implementer`, mediana real 0,9 min /
197 k, con factor ×0,45 y ×0,77 sobre la tabla S/M/L. `verifier`, 8 m / 700 k.
`integrator`, mediana de todos los repos: 5,8 m / 916 k.

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Línea base: `cargo build` (deja `target/debug/kiln` en HEAD), `cargo test --workspace`, clippy `-D warnings` y la prueba `--ignored` filtrada `canonical`, todo con salida a `baseline/` | `.sdlc/derived-mesh-verify/baseline/` | Recuentos frente a `CLAUDE.md` (657/0/29) | Done means | verifier | rama creada | 8m/700k | in progress |
| T2 | `Spec.mesh: Option<MeshRules>` (`components`, `genus`, `normals` como enum, `deny_unknown_fields`). En `spec::check`: `mesh_components` y `mesh_on_generator` | `crates/kiln-core/src/spec.rs` | 2: `an_existing_spec_has_no_mesh_rules` (lee `examples/specs/hoist.toml`, `None`, se serializa sin `mesh`); tabla `mesh_rules_are_checked` (clave desconocida → ilegible, `components = 0`, con `[generator]`) | R1 | implementer | T1 | 2m/200k | pending |
| T3 | Partir `check_i6_i7` en `check_i6` y `pieces_and_euler`, sin cambiar sus códigos ni sus detalles | `crates/kiln-core/src/canonical/topology.rs` | 1: `pieces_and_euler_counts_a_torus` (piezas 1, χ 0). Las `canonical_topology_*` y `tests/canonical_cases.rs`, intactas | R5 | implementer | T1 | 2m/200k | pending |
| T4 | `integrity.rs`, primera parte: colocación (un nodo, traslación) y modo de primitiva, rejilla con tolerancia, soldadura por primer uso y degenerados informados. Contar en `mesh::read` las primitivas que salta | `crates/kiln-core/src/integrity.rs` (nuevo), `lib.rs`, `mesh.rs` | 1 tabla: `offgrid`, área nula, triángulo que repite vértice, nodo girado, `mode = 1` y el octaedro soldado desde 3 primitivas | R2 | implementer | T2, T3 | 3m/300k | pending |
| T5 | `integrity.rs`, segunda parte: I1–I6 traducidos a `mesh_*`; piezas y χ frente a `components`/`genus`; volumen con signo frente a `normals` | `crates/kiln-core/src/integrity.rs` | 1 tabla: `removed`, `flipped`, `extra_component`, arista ×3, pellizco, todo invertido con `outward`, toro con `genus = 0`; positivos: octaedro y toro con `genus = 1` | R3 | implementer | T4 | 3m/300k | pending |
| T6 | Predicados exactos de autointersección en `i128`, portados de check.py `:58-136` (ASTRA) | `crates/kiln-core/src/integrity/intersect.rs` (nuevo) | 1 tabla de pares: se cruzan, se tocan en un borde, comparten arista plegada o no, comparten vértice con rayo dentro o fuera, coplanarios | R4 | integrator | T1 | 6m/900k | pending |
| T7 | Barrido de cajas ordenadas por x y regla `mesh_self_intersection` (primer par y número de pares) | `crates/kiln-core/src/integrity.rs` | 1: mutación `moved` → `mesh_self_intersection`; el octaedro y el toro sin cruces | R4 | implementer | T5, T6 | 2m/300k | pending |
| T8 | `validate::mesh_check` y su llamada en `verify_asset`, como check `mesh_integrity` tras `Check::own`. Documentar `[mesh]` en `README.md` y `skills/asset/SKILL.md` | `crates/kiln-core/src/validate.rs`, `crates/kiln-cli/src/verify.rs`, `README.md`, `skills/asset/SKILL.md` | 1 tabla: ficha existente sin `[mesh]` y GLB abierto → `None`; `[mesh]` y GLB abierto → `failed` con `mesh_open_edge`; `[mesh]` y octaedro → `passed` | R1, R3 | implementer | T7 | 3m/300k | pending |
| T9 | Cavidad real: prueba `#[ignore]` con `KILN_MESH_FIXTURE` (solo lee el GLB de ASTRA), con `genus = 1`/`inward` → sin hallazgos y con `genus = 0` → `mesh_topology`. Se corre una vez, con el log en `review/` | `crates/kiln-core/tests/integrity_fixture.rs` (nuevo) | 1: `the_published_cavity_passes_its_declared_topology` | R6 | integrator | T8 | 6m/900k | pending |
| T10 | Verificación: batería entera una vez, clippy, `--ignored` filtrada (`canonical`, `integrity`), `cargo build`. Dos flujos vecinos: `kiln verify` de un asset de `examples/` sin `[mesh]` (mismos checks que en la línea base) y `kiln check --project examples` sin hallazgos nuevos | — (logs en `review/`) | Salidas en archivo | Done means | verifier | T9 | 8m/700k | pending |
| T11 | Revisión: tres `reviewer` (bugs, seguridad, conformidad) y `review/findings.md` | `review/` | Hallazgos con archivo:línea | Done means | integrator | T10 | 15m/2M | pending |
| T12 | **ASTRA, después de entregar EST-004 y de unir esta cadena** (con `target/debug/kiln` recompilado). Añadir `~/Kiln/target/debug/kiln check --project .` antes de `build` en Commands de `CLAUDE.md`. `[mesh]` con `components = 1`, `genus = 1`, `normals = "inward"` en `kiln/specs/historic_cavity.toml`. Después, `kiln check`, `verify` (`mesh_integrity: pasa`) y `publish` con el GLB de bytes idénticos. Retirar de check.py lo que Kiln ya cubra **solo si el desarrollador lo aprueba** (pregunta abierta) | ASTRA `CLAUDE.md`, `kiln/specs/historic_cavity.toml`, `godot/assets/kiln/historic_cavity/asset.json` | El sha del GLB no cambia; `kiln verify` pasa; el check.py actual, en verde | R7 | integrator | T11, EST-004 entregada | 20m/1.5M | pending |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1 | 8m | revisar los recuentos de la línea base |
| 2 | T2, T3, T6 | 6m | `git diff --stat` |
| 3 | T4 | 3m | ídem |
| 4 | T5 | 3m | ídem |
| 5 | T7 | 2m | ídem |
| 6 | T8 | 3m | ídem |
| 7 | T9 | 6m | — |
| 8 | T10 | 8m | — |
| 9 | T11 | 15m | decisiones sobre los hallazgos |
| 10 | T12 (ASTRA, más tarde) | 20m | `git diff` de ASTRA |

- **Estimated wall-clock**: unos 54 min en Kiln más 20 en ASTRA, sin contar las
  comprobaciones de main.
- **Sequential sum**: 78 min.
- **Estimated tokens**: unos 8,3 M.
- **Tamaño**: unas 350–400 líneas nuevas sin pruebas, algo por encima de las
  250–350 de la propuesta. Se reparten así:
  - spec: unas 40;
  - topology: unas 20;
  - soldadura: unas 90;
  - reglas: unas 70;
  - predicados: unas 90;
  - barrido: unas 30;
  - verify: unas 30.

  La diferencia es la soldadura honesta: colocación, rejilla y degenerados
  informados en lugar de descartados.

## Order and dependencies

- T1 va primero: la línea base y el binario en HEAD.
- T2, T3 y T6 tocan archivos disjuntos.
- T4 necesita el tipo de T2 y las funciones de T3.
- T4, T5 y T7 comparten `integrity.rs`, así que van una detrás de otra. T6 vive
  en un submódulo propio.
- T8 cierra la integración.
- T12 espera dos cosas:
  - a que EST-004 se entregue en ASTRA. No se toca ASTRA mientras esté en curso
    (rama `est-004-historic-excavations`, con trabajo sin commit);
  - a que esta cadena se una a `main` de Kiln.

## Test strategy

TDD: cada fila escribe su prueba en rojo antes del código. Si una prueba falla,
se corrige el código.

- **Unit**: mallas mínimas en enteros (octaedro, toro, esfera UV de
  `topology.rs`). Las cinco mutaciones genéricas de check.py, más degenerado,
  arista ×3, pellizco y normales, en tablas.
- **Integration**: `validate::mesh_check`, con fichas y GLB sintéticos (T8). La
  cavidad real va en `#[ignore]` y no necesita Blender ni Godot (T9).
- **Manual verification**: el verifier compara el `validation.json` de un asset de
  `examples/` sin `[mesh]` con el de la línea base.

## Verification commands

```
cargo build
cargo test --workspace 2>&1 | tee .sdlc/derived-mesh-verify/review/final-test.log
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee .sdlc/derived-mesh-verify/review/final-clippy.log
KILN_MESH_FIXTURE=~/.t3/projects/astra/godot/assets/kiln/historic_cavity/historic_cavity.glb cargo test -p kiln-core --test integrity_fixture -- --ignored
```

## Implementation risks

- Si se parte `check_i6_i7`, `canonical::validate` y K013 deben seguir idénticos
  (R5).
- `mesh::read` lo usan `verify_recipe` y `check`: contar las primitivas que salta
  no debe cambiar lo que devuelve.
- `Spec` se serializa al editar materiales y piezas (`spec.rs:735-822`): `mesh`
  debe llevar `skip_serializing_if`.
- Con `[mesh]` en la ficha de ASTRA cambia el sha de la ficha y hay que
  republicar (T12).

## Review

Pendiente (T11).

## Estimate vs actual

Pendiente.

## Outside this plan

- Pregunta 2 (severidad): recomendado error. Pregunta 4 (escala): recomendado
  dejarla.
- Retirar comprobaciones de check.py en ASTRA: lo decide el desarrollador en T12.
- `derived_from`, `zones_match_normals`, varios nodos o giro y escala.
