# Review: K016, núcleo compartido

## Verificado

- `cargo test --workspace`: 682 pasan, 0 fallan, 30 ignorados (antes 667).
  Todos los tests de MCP pasan ya por el puente y un núcleo propio (cada test
  con su `KILN_HOME`, página en un puerto libre y `KILN_CORE_IDLE_S=3`).
- `cargo clippy --workspace --all-targets -- -D warnings`: limpio (arreglados
  dos avisos nuevos de clippy 1.97 en `projects.rs` y `mcp_guards.rs`).
- `scripts/ui_test.sh` con Chromium: 476 comprobaciones y 15 de recetas pasan.
  Falla la comparación con la línea base en «Contexto: search "electricidad"
  narrows (4 dimmed)», que ahora sale con 7: pasa igual en `main` sin estos
  cambios, así que la línea base está desfasada (no se toca aquí).
- El criterio de la fase, de punta a punta con el binario: una sesión MCP
  lanza `generate` en un proyecto con un Godot falso de 25 s y se mata el
  cliente; la página del núcleo muestra «ocupado: generar rock» y el trabajo
  en curso (`shots/page-shows-mcp-job.png`). En `tests/core.rs` lo mismo con
  una sesión nueva que encuentra el trabajo terminado.
- La página abierta se recarga sola en el proyecto que crea otra sesión con
  `project_create` (`shots/page-follows-project.png`).

## Pendiente

- El revisor de la página sigue siendo el propio de Kiln (`claude -p`): la
  página no tiene agente al que pasarle las hojas.
- Por MCP, al acabar las vueltas sin aprobar no se restaura la mejor vuelta
  como hace el revisor propio; `review.json` dice cuál es (`best`).
- Windows: sin socket Unix, `kiln mcp` sigue en su propio proceso.
