# Plan: interfaz de Kiln fácil de usar, ágil y escalable (K003)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (2026-11-15)
- **Nothing is implemented until this file is approved.**

## Done means

`cargo test --workspace` y Clippy en verde; `scripts/ui_test.sh` en verde
(funciones puras, recorrido de regresión, teclado y presupuesto de
rendimiento); el recorrido sobre `examples`, `examples/content` y Astra
enseña todas las vistas que había; capturas de antes y después.

## Stop and ask if

- La base (U3) no se puede fijar sin perder trabajo de e10.
- Los módulos ES no cargan desde el binario sin un paso de compilación.
- Hace falta una dependencia nueva (Rust o JS).
- La maqueta no se aprueba.
- Un encargo pasa de 100 líneas nuevas: se parte antes.

## Tasks

### Fase 0: base y red de seguridad

| # | Task | Files | Test that proves it | Requirement | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|-------------|---------|---------|-----|--------|
| T0 | Fijar la base: worktree desde `main` con K002 y e10; línea base de tests y capturas de todas las vistas | — | tests en verde; capturas guardadas | R16 | main | — | 4m/400k | pending |
| T1 | Proyecto sintético: generador de un proyecto de 5.000 piezas y 2.000 hallazgos | `scripts/ui_fixture.sh` | `kiln check` sobre él termina | R15 | implementer | T0 | 3m/200k | pending |
| T2 | `scripts/ui_test.sh`: recorrido en Chrome sin ventana sobre la interfaz actual (cada vista carga, sin errores de consola) y medida de tiempos; guarda la línea base | `scripts/ui_test.sh`, `ui/tests/` | el script, y en rojo con un fallo provocado | R14, R15 | implementer | T1 | 5m/300k | pending |
| T3 | Maqueta navegable de la interfaz nueva (HTML estático con datos de ejemplo): inicio, navegación, lista grande, detalle, trabajo en curso, estados vacío y de error | scratch (se publica para aprobar) | aprobación del product owner | R10, U2 | implementer | T0 | 8m/400k | pending |

### Fase 1: cimientos

| # | Task | Files | Test that proves it | Requirement | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|-------------|---------|---------|-----|--------|
| T4 | Servir módulos: tabla embebida `/ui/*` con su tipo MIME y `--ui-dir` para desarrollo | `kiln-cli/src/serve.rs`, `main.rs` | test de respuesta; la página carga un módulo | R1 | implementer | T2 | 3m/250k | pending |
| T5 | `store.js` y `html` con escape por defecto | `ui/store.js`, `ui/html.js` | pruebas de funciones puras | R2, R12 | implementer | T4 | 3m/200k | pending |
| T6 | `api.js`: peticiones, errores uniformes y `ETag` | `ui/api.js` | pruebas | R7, R9 | implementer | T5 | 2m/200k | pending |
| T7 | `router.js`: rutas en el hash, atrás y adelante | `ui/router.js` | pruebas y recorrido | R3 | implementer | T5 | 2m/200k | pending |
| T8 | `index.js`: hallazgos por sujeto, piezas por kit, peor gravedad | `ui/index.js` | pruebas, incluida una de coste lineal | R4 | implementer | T5 | 2m/200k | pending |
| T9 | Servidor: caché del catálogo con huella de ficheros y `304` | `kiln-cli/src/serve.rs` | test con contador de lecturas | R7 | implementer | T4 | 3m/250k | pending |
| T10 | Servidor: `GET /api/jobs` | `kiln-cli/src/serve.rs` | test | R8 | implementer | T9 | 2m/150k | pending |

### Fase 2: piezas comunes y vistas

