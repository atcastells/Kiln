# Intent: medir en reposo las piezas con esqueleto (K009)

- **Originator**: Aaron Castells (product owner)
- **Date**: 2026-11-18
- **Status**: approved by product owner (2026-11-18; eligió «Arreglarlo en Kiln» al parar ASTRA T010-C2)
- **Slug**: k009-rig-rest-size

## The problem, in the originator's words

Surge en ASTRA T010-C2, al publicar los bocetos del guardia y del mozo. El
PO eligió esta salida entre tres:

> Arreglarlo en Kiln: tarea corta en el repo de Kiln (K009): medir las
> piezas con esqueleto en pose de reposo, con prueba roja primero. Vale para
> cualquier personaje futuro.

Lo medido: `kiln build` informa del tamaño del personaje posado (acción
`idle`, cuadro 1: 0,634 × 1,780 × 0,377 m) y `kiln verify` lo compara con el
GLB en reposo (0,630 × 1,775 × 0,333 m) con 1 cm de tolerancia, y falla con
`glb_size`.

## Who it affects

Quien publique un personaje cuya primera pose de animación se salga de su
caja en reposo. El técnico de T010-C1 pasó porque su mochila tapaba la
diferencia.

## What happens if we do nothing

Cada personaje nuevo tiene que ajustar su geometría o su pose para esconder
un fallo de la herramienta, o no se puede publicar.

## How we will know it is solved

Un personaje con esqueleto cuya pose inicial se sale de su caja en reposo
pasa `build → verify`, y `report.json` da su tamaño en reposo.

## Open questions

- Ninguna.

---
Next stage: Design reads this file and writes `spec.md`.
