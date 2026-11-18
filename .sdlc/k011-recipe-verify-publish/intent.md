# Intent: verificar y publicar un asset generado por receta (K011)

- **Originator**: Aaron Castells (product owner)
- **Date**: 2026-11-18
- **Status**: approved by product owner (2026-11-18)
- **Slug**: k011-recipe-verify-publish

## The problem, in the originator's words

> No ampliaría inmediatamente el vocabulario con decenas de operaciones
> geométricas. Priorizaría terminar K010 como una capacidad completa y
> verificable:
>
> 1. Cerrar revisión, pruebas y eventual integración de
>    k010-generated-assets.
> 2. Conectar generate con el pipeline existente de validación y
>    publicación, sin sustituir build → verify → publish.
> 3. Incorporar detección básica de geometría degenerada, pliegues o
>    autointersecciones y comprobar que los presupuestos geométricos son
>    razonables.
> 4. Validar varias semillas de una misma receta, no solamente una malla
>    favorable.
> 5. Crear una prueba piloto con un asteroide exterior en Godot, sin fingir
>    todavía que tiene excavaciones, instalaciones o colisiones funcionales.
>
> Las cavidades, los puertos y las estructuras modulares deberían formar un
> incremento posterior, cuando hayamos definido cómo se representan sus
> relaciones espaciales.

Esta entrega cubre los puntos 2, 3 y 4. El 1 está hecho (K010 unido a `main`
en el PR #28). El 5 va en una entrega posterior.

## Lo que hay hoy (comprobado en el código, 2026-11-18)

- `kiln generate` deja un GLB por semilla en `<cache>/generated/<id>/` y un
  `generated.json` de la última semilla. Nada de eso llega al juego.
- `build`, `paint`, `verify`, `publish` y `review` rechazan un asset
  generado con «es un asset generado: aún no se monta ni se publica».
- `verify` y `publish` atan un GLB por su sha256; una receta no tiene un GLB
  único.
- `kiln check` valida operaciones, límites, orden y el peor caso de
  triángulos contra `budget_tris`. No mira la suma de amplitudes de ruido ni
  nada de la malla ya generada.
- Todas las operaciones del vocabulario mueven cada vértice a lo largo de su
  radio (`generate.gd`): leído en el guion, no medido, la malla solo puede
  cruzarse consigo misma si la suma de amplitudes de `noise` llega a 1.
- La esfera base trae por construcción dos triángulos sin área por segmento,
  en los polos.
- Medido en K010: 0,3 a 0,4 s por semilla con Godot sin ventana; una receta
  con amplitudes altas (0,5 más otras tres capas) pasa `check` y sale con
  picos muy empinados.

## Who it affects

Quien quiera usar en un juego un asset generado por receta: hoy se queda en
la caché de Kiln. Primer caso, el asteroide exterior de ASTRA.

## What happens if we do nothing

El originador no lo ha dicho. Lo comprobable: una receta se puede guardar y
ver, pero no hay forma de darla por buena ni de entregarla al juego, y una
receta que solo sale bien con una semilla pasa por válida.

## How we will know it is solved

Elegido por el originador: `kiln verify` sobre un asteroide genera la tanda
de semillas y escribe un informe con el peor caso; `kiln publish` deja la
receta, el guion y las semillas resueltas en la carpeta del juego y sube la
versión; una receta con amplitud total 1 o más no se puede publicar. Todo
por la orden de línea y por MCP. Sin botones nuevos en la página.

## Open questions

- El umbral a partir del cual los picos y la amplitud total dan aviso: sin
  medir.
- Si de verdad la malla no se cruza por debajo de amplitud total 1: leído en
  el guion, no medido.
- Si lo publicado basta para regenerar en un Godot limpio la misma malla
  que verificó Kiln: se deja para el piloto.

## Decisions

| Date | Question | Answer | Who |
|------|----------|--------|-----|
| 2026-11-18 | Alcance de esta entrega | Los puntos 2, 3 y 4 de la propuesta: verificar una receta con varias semillas y reglas de geometría, y publicarla. El piloto en Godot (5) va después | Aaron Castells |
| 2026-11-18 | Vocabulario | No se amplía en esta entrega | Aaron Castells |
| 2026-11-18 | Qué se publica | La receta, el guion generador de Godot con su versión y los valores ya sorteados de las semillas verificadas. Sin GLB horneados | Aaron Castells |
| 2026-11-18 | Qué semillas se verifican | Una tanda fija: 16 semillas derivadas de la de la ficha, siempre las mismas, con el número ajustable en `kiln.toml`. El informe da el peor caso | Aaron Castells |
| 2026-11-18 | Qué bloquea y qué avisa | Error lo roto: suma de amplitudes 1 o más, alguna semilla sobre el presupuesto de triángulos, triángulos sin área fuera de los polos. Aviso lo feo: picos empinados y amplitud total alta | Aaron Castells |
| 2026-11-18 | Señal de que está resuelto | Verificar, publicar y ver el rechazo, por la orden de línea y por MCP. Sin botones en la página ni prueba de regenerar en Godot | Aaron Castells |
| 2026-11-18 | Qué bloquea, corregido al medir (sustituye «suma de amplitudes 1 o más» de la fila anterior) | La cota del radio: empieza en 1, cada estirado la multiplica por su factor menor y cada ruido le resta su amplitud; a 0 o menos, error. Medido: «estirar a 0,2 y ruido 0,9» suma 0,9 y vuelve caras. Aprobado con la spec | Aaron Castells |
| 2026-11-18 | Cavidades, puertos y estructuras modulares | Incremento posterior, cuando esté definido cómo se representan sus relaciones espaciales | Aaron Castells |

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
