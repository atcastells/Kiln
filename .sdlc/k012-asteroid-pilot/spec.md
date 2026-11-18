# Spec: piloto de integración de asteroides procedurales en ASTRA (K012)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aaron Castells, product owner (2026-11-18)

## Requirements

«K12-n» es el criterio n de la propuesta citada en el intent; «P-n», su punto
n; «D-n», la fila n de la tabla de decisiones del intent.

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | ASTRA lleva tres fichas de receta del mismo asteroide, de 32, 64 y 128 segmentos, verificadas y publicadas con Kiln. | P-2, D-3, D-6, K12-1 | `kiln check` sin errores de esas fichas; `kiln verify` válido para las tres; en `godot/assets/kiln/asteroid_{32,64,128}/` están `recipe.json`, `kiln_generator.gd` y `asset.json`, y ningún GLB. |
| R2 | Un consumidor en Godot localiza un paquete, comprueba que es el que dice su contrato y que sabe leerlo, y construye la malla de una semilla de la tanda. | P-2, D-2, K12-1, K12-2 | Prueba sin ventana y sin servidor: para cada paquete y cada una de sus 16 semillas devuelve una malla; en la carpeta publicada no hay ningún `.glb`. |
| R3 | La malla construida es la que Kiln verificó. | P-4, D-4, K12-5 | Para cada semilla: los triángulos son exactamente los de `recipe.json` y cada medida difiere menos de 1 parte en 100 000. Medido antes: 6 partes en 100 millones. |
| R4 | La misma semilla da siempre la misma malla en el entorno probado; semillas distintas dan mallas distintas. | P-4, K12-3, K12-4 | Construir dos veces la misma semilla da el mismo hash de vértices e índices; tres semillas dan tres hashes y tres juegos de medidas distintos. Los hashes de este Mac se anotan; no se promete que coincidan en otro sistema o versión de Godot. |
| R5 | Un paquete ausente, roto o que el consumidor no sabe leer da un error controlado, sin error de guion. | P-2, K12-7 | Un caso de prueba por cada negativa de la lista de abajo; el registro de la prueba no contiene `SCRIPT ERROR`. |
| R6 | Una escena independiente enseña el asteroide: cámara que gira alrededor, luz, referencia de escala, semilla y paquete por parámetro, y en pantalla sus dimensiones, triángulos, vértices y tiempo de generación. | P-3, K12-6 | `godot --path godot res://tests/asteroid_procedural.tscn -- --asset=asteroid_64 --seed=42` abre, dibuja y enseña los datos; con un paquete que no existe enseña el error y no se cierra. Prueba sin ventana de que la escena carga y rellena sus datos. |
| R7 | Capturas de tres semillas por resolución desde varios ángulos, y métricas: tiempo de generación, triángulos, vértices, memoria aproximada, coste de dibujo y comportamiento al reconstruir la misma semilla. | P-6, D-5, D-7, D-8, K12-10 | `scripts/godot_station.sh --asteroid-shots DIR` deja las PNG y un `cost.json` con todo lo anterior, el equipo y el renderizador. |
| R8 | No cambia nada de Rust, del protocolo ni de la topología de la estación. | P-1, K12-8 | `git diff origin/main --stat` en ASTRA no toca `crates/`, `godot/proto/`, `godot/net/`, `godot/station/` ni `content/`. |
| R9 | Las regresiones de Godot y de Kiln pasan. | K12-9 | En ASTRA: `scripts/godot_station.sh` con sus fases por defecto, más la fase nueva; `cargo test --workspace` y `clippy`. En Kiln: `cargo test --workspace` y `clippy`, sin cambios de código previstos. |
| R10 | Queda escrito qué información espacial da el generador y qué falta para relacionarlo con AST-EST. | P-5, K12-10 | Sección propia en el handoff, contra §3 y §8 de `docs/design/estaciones-asteroides.md`, sin proponer regiones geológicas. |
| R11 | Cada avance se enseña con capturas. | D-7 | Cada tanda del plan termina con una captura en `.sdlc/k012-asteroid-pilot/shots/`, y el mensaje de estado la enseña. |

## Design

### Medido antes de escribir esto (2026-11-18, este Mac, Godot 4.7.1)

Sobre una copia de ASTRA fuera del repo, con su propio `kiln.toml`:

- `kiln check` no da ningún hallazgo por las tres fichas; `verify` y
  `publish` pasan para las tres. 4,3 a 5,8 s por verificación.
- El guion publicado, llamado dentro del proyecto de Godot de ASTRA, sin
  ventana: las 16 semillas de los tres paquetes dan los triángulos
  publicados y unas medidas que difieren como mucho 6 partes en 100
  millones. El sha256 del guion coincide con el del contrato.
