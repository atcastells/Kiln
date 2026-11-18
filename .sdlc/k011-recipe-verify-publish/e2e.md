# K011: recorrido real de verificar y publicar una receta (2026-11-18)

Rama `k011-recipe-verify-publish`, binario de `cargo build --release`.
Godot 4.7.1, glTF Validator 2.0.0-dev.3.10, este Mac. Proyecto: copia de
`examples` fuera del repo con `[publish] dir = "godot/assets/kiln"`. Lo
recorrió el verificador, no quien escribió el código.

Ficha `specs/asteroid.toml`: `budget_tris = 6000`, semilla 42, esfera de
radio 8 a 40 con 64 segmentos, estirado en x de 1,0 a 1,8, ruido 0,2 a 0,3
(tamaño 0,6), ruido 0,05 a 0,1 (tamaño 0,1), facetado.

## El guion partido no cambia ninguna malla (R8)

Antes de partir `generate.gd` se guardó el sha256 de siete GLB hechos con el
guion de `origin/main`; después de partirlo se regeneraron con
`kiln generate` y se compararon con `shasum -c`: los seis que se regeneran
coinciden. El séptimo ya no se genera, porque su receta la rechaza ahora la
cota del radio.

| Receta y semilla | Segmentos | sha256 (primeros 16) | Tras partir |
|---|---|---|---|
| suave, 1 | 64 | `d138282a903ed303` | igual |
| suave, 2 | 64 | `40c5dce1cd891222` | igual |
| facetada, 1 | 16 | `bec6327a9af19e13` | igual |
| suave de alta resolución, 3 | 256 | `a52e699fd7e52384` | igual |
| fea de alta resolución, 4 | 256 | `2987a25d693c6087` | igual |
| elipsoide sin ruido, 1 | 64 | `1a6bdb88f4e968a0` | igual |
| estirar 0,2 y ruido 0,9, 2 | 64 | `acff5bd8011ed1d6` | ya no se genera |

## Reglas sin generar

| Receta | `kiln check` |
|---|---|
| la de arriba | 0 errores, 0 avisos |
| estirar y a 0,2 y luego ruido 0,9 | error `generator_radius`, sale con 1 |
| ruido 0,9 y luego estirar y a 0,2 | solo aviso `generator_amplitude` |
| amplitudes 0,2 a 0,5 y 0,05 a 0,15 | aviso `generator_amplitude` («suman 0.65») |

## Verificar

- `kiln verify --asset asteroid`: 4,77 s la primera vez, 4,60 s la segunda,
  para 16 semillas.
- Comprobaciones: `kiln` pasa con un aviso, `gltf_validator` pasa,
  `godot_import` pasa.
- Las 16 semillas, empezando por la 42: 4 224 triángulos, 128 sin área (los
  de los polos), 0 caras vueltas.
- Peor caso de caras empinadas: 11,5 %, por encima del 10 % del umbral: de
  ahí el aviso.
- Dos ejecuciones dan el mismo informe, con el mismo sha256 por semilla.
- Recuento aparte, con un lector propio en Python sobre dos GLB de la tanda:
  los mismos triángulos, sin área y caras vueltas que dice el informe.

## Publicar

- `godot/assets/kiln/asteroid/` queda con `recipe.json`,
  `kiln_generator.gd` y `asset.json`, y ningún GLB. Versión 1; publicar otra
  vez no la sube. Ninguno de los JSON lleva rutas del equipo.
- **El guion publicado, llamado como lo haría un juego:** en un proyecto de
  Godot vacío con una copia de esa carpeta, `build` sobre las 16 semillas de
  `recipe.json` da los mismos triángulos y unas medidas que difieren como
  mucho en 3,8 millonésimas de metro de las publicadas. Sin errores de
  GDScript.
- Con la receta cambiada después de verificar: se niega («cambió después de
  verificarla») y no toca nada. Tras verificar de nuevo, versión 2.
- Sin `validation.json`: se niega («verifica antes el asset»).

## Lo que no pasa

- La receta de cota negativa: `verify` falla, deja un informe inválido sin
  semillas y no genera ningún GLB; `publish` se niega.
- Con un Godot de pega que escribe basura como GLB: `verify` falla en 0,7 s
  con 16 hallazgos `generated_unreadable`, sin pánico ni cuelgue.
- `build`, `paint` y `review` sobre el asteroide: «es un asset generado: no
  se monta, se genera y se verifica», sin escribir nada.

## Flujos vecinos

- `hoist`, un asset de piezas: montar, verificar y publicar pasan como
  antes; su informe no lleva bloque `recipe`.
- La vista previa de K010: `kiln generate --seed 5` sigue dejando su GLB y
  su `generated.json`, y verificar después no los toca.

## Cierre de riesgos, con Godot de verdad

- Con `kiln verify --asset asteroid` en marcha: una segunda verificación,
  `kiln publish` y `kiln generate --seed 5` salen con 2 y el mensaje
  «ya se está generando o verificando: espera a que termine». La primera
  termina con 0 y no queda cerrojo en la caché. Después, publicar funciona.
- El informe anterior se borra al empezar a verificar (test con un Godot que
  falla: no queda informe).
- Publicar tras cambiar `verify_seeds` o `max_tris` en `kiln.toml`: se niega
  («las reglas de kiln.toml cambiaron») por test, sin Godot.

## Sin comprobar

- La puerta por versión o hash del guion con un guion de verdad distinto
  (sí por tests con el informe escrito a mano).
- Las herramientas MCP con Godot de verdad (sí con un Godot que no existe).
- El recuento aparte de caras vueltas solo se hizo sobre mallas con cero.
- Otra máquina u otra versión de Godot.
