# Plan: Materiales por zona (entrega 8)

- **Derived from**: `spec.md`
- **Status**: APPROVED BY ENGINEER (Aarón, 2026-11-13)
- **Nothing is implemented until this file is approved.**

## Hecho significa

- Una ficha con dos zonas en una pieza y una variante se monta con
  `kiln build`: el GLB lista materiales `parte/zona` y sale un segundo GLB de la
  variante sin volver a montar.
- En la vista 3D, cambiar una zona se ve al instante y la ficha cambia en disco.
- Godot 4.7.1 importa el GLB montado con los materiales externos de la paleta.
- Una pieza de una ranura con dos trozos sueltos sale con dos zonas.
- `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`
  en verde.

## Parar y preguntar si

- La lectura de la decisión 1 (ficha mientras se prueba, catálogo al validar)
  no es la que querías.
- Un test que ya existe falla por algo ajeno al cambio.
- Partir por textura da zonas inservibles en el modelo de prueba.
- Hace falta una dependencia nueva en `kiln-core`.

## Tareas

Cada encargo cubre un punto y menos de 100 líneas nuevas.

| # | Tarea | Ficheros | Test que lo prueba | Requisito | Corre en | Depende | Est | Estado |
|---|-------|----------|--------------------|-----------|----------|---------|-----|--------|
| T1 | `Part.materials` y `[[variants]]` en la ficha, con sus avisos (zona que no existe, clave fuera de la paleta) | `kiln-core/src/spec.rs` | Test unitario: lee una ficha con zonas y variante; avisa de zona y clave malas | R1, R8 | implementer | — | 2m/200k | done |
| T2 | Vínculos de Godot: función pura que pone `_subresources.materials` en el texto de un `.import`, y la plantilla `[materials] godot` | `kiln-core/src/godot.rs`, `kiln-core/src/config.rs` | Test unitario con el `.import` real de Godot 4.7.1 como fixture: añade, cambia y conserva el resto | R4 | implementer | — | 2m/200k | done |
| T3 | Validar: pasar las zonas de una pieza de la ficha a los vínculos del catálogo | `kiln-core/src/bindings.rs` | Test unitario: tras validar, el vínculo está en el catálogo | R9 | implementer | — | 1m/150k | done |
| T4 | `set_material(parte, zona, clave, variante)` y quitar zonas validadas de la ficha | `kiln-core/src/spec.rs` | Test unitario: escribe y borra una zona, en el base y en una variante | R3, R9 | implementer | T1 | 1m/150k | done |
| T5 | El plan resuelve `zones` por copia (ficha → `material` → catálogo → modelo) y escribe `materials.json`, también por variante | `kiln-core/src/build.rs` | Test unitario del orden de resolución y del mapa `parte/zona` | R1, R2, R6, R8 | implementer | T1 | 2m/300k | done |
| T6 | `kiln/zones.toml`: cómo se parte cada pieza del catálogo (`loose`, `texture`), y el plan lo lleva | `kiln-core/src/zones.rs` (nuevo), `kiln-core/src/lib.rs` | Test unitario: lee el fichero y lo aplica a una pieza | R7 | implementer | — | 1m/150k | done |
| T7 | El montaje nombra los materiales `parte/zona` y conserva el del modelo en zonas sin elegir | `kiln-cli/blender/assemble.py` | `kiln build` sobre el proyecto de prueba: `glb.rs` lee los nombres esperados | R2, R6 | main | T5 | 5m/2M | done (cause: none) |
| T8 | «Aplicar materiales»: `repaint.py`, `kiln paint --asset ID [--variant V]` | `kiln-cli/blender/repaint.py` (nuevo), `kiln-cli/src/production.rs`, `kiln-cli/src/main.rs` | Antes y después: mismas medidas y triángulos, otros materiales; sale el GLB de la variante | R5, R8 | main | T7 | 5m/2M | done (cause: none) |
| T9 | Partir la malla al cargarla: por trozos sueltos y por textura | `kiln-cli/blender/assemble.py` | `kiln build` con una pieza de dos islas: dos zonas en el GLB | R7 | main | T6, T7 | 5m/2M | done (cause: glTF repeats vertices at hard edges; islands had to merge by position) |
| T10 | Escribir el `.import` tras montar o aplicar; lanzar `godot --import` si falta | `kiln-cli/src/production.rs` | Script de Godot que carga el GLB y dice qué material lleva cada zona | R4 | main | T2, T8 | 4m/1.7M | done (cause: a .blend inside the Godot project stopped its import; moved to a hidden folder) |
| T11 | Rutas `POST /api/material`, `/api/paint`, `/api/validate` | `kiln-cli/src/serve.rs` | `curl` contra `kiln serve`: la ficha cambia; sin `X-Kiln` se rechaza | R3, R5, R9 | implementer | T3, T4, T8 | 2m/300k | done (cause: run on main instead of implementer; added /api/split) |
| T12 | Vista 3D: desplegable por `parte/zona`, cambio al instante, selector de variante, botones «Aplicar materiales» y «Validar» | `kiln-cli/ui/index.html` | Captura antes y después en el navegador | R3, R8 | main | T11 | 6m/2.5M | done (cause: none) |
| T13 | Ejemplo con zonas, una variante, paleta y kit mínimos; README y skill del asset | `examples/`, `README.md`, `skills/asset/SKILL.md` | `kiln check` del ejemplo sin errores | R1, R8 | implementer | T1 | 2m/200k | done (cause: run on main; the kits needed Blender to generate) |
| T14 | Verificación de punta a punta y dos flujos vecinos (revisor visual, traer de fuera) | — | Informe del verificador con capturas | todos | verifier | T12, T13 | 4m/900k | done (cause: 21 min; the verifier also probed the routes for input holes and found three, fixed after) |

