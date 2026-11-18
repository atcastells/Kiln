# Intent: assets generados en el juego a partir de una receta (K010)

- **Originator**: Aaron Castells (product owner)
- **Date**: 2026-11-18
- **Status**: approved by product owner (2026-11-18)
- **Slug**: k010-generated-assets

## The problem, in the originator's words

> Para trabajar en generación procedural quiero soporte para gestionarlo
> desde aquí, la conexión entre godot y blender.

> Espero algo tipo B o C, que kiln tenga algún tipo de standard para la
> generación, ya que eso se ejecutaría dentro del juego.

> De momento sería un asteroide.

> Deberíamos ser capaces de sacar esos "sliders" que tendría este modelo
> procedural.

## Lo que hay hoy (comprobado en el código, 2026-11-18)

- Kiln no tiene nada de generación: `procedural`, `geometry nodes` y `seed`
  no aparecen en `crates/`, `skills/` ni el README.
- Una ficha solo se compone de piezas con cinco orígenes (`reuse`, `variant`,
  `search`, `retouch`, `asset`; `crates/kiln-core/src/spec.rs`, `KINDS`).
- La skill `skills/asset/SKILL.md` desgrana una petición en 3 a 10 piezas
  rígidas y prefiere reutilizar; un asteroide pedido hoy saldría como un
  único GLB fijo, sin variación de forma.
- `[[variants]]` solo cambia materiales, no geometría.
- Verificar y publicar atan un GLB por sha256; una receta no tiene GLB.
- Kiln solo lanza Godot sin ventana y no ejecuta guiones del proyecto.

## Lo que hay en ASTRA (comprobado en su repo, 2026-11-18)

El originador dice que es para ASTRA y que «dentro de este asteroide irá una
estación donde vivirá gente más adelante».

- Rust es la autoridad (`CLAUDE.md:5`) y guarda el mundo como rectángulos en
  una rejilla 2D en decímetros, sin eje de altura, más un grafo de módulos y
  puertas (`crates/astra_sim/src/space.rs`). No usa mallas ni física 3D.
- La distribución procedural ya se genera solo en Rust, desde una semilla y
  con un generador de números propio (`crates/astra_worldgen/src/generate.rs`,
  `crates/astra_core/src/rng.rs`); regla de determinismo en T-077.
- Godot representa lo que Rust genera (T-072) y hace física local (T-019).
  Sus guiones no usan semillas.
- La distribución está acotada por «las limitaciones del asteroide» (T-076),
  sin definir.
- V1 no tiene exterior del asteroide (`docs/design/decisiones.md:85`,
  `kiln/context.toml:195`).
- No hay código ni datos de asteroide en ningún `.rs`.

## Who it affects

Quien pida a Kiln un asset cuya forma debe variar en cada ejemplar dentro
del juego. Primer caso: los asteroides.

## What happens if we do nothing

El originador no lo ha dicho. Lo comprobable: un asteroide pedido hoy sale
como un único GLB fijo montado con piezas, igual en cada ejemplar, y Kiln no
tiene forma de describir un modelo que el juego genere.

## How we will know it is solved

Elegido por el originador: se escribe «crear un asteroide», el agente escribe
una ficha con receta en vez de piezas, la página pinta sus sliders, se
cambian valores y semilla, se pulsa Aplicar y se ve otro asteroide. La misma
semilla con los mismos valores da siempre la misma malla.

## Open questions

- Cuánto tarda cada «Aplicar» (lanzar Godot sin ventana y exportar el GLB);
  sin medir.
- Qué operaciones entran en el primer vocabulario.
- Qué se verifica y se publica cuando no hay GLB: fuera de esta entrega,
  para la siguiente.
- Si la misma semilla da la misma malla en Godot en cualquier máquina; sin
  probar.

## Decisions

| Date | Question | Answer | Who |
|------|----------|--------|-----|
| 2026-11-18 | Dónde corre la generación | Dentro del juego; Kiln da un estándar, no hornea en Blender | Aaron Castells |
| 2026-11-18 | Primer caso | Un asteroide | Aaron Castells |
| 2026-11-18 | Qué produce el desglose | Los "sliders" (parámetros) del modelo procedural | Aaron Castells |
| 2026-11-18 | Vocabulario cerrado o código libre por asset | Cerrado | Aaron Castells |
| 2026-11-18 | Quién necesita la forma | Rust sortea los rangos con su semilla y conoce una envolvente simple; Godot recibe los valores resueltos y genera la malla | Aaron Castells |
| 2026-11-18 | Alcance de la primera entrega | La roca exterior, en Kiln: malla vista desde fuera con sliders y semilla, en la página. Sin tocar ASTRA; la envolvente para `astra_worldgen` y la roca vista desde dentro quedan fuera | Aaron Castells |
| 2026-11-18 | Vista previa de los sliders | Botón Aplicar: Godot sin ventana con el generador real exporta un GLB de caché que carga la vista 3D. Sin generador en JavaScript | Aaron Castells |
| 2026-11-18 | Publicar en esta entrega | No: termina en la página. Publicar receta y generador, y verificar sin GLB, van en una entrega posterior | Aaron Castells |
| 2026-11-18 | Señal de que está resuelto | Pedir, ajustar y ver: petición, ficha con receta, sliders, Aplicar y otro asteroide; misma semilla y valores, misma malla | Aaron Castells |

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
