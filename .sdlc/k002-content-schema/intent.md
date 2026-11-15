# Intent: validación de contenido genérica, declarada por el juego (K002)

- **Originator**: Aaron Castells
- **Date**: 2026-11-15
- **Status**: approved by product owner (2026-11-15)
- **Slug**: k002-content-schema

## The problem, in the originator's words

> ok planificalo, y si hay algo especifico en kiln para astra miraremos de
> hacerlo generico

Contexto de la conversación: K001 (PR #14, en `origin/main`) valida
contenido de ASTRA, pero con un esquema fijo en el código de Kiln
(`Sector`, `Door`, `Object`, `Installation`), una sección
`[integrations.astra]`, códigos `astra_*`, el subcomando `astra-manifest`
y las claves de Godot `astra_id` y `astra_connects`. Al usarlo desde Astra
(tarea 004) ha hecho falta mantener a mano una vista de su biblioteca de
módulos en ese esquema, y Kiln no puede comprobar tamaños, conectores ni
anclajes. En la interfaz no se ve nada cuando todo está bien.

## Who it affects

Cualquier juego que use Kiln y tenga definiciones de contenido propias;
hoy, Astra.

## What happens if we do nothing

Kiln lleva dentro el esquema de un juego concreto, contra su propia regla
(«Kiln no supone nada de un proyecto concreto»). Cada cambio de esquema en
Astra exige cambiar Kiln, y Astra mantiene un fichero duplicado.

## How we will know it is solved

- En el código de Kiln no queda nada con el nombre de un juego: el esquema
  lo declara cada proyecto en su `kiln.toml`.
- La fixture de K001 pasa las mismas comprobaciones declarando su esquema.
- Astra valida `content/astra/modules.ron` directamente, con conectores y
  anclajes comparados con sus escenas, y borra `data/astra/modules.ron`.
- La interfaz de Kiln enseña los conjuntos de contenido y sus elementos,
  también cuando no hay errores.

## Open questions

- Si se conserva `[integrations.astra]` como alias. Propuesta: no; Astra
  es el único usuario y se migra en esta misma entrega.
- Si el lector de RON genérico puede leer variantes con nombre sin un
  `enum` de Rust: hay que probarlo antes de comprometer el diseño.
- `~/Kiln` está en `e9-new-game` con el trabajo de `e10-wizard-chat` sin
  commitear, y su `main` local va por detrás de `origin/main`.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