- Tiempo medio por malla: 0,7 ms a 32 segmentos, 2,6 ms a 64, 9,9 ms a 128.
- Con ventana (Metal, Apple M4 Max, Forward+): tres asteroides en pantalla,
  12 llamadas de dibujo; 9 828, 38 052 y 149 796 primitivas dibujadas.
- Godot deja un `kiln_generator.gd.uid` junto al guion la primera vez.
- Una copia sin caché de importación falla con las fuentes del tema hasta
  que se importa una vez; no tiene que ver con el piloto.

Capturas de ese avance, en `shots/`: `01-…` (la receta facetada y suave a
las tres resoluciones, en Blender) y `02-…` (tres semillas por resolución
dibujadas por Godot en el proyecto de ASTRA).

### Las fichas (R1), en ASTRA

`kiln/specs/asteroid_32.toml`, `asteroid_64.toml` y `asteroid_128.toml`,
iguales salvo `segments`. Como el sorteo no depende de los segmentos, la
misma semilla da el mismo asteroide a las tres resoluciones.

```toml
[asset]
id = "asteroid_64"
name = "Asteroide de prueba, 64 segmentos"
budget_tris = 20000
status = "layout"

[generator]
seed = 42

[[generator.ops]]
op = "sphere"
radius_m = [800, 1200]
segments = 64

[[generator.ops]]
op = "stretch"
x = [1.1, 1.6]
y = [0.7, 0.95]
z = [0.85, 1.15]

[[generator.ops]]
op = "noise"
amplitude = [0.18, 0.26]
size = 1.2

[[generator.ops]]
op = "noise"
amplitude = [0.05, 0.08]
size = 0.45

[[generator.ops]]
op = "noise"
amplitude = [0.015, 0.025]
size = 0.15

[[generator.ops]]
op = "facet"
```

- **Tamaño.** Radio de 800 a 1 200 m: el asteroide mide de 2,0 a 3,3 km de
  largo. Es un valor de prueba; AST-EST §10 no aprueba tamaños.
- **Facetado.** Propuesto por coherencia con el Neo-PSX de ASTRA; la versión
  suave también verifica. Triplica los vértices (uno por esquina de cara).
- **Sin material en la receta.** La paleta de ASTRA no tiene ninguna clave
  de roca y el piloto no la amplía; el aspecto lo pone la escena de prueba.
- **Triángulos:** 1 088, 4 224 y 16 640, bajo el tope de 20 000 de ASTRA.
- Los tres paquetes publicados se confirman en el repo de ASTRA, con el
  `.uid` que crea Godot, como el resto de `godot/assets/kiln/`.

### El consumidor (R2 a R5): `godot/procedural/recipe_package.gd`

Carpeta nueva, fuera de `station/`. `extends RefCounted`, sin estado global:

- `static func open(id: String) -> Dictionary`: devuelve el paquete o
  `{"error": texto}`.
- `static func build(package: Dictionary, seed: int) -> Dictionary`:
  devuelve `{"mesh", "tris", "vertices", "size_m", "ms", "hash"}` o
  `{"error"}`.

`open` se niega, con un mensaje distinto en cada caso, cuando:

1. el id no es un nombre simple (letras, cifras, `_`, `-`);
2. falta la carpeta `res://assets/kiln/<id>/` o alguno de sus tres ficheros;
3. `asset.json` o `recipe.json` no son JSON o no son objetos;
4. `asset.json` no dice `kind = "generated"`;
5. `generator_version` no es una que el consumidor conozca (hoy, 1);
6. el sha256 de `kiln_generator.gd` no es el `script_sha256` del contrato;
7. el sha256 de `recipe.json` no es el `recipe_json_sha256` del contrato;
8. `recipe.json` no trae semillas.

`build` se niega cuando la semilla no está en la tanda o cuando su entrada no
tiene la forma que espera el guion (cada operación con su `op`, sus
parámetros numéricos y, en `noise`, su semilla de ruido): el guion publicado
no valida lo que recibe, y en GDScript eso sería un error de guion.

El guion solo se carga de esa carpeta y solo si su hash coincide con el
contrato. Eso prueba que el fichero es el que Kiln publicó, no que sea de
fiar por sí mismo: quien pueda escribir en el repo puede cambiar las dos
cosas.

El consumidor no sortea nada ni exporta GLB. Compara la malla con el
paquete (R3) y calcula un hash de vértices e índices (R4).

### La escena (R6): `godot/tests/asteroid_procedural.tscn`

Con su guion `asteroid_procedural.gd`. No usa `station/`, ni la red, ni el
servidor. Parámetros tras `--`: `--asset=ID` (por defecto `asteroid_64`) y
`--seed=N` (por defecto, la primera de la tanda).

