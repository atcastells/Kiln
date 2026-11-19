# Spec: Kiln produce y valida el artefacto de geometría canónica v0

- **Derived from**: `intent.md` (aprobado el 2026-11-18)
- **Status**: approved (2026-11-18), con D1–D6, `class` radial = 0 / general = 1, R9 y R10
- **Approved by**: product owner. No hay policy owner: no toca datos personales ni de negocio.
- **R10 y R9** no trazan a una línea literal del intent: R10 sale de S1 §5.2 y R9 es una restricción del repo Kiln necesaria para R8. Aceptados expresamente con la spec.
- **Norma**: AST-ESP-001-S1 (ASTRA, `docs/design/ast-esp-001-s1-geometria-canonica-v0.md`) (en adelante, S1). Esta spec no cambia nada de S1; donde S1 deja algo abierto, lo propone en el apartado *Decisiones*.

## Lo que hay hoy en Kiln

Leído en `~/Kiln`, rama `k012-asteroid-pilot` (`2651608`).

- `kiln verify` de una receta construye cada semilla de la tanda (`[generate] verify_seeds`, 16 por defecto) en Godot sin ventana: `crates/kiln-cli/godot/generate.gd` llama a `kiln_generator.gd`, exporta un GLB a `kiln/cache/generated/<id>/verify/` y Kiln lo lee con `kiln_core::mesh::read` (posiciones `f32`, índices `u32`) en `crates/kiln-cli/src/verify_recipe.rs`. Ese es «el GLB que Kiln lee».
- `kiln publish` de una receta (`crates/kiln-cli/src/ops/publish_recipe.rs`) no lanza Godot ni lee GLB: comprueba `validation.json` y escribe `recipe.json`, `kiln_generator.gd` y, el último, `asset.json`. `version` solo sube si cambian la receta, el guion o la versión del generador.
- El vocabulario de generación es `sphere`, `stretch`, `noise` y `facet` (`crates/kiln-core/src/generate/vocab.rs`).
- Kiln no supone nada de un proyecto concreto: lo que dependa de ASTRA va en su `kiln.toml`.
- La cadena de K012 vive en Kiln (`~/Kiln/.sdlc/k012-asteroid-pilot/`), no en ASTRA. La pregunta 1 del intent suponía lo contrario.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Al verificar una receta del vocabulario de esfera con la geometría canónica activada, Kiln obtiene de cada GLB de la tanda una geometría canónica: cuantiza (S1 §5.5), une enteros iguales, descarta los triángulos que repiten vértice y exige `2 + S·S/2` vértices y `S·S` triángulos (S1 §2.2) | «Kiln, al publicar un paquete del vocabulario de esfera, escribe un artefacto» | Prueba con la esfera de 8 segmentos de los tests actuales: 34 vértices, 64 triángulos. Un recuento distinto se rechaza con su código |
| R2 | Un validador de Kiln comprueba I1–I9 (S1 §3.1) con aritmética entera exacta (`i128` donde haga falta) y devuelve el primer incumplido con código y elemento (S1 §3.3). Sin aceptación parcial ni reparación. `I8_not_radial` e `I9` dicen «forma correcta que v0 no admite» | «El validador de Kiln comprueba I1–I9 con enteros y rechaza con código y elemento» | Cada uno de los 22 negativos da el código de `measurements.md` §8 |
| R3 | La verificación falla si alguna semilla de la tanda no da geometría canónica válida; el informe dice semilla, código y elemento. El paso canónico corre en todas las semillas aunque fallen comprobaciones anteriores (`generated_flipped` y demás) | Mismo punto | Prueba: con la opción activa, una cara invertida da `I5_inconsistent_orientation` del validador canónico junto a `generated_flipped`, en todas las semillas. Las recetas con cota radial ≤ 0 las rechaza antes la regla estática `generator_radius` (decisión 002); su `I8_not_radial` se prueba en la vía A (T10). |
| R4 | `kiln publish` escribe en el paquete un artefacto por semilla de la tanda (contenedor en D3) y añade a `asset.json`, por semilla, `geometry_sha256` (S1 §6.2), más la procedencia de S1 §6.3: versión de Kiln y del motor. `recipe_sha256`, `script_sha256` y `generator_version` ya están | «añade al manifiesto el `geometry_sha256` (§6.2) y la procedencia del §6.3» | Prueba: el `geometry_sha256` del manifiesto es el SHA-256 del artefacto, y publicar dos veces lo mismo da los mismos bytes |
| R5 | Antes de escribir nada, `publish` vuelve a validar cada artefacto (I1–I9 y hash). Si uno falla, no se publica ningún archivo | «no hay aceptación parcial» | Prueba: un artefacto alterado en la caché hace fallar la publicación y deja el paquete como estaba |
| R6 | Los 67 positivos y los 22 negativos de S1 §8 dan el veredicto de `measurements.md` §8. Todo caso que puede salir de una receta se lee del GLB que exporta `generate.gd`; los demás, por la vía que les corresponde (tabla *Casos*) | «Los 67 casos positivos y los 22 negativos… leyendo el GLB que Kiln lee de verdad» | Tabla de resultados en el cierre, caso por caso y con su vía. Una discrepancia con M§8 se explica, no se ajusta |
| R7 | Se mide si el mismo paquete da el mismo `geometry_sha256` en x86-64 (D5), o se deja escrito por qué no se pudo | «Queda medido si el mismo paquete da el mismo `geometry_sha256` fuera de arm64» | Tabla de los 48 hashes en los dos equipos, con la naturaleza de cada uno (nativo o emulado) |
| R8 | K012 sigue igual: `recipe.json`, `kiln_generator.gd` y su `script_sha256` no cambian de bytes; el consumidor de ASTRA y sus pruebas pasan sin tocarlos | «K012 no cambia» | `git diff` sin cambios en esos archivos ni en `godot/procedural/` y `godot/tests/asteroid_*`; fase `asteroid` de `scripts/godot_station.sh` en verde |
| R10 | Si el GLB trae una transformación en el nodo de la malla, la geometría canónica se rechaza; no se aplica. Hoy `kiln_core::mesh::read` no la lee (comprobado: no hay `matrix`, `translation` ni `transform` en `mesh.rs`) | S1 §5.2 y «leyendo el GLB que Kiln lee de verdad»: lo que Kiln lee debe ser lo que hay | Prueba con un GLB a mano con `translation` en el nodo: rechazo con código propio |
| R9 | Kiln sigue sin saber de ASTRA: la geometría canónica se activa en `kiln.toml` del proyecto; sin activarla, verify y publish hacen exactamente lo de hoy | Restricción del repo Kiln (`CLAUDE.md`, «Kiln no supone nada de un proyecto concreto»); necesaria para R8 en otros proyectos | `cargo test --workspace` de Kiln en verde sin cambiar ningún test existente; `examples/` publica igual que antes |

