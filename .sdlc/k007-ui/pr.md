# PR: la UI de un juego en Kiln (K007)

- **Deriva de:** `plan.md`, aprobado por el ingeniero el 2026-11-17. Spec con dos enmiendas.
- **Rama:** `k007-ui`, que parte de `d2cece3` (K006, ya fusionada en `origin/main` con el PR #23).
- **Estado:** listo para revisión humana. **Nadie aprueba su propio PR.**

## Qué hace

Kiln lee la UI de un juego de Godot sin abrir Godot. Sin `[ui]` en `kiln.toml`, todo funciona como antes.

- **Catálogo** (`catalog what=ui`): las variaciones del tema con su herencia, sus colores, su tamaño y dónde se usan, más las fuentes y los colores del tema.
- **Cuatro comprobaciones**, cada una con nivel `warn`, `error` u `off` (por defecto `warn`):
  - contraste WCAG: el texto con contorno se mide contra el contorno, el resto contra parejas declaradas, y un fondo translúcido vale el peor caso entre negro y blanco;
  - glifos, contra la tabla `cmap` de las fuentes;
  - tema coherente: variaciones que faltan o que nadie usa;
  - estilo escrito fuera del tema.
- **Objetivo de UI en `status`:** los criterios son afirmaciones fijadas del contexto; su estado sale de `ui_target.toml` o de las comprobaciones, y `status` da `met`, `total`, `percent` y los criterios abiertos.
- **Rutas:** todas quedan confinadas a la raíz del proyecto.

## Cambios

- `crates/kiln-core/src/ui/`, nuevo: `theme.rs`, `cmap.rs`, `contrast.rs`, `rules.rs`, `target.rs` y `mod.rs` (`inside`).
- `crates/kiln-core/src/config.rs` (`[ui]`), `check.rs` (llama a las reglas) y `lib.rs`.
- `crates/kiln-cli/src/ops/read.rs` (`status` y `catalog what=ui`) y `mcp/read.rs` (descripción de la herramienta).
- Pruebas y fixtures: `crates/kiln-core/tests/fixtures/ui/`, con IBM Plex Mono y su `OFL.txt`, y `crates/kiln-cli/tests/routes.rs` (la expectativa de `settle`).
- Ejemplo: Puerto Niebla con `[ui]`, tema, `hud.gd`, fuente, `ui_target.toml` y los criterios C1–C3.
- `README.md`: «Entrega 13: la UI del juego».
- Cadena `.sdlc/k007-ui/`: intent, spec, plan, review y pr.

## Verificación

- `cargo test --workspace`: 400 pruebas en verde, 0 fallos. `cargo clippy --workspace --all-targets -- -D warnings`: limpio.
- **Sobre ASTRA** (su `kiln.toml` con `[ui]` y su `kiln/ui_target.toml`, en el commit de ASTRA que acompaña a este):
  - `check` da 0 avisos y `status` da `ui_target` 12/15, con C4, C6 y C15 abiertos;
  - los contrastes coinciden con la tabla corregida de `.sdlc/ui-target/spec.md` de ASTRA a ±0,1;
  - un «⚠» en un texto y `HudTime` sin uso dan su aviso, y el objetivo baja a 10/15;
  - `scripts/kiln_check.sh` sigue en OK.
- **Revisión:** verifier y tres pasadas, en `review.md`. Todos los IMPORTANT están arreglados o aceptados en la spec.

## Riesgos

- El servidor MCP en marcha usa el binario anterior hasta que se reinicie.
- Quedan abiertos un escaneo de usos repetido y la falta de límite de tamaño en las lecturas de UI (ver `review.md`).
- `split_writes_the_zones_file` es una prueba inestable, ajena a esta tarea.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
