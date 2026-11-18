# Spec: medir en reposo las piezas con esqueleto (K009)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: product owner (Aaron Castells, 2026-11-18)

## Lo que ya se sabe (medido el 2026-11-18)

- `crates/kiln-cli/blender/assemble.py` mide el tamaño con `bound_box` de
  las mallas tras exportar. En una pieza con esqueleto la malla está
  deformada por la acción que el fichero de montaje tiene activa (`idle`,
  cuadro 1), así que la caja es la de esa pose.
- `kiln verify` compara ese tamaño con las cotas del GLB, que son las del
  reposo (`validate.rs`, hallazgo `glb_size`, tolerancia de 0,01 m).
- El humanoide de prueba de K008 no lo destapa: su `idle` empieza en reposo.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | El tamaño que informa `build` de una pieza con esqueleto es el de su pose de reposo | «medir las piezas con esqueleto en pose de reposo» | Prueba con Blender real: un humanoide cuyo `idle` empieza con los brazos al frente da en `report.json` el tamaño en reposo (0,62 × 1,67 × 0,22 m, ±0,01) |
| R2 | El GLB construido no cambia | «Vale para cualquier personaje futuro» (sin romper los actuales) | `build_keeps_rig` sigue en verde: mismos huesos, pesos y deformación; el técnico de ASTRA reconstruido es igual al publicado |
| R3 | Las piezas sin esqueleto se miden como hoy | — (no regresión) | `blender_contract`, `blender_piece` y el resto de la batería en verde |

## Design

Al medir, `assemble.py` pone las armaduras de las piezas con esqueleto en
posición de reposo (`pose_position = "REST"`), actualiza la escena, mide y
restaura. La exportación ya ha ocurrido antes, así que el GLB no cambia. El
fixture `rig_humanoid.py` acepta un argumento `reach` que hace empezar
`idle` con los brazos al frente.

## Policy applied

- **Personal data**: none
- **Sensitive business data**: none
- **Credentials**: none
- **Decisions about people / high risk**: no

## Out of scope

- Medir la envolvente de todas las animaciones.
- Cambiar la tolerancia de `glb_size`.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Restaurar mal la posición de la armadura | El fichero de montaje queda en reposo | Se restaura el valor previo; el `.blend` de montaje ya se guardó antes |

---
Next stage: Build reads this file and writes `plan.md`.
