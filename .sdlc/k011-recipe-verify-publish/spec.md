# Spec: verificar y publicar un asset generado por receta (K011)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aaron Castells, product owner (2026-11-18)

## Requirements

«P-n» es el punto n de la propuesta citada en el intent; «D-n», la fila n de
su tabla de decisiones; «Señal», su apartado «How we will know it is solved».

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | `kiln check` da error si, en el peor caso de sus rangos, el radio de la receta puede llegar a cero (`generator_radius`), y aviso si la suma de amplitudes de ruido pasa de 0,5 (`generator_amplitude`). | P-3, D-6 | Unitarios en `kiln-core` con las recetas medidas: «estirar a 0,2 y ruido 0,9» da error; «ruido 0,9 y estirar a 0,2» no; la receta del README no da ninguno de los dos. |
| R2 | Kiln lee de un GLB las posiciones y los índices de su malla y mide: triángulos, triángulos sin área, caras vueltas hacia dentro y fracción de caras empinadas. | P-3 | Unitarios con mallas hechas a mano (un tetraedro, uno con una cara vuelta, uno con un triángulo sin área); un GLB truncado o con índices fuera de rango da error, no pánico. |
| R3 | `kiln verify` sobre un asset generado genera una tanda fija de semillas y comprueba cada malla. | P-4, D-5, Señal | Con Godot (`--ignored`): verifica 16 semillas por defecto, las mismas en dos ejecuciones; el número sale de `[generate] verify_seeds`. Con un Godot de pega: la tanda lleva la semilla de la ficha y las derivadas con valores fijados en un test. |
| R4 | La verificación falla por lo roto y avisa de lo feo. | P-3, D-6 | Error: alguna semilla con caras vueltas (`generated_flipped`), con triángulos sin área de más (`generated_degenerate`), sobre el presupuesto (`generated_tris`) o que Kiln no lee. Aviso: fracción de caras empinadas sobre el umbral (`generated_steep`). Tests con mallas de pega para cada código. |
| R5 | El informe queda en `validation.json`, con el peor caso de la tanda y lo medido en cada semilla. | P-4, D-5, Señal | El informe trae `valid`, las comprobaciones, los hallazgos y un bloque `recipe` con el hash de la receta, la versión y el hash del generador, y por semilla: valores sorteados, triángulos, medidas y fracción empinada. `spec_get` lo devuelve como hoy. |
| R6 | `kiln publish` publica un asset generado solo si su última verificación pasó y corresponde a la receta y al generador de ahora. | P-2, D-4, Señal | Sin verificar, con la verificación fallida, o con la receta cambiada después de verificar: se niega con un mensaje que dice qué hacer y no escribe nada. Tests de `kiln-cli`. |
| R7 | Publicar deja en `[publish] dir/<id>/` la receta, el guion generador y los valores ya sorteados de las semillas verificadas, y sube la versión cuando cambia lo publicado. Sin GLB. | P-2, D-4, Señal | Tras publicar existen `recipe.json`, `kiln_generator.gd` y `asset.json` con `version`; publicar otra vez sin cambios no sube la versión; cambiar la receta, verificar y publicar, sí. Ningún `.glb` en la carpeta. |
| R8 | El guion que se publica es el mismo que usa Kiln para generar. | P-2, D-4 | `generate.gd` deja de construir la malla: la pide a `kiln_generator.gd`. El test del vocabulario apunta a ese fichero. Las pruebas con Godot de K010 siguen dando los mismos GLB byte a byte. |
| R9 | Verificar y publicar valen por la orden de línea y por MCP, sin herramientas ni rutas nuevas. `build`, `paint` y `review` siguen rechazando un asset generado. | P-2, Señal | Las herramientas MCP `verify` y `publish` ya existentes hacen lo anterior. El test `the_five_operations_refuse_a_generated_asset_before_any_tool` pasa a cubrir tres operaciones y el mensaje cambia a «es un asset generado: no se monta, se genera y se verifica». `mcp_guards.rs` y `coverage.rs` no cambian. |

