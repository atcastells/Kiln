# Plan: K016, núcleo compartido

- **Derived from**: `spec.md`.
- **Status**: hecho en la rama `k016-shared-core`.

| # | Tarea | Req. | Archivos | Prueba | Status |
|---|-------|------|----------|--------|--------|
| T1 | Lista de trabajos en disco | R3 | `jobs.rs`, `agent.rs` | `jobs.rs` | done |
| T2 | Trabajos con proyecto y bloqueo por proyecto | R5 | `agent.rs`, `ops/production.rs` | `tests/core.rs` | done |
| T3 | Núcleo: cerrojo, socket, sesiones MCP, salida por inactividad, `kiln stop` | R2, R7, R8 | `core.rs`, `main.rs` | `tests/core.rs` | done |
| T4 | `kiln mcp` como puente | R1, R4 | `core.rs`, `mcp/mod.rs` | `tests/core.rs`, `tests/common` | done |
| T5 | Página de proyecto cambiante, `/api/session`, lista por proyecto | R6 | `serve.rs`, `ui/app/shell.js`, `ui/app/api.js` | `tests/core.rs`, Chrome | done |
| T6 | D1: `review` sin segundo Claude, `review_submit`, prompt | R9 | `review.rs`, `ops/production.rs`, `mcp/production.rs`, `mcp/prompts.rs` | `review.rs`, `mcp_production.rs` | done |
| T7 | README, CLAUDE.md | — | — | — | done |
