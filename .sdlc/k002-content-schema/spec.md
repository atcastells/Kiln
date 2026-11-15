# Spec: validación de contenido genérica, declarada por el juego (K002)

- **Derived from**: `intent.md`
- **Status**: approved (2026-11-15)
- **Approved by**: Aaron Castells

## Cómo está hoy (K001, commit `4d5016b`)

- `kiln-core/src/astra/mod.rs:12-31`: `enum Definition` con cuatro
  variantes fijas; `refs.rs` comprueba referencias por tipo con reglas
  escritas a mano; `manifest.rs` compara IDs, escena del sector y extremos
  de la puerta con el JSON exportado de Godot.
- `config.rs:43-54`: `[integrations.astra]` (`enabled`, `definitions`,
  `godot_manifest`, `greybox`).
- `kiln-cli/src/astra.rs` y `godot/astra_manifest.gd`: subcomando
  `astra-manifest`; lee `metadata/astra_id` y `metadata/astra_connects`.
- Once códigos `astra_*`. `Finding` no tiene campo de origen.
- La interfaz solo enseña los hallazgos en la lista global.
- 78 tests en el workspace (36 de K001), más uno con Godot marcado
  `#[ignore]`.

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Conjuntos de contenido declarados en `kiln.toml` (`[content.<nombre>]`): globs, manifiesto, `greybox`, clave de metadatos del ID y sus tipos | nada con nombre de juego | grep de `astra` en `crates/` solo da resultados en tests y fixtures |
| R2 | Cada tipo declara: campo de ID, campo de escena, campo de asset, referencias (campo → tipo exigido, simple o lista), campos que deben ser distintos entre sí, y metadatos de nodo a comparar | esquema declarado | un test por regla, con un esquema inventado que no es el de ASTRA |
| R3 | Lector de RON sin tipos de Rust por juego: lista de variantes con nombre, o un registro con una lista dentro (`root`) cuyos elementos son de un tipo declarado | Astra valida su fichero directamente | lee la fixture de K001 y `content/astra/modules.ron` |
| R4 | Sublistas con marcadores: una lista de un elemento (conectores, anclajes) se compara con nodos hijos de la escena por nombre y metadatos | conectores y anclajes | cambiar un `offset` en una escena da un hallazgo |
| R5 | Códigos genéricos `content_*`, con el nombre del conjunto en el hallazgo | nada con nombre de juego | los once casos de K001 tienen su equivalente |
| R6 | `kiln content-manifest [--set NOMBRE]` sustituye a `astra-manifest`; el script de Godot recibe las claves a leer | nada con nombre de juego | el test con Godot pasa con la fixture migrada |
| R7 | Paridad: la fixture `examples/astra` (renombrada a `examples/content`) da los mismos hallazgos que con K001, con los códigos nuevos | la fixture pasa lo mismo | test que compara caso a caso |
| R8 | Interfaz: panel «Contenido» con cada conjunto, sus tipos, sus elementos y sus hallazgos; visible aunque no haya errores | se ve algo | `/api/catalog` (o ruta nueva) devuelve los conjuntos; comprobación manual con captura |
| R9 | Astra migrada: `kiln.toml` con su esquema, `data/astra/modules.ron` y su test borrados, `scripts/kiln_check.sh` adaptado | Astra sin duplicado | `scripts/kiln_check.sh` en verde, y en rojo con una escena rota y con un conector cambiado |
| R10 | Un proyecto sin `[content]` se comporta igual que antes; una clave de esquema desconocida es un error, no se ignora | no romper | `kiln check --project examples` da la misma salida antes y después |
| R11 | README: la sección ASTRA pasa a ser «Validar contenido», con el ejemplo y los comandos ejecutados | — | README revisado |

## Entrada de diseño de Astra (2026-11-15)

> Módulos con dimensiones, conexiones compatibles, función, capacidad,
> requisitos técnicos y restricciones espaciales. […] Kiln deberá validar
> estos metadatos.