## Design

### Medido antes de escribir esto (2026-11-18, Godot 4.7.1, este Mac)

Once recetas, cuatro semillas cada una, medidas leyendo el GLB:

| Receta | Cota del radio | Caras vueltas | Empinadas |
|---|---|---|---|
| esfera sin más | +1,00 | 0 | 0,000 |
| elipsoide 5 × 0,2 sin ruido | +0,20 | 0 | 0,000 |
| suave, 64 y 256 segmentos | +0,64 | 0 | 0,000 a 0,006 |
| facetada de 16 segmentos | +0,66 | 0 | 0,000 |
| fea de alta resolución (la del primer intento) | +0,31 | 0 | 0,349 |
| amplitudes 0,6 + 0,39 | 0,00 | 0 | 0,493 |
| amplitudes 0,7 + 0,5 | −0,21 | 0 | 0,615 |
| ruido 0,9 y luego estirar a 0,2 | +0,02 | 0 | 0,751 |
| estirar a 0,2 y luego ruido 0,9 | −0,70 | 54 a 158 | 0,837 |
| cinco ruidos de 1,0 | −4,00 | 264 a 378 | 0,905 |

Tres cosas salen de ahí:

- **La regla aprobada en el intent no basta.** «Suma de amplitudes 1 o más»
  deja pasar «estirar a 0,2 y ruido 0,9» (suma 0,9), que sí vuelve caras. La
  regla correcta sigue el orden de las operaciones (abajo).
- **Ninguna receta con cota positiva volvió una cara.** La cota es
  prudente: dos recetas con cota cero o negativa salieron bien.
- **Los triángulos sin área son siempre 2 × segmentos**, los de los polos.

### Reglas estáticas (R1), en `generate::check`

Cota inferior del radio, en unidades del radio de la esfera: empieza en 1;
cada `stretch` la multiplica por el menor de sus tres factores (el extremo
bajo si es un rango); cada `noise` le resta su amplitud (el extremo alto).
Si en algún paso llega a 0 o menos: `generator_radius`, error. La suma de
las amplitudes máximas por encima de 0,5: `generator_amplitude`, aviso.

### Leer la malla (R2), en `kiln-core`

Módulo nuevo `glb::mesh`: de un GLB saca las posiciones (`VEC3` `float`) y
los índices (`u16` o `u32`; sin índices, de tres en tres) de cada primitiva
de triángulos, con cada desplazamiento comprobado contra el tamaño del
fichero y un tope de vértices. Sobre eso, `generate::measure`:

- **Sin área**: el producto vectorial de dos lados mide menos de 10⁻⁹ veces
  el cuadrado de la distancia al centro.
- **Vuelta**: la normal de la cara apunta hacia el origen.
- **Empinada**: la normal forma más de 60° con la dirección desde el origen.
  Antes de medir se deshace el estirado total de la receta (los factores ya
  sorteados), porque un elipsoide limpio daba 0,86 sin esa corrección.

Se mide en Rust y no en el guion, para que quien genera no sea quien juzga.

### La tanda de semillas (R3)

`[generate] verify_seeds` en `kiln.toml`, por defecto 16, entre 1 y 64. La
primera es la semilla de la ficha; la número `k` (desde 1) son los 32 bits
bajos de `mix("<semilla>/verify/<k>")`, con el `mix` de K010.

### Verificar (R3 a R5)

`verify_asset`, cuando la ficha lleva receta:

1. Reglas estáticas; con algún error, no genera nada.
2. Por cada semilla, genera en `<cache>/generated/<id>/verify/`, con su
   propia carpeta de trabajo, sin tocar los GLB de la vista previa.
