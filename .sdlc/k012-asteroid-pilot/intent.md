# Intent: piloto de integración de asteroides procedurales en ASTRA (K012)

- **Originator**: Aaron Castells (product owner)
- **Date**: 2026-11-18
- **Status**: approved by product owner (2026-11-18)
- **Slug**: k012-asteroid-pilot
- **Fuente**: «ASTRA V2 — K012: Piloto de integración de asteroides
  procedurales» (pegado en la sesión, estado «propuesta de siguiente tarea»).

## The problem, in the originator's words

> **Dependencias obligatorias:** K009 fusionado en main de Kiln. K011
> fusionado, con los riesgos de verificación obsoleta y concurrencia
> resueltos. Pipeline de recetas verificadas y publicadas disponible.
>
> **1. Objetivo.** Demostrar que una receta procedural publicada por Kiln
> puede reproducirse dentro de un proyecto Godot de ASTRA, generando un
> asteroide exterior determinista. El piloto será independiente del
> escenario jugable actual. No debe modificar la simulación autoritativa de
> Rust.
>
> **2. Integración Kiln → Godot.** Consumir el paquete publicado por K011:
> recipe.json, kiln_generator.gd, asset.json. El consumidor deberá:
> 1. Localizar la receta publicada.
> 2. Comprobar la compatibilidad de su versión y los recursos necesarios.
> 3. Seleccionar una semilla explícita.
> 4. Invocar el generador publicado y construir la malla.
> 5. Crear la representación visual correspondiente.
> 6. Informar de errores sin bloquear el resto del proyecto.
>
> No aceptar scripts procedurales arbitrarios descargados de fuentes no
> confiables. El piloto utilizará exclusivamente assets locales publicados y
> controlados.
>
> **3. Escena de prueba.** Crear una escena independiente, por ejemplo
> godot/tests/asteroid_procedural.tscn. Debe disponer de: asteroide
> generado; cámara de inspección; iluminación básica; referencia de escala;
> semilla seleccionable mediante parámetro de ejecución; información de
> dimensiones, triángulos y tiempo de generación. La escena debe permitir
> observar el objeto desde distintos ángulos. No añadir todavía interiores,
> estructuras portuarias ni instalaciones industriales.
>
> **4. Variación procedural.** Validar al menos tres semillas diferentes de
> una misma receta. Comprobar: misma receta y semilla producen resultados
> equivalentes; semillas diferentes generan morfologías diferentes; las
> dimensiones respetan los parámetros declarados; los triángulos permanecen
> dentro del presupuesto; la malla no presenta errores geométricos
> detectados por los validadores disponibles. No asumir reproducibilidad
> binaria entre sistemas operativos o versiones de Godot sin pruebas
> específicas.
>
> **5. Integración arquitectónica futura.** El asteroide se considera
> exclusivamente el volumen natural inicial. No representa todavía:
> cavidades excavadas; regiones geológicas autoritativas; espacios
> presurizados; redes de transporte; infraestructura industrial; sectores
> habitados. Registrar qué información espacial ofrece realmente el
> generador y qué datos faltan para relacionarlo posteriormente con la
> arquitectura AST-EST. No inventar regiones geológicas a partir de ruido
> visual.
>
> **6. Rendimiento.** Medir: tiempo de generación de la malla; triángulos y
> vértices; memoria aproximada; coste de representación; comportamiento al
> reconstruir repetidamente la misma semilla. Utilizar varias resoluciones
> geométricas y documentar el hardware y renderizador empleados. No imponer
> todavía un presupuesto global para asteroides enormes.
>
> **7. Criterios de aceptación.**
>
> | ID | Criterio |
> |---|---|
> | K12-1 | ASTRA carga el paquete publicado por Kiln |
> | K12-2 | Godot genera la geometría sin GLB preexportado |
> | K12-3 | Una semilla produce una morfología reproducible en el entorno probado |
> | K12-4 | Tres semillas muestran variaciones verificables |
> | K12-5 | Las medidas coinciden con las expectativas del paquete publicado, dentro de tolerancias documentadas |
> | K12-6 | La escena se abre correctamente y permite inspeccionar el resultado |
> | K12-7 | Una receta ausente o incompatible produce un error controlado |
> | K12-8 | No se modifican Rust, protocolo ni topología de la estación |
> | K12-9 | Las regresiones pertinentes de Godot y Kiln pasan |
> | K12-10 | Se entregan capturas, métricas y documento de cierre |
>
> **8. Fuera de alcance.** No implementar: excavación procedural; cráteres y
> vetas minerales; colisiones detalladas; generación de estaciones
> interiores; conexión con minería o economía; movimiento de naves;
> generación de múltiples asteroides simultáneos; optimización mediante LOD
> o streaming. Estas capacidades se estudiarán posteriormente, cuando exista
> evidencia del piloto.
>
> **9. Entregables.** Escena Godot, consumidor de recetas, pruebas, capturas
> de tres semillas, métricas de rendimiento, revisión de código y handoff
> con estado Git.
>
> **Condición de cierre:** ASTRA puede representar de forma reproducible un
> asteroide generado mediante una receta validada y publicada por Kiln, sin
> modificar la simulación existente.

