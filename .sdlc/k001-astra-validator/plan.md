# Plan: Validar contenido de ASTRA con Kiln (K001)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aarón, 2026-11-15)
- **Base**: rama `k001-astra-validator` desde `origin/main` `f167020` (worktree en `.claude/worktrees/k001-astra-validator`)
- **Nothing is implemented until this file is approved.**

## Hecho significa

- Con el `examples/kiln.toml` actual, `kiln check --project examples` da la
  misma salida y el mismo código de salida que en la línea base (T0),
  comparadas con `diff`.
- `kiln check --project examples/astra` sale con 0 y sin ningún `ERROR`.
- Una copia de la fixture con un ID duplicado, una referencia rota y un RON
  mal formado sale con 1, y cada hallazgo lleva su `code` `astra_*` y
  `<fichero>#<id>`.
- `kiln astra-manifest --project examples/astra` regenera con Godot 4.7 un
  JSON igual al que está en el repo. Cambiar `astra_connects` de la puerta en
  el `.tscn`, exportar y validar da `astra_manifest_mismatch`.
- Un unit test por cada código de la spec (11), además del caso válido.
- README con la sección ASTRA y los comandos que de verdad se ejecutaron.
- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`
  en verde.

## Parar y preguntar si

- Al empezar Build seguimos en `e9-new-game` con la entrega 10 sin
  commitear: K001 necesita su propia rama, que crea Aarón.
- La línea base (T0) ya está roja sin el cambio.
- `ron` 0.12 no da línea y columna en sus errores, o su forma de escribir
  los enums no encaja con el esquema de la spec.
- Godot 4.7 no expone los metadatos de los nodos de una `PackedScene` sin
  instanciarla, y hay que instanciar escenas con scripts.
- Meter `astra::run` en `open_with` cambia la salida de `examples`.
- Un test que ya existe falla por algo ajeno al cambio.

## Tareas

Cada encargo cubre un punto y menos de 100 líneas nuevas. `astra` es un
módulo con carpeta (`astra/mod.rs`, `refs.rs`, `manifest.rs`), para que las
tareas no se pisen el fichero.

| # | Tarea | Ficheros | Test que lo prueba | Requisito | Corre en | Depende | Est | Estado |
|---|-------|----------|--------------------|-----------|----------|---------|-----|--------|
| T0 | Línea base: commit, `cargo test --workspace`, salida y código de `kiln check --project examples` guardados en el scratchpad | — | Los comandos en verde y su salida guardada | R1 | main | — | 2m/500k | done (cause: none; base f167020, 41 tests, check exit 0, clippy clean) |
| T1 | `IntegrationsSection` y `AstraSection` (`enabled`, `definitions`, `godot_manifest`, `greybox`) en `Config` | `kiln-core/src/config.rs` | Unitario: sin sección da `None`; con sección, los campos; `greybox` y `enabled` son `false` por defecto | R1, R6 | implementer | T0 | 1m/150k | done (cause: none) |
| T2 | Dependencia `ron`; `astra/mod.rs` con `enum Definition`, `parse_file` → definiciones o `astra_ron_syntax` (línea:columna); `refs.rs` y `manifest.rs` vacíos y declarados | `kiln-core/Cargo.toml`, `kiln-core/src/astra/mod.rs`, `astra/refs.rs`, `astra/manifest.rs` (nuevos), `kiln-core/src/lib.rs` | Unitario: el RON de la spec da 5 definiciones; uno roto da `astra_ron_syntax` con el fichero como subject y la línea en el mensaje | R2, R3, R4 | implementer | T0 | 2m/200k | done (cause: ron 0.12 error API is span.start.line/col, not position) |
| T3 | `refs::check`: `astra_duplicate_id`, `astra_unknown_ref`, `astra_wrong_kind`, `astra_door_endpoint` | `kiln-core/src/astra/refs.rs` | Unitario: uno por código, más el caso válido sin hallazgos | R4 | implementer | T2 | 2m/200k | done (cause: none; non-test code slightly over 100 lines) |
| T4 | `refs::check_external`: `astra_asset_missing` (Error, o Warn con greybox) contra las claves de pieza del catálogo; `astra_scene_missing` bajo `tools.godot_project` | `kiln-core/src/astra/refs.rs` | Unitario: asset ausente con `greybox` true/false; escena que existe y que no (carpeta temporal) | R4, R5, R6 | implementer | T1, T3 | 2m/200k | done (cause: none; red run shown, 5 failed then green) |
| T5 | `manifest.rs`: tipos del JSON, `load` (`astra_manifest_missing`, `astra_manifest_syntax`), `compare` (`astra_manifest_mismatch` por ID, escena del sector y `connects`) | `kiln-core/src/astra/manifest.rs` | Unitario: falta el fichero, JSON roto, ID solo en un lado, escena distinta, `connects` distinto, y caso coincidente sin hallazgos | R8 | implementer | T2 | 2m/200k | done (cause: no red run, compensated by a mutation check in main: 6 tests fell) |
| T6 | `astra::run` (globs, `astra_no_definitions`, une T3–T5) llamado desde `open_with` tras `check::run` solo con la sección activa; fixture `examples/astra/` (kiln.toml, RON, kit con LICENSE.md, proyecto Godot con dos `.tscn` y metadatos, manifiesto escrito a mano) | `kiln-core/src/astra/mod.rs`, `kiln-core/src/lib.rs`, `examples/astra/**` | Integración: abrir `examples/astra` no da ningún `Error`; `diff` de `kiln check --project examples` contra T0 vacío | R1, R3, R4 | main | T1, T4, T5 | 4m/1.5M | done (cause: kits placed outside the Godot project so Godot does not import GLBs into the fixture; R1 diff identical; broken copy exits 1 with 8 errors) |
| T7 | `astra_manifest.gd` y `kiln astra-manifest` (escenas de los `Sector` → Godot headless → `godot_manifest`) | `kiln-cli/godot/astra_manifest.gd` (nuevo), `kiln-cli/src/production.rs`, `kiln-cli/src/main.rs` | Test `#[ignore]` con Godot: regenerar da el JSON del repo; cambiar `astra_connects` → `astra_manifest_mismatch`. Ejecutado a mano en local | R7, R8 | main | T6 | 6m/2M | done (cause: exporter in new kiln-cli/src/astra.rs instead of production.rs (already 300+ lines); RON loading extracted to astra::load_files and astra::scenes; Godot test green first try; committed manifest is now the one Godot wrote) |
| T8 | Sección ASTRA del README: configuración, esquema provisional, exportar, validar, códigos | `README.md` | Los comandos del README, copiados y ejecutados, dan lo que dice | R9 | main | T7 | 1m/300k | done (cause: none) |
| T9 | Verificación de punta a punta, más dos flujos vecinos: `kiln check` y `kiln godot` sobre `examples`, y los hallazgos en `kiln serve` | — | Informe del verificador | todos | verifier | T8 | 4m/900k | done (cause: amber, 12 broken cases plus a byte comparison against a build of f167020; fixed in main with red tests: manifest mismatch cascade after a broken RON, res:// with "..", godot_manifest outside the project; other NITs listed under "Fuera de este plan") |

## Calendario

| Tanda | Tareas (en paralelo) | La más larga | Después (main) |
|-------|----------------------|--------------|----------------|
| 0 | T0 | 2m | guardar la línea base |
| 1 | T1, T2 | 2m | batería entera, `git diff --stat` |
| 2 | T3, T5 | 2m | batería entera, `git diff --stat` |
| 3 | T4 | 2m | batería entera |
| 4 | T6 | 4m | `diff` contra T0 |
| 5 | T7 | 6m | exportación real con Godot |
| 6 | T8 | 1m | — |
| 7 | T9 | 4m | leer el informe |

- **Reloj estimado**: 21 min, más las comprobaciones entre tandas
- **Suma secuencial**: 25 min
- **Tokens estimados**: ~6,2M (casi todo en T6 y T7, en main)

Estimaciones: la mediana real de cada ejecutor en todo el histórico
(implementer 0,7m/125k, main 2,2m/1,7M, verifier 4m/905k), redondeada al
alza según el tamaño de la tarea.

## Orden y dependencias

T0 va antes que nada, porque R1 se demuestra con un `diff` contra él. T1 y
T2 tocan ficheros distintos y van juntas. T3 y T5 solo necesitan los tipos
de T2, y van en ficheros separados. T4 escribe en el mismo `refs.rs` que T3
y lee el `greybox` de T1. T6 une todo y crea la fixture. T7 necesita la
fixture para exportar de verdad.

## Estrategia de test

TDD: el test en rojo antes de implementar. Si un test falla, se arregla el
código, no el test.

- **Unitarios**: el parseo, cada uno de los 11 códigos, `greybox` con sus
  dos valores y el caso válido sin hallazgos. RON y JSON en cadenas, y
  carpeta temporal solo para escenas y manifiesto.
- **Integración**: `open` de `examples/astra` sin errores; `open` de
  `examples` igual que en T0.
- **Con Godot** (`#[ignore]`, a mano): exportar la fixture y comparar; y la
  discrepancia tras cambiar la puerta.
- **Verificación manual** (T9): la fixture rota en una copia temporal, con
  la salida distinta de cero; y los dos flujos vecinos.

## Comandos de verificación

```
cargo build --workspace
cargo test --workspace
cargo test -p kiln-cli -- --ignored astra     # requiere Godot
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p kiln-cli -- check --project examples/astra
cargo run -p kiln-cli -- astra-manifest --project examples/astra
```

## Riesgos de implementación

- `open_with` lo usan `check`, `index`, `serve` y el asistente
  (`new_game.rs:214`): la llamada nueva va detrás de un `if let` sobre la
  sección. Sin la sección no se ejecuta nada nuevo.
- En la base `f167020`, `open_with` ordena siempre los hallazgos al final
  (`lib.rs:52-54`): los de ASTRA salen ordenados como los demás.
- El texto de `.tscn` con metadatos escrito a mano puede no ser lo que
  Godot guardaría. T7 lo comprueba abriendo las escenas con Godot de
  verdad.
- La columna de 34 caracteres del `subject` en `kiln check` se queda corta
  con `<fichero>#<id>`. Solo afecta a la alineación, no se cambia.

## Estimación frente a real

Se rellena al cerrar Build con `.sdlc/k001-astra-validator/actuals.jsonl`.

| # | Corre en | Est | Real | Desviación | Causa |
|---|----------|-----|------|------------|-------|
| T1 | implementer | 1m/150k | 0,4m/112k | −60 % / −25 % | ninguna |
| T2 | implementer | 2m/200k | 0,8m/153k | −60 % / −24 % | ninguna (sorpresas menores del API de ron 0.12) |
| T3 | implementer | 2m/200k | 0,85m/24k* | — | sin línea en `actuals.jsonl`; tiempo del informe del agente |
| T4 | implementer | 2m/200k | 0,5m/23k* | — | ídem |
| T5 | implementer | 2m/200k | 0,5m/21k* | — | ídem |
| T9 | verifier | 4m/900k | 7m/1330k | +75 % / +48 % | 12 casos rotos y una compilación aparte de `f167020` para comparar bytes |

\* Medidas del informe del propio agente (duración y tokens de subagente), no
del hook: el hook no registró T3–T5. Las tareas de main (T0, T6–T8) no tienen
línea en `actuals.jsonl`.

Desviación > 50 %: T1 y T2 (por debajo, tareas pequeñas y bien acotadas) y
T9 (por encima, más casos rotos de los previstos).

## Fuera de este plan

- Validación incremental o selectiva (`kiln check --only astra`).
- Una vista propia de ASTRA en `kiln serve`.
- Detectar un manifiesto desfasado.
- Lo que excluye el §6 del encargo.
- NITs del verificador sin arreglar: globs de `definitions` con `..` o rutas
  absolutas (el mismo fichero escrito de dos formas sale como duplicado);
  `manifest::load` llama «ausente» a cualquier fallo de lectura; el test con
  Godot compara el manifiesto parseado, no los bytes (el verificador sí
  comparó bytes: idénticos); sin prueba en Windows.

---
Siguiente etapa: Build implementa, Test verifica, Deploy abre el PR.
