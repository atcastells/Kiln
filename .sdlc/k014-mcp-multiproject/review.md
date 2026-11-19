# Review: K014, MCP multiproyecto

## Verificado

- `cargo test --workspace --no-fail-fast`: 650 pasan, 0 fallan, 29 ignorados
  (antes 635; +15: 4 del registro, 1 de URIs `file://`, 8 de
  `tests/mcp_projects.rs`, 2 de cobertura de la CLI).
- `cargo clippy --workspace --all-targets -- -D warnings`: limpio.
- Sesión real por stdio con el binario de release, sin `--project`, arrancada
  desde una copia de `examples/`: `status` abre Puerto Niebla por la carpeta
  de trabajo; `project_create` crea Faro y lo deja abierto; `projects` lista
  los dos (Faro abierto); `project_open` vuelve a Puerto Niebla.
- Las roots del cliente se prueban con un cliente de test que declara `roots`
  y contesta `roots/list` desde una carpeta vacía y un registro vacío, de modo
  que solo las roots pueden abrir el proyecto.

## Cambios en tests existentes

- `tests/mcp_guards.rs`: la superficie fijada pasa de 31 a 35 herramientas
  (`content_manifest`, `project_create`, `project_open`, `projects`). Es el
  propósito de ese test: obliga a declarar cada herramienta nueva.
- `tests/common/mod.rs`: cada `kiln mcp` de test recibe un `KILN_HOME`
  temporal, para no escribir en el registro del usuario.

## No verificado

- Con Claude Code como cliente real (`claude mcp add --scope user`): no se
  lanzó un `claude -p` anidado desde esta sesión. Queda para probar a mano.
- Si Claude Code manda roots: no se midió. Sin ellas decide la carpeta de
  trabajo, que Claude Code fija en la del proyecto.
- Las roots están marcadas obsoletas en el protocolo (SEP-2577) en rmcp 3.8;
  se usan con `#[allow(deprecated)]` y la carpeta de trabajo cubre su falta.

## TDD

Los tests de integración se escribieron junto al código, no antes; el primer
`cargo test` de la batería nueva ya salió en verde, así que no hay un rojo
registrado.