## Calendario

| Tanda | Tareas (en paralelo) | La más larga | Después (main) |
|-------|----------------------|--------------|----------------|
| 1 | T1, T2, T3, T6 | 2m | `git diff --stat`, batería entera |
| 2 | T4, T5, T13 | 2m | `git diff --stat`, batería entera |
| 3 | T7 | 5m | montar el proyecto de prueba |
| 4 | T8, T9 | 5m | montar y aplicar |
| 5 | T10, T11 | 4m | prueba con Godot y `curl` |
| 6 | T12 | 6m | capturas |
| 7 | T14 | 4m | leer el informe |

- **Reloj estimado**: 28 min, más las comprobaciones entre tandas
- **Suma secuencial**: 42 min
- **Tokens estimados**: unos 13 M

## Orden y dependencias

La ficha (T1) va primero porque el plan, la escritura y el ejemplo la leen. El
montaje (T7) fija los nombres `parte/zona`, de los que dependen aplicar,
partir, Godot y la vista 3D. T9 y T7 tocan el mismo fichero, por eso van en
tandas distintas.

## Estrategia de pruebas

TDD en el núcleo: test rojo antes de implementar. Si un test falla, se arregla
el código.

- **Unitarias**: ficha, orden de resolución, `materials.json`, texto del
  `.import`, validar, `zones.toml`.
- **Integración**: `kiln build`, `kiln paint` y `kiln godot` sobre el proyecto
  de prueba fuera del repo, con Blender 5.2.3 y Godot 4.7.1.
- **Verificación manual**: la vista 3D, con capturas; el verificador repasa
  además el revisor visual y traer un modelo de fuera.

## Comandos de verificación

```
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Riesgos de implementación

- `Item` cambia `material` + `slots` por `zones`: lo leen `assemble.py`,
  `render.py` (si usa el plan) y la página. Hay que repasar los tres.
- Las fichas con `material` tienen que montar igual que antes.
- Los nombres con `/` valen en glTF y en Godot (probado); en three.js se busca
  por nombre exacto.
- `render.py` del revisor no debe cambiar de aspecto por los materiales nuevos.

## Estimado frente a real

Se rellena al cerrar Build desde `actuals.jsonl`.

| # | Corre en | Est | Real | Desviación | Causa |
|---|----------|-----|------|------------|-------|

## Hallazgos del verificador (T14)

Veredicto ámbar: todo pasa, con tres fallos importantes, ya arreglados y
vueltos a probar.

| Hallazgo | Arreglo |
|----------|---------|
| El nombre de una variante no se validaba y acababa en rutas de fichero | `spec::plain_id` (letras, números, guiones); lo exigen `set_material`, `check` (`variant_id`) y `kiln paint`. Test `a_variant_id_that_is_not_a_plain_name_is_refused` |
| `/api/split` escribía `zones.toml` aunque devolviera error, y aceptaba piezas que no existen | La pieza tiene que estar en el catálogo antes de escribir |
| Volver a montar dejaba viejos los GLB de las variantes | `kiln build` vuelve a aplicar cada variante de la ficha |
| Validar una zona de una pieza partida daba el aviso `binding_slot` | El aviso no sale en piezas partidas |
| «Validar» aceptaba piezas `retouch` | Solo `reuse` y `variant` |

Sin arreglar, por decisión: la ficha pierde los comentarios al guardarla
desde la página (ya pasaba con `set_source`); `/api/material` acepta una zona
que no existe en una pieza partida (las zonas solo se conocen al montar);
`/api/bind` tiene el mismo patrón de escribir antes de comprobar (es anterior
a esta entrega).

## Fuera de este plan

- Partir en zonas por reglas y a mano.
- Proponer vínculos con SigLIP 2.
- `CLAUDE.md` y `REVIEW.md` del repo.

---
Siguiente etapa: Build implementa, Test verifica, Deploy abre la PR.
