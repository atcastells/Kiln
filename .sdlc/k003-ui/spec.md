# Spec: interfaz de Kiln fácil de usar, ágil y escalable (K003)

- **Derived from**: `intent.md`
- **Status**: approved (2026-11-15)
- **Approved by**: Aaron Castells

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | La interfaz se parte en módulos ES servidos por Kiln, sin compilación ni dependencias nuevas: estado, cliente de API, rutas, y un módulo por vista | escalable | `index.html` queda como armazón; cada vista en su fichero; el binario los sigue embebiendo |
| R2 | Estado único con suscripciones; una vista se repinta solo cuando cambia lo que usa | ágil | escribir en el buscador no repinta el árbol ni el panel lateral |
| R3 | Rutas en la URL (`#/vista/elemento`): enlace directo, atrás y adelante, y recargar conserva la vista y la selección | se pierde el sitio | prueba en navegador sin ventana |
| R4 | Índices calculados una vez por catálogo (hallazgos por sujeto, piezas por kit); ninguna vista recorre todos los hallazgos por fila | no escala | prueba con catálogo sintético: tiempo lineal |
| R5 | Listas y tablas con ventana virtual, orden, filtro y contador; se acaba el corte silencioso de 80 materiales | no escala | 5.000 filas: menos de 60 nodos en el DOM y desplazamiento fluido |
| R6 | Buscador con espera de 120 ms y resultados agrupados; paleta de comandos con Ctrl/Cmd+K para ir a una vista, una pieza o una acción | fácil, ágil | prueba de teclado |
| R7 | El servidor guarda el catálogo en memoria y lo invalida por fecha de los ficheros; responde `304` con `ETag` cuando no cambió | ágil | dos peticiones seguidas: la segunda no relee el disco (contador en test) |
| R8 | Trabajos: `GET /api/jobs` lista los activos; la interfaz se vuelve a enganchar tras recargar; sondeo con espera creciente; el registro solo se desplaza si el usuario está al final | calla, se pierde el sitio | recargar a mitad de un trabajo lo sigue mostrando |
| R9 | Respuesta a toda acción: estado ocupado visible con el motivo, botones deshabilitados con explicación, avisos que se cierran, errores con qué hacer | calla | ninguna acción queda sin respuesta en la prueba de recorrido |
| R10 | Pantalla de inicio: qué tiene el proyecto, qué falla (por gravedad) y siguiente paso sugerido; estados vacíos con una acción | primera pantalla | en Astra, el inicio enseña el contenido validado y no un árbol vacío |
| R11 | Accesible: controles nativos, foco visible, `aria-live` en avisos y registros, diálogo propio en lugar de `prompt()`, contraste AA, usable a 900 px | controles | revisión automática de roles y recorrido solo con teclado |
| R12 | Enlaces externos solo con esquema `http` o `https`; todo dato del proyecto pasa por escape | riesgo | test con una URL `javascript:` |
| R13 | El visor 3D se carga bajo demanda y libera su contexto al cerrar | ágil | abrir y cerrar 20 veces sin agotar contextos |
| R14 | Pruebas de interfaz que corren sin Node: funciones puras en una página de pruebas y un recorrido en Chrome sin ventana, lanzados por un script | escalable | `scripts/ui_test.sh` en verde y en rojo con un fallo provocado |
| R15 | Presupuesto de rendimiento medido con un proyecto sintético de 5.000 piezas y 2.000 hallazgos | ágil | primera vista en menos de 300 ms, filtro en menos de 50 ms, medido por el script |
| R16 | Nada de lo que hay se pierde: contexto, árbol, contenido, catálogo, fichas, construcción, revisión, 3D y el asistente de inicio | — | recorrido de regresión sobre `examples` y `examples/content` |
| R17 | README: cómo está organizada la interfaz, cómo añadir una vista y cómo probarla | escalable | README revisado |

## Design

- **Ficheros** bajo `crates/kiln-cli/ui/`: `index.html` (armazón y
  estilos base), `app.js` (arranque), `store.js`, `api.js`, `router.js`,
  `index.js` (índices derivados), `ui/` (lista virtual, diálogo, avisos,
  paleta) y `views/` (inicio, contexto, árbol, contenido, catálogo, ficha,
  trabajo, 3D, asistente). Se sirven en `/ui/<ruta>` desde una tabla
  embebida con `include_str!`; en desarrollo, `kiln serve --ui-dir RUTA`
  los lee del disco para no recompilar.
- **Sin marco ni compilación.** Módulos ES nativos y plantillas con una
  función `html` pequeña que escapa por defecto. Es lo que mantiene a
  Kiln como un solo binario.
- **Estado.** Un almacén con claves (`catalog`, `context`, `route`,
  `query`, `job`, `selection`) y suscripción por clave. Las vistas son
  funciones de esas claves.
- **Servidor.** Caché del catálogo con huella de los ficheros del
  proyecto; `ETag`; `GET /api/jobs`; sin cambiar el formato de las
  respuestas que ya existen.
- **Visual.** Se conservan la paleta y el tono actuales. Cambia la
  jerarquía: barra superior con proyecto, buscador y estado; navegación
  lateral con contadores; zona de trabajo; panel de detalle plegable.
  Antes de programar las vistas se entrega una maqueta navegable para
  aprobarla.

Decisiones, como propuestas:

- **U1** Varios ficheros de interfaz en lugar de uno, siempre embebidos y
  sin Node.
- **U2** La maqueta se aprueba antes de construir las vistas.
- **U3** Base: `main` con K002 y e10 ya mergeados. Si no lo están, se
  construye sobre una rama que los integre y se dice.
- **U4** Tamaño objetivo: 5.000 piezas y 2.000 hallazgos.
- **U5** Sin paginación en el servidor todavía: la ventana virtual en el
  cliente basta para ese tamaño; se mide y se decide con el dato.

## Policy applied

Sin datos personales, de negocio ni credenciales. El servidor sigue
escuchando solo en `127.0.0.1` y exigiendo su cabecera en las escrituras.
Entrada externa: datos del proyecto y de sitios de assets que llegan a la
página; todo se escapa y los enlaces se filtran por esquema.

## Out of scope

Aplicación de escritorio, edición de contenido desde la interfaz, grafo
editable, servidor MCP, y cambios en lo que Kiln valida.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| K002 y e10 tocan el mismo fichero y no están en `main` | conflictos o trabajo perdido | U3; el primer encargo integra y fija la base |
| Reescribir 900 líneas sin pruebas rompe algo que nadie nota | regresión | R14 antes que nada: recorrido de regresión sobre la interfaz actual, que después debe seguir pasando |
| Sin datos grandes reales, los objetivos son supuestos | optimizar lo que no duele | proyecto sintético y medida antes y después |
| La maqueta retrasa | tiempo | es un encargo, y evita rehacer vistas |
| Módulos ES desde un binario: rutas y tipos MIME | la página no carga | prueba de riesgo en el primer encargo de código |

---
Next stage: Build reads this file and writes `plan.md`.
