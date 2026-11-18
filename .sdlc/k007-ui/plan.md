# Plan: la UI de un juego en Kiln (K007)

- **Derived from**: `spec.md`, aprobada por el PO el 2026-11-17.
- **Status**: APPROVED BY ENGINEER (Aaron Castells, 2026-11-17)
- **Nothing is implemented until this file is approved.**
- **Rama**: `k007-ui` en `~/Kiln` (parte de `d2cece3`, el último commit de K006). En ASTRA, `task-008-economic-circuit` en el árbol principal. Al terminar, todo queda commiteado en esos dos checkouts principales, no en worktrees (petición del originador).

## Done means

- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings` en verde en Kiln. Cada prueba nueva estuvo en rojo antes de implementarla.
- Con `[ui]` en el `kiln.toml` de ASTRA (`acc541f` más T8):
  - `check` da los contrastes de la tabla *Contraste de partida* de `.sdlc/ui-target/spec.md` de ASTRA, con ±0,1;
  - no hay avisos de glifos, tema coherente ni estilo fuera del tema;
  - `status` da `ui_target` 12/15, con C4, C6 y C15 abiertos.
- Un «⚠» en un texto de prueba da un aviso de glifo, y `HudTime` añadida al tema sin uso da un aviso de tema coherente. Se comprueba en una copia temporal, nunca en el árbol de ASTRA.
- `README.md` de Kiln con la sección de UI.
- Un verifier y tres pasadas de revisión (bugs, seguridad, conformidad) en `review.md`.
- Commits con confirmación: uno en Kiln (`k007-ui`) y uno en ASTRA, cada uno solo con sus ficheros.

## Stop and ask if

- El `.tres` de ASTRA tiene algo que el lector por texto no entiende y haría falta Godot.
- Una cifra de contraste no coincide con la tabla de ASTRA en más de 0,1: primero se revisa la medida a mano.
- Hace falta una dependencia nueva de Rust.

## Tasks

| # | Task | Files | Test that proves it | Requirement (spec) | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|--------------------|---------|---------|-----|--------|
| T1 | Sección `[ui]` en la configuración: tema, scripts, escenas, `target`, `contrast` (umbrales, parejas) y nivel de cada regla, por defecto `warn` | `crates/kiln-core/src/config.rs` | Prueba: un `kiln.toml` sin `[ui]` carga igual; con `[ui]`, los valores y los niveles por defecto | R7, R8 | implementer | — | 10m/400k | done (rojo de compilación y después verde; ~136 líneas con pruebas) |
| T2 | Lector del tema `.tres` por texto: fuentes, cajas de estilo y variaciones con herencia | `crates/kiln-core/src/ui/theme.rs` (nuevo), `crates/kiln-core/tests/fixtures/ui/theme.tres` | Prueba con un tema de ejemplo: colores, `base_type` encadenado, cajas con alfa; un `.res` da error legible | R1 | implementer | T1 | 15m/600k | done (rojo y después verde; ~180 líneas; la prueba contra un tema real solo corre con `KILN_EXTERNAL_THEME`, sin rutas de la máquina) |
| T3 | Lector de la tabla `cmap` (formatos 4 y 12) | `crates/kiln-core/src/ui/mod.rs` y `ui/cmap.rs` (nuevos; `pub mod ui` en `lib.rs`), fuente de prueba OFL con su licencia en `tests/fixtures/ui/` | Prueba: «✓» y «×» presentes y «⚠» ausente en IBM Plex Mono | R3 | implementer | — | 15m/500k | done (rojo y después verde; fuente de prueba de 136 KB con `OFL.txt`) |
| T4 | Contraste WCAG con alfa, contorno automático y parejas | `crates/kiln-core/src/ui/contrast.rs` | Prueba: #E7EDE8/#111D26 da 14,4; #DD8179/#081015 da 6,8; el contorno heredado de `HudLabel` se aplica a `HudDenied`; la pareja `non_text` usa 3:1 | R2 | implementer | T2 | 10m/400k | done (rojo y después verde. Enmienda: un fondo semitransparente se compone sobre negro y sobre blanco y vale el peor. Decisión del PO al ver que la tabla de ASTRA se había medido como opaca; prueba en rojo primero) |
| T5 | Las cuatro reglas en `check`: contraste, glifos (literales de scripts y `text` de escenas), tema coherente y estilo fuera del tema, cada una con su nivel | `crates/kiln-core/src/check.rs`, `ui/rules.rs`, `tests/fixtures/ui/` | Prueba con un proyecto de ejemplo: un hallazgo por regla cuando se provoca, ninguno cuando no; `level="error"` hace fallar `check`; `off` no avisa | R2–R5, R7 | implementer | T2–T4 | 20m/800k | done (rojo y después verde; ~190 líneas + 90 de pruebas; `off` silencia también el aviso informativo; limitaciones del escáner anotadas para la revisión) |
| T6 | `ui_target`: leer `ui_target.toml` y las afirmaciones `Cn ·`; marcar los `kiln:<regla>` con las reglas; `status` con `ui_target`; `catalog what=ui` | `crates/kiln-core/src/ui/target.rs`, `crates/kiln-cli/src/ops/read.rs` | Prueba: 3 criterios de ejemplo (uno de Kiln que falla, uno `po` cumplido, uno `test` pendiente) dan 1/3 y la lista de abiertos | R6, R9 | implementer | T5 | 15m/600k | done (desviación: la prueba no fue en rojo primero; se comprobó rompiendo el código a propósito, mutación detectada; ~8 líneas de escaneo duplicadas en `rules.rs`) |
| T7 | Ejemplo y documentación: `[ui]` en `examples/kiln.toml` con un tema mínimo; sección «UI» del README | `examples/`, `README.md` | `kiln check --project examples` sin errores nuevos; el README describe `[ui]`, D1 y D2 | R8, R9 | main | T6 | 10m/300k | done (ejemplo Puerto Niebla con `[ui]`, tema, `hud.gd`, fuente OFL, `ui_target.toml` y C1–C3 en `look`; `check --project examples` igual antes y después, 0 avisos. README «Entrega 13». Desviación: tocó las pruebas de `ops/read.rs` y `tests/routes.rs`, que suponían el ejemplo sin `[ui]` (aprobado por el coordinador). Batería: 377 en verde, Clippy limpio) |
| T8 | ASTRA: `[ui]` en `kiln.toml` (tema, scripts y escena de UI, parejas de la tabla de contraste, umbrales 4,5/3), `kiln/ui_target.toml` con C1–C15 (marca, estado y evidencia de la spec de ASTRA) | `~/.t3/projects/astra/kiln.toml`, `~/.t3/projects/astra/kiln/ui_target.toml` | La señal de cierre de *Done means* con el binario de K007; `scripts/kiln_check.sh` sigue en «OK» | R2–R6 | main | T6 | 15m/500k | done (`[ui]`, 10 parejas y `kiln/ui_target.toml` en ASTRA. Copia limpia: 0 avisos de UI y `ui_target` 12/15, C4, C6 y C15 abiertos. Provocado: «⚠» da `ui-glyph`, `HudTime` da `ui-variation-unused` y el objetivo baja a 10/15. Contrastes de peor caso = tabla corregida de ASTRA) |
| T9 | Verifier, tres pasadas de revisión, `review.md`, `pr.md` y commits con confirmación | `.sdlc/k007-ui/` | `review.md` sin IMPORTANT abiertos; `cargo test` y Clippy en verde | todos | main | T7, T8 | 20m/800k | done (verifier ámbar y tres pasadas en `review.md`. Los IMPORTANT, arreglados en los encargos A y B con prueba en rojo o mutación, o aceptados en la enmienda 2 de la spec. Después: 400 pruebas en verde y Clippy limpio; aceptación sobre ASTRA 12/15 y contrastes a ±0,1; commits pedidos por el ingeniero) |

## Order and dependencies

T1 → T2 → T4 → T5 → T6 → T7 y T8 → T9. T3 no depende de nada y puede ir en paralelo con T2.

## Test strategy

- **Pruebas unitarias en `kiln-core`**, con fixtures pequeños en `crates/kiln-core/tests/fixtures/ui/` (tema, escena, scripts y una fuente OFL). Ninguna prueba lee ASTRA, para que Kiln siga siendo genérico.
- **Rojo primero** en cada tarea, y cada agente corre solo sus pruebas. La batería entera (`cargo test --workspace` y Clippy) la corre la orquestación una vez por tarea.
- **ASTRA solo como aceptación** (T8): se ejecuta el binario sobre el proyecto y se comparan las cifras. Los casos provocados («⚠», `HudTime`) se prueban en una copia temporal.

## Verification commands

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
target/debug/kiln check --project ~/.t3/projects/astra
```

Y la herramienta `status` de Kiln por MCP, sobre ASTRA.

## Implementation risks

- **Herencia del `.tres`:** `HudConfirmed` hereda de `HudLabel`, y el lector debe resolver la cadena y detectar ciclos.
- **La fuente de prueba sube el tamaño del repo** (unos 100 KB): se usa un subconjunto si existe uno con licencia OFL. Si no, la fuente entera con su `OFL.txt`.
- **El servidor MCP en marcha usa el binario anterior.** Para probar `status` por MCP hay que reiniciarlo. Si no se puede desde aquí, se prueba con la CLI y se anota.
- **Otra sesión trabaja en ASTRA**, en su propio worktree. T8 solo toca `kiln.toml` y `kiln/ui_target.toml` del árbol principal, que su rama no toca (comprobado el 2026-11-17).

## Estimate vs actual

Se rellena al cerrar Build.

| # | Runs on | Est | Actual | Deviation | Cause |
|---|---------|-----|--------|-----------|-------|

## Outside this plan

- Entregas 4 y 5: el revisor visual de UI y las fichas de componente.
- Arreglar C4, C6 y C15 en ASTRA.
- Una vista nueva en la página de Kiln.
