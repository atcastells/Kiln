# Handoff: K012, piloto de integración de asteroides procedurales en ASTRA

- **Fecha**: 2026-11-18
- **Estado**: **hecho**. La condición de cierre se cumple en el entorno
  probado: ASTRA representa de forma reproducible un asteroide generado con
  una receta verificada y publicada por Kiln, sin tocar la simulación.
- **Entorno**: un solo equipo. Apple M4 Max, macOS 27.0.0, Godot 4.7.1,
  Forward+ sobre Metal. Nada se ha probado en otro sistema ni en otra
  versión de Godot.

## Criterios de aceptación

| ID | Criterio | Estado | Evidencia |
|---|---|---|---|
| K12-1 | ASTRA carga el paquete publicado por Kiln | hecho | `Package.open` abre `asteroid_32`, `asteroid_64` y `asteroid_128`; test `asteroid_recipe_test.gd`; el verificador volvió a publicar `asteroid_64` en una copia y salió idéntico al del repo |
| K12-2 | Godot genera la geometría sin GLB preexportado | hecho | las 48 semillas (16 por paquete) se construyen en Godot; no hay ningún `.glb` en las carpetas publicadas |
| K12-3 | Una semilla produce una morfología reproducible en el entorno probado | hecho, en este Mac | la semilla 42 construida dos veces da el mismo hash; 200 reconstrucciones por resolución, el mismo hash (`cost.json`, `same_hash`) |
| K12-4 | Tres semillas muestran variaciones verificables | hecho | tres hashes y tres juegos de medidas distintos (test); hojas `shots/06-hoja-*.png` |
| K12-5 | Las medidas coinciden con el paquete, dentro de tolerancias documentadas | hecho | triángulos exactos; medidas a menos de 1 parte en 100 000 (lo medido: 6 en 100 millones) |
| K12-6 | La escena se abre y permite inspeccionar el resultado | hecho | `godot/tests/asteroid_procedural.tscn`; `shots/07-escena-final-128.png`; prueba sin ventana de sus datos |
| K12-7 | Una receta ausente o incompatible produce un error controlado | hecho | un caso por negativa, sin ningún error de guion; `shots/05-escena-paquete-inexistente.png` |
| K12-8 | No se modifican Rust, protocolo ni topología | hecho | `git diff origin/main --stat` no toca `crates/`, `godot/proto/`, `godot/net/`, `godot/station/`, `content/` ni `kiln.toml` |
| K12-9 | Las regresiones de Godot y Kiln pasan | hecho | ASTRA: todas las fases por defecto, 1 050 tests de Rust, `clippy` limpio. Kiln: 562 tests, `clippy` limpio, sin cambios de código |
| K12-10 | Capturas, métricas y documento de cierre | hecho | `shots/`, `metrics/cost.json` y este fichero |

## Qué hay en ASTRA

Rama `k012-asteroid-pilot`.

- **Fichas**: `kiln/specs/asteroid_{32,64,128}.toml`, iguales salvo los
  segmentos. La misma semilla da el mismo asteroide a las tres resoluciones.
- **Paquetes**: `godot/assets/kiln/asteroid_{32,64,128}/`, cada uno con
  `recipe.json`, `kiln_generator.gd` y `asset.json` (versión 1).
- **Consumidor**: `godot/procedural/recipe_package.gd`.
  - `open(id)` comprueba el id, los tres ficheros, los dos JSON, que el
    contrato es de un paquete generado, la versión del generador, el hash del
    guion contra el contrato **y contra el que ASTRA lleva fijado en su
    código**, y el hash de `recipe.json`.
  - `build(paquete, semilla)` solo construye semillas de la tanda; comprueba
    la forma y los límites de la entrada antes de llamar al guion (como
    mucho 256 segmentos y 16 operaciones); compara la malla con lo publicado
    y da su hash.
- **Escena**: `godot/tests/asteroid_procedural.tscn`. Se abre con
  `godot --path godot res://tests/asteroid_procedural.tscn -- --asset=asteroid_64 --seed=42`.
- **Pruebas**: `godot/tests/asteroid_recipe_test.gd`, fase `asteroid` de
  `scripts/godot_station.sh`, entre las de por defecto; sin servidor; 2,6 s
  con el servidor ya compilado.
- **Capturas y métricas**: `scripts/godot_station.sh --asteroid-shots DIR`.

## Hashes de este Mac

Semilla 42, sha256 de los vértices e índices de la malla:

| Paquete | Hash |
|---|---|
| `asteroid_32` | `9933203f5b70e0f99e7c058232cb9c3df58cb31fa79e50877776dae8be44e3d8` |
| `asteroid_64` | `2bace5098284fe2d2b60880b3ba17186979b26be4941dace9d7b999ff2f824f4` |
| `asteroid_128` | `a29f1c0de26f47179e5ac4a0b3539fe118bb3ab0bde0d2b267228bc807c7406f` |

