# Plan: K016b, la página respeta la confianza de K014

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aaron, 2026-11-22: «Aprobado, sigue»)
- **Nothing is implemented until this file is approved.**
- **Branch**: `k016-page-trust`, desde `origin/k016-shared-core`, en el worktree
  `~/Kiln-k016-page-trust`. PR contra `k016-shared-core`; no se une sin que
  Aaron lo diga.

## Done means

Una sesión MCP abre una carpeta sin confianza, `/api/session` dice
`trusted: false`, las diez rutas de R2 dan 403 con «kiln trust» sin lanzar nada,
y tras `kiln trust --project` la misma página lanza; la página pinta el aviso.
`cargo test --workspace` (694 + las nuevas, 0 fallan), `clippy -D warnings`
limpio y `scripts/ui_test.sh` en verde. PR abierto contra `k016-shared-core`.

## Stop and ask if

- Una ruta de la página lanza programas y no estaba en R2.
- Hace falta cambiar el formato de `trusted.json` o el protocolo `Hello`.
- Un test existente falla por algo ajeno a este cambio (medir antes sin él).
- The task needs to open windows or apps on the developer's screen.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Prueba en rojo: sesión MCP sin `--project` abre `slow_project` → `trusted: false`; tabla de las rutas de R2 → 403 con «kiln trust», sin marca `ran` ni trabajos; `kiln trust --project` → `trusted: true` y `/api/godot` lanza. Una aserción más en `the_page_follows_the_project_a_session_opens`: tras `kiln serve --project` → `trusted: true`. | `crates/kiln-cli/tests/core.rs` | `an_untrusted_project_is_read_only_on_the_page` (nueva) + aserción en la existente | R1, R2 | main | — | 10m/1.5M | done (cause: the positive case uses /api/generate; /api/godot measure does not call the fixture's fake Godot) |
| T2 | seam: el aval cruza núcleo, MCP y página y no compila por partes. `Page.vouched`, `Shared::show(.., vouched)`, `Page::trusted()`; `Kiln::open(.., vouched)` y sus llamadas; `made` pasa `fresh`; `open_page` pasa `true`; `/api/new` de la página `page.vouched \|\| fresh`; `RUNS` + brazo 403 en `handle` y `handle_new`; `trusted` en `session()`. | `crates/kiln-cli/src/core.rs`, `crates/kiln-cli/src/mcp/mod.rs`, `crates/kiln-cli/src/mcp/projects.rs`, `crates/kiln-cli/src/serve.rs` | T1 en verde | R1, R2 | main | T1 | 20m/3M | done (cause: none) |
| T3 | Aviso en la página: función pura que arma el aviso a partir de la sesión; `shell.js` guarda `trusted` y lo pinta; estilo. | `crates/kiln-cli/ui/app/ui/feedback.js`, `crates/kiln-cli/ui/app/shell.js`, `crates/kiln-cli/ui/app/index.html`, `crates/kiln-cli/ui/app/styles.css`, `crates/kiln-cli/ui/tests/unit/feedback.test.mjs` | `trust notice` en `feedback.test.mjs` (con `false` pinta la orden; con `true`, nada) | R3 | implementer | — | 2m/300k | done (cause: review added api.js for the 403 text and the no-project case) |
| T4 | Batería: `cargo test --workspace`, clippy, `scripts/ui_test.sh` (salida a fichero); luego `verifier` sobre el cambio y dos flujos vecinos (página con `kiln serve --project` sigue lanzando; asistente de juego nuevo en carpeta vacía). | — | — | R1–R3 | main + verifier | T2, T3 | 15m/2M | done (cause: ui_test.sh red on one baseline name, also red on the base without the change) |
| T6 | Arreglos de la revisión: el 403 llega al botón con el texto del servidor (`api.js`); sin proyecto no hay aviso; `kiln serve` sin `--project` no avala la carpeta actual (`Opts.vouched`, `Hello::Open.vouched`, `main.rs`). | `crates/kiln-cli/ui/app/api.js`, `crates/kiln-cli/ui/tests/unit/api.test.mjs`, `crates/kiln-cli/ui/app/ui/feedback.js`, `crates/kiln-cli/ui/app/shell.js`, `crates/kiln-cli/ui/tests/unit/feedback.test.mjs`, `crates/kiln-cli/src/main.rs`, `crates/kiln-cli/src/core.rs`, `crates/kiln-cli/src/serve.rs`, `crates/kiln-cli/tests/core.rs` | aserciones en `api.test.mjs`, `feedback.test.mjs` y `an_untrusted_project_is_read_only_on_the_page` (sin pruebas nuevas) | R1, R3 | main | T5 | — | done (cause: not in the plan; found by the review) |
| T5 | Revisión: tres `reviewer` (bugs, security, conformance) en paralelo; consolidar en `review.md`; push de la rama y PR contra `k016-shared-core`. | `.sdlc/k016-page-trust/review.md` | — | — | main + reviewer | T4 | 15m/3M | in progress |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T1, T3 | 10m | ver T1 en rojo por la razón esperada |
| 2 | T2 | 20m | `git diff --stat` contra Files |
| 3 | T4 | 15m | arreglar lo que salga |
| 4 | T5 | 15m | PR |

- **Estimated wall-clock**: ~60m
- **Sequential sum**: 62m
- **Estimated tokens**: ~9.8M

## Order and dependencies

T1 antes que T2 (TDD). T3 es independiente (solo UI, lee un campo nuevo que
el servidor aún no manda: con `undefined` no pinta nada). T4 necesita todo.

## Test strategy

- **Integration** (`tests/core.rs`, núcleo real con `KILN_HOME` temporal): R1 y
  R2 en una prueba con tabla de rutas; el Godot falso del proyecto prueba que
  no se lanzó nada.
- **Unit (UI)**: R3, la función pura del aviso.
- **Manual (verifier)**: la página real con un proyecto sin confianza muestra
  el aviso y el error del botón; con `kiln serve --project` sigue lanzando.
- Línea base de `tests/core.rs` medida antes de T1 en este árbol.

## Verification commands

```
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/ui_test.sh
```

## Implementation risks

- `Shared::show` cambia de firma: tres llamadas (MCP `show`, `open_page`,
  `/api/new`). `Kiln::open` tiene llamadas en `mod.rs` (`new`, raíces) y en
  `projects.rs` (`project_open`).
- K017 (PR #38) va encima de K016: al unir esto en `k016-shared-core`, el #38
  tendrá que traerlo; K017 no toca estas líneas, no se esperan conflictos.

## Review

Detalle en `review.md`.

- **bugs**: 1 important (aviso con orden vacía sin proyecto abierto), arreglado; 5 nits, 1 arreglado.
- **security**: 1 important (`kiln serve` sin `--project` avalaba la carpeta actual), arreglado; 3 nits.
- **conformance**: 1 important (el 403 no llegaba al botón con su texto, R3), arreglado; 3 nits.

## Estimate vs actual

Sin `actuals.jsonl` en este worktree: el hook `task-actuals` no dejó medidas,
así que no hay desviación medida que apuntar. Fuera de estimación: T6, nacida
de la revisión.

## Outside this plan

- Apuntar en `trusted.json` los proyectos que crea el asistente de la página
  (hoy solo los avala para la vida del núcleo).