- Asteroide con un material gris mate propio de la escena.
- Cámara que gira alrededor con el ratón y con las flechas, y acerca con la
  rueda.
- Una luz direccional y luz ambiente tenue sobre fondo oscuro.
- Referencia de escala: una barra de 100 m con su rótulo.
- Rótulo con paquete, semilla, medidas, triángulos, vértices y milisegundos
  de generación. Si `open` o `build` fallan, el rótulo enseña el error.
- Textos en castellano, con el tema de ASTRA.

### Pruebas y capturas (R2 a R7, R9)

- `godot/tests/asteroid_recipe_test.gd`, sin ventana ni servidor: R2 a R5 y
  la carga de la escena (R6). Fase nueva `asteroid` en
  `scripts/godot_station.sh`, dentro de las fases por defecto.
- Los paquetes rotos de R5 se fabrican en `user://` durante la prueba,
  copiando uno bueno y estropeándolo; para eso `open` admite una carpeta
  base distinta solo desde las pruebas.
- `godot/tests/asteroid_shots.gd`, con ventana y sin servidor, lanzado por
  `scripts/godot_station.sh --asteroid-shots DIR`, siguiendo el modo
  `--crew-shots` que ya existe: por cada resolución, tres semillas desde
  tres ángulos, y un `cost.json` con:
  - tiempo de generación (media y máximo de 20 construcciones);
  - triángulos y vértices;
  - memoria aproximada: bytes de los arrays de la malla y diferencia de
    memoria estática antes y después de construirla;
  - coste de dibujo: llamadas, primitivas y tiempo de cuadro con el
    asteroide en pantalla y sin él;
  - reconstruir 200 veces la misma semilla: tiempos y si la memoria crece;
  - equipo, sistema, versión de Godot y renderizador.
- **Dónde se mide.** Todo en este Mac: sin ventana para generación y
  comparación, con ventana (Metal) para capturas y coste de dibujo. El
  contenedor Linux queda solo para una pasada sin ventana que compare los
  hashes de malla con los del Mac; su resultado se anota, sea cual sea, y no
  condiciona la aceptación.

### Dónde va cada cosa

- **ASTRA** (rama nueva desde `origin/main`): fichas, paquetes publicados,
  `godot/procedural/`, la escena, las dos pruebas y el cambio en
  `scripts/godot_station.sh`.
- **Kiln** (esta rama): la cadena `.sdlc/k012-asteroid-pilot/`, con las
  capturas, las métricas y el handoff. Sin cambios de código previstos.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.
- **Entrada externa**: los ficheros del paquete se tratan como datos que
  pueden estar rotos: el id se valida antes de formar una ruta, los JSON se
  comprueban antes de usarse, y el guion solo se carga de la carpeta
  publicada y con el hash del contrato. Nada se descarga.

## Out of scope

- Lo que la propuesta excluye (P-8): excavación, cráteres y vetas,
  colisiones, interiores, minería o economía, naves, varios asteroides a la
  vez, LOD o streaming.
- Sortear semillas dentro del juego (D-2) y subir el tope de triángulos de
  ASTRA (D-3).
- Exportar GLB desde ASTRA o añadir validadores a Kiln (D-4).
- Una clave de roca en la paleta de ASTRA y un material definitivo.
- Regiones geológicas o cualquier dato de AST-EST más allá del registro de
  R10.
- Enlazar la escena desde el juego: se abre solo a propósito.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Los polos de la esfera se notan, más a 128 segmentos | El asteroide tiene una estrella visible arriba y abajo | Es un límite conocido del vocabulario de Kiln; se enseña en las capturas y se anota para decidir si hace falta otra forma base. |
| La malla no coincide entre el Mac y Linux | No hay reproducibilidad entre sistemas | La aceptación solo pide el entorno probado; la comparación se hace y se anota. |
| El tiempo de cuadro en una escena casi vacía dice poco | El coste de dibujo se lee de más | Se da junto a llamadas y primitivas y con la escena sin asteroide como referencia; no se fija ningún presupuesto. |
| Meter la fase nueva entre las de por defecto alarga la regresión de ASTRA | Segundos de más en cada pasada | No arranca servidor; si pasa de 10 s se saca de las fases por defecto. |
| Otra sesión trabaja en la copia de ASTRA | Cambios pisados | Antes de cambiar de rama en ASTRA se mira su estado; lo ajeno no se toca. |
| El `.uid` que crea Godot no se confirma | El siguiente que abra el proyecto ve ficheros sin seguimiento | Se confirma con el paquete, como hace ASTRA con sus guiones. |

---
Next stage: Build reads this file and writes `plan.md`.
