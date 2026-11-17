# Plan: K006, entrega A — Kiln (R1–R10)

- **Derived from**: `spec.md` (aprobado 2026-11-17)
- **Status**: APPROVED BY ENGINEER (2026-11-17), con P1; P2 = (a), hecho; P3 = rama `k006-art-pipeline`
- **Nothing is implemented until this file is approved.**

La entrega B (la puerta en ASTRA, R11–R14) tendrá su propio `plan-b.md`
cuando esta cierre y estén resueltas D8 y D9.

## Decisiones de este plan que necesitan tu visto bueno

- **P1. Quién exporta los `.blend` de los kits.** El spec no añade un verbo
  para eso. Propongo que lo hagan `build` (las piezas que usa el asset) y el
  trabajo que hoy mide con Godot (`kiln godot`, `godot_measure`), que pasa a
  medir también lo que solo abre Blender. El nombre se queda corto, pero no
  hay verbo nuevo.
- **P2. El adaptador de `gltf_validator` no se puede probar aquí**: no está
  instalado. Opciones: (a) lo instalas tú y lo pruebo de verdad, o (b) lo
  escribo contra el formato de informe documentado y queda como «sin
  verificar» en el handoff. Recomiendo (a); con (b) no lo doy por probado.
  **Hecho (a)**: `gltf_validator` 2.0.0-dev.3.10 en `~/.local/bin`, compilado
  para arm64 desde la etiqueta `2.0.0-dev.3.10` con Dart 2.19.6 (el binario
  publicado es solo x86_64 y aquí no hay Rosetta).
- **P3. Rama.** Todo va en una rama nueva desde `main`, que creas tú o me
  pides que cree, antes de T0.

## Done means

- `cargo test --workspace` y
  `cargo clippy --workspace --all-targets -- -D warnings` en verde, sin haber
  tocado ningún test existente.
- `cargo test -p kiln-cli -- --ignored` en verde con Blender 5.2.3 y Godot
  4.7.1: los dos de hoy más los nuevos (anclaje, pieza `.blend`, verificar,
  dos montajes con el mismo hash estructural).
- En una copia de `examples`: `kiln tools`, `build`, `verify` y `publish` de
  `hoist` por CLI y por MCP; `validation.json` y `asset.json` escritos; un
  GLB truncado no se publica.
- `kiln check` sobre `examples` da los mismos hallazgos antes y después.
- README y test de cobertura MCP con `tools`, `verify` y `publish`.
- El `verifier` ha pasado.

## Stop and ask if

- Un test existente falla por algo ajeno al cambio.
- Dos montajes seguidos no dan el mismo hash estructural tras ordenar y
  redondear (R8): habría que redefinir «equivalente».
- Mover `production.rs` o `review.rs` al lanzador cambia lo que dicen o
  devuelven.
- Hace falta una dependencia de Rust que no sea `sha2`.
- Godot sin ventana no puede cargar un GLB en un proyecto temporal sin
  importarlo antes en el editor.
