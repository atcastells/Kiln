# Intent: el montaje no gira las piezas con modelo (K005)

- **Originator**: Aaron Castells (hallado en la prueba de punta a punta de K004)
- **Date**: 2026-11-16
- **Status**: approved by product owner (2026-11-16)
- **Slug**: k005-turn-imported

## The problem, in the originator's words

> adjunto output de una prueba del e2e

De esa prueba (`.sdlc/k004-mcp/e2e.md`, «Bug encontrado»): `plan.json` lleva
`roof_1` y `roof_2` de la caseta de pescadores con `turn = [∓56.3, 0, 0]`,
pero en el GLB los dos nodos salen sin rotación. Solo gira `net_rack`, que es
una caja.

## Qué pasa hoy

- `crates/kiln-cli/blender/assemble.py:137-152`: una pieza con modelo es
  `src.copy()` del objeto importado; después se asigna
  `o.rotation_euler = Euler(..., "XYZ")`.
- El importador glTF de Blender deja el objeto con
  `rotation_mode = 'QUATERNION'` (comprobado en Blender 5.0.1). La copia lo
  hereda y Blender ignora `rotation_euler`. Las cajas de `box()` están en
  `XYZ` y sí giran.
- Viene de la entrega 4. No es de K004: K004 solo lo hizo visible.

## Who it affects

Todo asset con `turn` en una pieza que tiene modelo: los ejemplos `hoist` y
`cargo_quay`, y cualquier ficha que escriba un agente.

## What happens if we do nothing

Los montajes salen con las piezas mal orientadas. El revisor visual lo ve,
pero atribuye mal la causa (en la prueba dijo que dependía del número de
copias) y gasta vueltas de revisión en corregir lo que no es.

## How we will know it is solved

- En el GLB montado, cada copia con `turn` lleva la rotación de su `plan.json`,
  tenga modelo o sea una caja.
- Hay un test que lo comprueba leyendo el GLB.

## Open questions

- Si el test puede lanzar Blender en la batería normal o va aparte (hoy
  ningún test de Kiln lanza Blender).
- Si el mismo fallo afecta a `paint` o a las piezas-asset (fuente `asset`).

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
