# Spec: K016b, la página respeta la confianza de K014

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aaron, 2026-11-22

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | La página sabe si el proyecto que muestra es de confianza: `GET /api/session` añade `trusted` (bool). Confía si la persona lo avaló en este núcleo (`kiln serve --project`, `kiln mcp --project`, o Kiln lo creó en una carpeta vacía) o si está en `trusted.json`; la lista se lee en cada petición, así `kiln trust` cuenta al momento. | «la página sigue al proyecto … aunque la carpeta no sea de confianza» | Sesión MCP sin `--project` abre una carpeta fuera de la lista → `trusted: false`; tras `kiln trust --project` → `true` sin reabrir. `kiln serve --project` → `true`. |
| R2 | Con `trusted: false`, los POST de la página que lanzan Blender, Godot o Claude responden 403 con un mensaje que nombra `kiln trust --project <carpeta>`, y no lanzan nada ni crean trabajo. Son: `/api/build`, `/api/paint`, `/api/generate`, `/api/review`, `/api/godot`, `/api/import`, `/api/ask`, `/api/chat`, `/api/context/sort`, `/api/context/refine` (y `/api/chat` del asistente de juego nuevo). | «los POST de la página que lanzan Blender o Godot … pueden correr sobre un proyecto ajeno» | Tabla de rutas → 403 + «kiln trust»; el Godot falso del proyecto no deja su marca; `/api/jobs` sigue vacío. |
| R3 | La página lo dice: con `trusted: false` muestra un aviso fijo («Proyecto sin confianza: Kiln no lanzará Blender, Godot ni Claude aquí…» con la orden `kiln trust --project <carpeta>`), y el 403 llega como el error del botón pulsado. | «la página lo dice» (encargo) | Prueba unitaria de UI: la sesión con `trusted: false` pinta el aviso con la orden; con `true` no. |

## Design

- `core::Page` gana `vouched: bool`. `Shared::show(dir, ctx, vouched)`: con la
  misma carpeta el aval se conserva (`p.vouched || vouched`), así una segunda
  sesión sin aval no le quita a la página el que dio la persona; con otra
  carpeta se reemplaza. `Page::trusted()` = `vouched` o
  `registry::is_trusted(home, dir)` leído en ese momento.
- Avales, los mismos que el MCP: `Kiln::open` recibe `vouched` (true solo
  desde `--project`); `Kiln::made` pasa `fresh`; `serve::open_page` (de `kiln
  serve --project` y `Hello::Open`, que llega por el socket 0600 del usuario)
  pasa `true`; el `/api/new` de la página pasa `page.vouched || fresh`.
- `serve.rs`: una lista `RUNS` con las rutas de R2 y un brazo tras el de
  `from_page` en `handle` y `handle_new`: si la ruta está en `RUNS` y la página
  no confía → 403. El mensaje se dirige a la persona (el del MCP se dirige al
  agente). `session()` añade `trusted`.
- UI: `shell.js` guarda `trusted` de `/api/session` y pinta el aviso; la
  función que lo arma es pura y se prueba con `node --test`.
- Se mantiene el modelo del MCP: sin confianza se lee y se edita (fichas,
  materiales, contexto); solo se bloquea lanzar programas. Responde la
  pregunta abierta del intent.

## Policy applied

- **Personal data**: none.
- **Sensitive business data**: none.
- **Credentials**: none.
- **Decisions about people / high risk**: no. Es un control de seguridad local
  (qué programas se ejecutan), no decide sobre personas.

## Out of scope

- Apuntar en `trusted.json` lo que crea el asistente de la página en una carpeta
  vacía (el MCP sí lo hace en `made`); aquí el aval vale para la vida del núcleo.
- Botones deshabilitados uno a uno: basta el aviso y el 403 por botón.
- Las órdenes de terminal (`kiln build`…), que la persona lanza a mano.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Se olvida una ruta que lanza programas | El hueco sigue en esa ruta | Lista `RUNS` única y probada en tabla; revisión de seguridad repasa cada `agent::start`/`job_in` de `serve.rs`. |
| La persona que usa `kiln serve --project X` pierde el aval cuando una sesión reabre X | Página de solo lectura sin motivo | El aval se conserva con la misma carpeta (R1). |
| Rutas comparadas sin canonizar | Falso «sin confianza» | `trust`, `project_open` y `serve` ya canonizan; la prueba usa la ruta tal como la abre el MCP. |

---
Next stage: Build reads this file and writes `plan.md`.
