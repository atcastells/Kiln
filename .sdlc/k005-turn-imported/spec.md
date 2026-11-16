# Spec: el montaje gira también las piezas con modelo (K005)

- **Derived from**: `intent.md`
- **Status**: approved (2026-11-16)
- **Approved by**: Aaron Castells

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Cada copia que `assemble.py` coloca usa el modo de rotación `XYZ` antes de recibir su `turn`, venga de un modelo importado, de otro asset montado (fuente `asset`) o sea una caja | «cada copia con `turn` lleva la rotación de su `plan.json`, tenga modelo o sea una caja» | En el GLB montado, el nodo de una pieza con modelo y `turn = [0, 90, 0]` lleva la rotación de 90° en Y (cuaternión `[0, 0.7071, 0, 0.7071]` con tolerancia 1e-3); la caja con el mismo `turn`, la misma |
| R2 | Un test lo comprueba montando de verdad con Blender y leyendo las rotaciones del GLB; va marcado `#[ignore]` como el de Godot (`content.rs:148`), porque la batería normal no lanza Blender | «Hay un test que lo comprueba leyendo el GLB»; pregunta abierta sobre Blender | `cargo test -p kiln-cli -- --ignored turn` en verde con el arreglo y en rojo sin él |
| R3 | Las medidas del revisor (`measures.json`: apoyos y piezas que se atraviesan) salen del montaje ya girado | «el revisor gasta vueltas en corregir lo que no es» | Montando la caseta de la prueba de K004, `roof_1` y `roof_2` dejan de salir como «no se apoya en nada» por estar sin girar (comprobación manual con la ficha de `e2e.md`) |
| R4 | README: en «Entrega 4» o en el apartado de `turn`, una línea que diga que los montajes hechos antes de este arreglo con `turn` en piezas con modelo hay que rehacerlos | «Los montajes salen con las piezas mal orientadas» | README revisado |

## Design

- **Un solo cambio en `crates/kiln-cli/blender/assemble.py`**, en `build()`:
  `o.rotation_mode = "XYZ"` justo antes de `o.rotation_euler = …`
  (línea 152). Ahí pasan todas las copias: las de `src.copy()`
  (modelos y assets montados, que vienen de `import_model`) y las de
  `box()`. No se toca `import_model`: la caché guarda el original y solo
  las copias se colocan.
- **`paint.py` no cambia**: abre el `.blend` ya montado y no recoloca.
  `render.py` gira sus cámaras, no las piezas.
- **El test** (`crates/kiln-cli/tests/blender_turn.rs`, `#[ignore]`):
  crea un proyecto temporal mínimo con un kit de una pieza GLB (se
  escribe en el test: un cubo de 1 m con un material, el mismo formato que
  los tests de `kiln-core/src/glb.rs`) y una ficha con dos piezas, una
  `reuse` de ese GLB y otra `search` (sale caja), las dos con
  `turn = [[0, 90, 0]]`. Llama a `production::build_asset` a través de
  `kiln build` (el binario) y lee los nodos del GLB montado (el JSON del
  primer chunk) para comparar sus `rotation`. Si `blender` no está en el
  PATH, el test falla diciendo que se lance con Blender instalado.
- Sin dependencias nuevas.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.

## Out of scope

- Que `kiln check` avise de montajes viejos hechos antes del arreglo: se
  documenta (R4), no se detecta.
- Cambiar cómo el revisor atribuye las causas.
- Rotaciones en Godot (los `.import`): Godot lee la rotación del GLB, así
  que se arregla sola.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Algún asset montado dependía sin saberlo de que no girase (fichas escritas mirando el resultado) | Se ve distinto al rehacerlo | R4 lo avisa; los ejemplos `hoist` y `cargo_quay` se montan antes y después y se comparan las hojas |
| El test con Blender no corre en la batería normal | Una regresión futura pasa sin verse | Se corre en la etapa de Test de K005 y queda documentado junto al de Godot |
| Euler `XYZ` con el cambio de ejes `(rx, -rz, ry)` de la línea 152 da un orden de giros distinto del que espera la ficha cuando hay giro en más de un eje | Giros compuestos mal | El test añade una copia con `turn = [30, 45, 0]` y compara con la caja, que ya usaba `XYZ` y es la referencia de hoy |

---
Next stage: Build reads this file and writes `plan.md`.
