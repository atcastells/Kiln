# Plan: K014, MCP multiproyecto

- **Derived from**: `spec.md`.
- **Status**: approved por Aaron al aprobar la fase 1 (2026-11-19). Rama `claude/project-thread-ckdldl`.

| # | Tarea | Req. | Archivos | Prueba | Status |
|---|-------|------|----------|--------|--------|
| T1 | Registro de proyectos | R1 | `kiln-cli/src/projects.rs` | unitarios | done |
| T2 | Proyecto activo en `Kiln`; `projects`, `project_open`, `project_create`; `folder` en `status` | R2–R5 | `mcp/mod.rs`, `mcp/projects.rs`, `mcp/new_game.rs`, `mcp/read.rs` | `tests/mcp_projects.rs` | done |
| T3 | Arranque sin `--project` (roots, cwd, último) | R6 | `main.rs`, `mcp/mod.rs` | `tests/mcp_projects.rs` | done |
| T4 | `content_manifest` y cobertura de la CLI | R7, R8 | `mcp/production.rs`, `ops/production.rs`, `tests/coverage.rs` | `coverage.rs` | done |
| T5 | README y CLAUDE.md: registro de usuario sin `--project` | — | `README.md` | — | done |

Done means: `cargo test --workspace` y `clippy -D warnings` en verde, y una
sesión real por stdio que abre dos proyectos.