3. Mide cada GLB y compara con lo esperado: triángulos igual a
   `sphere_tris`, sin área como mucho 2 × segmentos, ninguna vuelta. El
   presupuesto es `budget_tris` de la ficha si lo tiene y `rules.max_tris`
   siempre. Empinadas sobre `[generate] steep_max` (0,10 por defecto) avisa.
4. Las comprobaciones propias del GLB (`validate::glb_findings`) pasan por
   todas las semillas; el validador de Khronos y la importación en Godot,
   solo por la de la ficha.
5. Escribe `<build>/<id>/validation.json` con la forma de hoy más el bloque
   `recipe`.

### El guion (R8)

`godot/kiln_generator.gd`: `extends RefCounted`, con
`static func build(job: Dictionary) -> Dictionary` que devuelve la malla o
un error. Es lo que hoy hace `generate.gd` por dentro, sin cambiar un paso.
`generate.gd` queda como lanzador: lee el encargo, llama a `build`, exporta
el GLB y escribe las medidas.

### Publicar (R6, R7)

- **Qué se ata.** No hay un GLB único, así que la verificación guarda el
  sha256 de la receta (su tabla `[generator]` y `budget_tris`, en JSON con
  las claves ordenadas), la versión del generador y el sha256 del guion.
  Publicar los recalcula y se niega si alguno no cuadra.
- **Qué se escribe**, en `[publish] dir/<id>/`:
  - `recipe.json`: id, receta, versión del generador, y por cada semilla
    verificada sus operaciones con los valores ya sorteados, tal como las
    recibe `build`, más las medidas del peor caso;
  - `kiln_generator.gd`;
  - `asset.json`: el contrato, con `version` y el resumen de la
    verificación.
- **Versión.** Sube cuando cambia `recipe.json` o el guion.
- **Godot.** No hay nada que importar; publicar no lanza Godot para un
  asset generado y lo dice.
- Las reglas de carpeta de hoy (dentro del proyecto, sin seguir enlaces) se
  aplican igual.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.
- **Entrada externa**: el lector de mallas abre ficheros que deja Godot en
  la caché; aun así comprueba cada desplazamiento y pone tope a lo que lee.
  El número de semillas tiene tope. Lo publicado no lleva rutas del equipo.

## Out of scope

- El piloto del asteroide en una escena de Godot (P-5) y comprobar que lo
  publicado regenera la misma malla en un Godot limpio.
- Botones de Verificar y Publicar en la página (D-7).
- Ampliar el vocabulario, incluida la esfera de triángulos uniformes (D-2).
- Cavidades, puertos y estructuras modulares (D-3).
- Que ASTRA lea lo publicado.
- Publicar GLB horneados (D-4).
- Detector general de autointersecciones: con este vocabulario basta la
  cota del radio y las caras vueltas.
- Limpiar los GLB que deja cada semilla de la vista previa.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| La cota del radio rechaza recetas que saldrían bien | Hay que bajar amplitudes sin necesidad | Medido: dos de once recetas. Es el precio de una regla que no depende de la semilla; el mensaje dice qué operación la agota. |
| El umbral de caras empinadas (0,10) sale de once recetas | Avisos de más o de menos | Es solo aviso y se ajusta en `kiln.toml`. |
| La medida de empinadas es aproximada cuando el ruido va después del estirado | El aviso puede no saltar | La cota y las caras vueltas, que son las que bloquean, no dependen de ella. |
| 16 semillas a 256 segmentos son 32 MB de GLB en la caché | Disco | La carpeta de la tanda se vacía en cada verificación. |
| Nadie ha llamado a `kiln_generator.gd` desde un juego | Lo publicado podría no servir tal cual | La verificación genera con ese mismo fichero; el uso desde el juego es el piloto. |
| Partir `generate.gd` cambia la malla sin querer | Los GLB de K010 dejan de coincidir | R8 exige el mismo sha256 que antes del cambio, medido en la misma máquina. |

---
Next stage: Build reads this file and writes `plan.md`.