- Algo obliga a tocar ASTRA.
- The task needs to open windows or apps on the developer's screen.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T0 | seam: configuración y esqueleto, para que las tareas siguientes no compartan ficheros. `[tools] timeout_s`, `gltf_validator`, `gltf_transform`; `[anchors.<class>]` (`fits`, `opening_m`); `[publish] dir`; dependencia `sha2`; módulos vacíos `tools`, `validate`, `asset` en `lib.rs` y `run` en `main.rs` | `crates/kiln-core/src/config.rs`, `crates/kiln-core/src/lib.rs`, `crates/kiln-core/Cargo.toml`, `crates/kiln-cli/src/main.rs`, `crates/kiln-cli/src/run.rs`, `crates/kiln-core/src/{tools,validate,asset}.rs` | Tests de `config`: valores por defecto y un `kiln.toml` sin las secciones nuevas se lee igual | R10 | main | — | 5m/2M | done (cause: none; 2 tests de `config`) |
| T1 | Detección: dada una ruta, decir si existe, su versión (salida de `--version` de Blender, Godot, `gltf_validator`, `gltf-transform`, `claude`, `curl`) y si es anterior a la mínima probada | `crates/kiln-core/src/tools.rs` | Unit: salidas reales de `--version` pegadas como texto; binarios falsos (scripts en una carpeta temporal): presente, ausente, viejo | R1 | implementer | T0 | 2m/250k | done |
| T2 | El lanzador: proceso con carpeta de trabajo, plazo (mata al vencer), registro completo a fichero y resultado con código, duración y resumen del error | `crates/kiln-cli/src/run.rs` | Unit con `sh -c`: uno que duerme se corta y lo dice; uno que falla deja el registro; uno bueno devuelve su salida | R2 | implementer | T0 | 2m/250k | done |
| T3 | Nodos del GLB: nombre, padre, transformación local, extras, y triángulos y materiales por malla | `crates/kiln-core/src/glb.rs` | Unit sobre un GLB escrito en el test: dos nodos anidados con extras | R4 | implementer | T0 | 2m/250k | done (triángulos por malla: lógica propia, la de `from_doc` no se podía reutilizar sin tocarla) |
| T4 | Anclajes en la ficha: `[[anchors]]` (`id`, `class`, `at`, `turn`), ids planos y sin repetir | `crates/kiln-core/src/spec.rs` | Unit: se lee y se escribe igual; id repetido o no plano es un hallazgo; una ficha sin anclajes se serializa como hoy | R6 | implementer | T0 | 2m/250k | done |
| T5 | `status` (MCP) lleva las herramientas; `kiln tools` las imprime | `crates/kiln-cli/src/ops/read.rs`, `crates/kiln-cli/src/main.rs`, `crates/kiln-cli/tests/mcp_read.rs` | Test nuevo en `mcp_read.rs`: `status` trae `tools` con Blender ausente cuando `[tools] blender` apunta a nada | R1 | implementer | T1 | 2m/250k | done |
| T6 | Blender y Godot pasan por el lanzador en montar, pintar, medir, revisar y exportar contenido; los registros van a `<build>/<id>/.kiln/logs/` | `crates/kiln-cli/src/production.rs`, `crates/kiln-cli/src/review.rs`, `crates/kiln-cli/src/content.rs` | Batería de hoy y los dos ignorados en verde; un `[tools] blender` falso que duerme con `timeout_s = 1` da el error de plazo (test nuevo en `tests/run_timeout.rs`) | R2 | main | T2 | 6m/3M | done (cause: none; el error de Blender añade la ruta del registro) |
| T7 | Hash estructural: nodos por nombre, jerarquía, transformaciones a 1e-4, triángulos y materiales por malla | `crates/kiln-core/src/glb.rs` | Unit: dos GLB con los nodos en otro orden dan el mismo hash; mover un nodo 1 mm lo cambia | R4 | implementer | T3 | 2m/250k | done |
| T8 | Hallazgos del GLB en sí: no es GLB, referencias rotas (búfer, imagen, material, nodo), nombres repetidos o vacíos, escala negativa o no uniforme en la raíz | `crates/kiln-core/src/validate.rs` | Un test por hallazgo, con GLB rotos escritos en el test | R5 | implementer | T3 | 3m/350k | done |
| T9 | Hallazgos de `check` para anclajes: clase que no está en `kiln.toml`; asset cuya ficha no cabe en `opening_m` de su clase | `crates/kiln-core/src/check.rs` | Tests de `check`: los dos hallazgos, y ninguno en un proyecto sin `[anchors]` | R6 | implementer | T4 | 2m/250k | done (la llamada va en `lib.rs`, tras leer los montajes) |
| T10 | El plan de montaje lleva los anclajes y `assemble.py` los exporta como vacíos `anchor_<id>` | `crates/kiln-core/src/build.rs`, `crates/kiln-cli/blender/assemble.py`, `crates/kiln-cli/tests/blender_anchor.rs` | Ignorado, con Blender: el GLB lleva `anchor_mount` en su posición y giro. Rojo primero | R6 | main | T4, T6 | 6m/3M | pending |
| T11 | Hallazgos del GLB contra su ficha: material fuera de la paleta, triángulos sobre el presupuesto, medidas que no cuadran, pieza pendiente (`kiln_todo`), anclaje que falta | `crates/kiln-core/src/validate.rs` | Un test por hallazgo | R5 | implementer | T8, T4 | 3m/350k | pending |
| T12 | Catálogo: un `.blend` en un kit es una pieza, con el sha256 del `.blend`; sus medidas salen del GLB de la caché si corresponde a ese sha256, y si no consta desfasada | `crates/kiln-core/src/catalog.rs` | Unit con una caché escrita en el test: al día, desfasada, sin exportar | R3 | implementer | T3 | 3m/350k | pending |
| T13 | Exportar los `.blend`: `export_piece.py` (ajustes fijos, solo mallas, transformaciones aplicadas) y su llamada desde `build` y desde el trabajo de medir (P1) | `crates/kiln-cli/blender/export_piece.py`, `crates/kiln-cli/src/production.rs`, `crates/kiln-cli/tests/blender_piece.rs` | Ignorado, con Blender: el test crea un `.blend` con una cámara y un modificador, y la pieza entra con sus medidas; tocar el `.blend` la marca desfasada | R3 | main | T6, T12 | 7m/3M | pending |
| T14 | El contrato: `asset.json` a partir de la ficha, el catálogo, `sources.json`, `report.json`, los hashes y las versiones de las herramientas | `crates/kiln-core/src/asset.rs` | Unit: cada campo sale de su fuente; una pieza de fuera trae licencia y autor; sin `sources.json` constan como propios | R8 | implementer | T7, T12 | 3m/350k | pending |
| T15 | `build` y `paint` escriben `asset.json`; `spec_get` devuelve el contrato y el último informe | `crates/kiln-cli/src/production.rs`, `crates/kiln-cli/src/ops/specs.rs`, `crates/kiln-cli/tests/blender_contract.rs` | Ignorado: dos montajes seguidos de `hoist` dan el mismo hash estructural. Test de `spec_get` con un `asset.json` de fixture | R8 | main | T13, T14 | 6m/3M | pending |
| T16 | Informe de verificación: estructura de `validation.json`, veredicto, y lectura del informe de `gltf_validator` (P2) | `crates/kiln-core/src/validate.rs` | Unit: errores → no válido; avisos → válido; validador ausente → válido y consta | R7 | implementer | T11 | 2m/250k | pending |
| T17 | `import_check.gd` y la operación `verify`: R5, `gltf_validator` si está, y Godot cargando el GLB en un proyecto temporal de la caché | `crates/kiln-cli/godot/import_check.gd`, `crates/kiln-cli/src/ops/production.rs`, `crates/kiln-cli/tests/godot_verify.rs` | Ignorado, con Godot: un GLB bueno pasa; uno truncado falla con el motivo | R7 | main | T15, T16 | 7m/3M | pending |
| T18 | Publicar, la parte de ficheros: solo con `validation.json` válido y del GLB actual; copia GLB, variantes y `asset.json` a `[publish] dir`/`<id>/`; la versión sube solo si cambia el sha256; destino confinado al proyecto | `crates/kiln-cli/src/ops/publish.rs`, `crates/kiln-cli/src/ops/mod.rs` | Unit: sin validación no publica; GLB cambiado después no publica; dos veces lo mismo no sube la versión; `dir = "../fuera"` se rechaza | R9 | implementer | T16 | 3m/350k | pending |
| T19 | `verify` y `publish` como trabajos MCP y subcomandos; `publish` hace importar a Godot y juzga el resultado | `crates/kiln-cli/src/mcp/production.rs`, `crates/kiln-cli/src/main.rs`, `crates/kiln-cli/src/ops/publish.rs`, `crates/kiln-cli/tests/mcp_production.rs`, `crates/kiln-cli/tests/coverage.rs` | Tests MCP: las dos herramientas están, con sus anotaciones, y rechazan ids no planos; cobertura en verde | R7, R9, R10 | main | T17, T18 | 6m/3M | pending |
| T20 | README: entrega K006 (herramientas, anclajes, piezas `.blend`, verificar, contrato, publicar, tests ignorados) y tabla MCP | `README.md` | Revisión | R10 | implementer | T19 | 2m/250k | pending |
| T21 | Regresión y recorrido: hallazgos de `examples` antes (guardados en T0) y después; batería, Clippy, ignorados; recorrido de «Done means» en una copia de `examples`, por CLI y por MCP | — (scratchpad) | Lo de «Done means» | todos | main | T19 | 8m/3M | pending |
| T22 | Verificación: build, batería, el recorrido, y dos flujos de al lado: `kiln review` sin `claude` (hojas y medidas) y `content-manifest` con Godot | — | Informe del verifier | todos | verifier | T21, T20 | 5m/1.3M | pending |

