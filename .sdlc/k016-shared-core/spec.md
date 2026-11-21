# Spec: K016, núcleo compartido

- **Derived from**: `intent.md` y la fase 2 del plan.
- **Decisiones** tomadas aquí:
  - El núcleo es `kiln serve`: un proceso por `KILN_HOME`, que sirve la página
    en 127.0.0.1 y las sesiones MCP por un socket Unix `KILN_HOME/core.sock`
    (0600). Nada nuevo escucha en red.
  - Cada conexión al socket es una sesión MCP completa (rmcp sobre el socket),
    con su proyecto activo propio, como en K014; comparte trabajos y página.
  - La página muestra el último proyecto que abrió cualquier sesión.
  - D1: por MCP revisa el agente del cliente; el revisor propio de Kiln queda
    como opción (`reviewer = "kiln"`) y en la página.

## Requirements

| # | Requisito | Aceptación |
|---|-----------|------------|
| R1 | `kiln mcp` es un puente por stdio: se conecta al núcleo de `KILN_HOME` o lo arranca en segundo plano (su propio grupo de procesos), le pasa `--project`, `--config` y la carpeta de trabajo, y copia el protocolo en los dos sentidos. Sin carpeta de Kiln, fuera de Unix, con `KILN_CORE=off` o si el núcleo no arranca en 15 s, sirve en su propio proceso como antes. | `tests/core.rs`; todos los tests de MCP pasan por el núcleo |
| R2 | Un solo núcleo por `KILN_HOME` (cerrojo `core.lock`); `core.json` dice pid, puerto y versión. | `tests/core.rs` |
| R3 | Los trabajos se guardan en `jobs.json` mientras cambian (cada 0,4 s como mucho) y se cargan al arrancar; uno que estaba en marcha vuelve como fallido y lo dice. Los ids no cambian. | `jobs.rs`, `tests/core.rs` |
| R4 | Un trabajo lanzado por MCP sigue al cerrarse el cliente; otra sesión lo ve con `job` y `jobs`. | `tests/core.rs` |
| R5 | Bloqueo por proyecto: un solo trabajo de escritura (montar, pintar, generar, verificar, publicar, revisar, medir, traer, manifiesto) a la vez en un proyecto, venga de donde venga; `review_submit` también espera. | `tests/core.rs` |
| R6 | La página sigue al núcleo: `GET /api/session` da `rev` y carpeta; la página la consulta cada 4 s, se recarga si cambia y refresca la lista de trabajos, que muestra los del proyecto venga de donde venga. Sin proyecto abierto, la página lo dice. Las escrituras de la página llevan `x-kiln-rev`: si el núcleo ya muestra otro proyecto, se rechazan con 409 en vez de caer en él. | `tests/core.rs`, prueba en Chrome |
| R7 | `kiln serve --project DIR` con un núcleo en marcha le pide mostrar `DIR` y sale. `kiln stop [--force]` lo cierra guardando los trabajos; sin `--force` se niega si hay trabajos en marcha. | `tests/core.rs`, `coverage.rs` |
| R8 | Un núcleo arrancado por `kiln mcp` se cierra tras 30 min sin sesiones, sin trabajos en marcha y sin peticiones de la página (`KILN_CORE_IDLE_S`); si su puerto está ocupado sigue sin página. | manual, tests con `KILN_CORE_IDLE_S=3` |
| R9 | D1: `review` por MCP monta y saca las hojas de una vuelta sin revisor; `review_submit` recibe la revisión en TOML, aplica sus correcciones a la ficha y guarda `review.json`; `next_round` monta la siguiente. `reviewer = "kiln"` mantiene el revisor propio. El prompt `kiln-review` guía al cliente como revisor. | `review.rs`, `mcp_production.rs` |

## Security

El socket está en la carpeta de Kiln del usuario con permisos 0600: solo el
mismo usuario, que ya puede ejecutar `kiln trust`, puede abrir una sesión. La
página sigue en 127.0.0.1 con la cabecera `x-kiln` para escribir. El saludo
del socket se lee con límite de 64 KB. `review_submit` limita la revisión a
100.000 caracteres y valida el asset como el resto.