Guion publicado (`kiln_generator.gd`, versión 1 del generador), el mismo en
los tres paquetes y el que ASTRA lleva fijado:
`3c5ae7c6e5781fe452bf2468ed085e9ad997907d283656cc6f1e55b068ca36f0`.

No se ha comprobado que estos hashes de malla salgan iguales en otro sistema
o en otra versión de Godot.

## Métricas

De `metrics/cost.json`, con ventana. Semilla 42.

| | 32 segmentos | 64 | 128 |
|---|---|---|---|
| Triángulos | 1 088 | 4 224 | 16 640 |
| Vértices (facetado: tres por triángulo) | 3 264 | 12 672 | 49 920 |
| Medidas (m) | 2 565 × 1 676 × 2 050 | 2 601 × 1 681 × 2 076 | 2 631 × 1 689 × 2 102 |
| Solo el guion publicado (ms, media) | 1,2 | 3,0 | 10,9 |
| Llamada entera del consumidor (ms, media de 20) | 1,6 | 4,1 | 13,5 |
| Primera construcción del proceso (ms) | 10,3 | 6,5 | 16,1 |
| Datos de la malla en memoria (bytes) | 156 672 | 608 256 | 2 396 160 |
| Primitivas dibujadas, con sombras / sin sombras | 2 176 / 1 088 | 8 448 / 4 224 | 33 280 / 16 640 |
| Llamadas de dibujo, con sombras / sin sombras | 2 / 1 | 2 / 1 | 2 / 1 |
| Tiempo de cuadro con el asteroide (ms, mediana) | 4,2 | 3,6 | 4,3 |
| Tiempo de cuadro de la escena vacía (ms, mediana) | 3,4 | 4,2 | 4,0 |

Sin ventana, la llamada entera sale más rápida: 0,7, 2,6 y 10,2 ms. Las dos
series no son comparables entre sí.

**Cómo leerlas:**

- **El coste de dibujo no se distingue del de la escena vacía.** Con la
  sincronía vertical apagada el cuadro tarda unos 4 ms con asteroide y sin
  él: a estas cifras de triángulos lo que se mide es el suelo del sistema,
  no el asteroide. No sirve de presupuesto.
- **Las sombras duplican las primitivas**: la malla se dibuja una vez para
  la luz y otra para la cámara.
- **La memoria de vídeo no se pudo medir.** Godot informa de 0 bytes de
  diferencia en los búferes al añadir el asteroide; el dato fiable son los
  bytes de los arrays de la malla.
- **Reconstruir 200 veces la misma semilla no acumula memoria**: 392 bytes
  de diferencia, iguales a las tres resoluciones, y el mismo hash las 200
  veces.
- **El tiempo crece con el cuadrado de los segmentos**, como los triángulos.

## Qué da el generador y qué falta frente a AST-EST

Contra `docs/design/estaciones-asteroides.md` de ASTRA, §3 y §8. Es un
registro de hechos, no una propuesta.

**Lo que hay hoy:**

- Una malla cerrada con forma de asteroide, sus tres medidas, sus triángulos
  y sus vértices.
- La misma malla para la misma semilla, en este equipo, y un hash para
  comprobarlo.
- Las operaciones y los valores sorteados de cada semilla, en `recipe.json`.
- Un contrato con la versión del generador y los hashes de lo publicado.

**Lo que es:** solo el *volumen natural* de §3.2, y solo su superficie
exterior. El piloto no representa volumen excavado ni volumen funcional.

**Lo que falta para relacionarlo con AST-EST:**

| AST-EST | Qué pide | Qué da el generador |
|---|---|---|
| §3.1, AST-EST-002 | Regiones estructurales con estabilidad, cohesión, excavabilidad y recursos | Nada. El relieve sale de ruido visual y no se debe leer como geología |
| §3.2 | Volumen natural, excavado y funcional, distintos entre sí | Solo la superficie del natural; ningún dato del interior |
| §3.3 | Cavidades, galerías, roca remanente | Nada: la malla es una cáscara, sin interior ni forma de preguntar «¿este punto está dentro?» |
| §8.1 | Morfología antes que geometrías incompatibles | La forma existe, pero nadie puede consultarla salvo dibujándola: Rust no la conoce |
| §8.3 | Volúmenes reales, sin interferencias | No hay volumen consultable; solo la caja de medidas |
| §8.5 | Topología, reservas e identificadores persistentes antes del detalle visual | El asteroide no tiene identidad en el mundo: es un paquete y una semilla |
| §8.6 | Explorar en otro orden no cambia la región | Se cumple para la malla: depende solo de receta y semilla |
| §8.7 | Rust conserva estado y topología; Godot representa; Kiln prepara y valida | Coincide: Kiln sortea y verifica, Godot solo construye lo ya sorteado. Rust no interviene todavía |
| §8.8 | Fallos explícitos | Coincide: un paquete que no cuadra no se dibuja y se dice por qué |
| §10 | Tamaños de asteroides sin aprobar | El de 2,0 a 3,3 km del piloto es un valor de prueba, no canon |