## Lo que hay hoy (comprobado en los repos, 2026-11-18)

- **Dependencias cumplidas.** K009 está en `origin/main` de Kiln (PR #26 y
  #27) y K011 también (PR #29, `1e96665`). Los riesgos de verificación
  obsoleta y concurrencia los cierra el commit `3605e45`, con su recorrido
  en `.sdlc/k011-recipe-verify-publish/e2e.md`, «Cierre de riesgos».
- **ASTRA no tiene ni ficha ni paquete de asteroide.** `kiln/specs/` no
  lleva ninguna ficha con `[generator]` y `godot/assets/kiln/` solo tiene
  assets de piezas. El asteroide del recorrido de K011 vivía en una copia de
  `examples` fuera del repo.
- **El paquete** (`crates/kiln-cli/src/ops/publish_recipe.rs`):
  `recipe.json` lleva id, receta, `generator_version`, `script_sha256`,
  `recipe_sha256`, `budget_tris`, el peor caso y, por cada semilla
  verificada, `ops` con los valores ya sorteados, `tris` y `size_m`.
  `asset.json` lleva `kind: "generated"`, `version`, `files`, los mismos
  hashes más `recipe_json_sha256`, y el resumen de la verificación.
- **El guion publicado no sortea** (lo dice su cabecera: «draws nothing
  itself»). `kiln_generator.gd` expone
  `static func build(job) -> {"mesh", "tris"} | {"error"}` y recibe las
  operaciones ya resueltas. Quien convierte una semilla en valores es Kiln
  (Rust, `generate::resolve`). Desde el juego solo se pueden construir las
  semillas que trae `recipe.json`: 16 por defecto.
- **El guion no lleva constante de versión.** La versión del generador y el
  hash del guion están en los dos JSON; el juego solo puede comprobarlos
  calculando él mismo el sha256 del fichero.
- **Los segmentos van dentro de la receta.** Una receta publicada tiene una
  sola resolución, entre 8 y 256 segmentos.
- **El tope de ASTRA limita la resolución.** Una esfera de S segmentos tiene
  S² + 2S triángulos (`vocab::sphere_tris`) y `rules.max_tris` de ASTRA es
  20 000: `verify` rechaza todo lo que pase de 140 segmentos (19 880
  triángulos). 256 segmentos son 66 048.
- **Los validadores de geometría están en Kiln, no en Godot.** Triángulos
  sin área, caras vueltas y caras empinadas se miden en Rust leyendo un GLB
  (`generate::measure`), a propósito: «quien genera no es quien juzga».
- **Ya medido en K011**, en este Mac con Godot 4.7.1: el guion publicado,
  llamado desde un proyecto de Godot vacío, da para las 16 semillas los
  mismos triángulos y medidas a menos de 3,8 millonésimas de metro de las
  publicadas. Sin comprobar: otra máquina u otra versión de Godot.
- **ASTRA y las ventanas de Godot.** Las capturas con ventana van en
  contenedor Linux con Vulkan por software (`scripts/godot_window.sh`), no
  en el escritorio.
- **«AST-EST»** ya está en ASTRA: `docs/design/estaciones-asteroides.md`
  (PR #14, unido el 2026-11-18), el documento «Arquitectura de estaciones
  construidas en asteroides» con las decisiones AST-EST-001 a 019. Lo que
  toca a este piloto:
  - §3.2 distingue volumen natural, excavado y funcional; el punto 5 de la
    propuesta usa «volumen natural» en ese sentido.
  - AST-EST-002 y §3.1: las regiones estructurales (estabilidad, cohesión,
    excavabilidad, recursos) son propiedades conceptuales, sin valores
    fijados.
  - §8: reglas de coherencia para la generación; la 7 reparte autoridad
    (Rust conserva estado y topología, Godot representa, Kiln prepara y
    valida módulos).
  - §10 deja sin aprobar los tamaños de asteroides, cavernas y espesores.
  - §6: V1 no exige explorar el exterior ni pilotar naves.
- La hoja de ruta del prototipo también está ya en ASTRA
  (`docs/design/hoja-de-ruta-primer-prototipo.md`, PR #15): lista la
  exploración espacial libre entre lo que no es prerrequisito del primer
  prototipo.

## Who it affects

El originador no lo ha dicho. Lo comprobable: quien vaya a construir el
exterior de la estación de ASTRA sobre un asteroide generado.

## What happens if we do nothing

El originador no lo ha dicho. Lo comprobable: queda abierta la pregunta de
K011 de si lo publicado sirve tal cual dentro de un juego (su riesgo «nadie
ha llamado a `kiln_generator.gd` desde un juego»), y las capacidades del
punto 8 no tienen la evidencia que piden.

## How we will know it is solved

Los criterios K12-1 a K12-10 y la condición de cierre, citados arriba.

## Open questions

- **Qué datos faltan para relacionar el asteroide con AST-EST** (punto 5):
  el documento ya está en el repo; el registro lo escribe el piloto,
  comparando lo que da el generador con lo que piden §3 y §8. Dato de
  partida: el generador solo da una malla, sus medidas y sus triángulos;
  AST-EST no fija tamaños ni un formato de regiones.
- **«Compatibilidad de versión» y «recursos necesarios»** (punto 2.2): el
  paquete declara la versión del generador, los hashes y un nombre de
  material. Contra qué se compara la versión y qué más hay que comprobar:
  lo propone la spec.
- **Tolerancias de K12-5**: sin fijar. Dato de partida: 3,8 µm en el mismo
  equipo. Las fija la spec, medidas.
- **«Referencia de escala»** y **memoria aproximada**: qué objeto y qué
  medida, en la spec.
- **Si Mac frente a contenedor da la misma malla**: sin medir.

## Decisions

| Date | Question | Answer | Who |
|------|----------|--------|-----|
| 2026-11-18 | Dónde vive la cadena | Como K006: cadena y handoff en Kiln (`.sdlc/k012-asteroid-pilot/`), diff en ASTRA. Rama nueva en cada repo desde `origin/main` | Aaron Castells |
| 2026-11-18 | Qué semillas valen | Las tres del piloto y la semilla seleccionable salen de la tanda verificada de `recipe.json`. Sin sorteo en el juego | Aaron Castells |
| 2026-11-18 | Varias resoluciones | Tres fichas publicadas, de 32, 64 y 128 segmentos. El tope `max_tris` de ASTRA no se toca | Aaron Castells |
| 2026-11-18 | Cómo se comprueba la malla hecha en ASTRA | Comparándola con el paquete: mismos triángulos y medidas que `recipe.json` da para esa semilla. Los errores geométricos los juzgó `kiln verify` al publicar. No se exporta GLB ni se añade nada a Kiln | Aaron Castells |
| 2026-11-18 | Entorno | Generación, medidas y tiempos en este Mac sin ventana y en el contenedor Linux; coste de dibujo con Metal en el escritorio, con ventana, solo entre las 17:00 y las 08:00 (interpretación de quien escribe: la opción elegida se suma a Mac y contenedor) | Aaron Castells |
| 2026-11-18 | Receta del asteroide | Sus parámetros se proponen y se miden en la spec; el intent solo fija una ficha por resolución | Aaron Castells |
| 2026-11-18 | Documentación que falte de ASTRA | Se busca en el repo de ASTRA; si el documento no está, se lee el código. El de AST-EST y la hoja de ruta se añadieron al repo ese día | Aaron Castells |

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