Consecuencia para K002: el esquema declarativo tiene que poder exigir
campos y comprobar sus valores, no solo referencias. Se añade R12. Lo que
depende del generador (corredores transitables, redes funcionales,
compatibilidad con el asteroide) no es de Kiln: Kiln valida contenido, no
distribuciones.

| # | Requirement | Traces to | Acceptance criterion |
|---|-------------|-----------|----------------------|
| R12 | Reglas de campo por tipo: obligatorio, tipo de dato (entero, texto, lista, par), rango numérico, valor dentro de una lista cerrada, y compatibilidad entre conectores declarada como tabla (aplazada: ningún contenido usa todavía clases de conector) | entrada de diseño | un test por regla con un esquema inventado; en Astra, un `kind` desconocido o un `side` de conector desconocido da un hallazgo |

## Design

```toml
[content.station]
definitions = ["content/astra/**/*.ron"]
manifest = "kiln/content_manifest.json"
greybox = true
id_meta = "astra_id"            # metadato de Godot con el ID del nodo
root = "modules"                # opcional: el fichero es un registro con esta lista
default_kind = "Module"         # tipo de los elementos sin nombre de variante

[content.station.kinds.Module]
id = "key"
id_prefix = "sector:"           # el ID en la escena es prefijo + valor
scene = "scene"

[[content.station.kinds.Module.markers]]
field = "connectors"            # lista dentro del elemento
name = "name"
node = "conn_{name}"            # nodo hijo esperado
meta = ["side", "offset"]       # se comparan con sus metadatos
```

Para un esquema como el de K001: `refs = { connects = "Sector" }`,
`distinct = ["connects"]`, `node_meta = { connects = "astra_connects" }`,
`asset = "asset"`.

- `kiln-core/src/content/`: `schema.rs` (tipos de configuración y su
  validación), `value.rs` (árbol de valores leído del RON), `parse.rs`,
  `refs.rs`, `manifest.rs`, `markers.rs`, `mod.rs` (`run`). Sustituye a
  `astra/`.
- `Catalog` gana `content: Vec<ContentSet>` (nombre, tipos, elementos con
  fichero e ID) para la interfaz. `Finding` no cambia: el conjunto va en
  el `subject` (`<conjunto>:<fichero>#<id>`).
- Manifiesto: versión 2; por nodo, su ID, los metadatos pedidos y sus
  hijos marcadores.
- Interfaz: una pestaña más en `ui/index.html`, alimentada por el
  catálogo.

Decisiones, como propuestas:

- **G1** Se elimina `[integrations.astra]` sin alias; Astra se migra aquí.
- **G2** El esquema va en `kiln.toml`, no en un fichero aparte.
- **G3** Solo RON por ahora; el lector queda detrás de una función para
  poder añadir TOML o JSON.
- **G4** Trabajo en un worktree nuevo desde `origin/main`, rama
  `k002-content-schema`; no se toca el checkout con `e10-wizard-chat`.

## Policy applied

Sin datos personales, de negocio ni credenciales. Entrada externa: los
ficheros del proyecto (`kiln.toml`, RON, JSON del manifiesto); todo error
de lectura es un hallazgo, nunca un pánico. Sin dependencias nuevas
(`ron` ya está).

## Out of scope

Editar contenido desde Kiln, generar escenas, otros formatos, y cualquier
conocimiento del runtime o de los guardados de un juego.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `ron` no permite leer variantes con nombre sin un tipo de Rust | R3 no sale como está | primer encargo: prueba de riesgo; alternativa, un lector propio pequeño del subconjunto de RON usado |
| El esquema declarativo se queda corto o crece sin control | complejidad | solo las reglas que usan la fixture y Astra |
| Astra depende hoy de K001 | romper `scripts/kiln_check.sh` | la migración de Astra va en el mismo plan y se verifica al final |
| Otra sesión trabaja en `~/Kiln` (`e10-wizard-chat`) | conflictos en `index.html` y `serve.rs` | worktree aparte; los conflictos se resuelven al mergear, no aquí |

---
Next stage: Build reads this file and writes `plan.md`.