## Límites conocidos

- **Los polos se ven.** La esfera base de Kiln es de meridianos y paralelos
  y deja una estrella de triángulos arriba y abajo, clara en las vistas
  cenitales y más a 128 segmentos. Arreglarlo es otra forma base en el
  vocabulario de Kiln: una entrega aparte.
- **Solo las 16 semillas de la tanda.** El juego no sortea; para otra
  semilla hay que verificar y publicar de nuevo con Kiln.
- **Una resolución por paquete.** Cambiar de resolución es cargar otro
  paquete.
- **El hash fijado obliga a tocar ASTRA cuando cambie el generador de Kiln.**
  Es a propósito: un guion distinto pasa por revisión de código.
- **Sin material, sin colisión, sin LOD.** El gris lo pone la escena de
  prueba.
- **Reproducibilidad entre sistemas: sin comprobar.**
- **Un proyecto exportado no se ha probado.** El hash del guion se calcula
  sobre el fichero fuente; en una exportación el guion puede ir compilado.

## Capturas

En `shots/`, por orden de avance:

| Fichero | Qué enseña |
|---|---|
| `01-receta-facetada-32-64-128.png`, `01-receta-suave-32-64-128.png` | La receta a las tres resoluciones, en Blender, para elegir facetado o suave |
| `02-godot-astra-{32,64,128}-segmentos.png` | Tres semillas por resolución dibujadas por Godot en una copia de ASTRA, antes de escribir código |
| `03-paquetes-publicados-en-astra-64.png` | Los paquetes ya publicados en el repo de ASTRA |
| `04-consumidor-tres-semillas-128.png` | Tres semillas construidas a través del consumidor |
| `05-escena-de-inspeccion-64.png`, `05-escena-paquete-inexistente.png` | La escena, y la escena ante un paquete que no existe |
| `06-hoja-{32,64,128}-segmentos.png` | Tres semillas desde tres ángulos por resolución, con la barra de 100 m |
| `07-escena-final-128.png` | La escena con el código final |

Las tandas sin nada visible que enseñar (abrir el paquete, la fase de la
regresión, el modo del lanzador) no tienen captura: su evidencia es la
salida de las pruebas.

## Revisión

Tres pasadas (fallos, seguridad, conformidad) sobre el diff de ASTRA.
Hallazgos importantes, todos corregidos con su prueba:

- Un contrato con campos de tipo equivocado daba un error de guion y un
  resultado vacío en vez de un error controlado.
- Una entrada de semilla sin `material` daba un error de guion.
- Nada limitaba lo que un paquete podía pedir: una entrada con un millón de
  segmentos colgaba la construcción. Ahora hay topes, los de Kiln.
- El hash del guion solo se comparaba con un fichero que vive a su lado.
  Ahora ASTRA lleva el hash fijado en su código.
- Varias cifras de `cost.json` no decían nada (tiempo de cuadro atado a la
  sincronía vertical, memoria que no seguía a la malla). Se midieron de otro
  modo y el fichero dice qué es cada una.

Queda sin hacer, menor: la caché de guiones de Godot sirve el primer guion
cargado aunque el fichero cambie después dentro del mismo proceso; la
limpieza de la prueba seguiría un enlace plantado en `user://`; el modo de
capturas cuenta ficheros viejos si se reutiliza la carpeta; `kiln check` no
imprime ninguna línea propia de las fichas de receta cuando no tienen
hallazgos.

## Estado Git

- **ASTRA**, rama `k012-asteroid-pilot` desde `origin/main` (`8a4c9b9`):
  seis commits, del `cb1d6d0` (fichas y paquetes) al `fccba80` (correcciones
  de la revisión). En la copia de trabajo queda un PDF ajeno sin
  seguimiento, `.sdlc/first-character/estado-010c1-2026-11-18.pdf`, que no
  se ha tocado.
- **Kiln**, rama `k012-asteroid-pilot`: la cadena (`intent.md`, `spec.md`,
  `plan.md`, `context.md`), las capturas, las métricas y este handoff. Sin
  cambios de código. Lleva además dos commits de configuración del agente
  (permiso de push acotado y regla de unir PR).

## Siguiente paso, cuando se decida

Ninguno se inicia solo. Lo que el piloto deja a la vista:

- Si los polos molestan, una forma base de triángulos uniformes en Kiln.
- Antes de cavidades o puertos, decidir cómo conoce Rust la forma del
  asteroide: hoy solo existe como malla en Godot.