## Design

**Dónde.** Biblioteca nueva en `kiln-core` (`canonical`: cuantizar, unir, validar, serializar y hash), sin E/S ni dependencias nuevas (`sha2` ya está en `kiln-core`). `kiln-cli` la llama en dos sitios:

1. **Verify** (`verify_recipe.rs`): tras leer el GLB de cada semilla, si la geometría canónica está activa, construye el artefacto, lo valida y lo deja en la caché junto al GLB. `validation.json` gana por semilla `geometry_sha256` o el rechazo.
2. **Publish** (`publish_recipe.rs`): lee los artefactos de la caché, los revalida (R5), los copia al paquete y escribe `asset.json` el último, como hoy.

**Datos.** GLB (`f32`, metros) → `round(f64(x) × 1000)` mitades lejos de cero → `i32` → artefacto. Nadie más cuantiza. El centro es (0, 0, 0) del marco local del GLB (R10).

**Orientación.** Godot y glTF usan sentidos de giro opuestos y el exportador de Godot los convierte. M§8 midió sobre los arrays de Godot. En el GLB, el sentido hacia fuera puede salir ya bien o al revés: se mide en la primera tarea del plan, antes de escribir el validador, y se documenta. El validador no da la vuelta a nada: si el GLB llega al revés, el paso de construcción lo invierte de forma explícita y probada, una vez.

**Coste.** Por semilla: 18,5 KB a 32 segmentos, 74 KB a 64 y 295 KB a 128 (32 bytes de cabecera más 12 por vértice y 12 por triángulo). Los tres paquetes de K012 con 16 semillas: unos 6,2 MB nuevos en el repo de ASTRA.

## Decisiones que esta spec propone (las seis preguntas del intent)

| # | Pregunta | Propuesta |
|---|----------|-----------|
| D1 | Dónde viven código y cadena | Código **y cadena** en Kiln, como K012: `~/Kiln/.sdlc/k013-canonical-geometry/`, con este intent y esta spec movidos allí al aprobarla (en ASTRA no están en git). En ASTRA, solo los paquetes republicados, en la rama de trabajo |
| D2 | Identificador | **K013** |
| D3 | Contenedor | **El archivo es exactamente la secuencia de bytes que entra en el hash** (S1 §6.2): little-endian, sin cabecera extra. Así `geometry_sha256` = SHA-256 del archivo y no hay dos formatos que mantener. Nombre: `geometry/seed_<n>.bin` dentro del paquete; `files` de `asset.json` los enumera. Se lee sin ambigüedad: los recuentos van antes de los arrays y el tamaño debe ser exactamente 32 + 12·V + 12·T bytes. **Valor de `class`:** S1 dice `class (u32)` y `radial`, pero no fija el número, y los guiones de medición no calculan el hash (comprobado en `measure/geo.py`). Propuesta: `radial` = 0, `general` = 1 |
| D4 | Republicar K012 o paquete nuevo | **Republicar los tres paquetes** con artefacto. `recipe.json` y `kiln_generator.gd` no cambian de bytes; `asset.json` gana campos y archivos. Es lo que S1 §9.1 describe: «un archivo nuevo en el paquete», nada publicado se reescribe. **`version` sigue en 1**, porque la lógica actual de `publish_recipe.rs` solo la sube si cambian receta, guion o generador: un consumidor sabrá si hay artefacto por la presencia de `geometry_sha256`, no por la versión. La alternativa es subirla a 2 cuando el paquete gane artefacto |
| D5 | Igualdad fuera de arm64 | **Emulación** en Docker (`--platform linux/amd64`, Godot 4.7.1 `linux.x86_64`), declarada como emulada en los resultados. No es lo mismo que un equipo x86-64 nativo, y se dice. Si tienes un equipo x86-64 real, se mide además allí. Si la emulación no arranca, queda escrito por qué y R7 se cierra como «sin medir» |
| D6 | Los 19 positivos no publicados y los negativos | **Fixtures de la batería de Kiln, nunca publicados en ASTRA**, repartidos en las tres vías de la tabla *Casos*. Las que piden Godot van como pruebas `#[ignore]` (como las 15 actuales), con sus GLB en la caché de la prueba, no en el repo. Las sintéticas corren en la batería normal |