| # | Task | Files | Test that proves it | Requirement | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|-------------|---------|---------|-----|--------|
| T11 | Armazón: barra superior, navegación lateral con contadores, zona de trabajo y panel de detalle; estilos base | `ui/index.html`, `ui/app.js`, `ui/shell.js` | recorrido; captura | R10, R11 | implementer | T3, T7 | 4m/300k | pending |
| T12 | Lista virtual con orden, filtro y contador | `ui/ui/list.js` | pruebas y medida con 5.000 filas | R5 | implementer | T5 | 4m/300k | pending |
| T13 | Avisos, estado ocupado y diálogo propio | `ui/ui/feedback.js`, `ui/ui/dialog.js` | pruebas y recorrido de teclado | R9, R11 | implementer | T11 | 3m/250k | pending |
| T14 | Vista de inicio | `ui/views/home.js` | recorrido sobre `examples` y Astra | R10 | implementer | T8, T11 | 3m/250k | pending |
| T15 | Vista de catálogo (kits, piezas, materiales) sobre la lista virtual | `ui/views/catalog.js` | recorrido y medida | R5, R16 | implementer | T12 | 4m/300k | pending |
| T16 | Vista de contenido | `ui/views/content.js` | recorrido sobre `examples/content` | R16 | implementer | T12 | 2m/200k | pending |
| T17 | Vista de árbol: pinta solo lo visible y no recalcula aristas por nodo | `ui/views/tree.js` | recorrido y medida | R4, R16 | implementer | T11 | 5m/350k | pending |
| T18 | Vista de contexto y asistente de inicio (lo de e10 incluido) | `ui/views/context.js`, `ui/views/wizard.js` | recorrido | R16 | implementer | T13 | 5m/350k | pending |
| T19 | Ficha de asset, construcción y revisión | `ui/views/spec.js`, `ui/views/build.js` | recorrido | R16 | implementer | T13 | 5m/350k | pending |
| T20 | Trabajos: reenganche tras recargar, sondeo con espera creciente, registro que respeta el desplazamiento | `ui/views/job.js` | recorrido con recarga a mitad | R8 | implementer | T10, T13 | 3m/250k | pending |
| T21 | Visor 3D bajo demanda y liberado al cerrar | `ui/views/view3d.js` | abrir y cerrar 20 veces | R13 | implementer | T19 | 3m/200k | pending |
| T22 | Buscador con espera y paleta de comandos | `ui/ui/palette.js`, `ui/app.js` | prueba de teclado | R6 | implementer | T15 | 4m/300k | pending |
| T23 | Enlaces externos filtrados y repaso de escape | `ui/html.js`, vistas | test con `javascript:` | R12 | implementer | T19 | 2m/150k | pending |
| T24 | Retirar el fichero antiguo y repaso de accesibilidad a 900 px | `ui/index.html` | recorrido completo; revisión de roles | R11, R16 | implementer | T14 a T23 | 3m/250k | pending |

### Fase 3: cierre

| # | Task | Files | Test that proves it | Requirement | Runs on | Depends | Est | Status |
|---|------|-------|---------------------|-------------|---------|---------|-----|--------|
| T25 | Medida final contra la línea base y capturas de antes y después | — | `scripts/ui_test.sh`; tabla de tiempos | R15 | main | T24 | 3m/400k | pending |
| T26 | README | `README.md` | lectura contra el spec | R17 | main | T25 | 2m/600k | pending |
| T27 | Verificación: batería, recorrido, teclado, rendimiento y dos flujos vecinos (`kiln check`; Astra en el puerto 4331) | — | informe del verifier | todos | verifier | T26 | 10m/1.5M | pending |

## Schedule

Fase 0 en paralelo (T1 → T2 con T3). Fase 1 en serie por fichero; las
vistas de la fase 2 en paralelo de dos en dos cuando no comparten fichero.

- **Estimated wall-clock**: unos 80 min de encargos, más main; entre 2 y
  3 horas en la práctica
- **Estimated tokens**: unos 9M

## Test strategy

Primero la red: un recorrido automático sobre la interfaz de hoy, que la
nueva tiene que seguir pasando. Funciones puras probadas en una página de
pruebas sin Node. Medidas con el proyecto sintético antes y después. Rust
con sus tests de siempre.

## Verification commands

```
cargo test --workspace
cargo clippy --workspace --all-targets
scripts/ui_fixture.sh <dir>
scripts/ui_test.sh
target/debug/kiln serve --project examples/content --port 4399
```

## Implementation risks

- T17 y T18 son las vistas con más lógica heredada: si no caben, se parten.
- La interfaz antigua convive con la nueva hasta T24; hasta entonces se
  sirve la antigua por defecto.
- El Kiln del puerto 4331 sigue con el binario de K002 hasta que se
  relance.

## Outside this plan

- Mergear K002 y e10, y la PR de K003.
- Paginación en el servidor, si la medida la pide.

---
Next stage: Build implements, Test verifies, Deploy opens the PR.
