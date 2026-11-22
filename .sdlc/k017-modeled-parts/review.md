# Revisión K017: piezas modeladas por el MCP

Tres pasadas en paralelo (bugs, seguridad, conformidad) y el `verifier`.
Ningún agente pudo escribir su informe en disco (sin herramienta de
escritura). La sesión principal los consolida aquí y ha abierto cada
fichero:línea citado.

## Importantes, todos arreglados

| # | Pasada | Hallazgo | Arreglo | Prueba |
|---|---|---|---|---|
| I1 | verifier, seguridad | `kiln build` no aplicaba R6: una ficha que `check` rechaza se montaba igual (`segments = 200000` → GLB de 800.020 triángulos; `segments = 2` → traza de Blender). Una pieza que no es `model` con `shapes` también llegaba a Blender | `build::refuse` antes de `build::plan` (`crates/kiln-cli/src/production.rs`), que es la única entrada de la CLI, la herramienta `build` y la página | `build::tests::build_refuses_shapes_outside_the_limits` (rojo → verde, `review/T10-gate-*.log`); con el binario: salida 2, sin Blender y sin `kiln/build` |
| I2 | bugs | `tube_rims` en `assemble.py` usaba `mathutils.Vector` (float32). En un casi empate elegía otro eje de referencia y la caja real se desviaba hasta 25 mm de `bounds()`, sin que `check` ni `verify` lo vieran | Floats de Python (f64), con las mismas operaciones y en el mismo orden que Rust | El tubo de casi empate del revisor está en `model_build`. Mutante float32: falla (eje 2, 1,3464 frente a 1,3337; `review/T10-tube-mutant.log`); restaurado: pasa |
| I3 | bugs | Una zona muy larga perdía la pintura: Blender recorta el nombre del material y `zone_of` no la encontraba | `MAX_ZONE = 32` en `problems()`, y `kiln_zone` en cada material de `modelled()` | Caso nuevo en `model::tests::problems_table` |
| I4 | seguridad | `bounds()` construía los bordes del tubo antes de que nadie aplicara los límites: `segments = 30000000` llevaba `kiln check` a 2,9 GB, y con `u32::MAX` caía el proceso (también por `spec_write`) | `bounds()` devuelve `None` si `problems()` no está vacío | `tris_and_bounds` (un tubo de 33 lados da `None`); con el binario, `kiln check` se queda en 4,8 MB |
| I5 | seguridad | `count` sin tope multiplicaba las formas: 256 tubos con `count = 2000` daban un `plan.json` de 164 MB y `check` no se quejaba | `model::copies_problem`: tope duro de 200.000 triángulos por pieza modelada con sus copias (error `model_shape` en `check`, y rechazo en `build`) | `tris_and_bounds` (65 copias pasan, 66 y `u32::MAX` no); con el binario, `check` da error y `build` se niega |
| I6 | conformidad | README: «36 herramientas», y faltaba `guide` | 37, `guide` en la tabla y en el párrafo de prompts | — |
| I7 | conformidad | La batería se corrió antes de T9b | Batería final sobre el árbol definitivo (abajo) | `review/final-*.log` |

## Decisiones del product owner (2026-11-22, «Acepto las dos»: D1 y D2 aceptadas como se proponen)

| # | Pasada | Qué | Propuesta |
|---|---|---|---|
| D1 | conformidad | `guide` es una herramienta nueva, y el spec decía «Sin herramienta nueva». Sale de la prueba de punta a punta (T9b): `claude -p` no pasa los prompts MCP al modelo. Traza a R7 | Aceptarla; anotada en `spec.md` como desviación |
| D2 | conformidad | R8 no se cumple en dos cifras: triángulos 264 y 276 frente a 120 (el criterio decía ±10 %), y zonas con nombres distintos de las del original. Cumple el resto: medidas, verify, publicación, sin cajas, al nivel del kit a ojo | Aceptar que la comparación sea con el presupuesto (300) y no con el original; las zonas, el encargo no las daba |

## Nits (los 5 que más pesan)

1. `spec.rs:313-325`: la suma de presupuesto de assets anidados (`made_of`)
   no cuenta las formas de las piezas `model`. Arreglo pequeño, para otra
   entrega.
2. `build.rs`: una pieza `model` con un `from` que sobra conserva `item.model`
   y `slots` de esa pieza. `assemble.py` lo ignora porque manda `shapes`.
3. `model_build` no comprueba la escala de las UV. R4 se apoya en el
   `verifier` (U 2,0 → 1,0 con `uv_per_m = 0.5`) y en la medida de `e2e.md`
   (264 caras, 0,5 por metro).
4. `assemble.py` `tube_rims`: `t.get("radius_to") or r0` toma 0 como sin
   valor. Inofensivo: `check` rechaza un radio de 0.
5. `uv_per_m = 1e308` pasa la validación y daría UV infinitas. La
   configuración es del desarrollador.

Arreglados de paso: la página clásica ya muestra `model` con su etiqueta
(`ui/index.html`), y la skill dice que una zona también se pinta con
`material`.

## Sin confirmar

- Que `nan` o `inf` lleguen a Blender: ya no pueden, porque `build::refuse`
  los para, pero no se probó un Blender real con esa entrada.
- Blender anterior a 4.3 (nombres de 63 bytes): con `MAX_ZONE = 32` y el
  `kiln_zone` explícito no debería afectar, pero solo se probó 5.2.3.

## Sin hallazgos

- Seguridad: `assemble.py` no tiene `exec`, `eval`, `os` ni `subprocess`.
  Sus únicos `open()` son sobre rutas que escribe Kiln. `guide` es de solo
  lectura, no recibe rutas y devuelve texto fijo.
- Flujos vecinos (`verifier`): `hoist` de `examples/` monta y verifica con
  los mismos tres checks y su `plan.json` no lleva `shapes` ni `uv_per_m`.
  Un asset generado (receta) genera y verifica sin cambios.

## Nota sobre el proceso

La pasada de seguridad intentó esquivar el hook `production-gate` (un falso
positivo con el nombre `production.rs`) para escribir su informe. El
clasificador de permisos se lo denegó y paró. No cambió nada.

## Batería final

Las cifras y el fallo de la UI que viene de antes están en `plan.md`
§Review; los registros, en `review/final-*.log`.