## Casos: por qué vía entra cada uno

El paso de construcción de R1 (unir y descartar) arregla por diseño algunas entradas que M§8 rechaza, así que no todos los casos pueden pasar por él.

| Vía | Casos de S1 §8 / M§8 |
|---|---|
| **A.** Receta → `generate.gd` → GLB → construcción (R1) → validador | Positivos: las 48 semillas publicadas, las 16 escaladas, el cuerpo máximo, el borde (66). Negativos: las dos recetas negativas (`I8_not_radial`) |
| **B.** GLB real directo al validador, sin construcción | Negativos: arrays sin preparar, malla facetada, retícula sin soldar, esfera soldada bit a bit (`I3_duplicate_position`), y orden de triángulos de Godot (`I8_not_radial`). Si en el GLB el sentido de giro ya sale hacia fuera, este último se construye invirtiéndolo, y se dice |
| **C.** Artefacto sintético directo al validador | Positivos: el tetraedro centrado (`sphere` pide 8 segmentos como mínimo; no sale de una receta) y uno de los 66 de la vía A guardado como semilla de las mutaciones. Negativos: las 15 mutaciones restantes (triángulo quitado, invertido o repetido, vértices duplicados, fuera de rango, índice fuera, vértice sin usar, fundido, segundo cuerpo, tetraedro descentrado, voladizo, vértice a través del centro, toro, vacía, doble vuelta) |

Cuentas: 66 + 1 = 67 positivos (A y el tetraedro de C; el de mutaciones no cuenta aparte). Negativos: 2 + 5 + 15 = 22.

## Policy applied

- **Personal data**: none.
- **Sensitive business data**: none.
- **Credentials**: none.
- **Decisions about people / high risk**: no.

## Out of scope

- Rust de ASTRA (carga, I1–I9 al cargar, Q1–Q3) y si Q2 exige más de 128 bits: tarea 2 de S1 §10.
- Godot (derivación identidad): tarea 3.
- `space.rs`, `WorldState`, `FORMAT_VERSION` y el protocolo.
- La clase `general`, las regiones y la revisión de T-063 (S1 §9.2).
- Retirar `generated_flipped` o la tolerancia actual de `2 × segmentos` triángulos sin área: siguen tal cual; el validador canónico se añade, no los sustituye.
- Semillas fuera de la tanda verificada: el consumidor de K012 ya no las construye.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| El exportador de Godot cambia posiciones, orden o sentido de giro respecto a los arrays medidos | Los casos de M§8 no dan el mismo veredicto | Es justo lo que mide R6. Primera tarea del plan: exportar y comparar GLB contra arrays antes de escribir el validador |
| La emulación x86-64 da los mismos hashes por traducir a arm64 por debajo | Falsa seguridad sobre otros equipos | D5: el resultado se etiqueta «emulado», nunca «x86-64» a secas |
| El consumidor de K012 rechaza un `asset.json` con archivos o campos nuevos | Rompe R8 | Leído `godot/procedural/recipe_package.gd`: exige los tres archivos y comprueba `kind`, `generator_version`, `script_sha256` y `recipe_json_sha256`; no mira `files` ni rechaza campos de más. Riesgo bajo; lo confirma la fase `asteroid` |
| 6,2 MB binarios en el repo de ASTRA | Peso del repo | Aceptable para v0; si no, D4 alternativa: estrenar el artefacto en un paquete nuevo de pocas semillas |
| Una receta del vocabulario con cota negativa deja de verificar con la geometría canónica activa | Recetas que hoy pasan dejan de publicarse | Es el comportamiento que S1 §3.3 pide. Solo aplica con la opción activada (R9) |

---
Next stage: Build reads this file and writes `plan.md`.