## Schedule

| Wave | Tasks (in parallel) | Longest | Then (main) |
|------|---------------------|---------|-------------|
| 1 | T0 | 5m | hallazgos de `examples` guardados; batería |
| 2 | T1, T2, T3, T4 | 2m | `git diff --stat`, leer los diffs |
| 3 | T5, T6, T7, T8, T9 | 6m | batería e ignorados |
| 4 | T10, T11, T12 | 6m | `git diff --stat` |
| 5 | T13, T16 | 7m | ignorados |
| 6 | T14, T18 | 3m | `git diff --stat` |
| 7 | T15 | 6m | hash estructural medido |
| 8 | T17 | 7m | — |
| 9 | T19 | 6m | batería |
| 10 | T20, T21 | 8m | — |
| 11 | T22 | 5m | leer el informe |

T7 y T3 tocan `glb.rs`, y T8, T11 y T16 tocan `validate.rs`: van en olas
distintas.

- **Estimated wall-clock**: unos 61 min, más las comprobaciones entre olas.
- **Sequential sum**: 89 min.
- **Estimated tokens**: unos 28M (los 8 de `main` pesan 23M).

## Order and dependencies

T0 primero para que nadie más toque `config.rs`, `lib.rs` ni `Cargo.toml`.
El lanzador (T2, T6) antes de cualquier script nuevo, para que todos nazcan
con plazo y registro. La lectura del GLB (T3) antes del hash, la validación
y el catálogo de `.blend`. El contrato (T14) necesita hash y piezas con su
sha256. Verificar antes que publicar: publicar lee `validation.json`.

