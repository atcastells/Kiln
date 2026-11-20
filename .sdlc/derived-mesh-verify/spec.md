# Spec: integridad de malla en `kiln verify`, activable por ficha

- **Derived from**: `intent.md` (forma B, elegida el 2026-11-20)
- **Status**: draft
- **Approved by**: product owner (pendiente)

Las referencias son de `~/Kiln` en `305ec28` (rama `claude/project-thread-ckdldl`,
ya unida a `main` por el PR #35), salvo las marcadas como ASTRA.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Una ficha activa las comprobaciones de integridad con una tabla `[mesh]`. Sin esa tabla, nada cambia: ni el informe ni la serialización de la ficha | «Una ficha puede pedir a `kiln verify`…» | Prueba sobre `examples/specs/hoist.toml`: se lee con `mesh = None`, se serializa sin `mesh` y `validate::mesh_check` devuelve `None` incluso para un GLB abierto |
| R2 | Antes de comprobar nada, `verify` suelda la malla por mm y rechaza, con su código y su elemento, lo que no se puede soldar con honestidad: coordenadas fuera de la rejilla, primitivas que no son triángulos y colocaciones de nodo que no sean una traslación | «sale en rojo con su código y su elemento» | Prueba de tabla: `offgrid` da `mesh_offgrid`; una primitiva con `mode` distinto de 4 y un nodo girado dan `mesh_layout` |
| R3 | Reglas: borde abierto, arista no variedad, orientación coherente, vértice pellizcado, degenerados, componentes y característica de Euler frente a la topología declarada (`components`, `genus`), y sentido de las normales (`normals`) | Ídem; «La cavidad … pasa tal como está» | Prueba de tabla con mallas mínimas mutadas: un código por mutación (tabla de §Pruebas) |
| R4 | Autointersección exacta con enteros, con poda por cajas | «dos triángulos que se cruzan» | Mutación `moved` (un vértice empujado a través de la malla) → `mesh_self_intersection` con el par de triángulos |
| R5 | La geometría canónica de K013 no cambia | Restricción del proyecto | `canonical_topology_*` (`topology.rs:165-317`) y `tests/canonical_cases.rs`, sin tocar y en verde |
| R6 | La cavidad publicada de EST-004 pasa con `components = 1`, `genus = 1`, `normals = "inward"` | «La cavidad publicada de EST-004 pasa…» | Prueba `#[ignore]` con `KILN_MESH_FIXTURE=<ASTRA>/godot/assets/kiln/historic_cavity/historic_cavity.glb`: sin hallazgos; con `genus = 0` → `mesh_topology` |
| R7 | ASTRA pasa `kiln check` antes de `build` en su procedimiento y activa `[mesh]` en `historic_cavity.toml` | «El aviso de claves de paleta … sale antes del primer montaje» | CLAUDE.md de ASTRA, sección Commands; `kiln verify historic_cavity` con `mesh_integrity: pasa` |

## Design

### Clave de la ficha

Hoy, una ficha solo ajusta la verificación con `budget_tris` (`spec.rs:44-45`) y con
las anclas (`spec.rs:19-21`). Las secciones opcionales cuelgan de `Spec` con
`#[serde(default, skip_serializing_if = "Option::is_none")]`, como `generator`
(`spec.rs:22-24`). Se propone una sección hermana:

```toml
[mesh]                 # su sola presencia activa las reglas
components = 1         # por defecto 1
genus = 1              # género total de la malla; por defecto 0
normals = "inward"     # "outward" (por defecto, sólido) | "inward" (cavidad)
```

- `Spec.mesh: Option<MeshRules>`, con el mismo atributo que `generator`. Si falta
  la tabla, el campo queda en `None`, la ficha se serializa igual que hoy y la
  prueba sigue el patrón de `a_spec_without_anchors_serializes_as_before`
  (`spec.rs:1000-1004`).
- `MeshRules` lleva `#[serde(deny_unknown_fields)]` (precedente: `content/schema.rs:8`).
  Una clave mal escrita dentro de `[mesh]` hace la ficha ilegible (`spec_unreadable`,
  `spec.rs:173-178`) y no se ignora en silencio. `normals` es un enum.
- Lo que comprueba `kiln check` (`spec::check`, `spec.rs:323-470`):
  - `components = 0` da `mesh_components` (Error).
  - `[mesh]` en una ficha con `[generator]` da `mesh_on_generator` (Error). Las
    recetas ya tienen la geometría canónica (K013), y `verify` las desvía a
    `verify_recipe` (`verify.rs:25-27`).
- La característica que se espera es χ = 2·`components` − 2·`genus`, donde `genus`
  es la suma de los géneros de todas las componentes. Para la cavidad,
  2 − 2 = 0, como en check.py (`:417`, ASTRA).

### Dónde corre

- **Módulo nuevo `kiln-core/src/integrity.rs`.** Recibe las mallas de `mesh::read`
  (`mesh.rs:17-24`, todas las primitivas de todas las mallas) y la estructura de
  nodos, y devuelve `Vec<Finding>`.
- **`validate::mesh_check(spec, bytes) -> Option<(Check, Vec<Finding>)>`.**
  Devuelve `None` sin `[mesh]`. Con `[mesh]`, devuelve un `Check` llamado
  `mesh_integrity` (`passed`/`failed`) y sus hallazgos.
- **`verify_asset`** (`verify.rs:41-105`) lo llama si el GLB se lee, y añade el
  `Check` después de `Check::own` (`verify.rs:88`).
- **Efecto en `publish`.** `Report::new` deja `valid = false` si un check falla
  (`validate.rs:518-531`), y `publish` exige `valid` (`ops/publish.rs:211`). Un
  rojo de integridad, por tanto, bloquea `publish`.

### Soldadura por mm (antes de comprobar)

El GLB separa los vértices por material y por normal: la cavidad tiene 639
vértices para 151 posiciones (ASTRA `.sdlc/historic-excavations/work/T14-result.md:20`).

1. **Colocación (`mesh_layout`, Error).** Se exige un solo nodo con malla y una
   transformación que sea la identidad o solo una traslación. Una traslación no
   cambia la topología, ni el signo del volumen, ni la rejilla en mm si es entera
   en mm. La cavidad publicada lleva `translation = [-260, -455, -2.5]` y una
   malla con 3 primitivas (lectura del GLB de ASTRA). Cualquier otra transformación
   (giro, escala, matriz, varios nodos) da `mesh_layout`, porque `mesh::read`
   ignora los nodos (`mesh.rs:29-32`). Una primitiva con `mode` distinto de 4 da
   también `mesh_layout`: hoy `mesh::read` la salta en silencio (`mesh.rs:84-86`).
2. **Rejilla (`mesh_offgrid`, Error, primer vértice).** Cada coordenada debe cumplir
   |x·1000 − round(x·1000)| ≤ tolerancia. La tolerancia es 0,05 mm, como
   `STORE_TOL_MM` de check.py (`:37`, ASTRA), o medio ulp de `f32` en esa
   coordenada si es mayor. Por encima de 1024 m el medio ulp pasa de 0,05 mm.
3. **Soldadura.** Se usa `canonical::quantize` (`canonical/build.rs:10-17`). Los
   vértices se numeran por su primer uso en un triángulo, de modo que las
   posiciones sin uso del buffer no cuentan. Los triángulos siguen el orden de
   las primitivas. A diferencia de `canonical::build`, no se exigen los recuentos
   de una esfera (`build.rs:53-62`).
4. **Degenerados (`mesh_degenerate`, Error).** Un triángulo que, tras soldar, repite
   un vértice, o que tiene área nula exacta (producto vectorial en `i128` igual a
   cero), se informa con su índice, el total y los cinco primeros. No se descarta
   en silencio, como hace `build.rs:49-51`. Las comprobaciones de topología no
   corren si hay degenerados, porque suponen I2.

### Reglas y códigos

El orden es el de `canonical::validate`: gana el primer paso que falla, porque
cada paso supone los anteriores (`topology.rs:1-3`). Cada hallazgo lleva el
código, el elemento (triángulo o vértice soldado, con la primitiva y el índice
original del GLB) y su posición en metros. El mensaje va en castellano, como los
demás hallazgos (`validate.rs:300-306`).

| Paso | Código del hallazgo | Fuente en Kiln | Nuevo |
|------|---------------------|----------------|-------|
| Lectura del GLB | `mesh_unreadable` | `mesh::read` (sin elemento; añadido en T8, en la tabla desde T11) | sí |
| Colocación y modo | `mesh_layout` | `mesh.rs:29-86` (lectura) | sí |
| Rejilla | `mesh_offgrid` | — | sí |
| Degenerados | `mesh_degenerate` | — (`build.rs:49` descarta) | sí |
| Tamaño mínimo | `mesh_too_small` | I1 `topology.rs:37-39` | no |
| Borde abierto | `mesh_open_edge` | I4 `topology.rs:63-65` | no |
| Arista de más de dos triángulos | `mesh_non_manifold_edge` | I4 `topology.rs:66-68` | no |
| Orientación | `mesh_orientation` | I5 `topology.rs:69-77` | no |
| Vértice pellizcado | `mesh_pinched_vertex` | I6 `topology.rs:85-106` | no |
| Componentes y χ frente a lo declarado | `mesh_topology` | I7 `topology.rs:107-131`, generalizado | en parte |
| Sentido de las normales | `mesh_normals` | — (check.py `:424`) | sí |
| Autointersección | `mesh_self_intersection` | — (check.py `:58-136`, `:435-445`) | sí |

**Reutilizar `topology.rs` sin romper K013.**

- `check_i1_i5` se llama tal como está (`topology.rs:34`).
- `check_i6_i7` (`topology.rs:83-133`) se parte en dos funciones públicas:
  - `check_i6` (abanicos);
  - `pieces_and_euler(g) -> (piezas, χ, primer triángulo de la segunda pieza)`.
- `check_i6_i7` sigue existiendo con sus mismos códigos y detalles, y se apoya en
  esas dos funciones. Así, `canonical::validate` (`canonical/mod.rs:45-48`) no
  cambia.
- Los códigos `I*` se traducen a `mesh_*` en `integrity.rs`. La geometría
  canónica sigue informando de los suyos.

**Componentes y χ.** Con χ ≠ 2·`components` − 2·`genus` o con otro número de
piezas sale `mesh_topology`, y el detalle dice: «N piezas, V − E + F = x; la ficha
declara c componentes y género g, que dan y».

**Normales.** Se calcula el volumen con signo ×6 = Σ orient(0, a, b, c) en `i128`.
Con coordenadas menores que 2^30 mm, cada término cabe en 2^93 y la suma de 2·10^6
triángulos en 2^114. Con `outward` se exige > 0; con `inward`, < 0, como en
check.py (`:424-426`, ASTRA). Un volumen nulo da `mesh_normals` en los dos casos.

**Autointersección.** Es el predicado entero de check.py (`:58-136`, ASTRA),
portado a `i128`. Cubre segmento-triángulo, el caso coplanario y los pares que
comparten una arista o un vértice. La poda pasa de todos los pares a un barrido
de cajas ordenadas por un eje: O(n log n + k), donde k es el número de pares
cuyos intervalos se solapan en ese eje (no el de pares que se cruzan). El eje
es el de menor suma de longitudes de caja respecto a la extensión de la malla
en él, comparado en enteros; así un tubo de triángulos tan largos como él no
barre a lo largo de su eje. Peor caso que queda, O(n²): triángulos cuyas cajas
se solapan en los tres ejes, como un abanico de muchos triángulos alrededor de
un vértice o un tubo largo en diagonal (medido en T11, `review/T11-bench-after.log`).
El hallazgo da el primer par que se cruza y el número de pares.

**Qué entra y qué no, con honestidad.** Hoy no existe en Kiln ni el volumen con
signo ni la autointersección (`work/T0-result.md`, §2).

- El volumen con signo entra: cuesta unas 10 líneas.
- La autointersección entra como la pieza más cara (T6 y T7 del plan) y está en
  el criterio del intent («dos triángulos que se cruzan»).
- Si su coste se dispara, es el primer candidato a salir a una cadena propia
  (ver *Riesgos*).
- Quedan fuera dos genéricas de check.py:
  - `zones_match_normals` (`:453`): depende de las zonas de la paleta y de un
    ángulo propio del proyecto.
  - `one_mesh_object` (`:405`): en Kiln, el nombre del nodo ya lo comprueban
    `validate.rs:25-280`; la parte de «una sola malla colocada» la cubre
    `mesh_layout`.
- `distinct_vertices` (`:412`) deja de tener sentido: se suelda, y los
  duplicados del GLB son esperables.

### Severidad

Error (propuesta; pregunta 2 abierta en `intent.md`). Bloquea `publish`
(`ops/publish.rs:211`). No hay clave para bajarla a aviso.

## Pruebas

Mallas mínimas en `integrity.rs`, con coordenadas reales en mm:

- octaedro de ±1000 (`topology.rs:152-158`);
- toro de 4×4 con radios 3000 y 1000, género 1.

Las mutaciones van en una sola prueba de tabla, con el código y el elemento
esperados.

| Mutación | Origen | Sobre | Código esperado |
|----------|--------|-------|-----------------|
| `removed`: quitar el triángulo 0 | check.py `:246` | octaedro | `mesh_open_edge` |
| `flipped`: invertir el triángulo 0 | check.py `:248` | octaedro | `mesh_orientation` |
| `offgrid`: vértice 0 + 0,4 mm | check.py `:250` | octaedro | `mesh_offgrid` |
| `extra_component`: tetraedro suelto dentro | check.py `:252` | octaedro | `mesh_topology` (2 piezas) |
| `moved`: vértice 0 empujado a través de la cara opuesta | check.py `:243` (allí lo cazan comprobaciones propias de la cavidad; aquí, la genérica) | octaedro | `mesh_self_intersection` |
| triángulo con área nula | check.py `:422` | octaedro | `mesh_degenerate` |
| todas las caras invertidas con `outward` | check.py `:424` | octaedro | `mesh_normals` |
| el toro declarado con `genus = 0` | check.py `:417` | toro | `mesh_topology` |
| arista en tres triángulos y vértice pellizcado | `topology.rs:231-236`, `:295-302` | octaedro, esfera UV | `mesh_non_manifold_edge`, `mesh_pinched_vertex` |

Los positivos son el octaedro con `outward` y el toro con `genus = 1`. A ellos se
suman la cavidad real (R6, `#[ignore]`) y la prueba de R1 sobre la ficha existente.

## Policy applied

- **Personal data**: ninguno.
- **Sensitive business data**: ninguno. Son mallas de un juego.
- **Credentials**: ninguna.
- **Decisions about people / high risk**: no.

## Out of scope

- Booleanas: Kiln no deriva mallas.
- `derived_from` y la procedencia con hash (descartado el 2026-11-20).
- La correspondencia con los cortadores, la solera, la transición, el cuerpo
  canónico y las otras comprobaciones propias de la cavidad: siguen en el
  check.py de ASTRA.
- `zones_match_normals`, la reproducibilidad del guion (`reproducible_by_vertices`) y
  las versiones de Blender.
- Los avisos de la ficha en `build` (forma A): los cubre el paso `kiln check` en ASTRA.
- El aviso `scale` (pregunta 4, abierta, recomendado dejarlo).
- La integridad de las recetas, que ya tienen la geometría canónica, y de los
  GLB con varios nodos o con giro o escala.
- Cambios en ASTRA, salvo dos: el paso `kiln check` en su procedimiento y
  `[mesh]` en `historic_cavity.toml`. Retirar comprobaciones de check.py queda a
  decisión del desarrollador.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| La autointersección exacta pasa de 100 líneas o da falsos positivos en aristas y vértices compartidos | Encargo partido o rojo falso en la cavidad | Partida en dos filas (predicados; barrido y regla). R6 sobre la cavidad real antes del cierre. Si no cabe, sale a una cadena propia con decisión del PO |
| La tabla `[mesh]` mal escrita (por ejemplo `[meshes]`) se ignora en silencio, porque `Spec` no rechaza claves desconocidas | La ficha cree que comprueba y no comprueba | `verify` dice `mesh_integrity: pasa`; sin la tabla, la línea no sale. Rechazar las claves desconocidas en `Spec` cambiaría las fichas existentes: fuera de alcance |
| Partir `check_i6_i7` cambia un detalle de K013 | Rompe la geometría canónica | Las pruebas de K013 no se tocan; R5 |
| La ficha de ASTRA cambia de sha al añadir `[mesh]` | `asset.json` sale «cambiada» y hay que republicar | Previsto en la fila de ASTRA: republicar con los mismos bytes del GLB |
| `target/debug/kiln` no coincide con HEAD (binario del 19-11 a las 23:18; último commit a las 23:19) | Medidas con un binario viejo | La línea base compila antes de medir |
