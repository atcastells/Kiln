# Plan: validación de contenido genérica, declarada por el juego (K002)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (2026-11-15)
- **Nothing is implemented until this file is approved.**

## Done means

En un worktree de `origin/main`: `cargo test --workspace` y
`cargo clippy --workspace --all-targets` en verde; el test con Godot
(`--ignored`) pasa; `grep -ri astra crates/ --include=*.rs` solo da
resultados en tests; `kiln check --project examples` da la misma salida
que antes; en Astra, `scripts/kiln_check.sh` pasa leyendo
`content/astra/modules.ron` y falla con una escena rota y con un conector
cambiado; la interfaz enseña el panel «Contenido».

## Stop and ask if

- La prueba de riesgo (T1) dice que `ron` no sirve y el lector propio
  pasa de 150 líneas.
- Hace falta una dependencia nueva.
- Un test que ya existía falla por algo que no sea el cambio de nombre de
  códigos o de sección.
- Hay que tocar el checkout principal de `~/Kiln` o su trabajo sin
  commitear.
- Un encargo pasa de 100 líneas nuevas de código.

## Tasks

| # | Task | Files | Test that proves it | Requirement | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|-------------|---------|---------|-----|--------|
| T0 | Worktree desde `origin/main` (tras `git fetch`), rama `k002-content-schema`; línea base: tests, Clippy y salida de `kiln check --project examples` guardada | — | comandos en verde | R10 | main | — | 3m/300k | done |
| T1 | Prueba de riesgo: leer con `ron` la fixture de K001 y el fichero de Astra a un árbol de valores sin tipos por juego | scratch (informe) | informe con el código probado | R3 | implementer | T0 | 3m/200k | done |
| T2 | Tipos del esquema en la configuración y su validación (clave desconocida = error) | `kiln-core/src/content/schema.rs`, `config.rs` | unitarios | R1, R2, R10 | implementer | T1 | 2m/200k | done |
| T3 | Árbol de valores y lector de RON: lista de variantes, y registro con `root` y `default_kind` | `content/value.rs`, `content/parse.rs` | unitarios con los dos ficheros | R3 | implementer | T1 | 3m/250k | done |
| T4 | Índice de elementos e IDs duplicados | `content/mod.rs` | unitarios | R5 | implementer | T2, T3 | 2m/150k | done |
| T5 | Referencias por esquema: desconocida, tipo equivocado, campos distintos | `content/refs.rs` | un test por regla, esquema inventado | R2, R5 | implementer | T4 | 2m/200k | done |
| T6 | Referencias externas: asset en el catálogo (con `greybox`) y escena existente | `content/refs.rs` | unitarios | R2, R5 | implementer | T5 | 2m/150k | done |
| T7 | Manifiesto v2: tipos, carga y comparación de IDs, escena y metadatos de nodo | `content/manifest.rs` | unitarios | R5 | implementer | T4 | 3m/200k | done |
| T8 | Marcadores: sublistas contra nodos hijos | `content/markers.rs` | unitarios | R4 | implementer | T7 | 2m/200k | done |
| T8b | Reglas de campo: obligatorio, tipo, rango, lista cerrada y `within` para ítems de lista; tabla de compatibilidad aplazada: ningún contenido usa todavía clases de conector | `content/fields.rs`, `content/schema.rs` | un test por regla | R12 | implementer | T4 | 3m/250k | done |
| T9 | `content::run` en `open_with`, `Catalog.content`, y retirada de `astra/` | `content/mod.rs`, `lib.rs`, `catalog.rs`, borrar `astra/` | integración sobre la fixture | R1, R7 | implementer | T6, T8, T8b | 3m/250k | done |
| T10 | Fixture migrada a `examples/content` y test de paridad con K001 | `examples/content/**`, test en `content/mod.rs` | paridad caso a caso | R7 | implementer | T9 | 2m/200k | done |
| T11 | Script de Godot genérico y `kiln content-manifest [--set]` | `kiln-cli/godot/content_manifest.gd`, `kiln-cli/src/content.rs`, `main.rs` | test `#[ignore]` con Godot | R6 | implementer | T9 | 4m/250k | done |
| T12 | API: los conjuntos de contenido en el catálogo servido | `kiln-cli/src/serve.rs` | test de la respuesta | R8 | implementer | T9 | 2m/150k | done |
| T13 | Interfaz: panel «Contenido» | `kiln-cli/ui/index.html` | comprobación manual con captura | R8 | implementer | T12 | 4m/250k | done |
| T14 | README: «Validar contenido» | `README.md` | comandos ejecutados | R11 | main | T11 | 2m/800k | done |
| T15 | Astra: esquema en `kiln.toml`, borrar la vista y su test, adaptar `scripts/kiln_check.sh` | repo de Astra | el script, en verde y en rojo (escena y conector) | R9 | implementer | T11 | 3m/200k | done |
| T16 | Verificación: batería, Godot, fixture, Astra, y dos flujos vecinos (`kiln check` y `kiln serve` sobre `examples`) | — | informe del verifier | todos | verifier | T13, T15 | 8m/1.5M | done |

## Schedule

Un solo crate casi todo el tiempo: en serie. T7 → T8 puede ir en paralelo
con T5 → T6 solo si se comprueba que no comparten fichero de tests.

- **Estimated wall-clock**: unos 55 min de encargos, más main
- **Estimated tokens**: unos 6M

## Test strategy

TDD en cada encargo de Rust. Paridad con K001 como red de seguridad: los
36 tests de K001 se trasladan al motor nuevo con sus códigos equivalentes.
Godot real en el test `#[ignore]`, ejecutado a mano en T11 y T16.

## Verification commands

```
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p kiln-cli -- --ignored content
target/debug/kiln check --project examples
target/debug/kiln content-manifest --project examples/content
target/debug/kiln check --project examples/content
(en Astra) KILN=<binario nuevo> scripts/kiln_check.sh
```

## Implementation risks

- T9 borra `astra/`: los tests de K001 tienen que estar ya trasladados.
- T15 cambia Astra: se hace en su rama de trabajo, después de cerrar la
  tarea 004.
- El Kiln arrancado en el puerto 4331 usa el binario de K001; al terminar
  hay que relanzarlo con el nuevo o Astra dejará de validar.

## Outside this plan

- Mergear a `main` y ordenar el checkout con `e10-wizard-chat`.
- Abrir la PR: la abro al cerrar, la apruebas tú.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