## Test strategy

TDD: test en rojo antes de implementar. Si falla, se arregla el código.

- **Unit**: detección, lanzador, GLB, hash, hallazgos, anclajes, catálogo de
  `.blend`, contrato, informe, publicar.
- **Integración**: tests MCP sin herramientas externas; cinco tests
  `#[ignore]` con Blender o Godot reales, corridos por la orquestación tras
  las olas 3, 5, 7, 8 y al final.
- **Manual verification**: T21 y lo que mire el verifier.

## Verification commands

```
cargo build --workspace
cargo test --workspace
cargo test -p kiln-cli -- --ignored
cargo clippy --workspace --all-targets -- -D warnings
```

## Implementation risks

- `production.rs` lo tocan T6, T13 y T15, y `main.rs` T0, T5 y T19: siempre
  en olas distintas.
- T6 cambia cómo se lanzan Blender y Godot para todo lo que ya funciona. Los
  mensajes de error de hoy los leen la página y el MCP: se conservan.
- El catálogo corre en cada petición de la página: el sha256 de los `.blend`
  (T12) no puede romper el presupuesto de rendimiento de K003; se reutiliza
  su caché.
- `spec.rs` serializa las fichas que reescribe `spec_write`: una ficha sin
  anclajes tiene que salir idéntica (T4).

## Review

Filled in Deploy. A plan is not closed while this section is empty.

## Estimate vs actual

Filled at Build close from `.sdlc/k006-art-pipeline/actuals.jsonl`.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

- La entrega B: R11–R14, la puerta y ASTRA (`plan-b.md`).
- Lo que el spec deja fuera: glTF-Transform, puente MCP de Blender,
  animación, texturas.
- Los `.blend` en la interfaz web (vista, botones): la página los ve como
  una pieza más, sin nada propio.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
