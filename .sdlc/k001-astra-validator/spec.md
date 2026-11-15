# Spec: Validar contenido de ASTRA con Kiln (K001)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aarón, 2026-11-15

## Cómo está hoy

- `Config` (`kiln-core/src/config.rs:9`) tiene secciones con
  `#[serde(default)]` y no rechaza claves desconocidas.
- `Finding { severity, subject, code, message }` y `Severity` viven en
  `kiln-core/src/catalog.rs:86-99`. No tienen campo de fichero.
- `open_with` (`kiln-core/src/lib.rs:34`) indexa, aplica Godot, zonas y
  bindings, llama a `check::run` y carga specs. `kiln check`
  (`kiln-cli/src/main.rs:99`) imprime los hallazgos y sale con 1 si hay algún
  `Error`, y con 2 si falla la carga.
- `kiln godot` ya ejecuta Godot sin ventana con un script embebido
  (`kiln-cli/godot/measure.gd`, `production.rs:14,245`): JSON de entrada y
  JSON de salida. El exportador seguirá ese patrón.
- `kiln-core` ya depende de `glob`, `serde`, `serde_json` y `toml`. Falta una
  librería RON.
- Kiln no tiene ningún concepto de greybox.

## Requisitos

| # | Requisito | Traza (intent) | Criterio de aceptación |
|---|-----------|----------------|------------------------|
| R1 | Sección opcional `[integrations.astra]` en `kiln.toml`. Sin ella, no cambia nada. | Restricción "opt-in"; respuesta 1 | Con el `examples/kiln.toml` actual, `kiln check` da la misma salida y el mismo código de salida antes y después (se compara con diff). Todos los tests que ya existían siguen en verde. |
| R2 | Leer definiciones RON con `serde` + crate `ron`. TOML sigue siendo la configuración. | Restricción RON/TOML | Unit test: la fixture válida se deserializa. |
| R3 | Esquema RON provisional, marcado como no canónico: sector, puerta, objeto e instalación. | Respuesta 3 | La fixture tiene dos sectores unidos por una puerta, un objeto con asset y una instalación que lo usa. |
| R4 | Comprobaciones con `code` estable (tabla de abajo), elemento y fichero en cada hallazgo, y salida distinta de cero si hay un error. | Problema; criterio "códigos y ficheros concretos" | Un unit test por código. Con la fixture rota, `kiln check` sale con 1. |
| R5 | Las referencias a assets se resuelven contra el catálogo de Kiln (claves de pieza). Sin un inventario aparte. | Restricción "catálogo" | Un asset que no está en el catálogo es `Error`, o `Warn` si `greybox = true`. |
| R6 | Ajuste `greybox` en la sección ASTRA. | Respuesta 2 | Mismo test que R5, con los dos valores. |
| R7 | Exportador de manifiesto desde Godot, reproducible con un comando de Kiln. | Restricción "manifiesto desde Godot" | `kiln astra-manifest --project examples/astra` regenera el JSON con Godot headless, y el resultado coincide con el que está en el repo. |
| R8 | Comparar el manifiesto con el RON: IDs, conexiones puerta/sector y rutas de escena. | Criterio "discrepancia tras exportar" | Cambiar `connects` de la puerta en el `.tscn`, exportar y validar da `astra_manifest_mismatch`. |
| R9 | README: ejemplo mínimo, configuración, exportación y validación. | Criterio README | Sección nueva en el README con los comandos que de verdad se ejecutaron. |

## Diseño

### Configuración

```toml
[integrations.astra]
enabled = true                             # por defecto false
definitions = ["data/astra/**/*.ron"]      # globs relativos a la raíz
godot_manifest = "kiln/astra_scene_manifest.json"   # opcional
greybox = false                            # true: asset ausente = Warn
```

`Config` gana `integrations: IntegrationsSection` con `#[serde(default)]`, y
dentro `astra: Option<AstraSection>`. Si es `None` o `enabled = false`, el
validador no se ejecuta.

### Esquema RON (provisional, no canónico)

Un fichero `.ron` es una lista de definiciones. La cabecera del fichero de la
fixture dirá que es provisional.

```ron
[
    Sector(id: "sector:warehouse", scene: "res://astra/warehouse.tscn"),
    Sector(id: "sector:yard",      scene: "res://astra/yard.tscn"),
    Door(id: "door:warehouse_yard", connects: ("sector:warehouse", "sector:yard")),
    Object(id: "object:crate", asset: "<clave de pieza del catálogo>"),
    Installation(id: "installation:loading_bay", sector: "sector:warehouse", objects: ["object:crate"]),
]
```

Es un `enum Definition` de serde. Los IDs son cadenas estáticas de contenido,
no `PersistentId`, y el validador no genera identidades.

### Módulo `kiln-core/src/astra.rs`

- `run(root, config, &mut Catalog)`: expande los globs, parsea cada fichero,
  construye un índice `id → (kind, fichero)`, ejecuta las comprobaciones y,
  si hay manifiesto, las de correspondencia. Añade `Finding` a
  `catalog.findings`.
- Se llama desde `open_with` justo después de `check::run`, solo cuando la
  sección está activa. La ordenación de hallazgos ya existente queda igual.
- El fichero y el elemento van en el `subject`, con la forma
  `<fichero>#<id>`, por ejemplo `data/astra/world.ron#door:warehouse_yard`.
  En errores de sintaxis, el `subject` es solo el fichero y el mensaje lleva
  línea y columna. Así no cambia el struct `Finding`, que tiene 12
  constructores, y no choca con los subjects `asset:<id>` que la página usa
  para las specs.

### Códigos

| code | Severidad | Cuándo |
|------|-----------|--------|
| `astra_ron_syntax` | Error | El RON no parsea o no encaja con el esquema (línea:columna). |
| `astra_no_definitions` | Warn | Los globs no encuentran ningún fichero. |
| `astra_duplicate_id` | Error | El mismo ID aparece dos veces (cita los dos ficheros). |
| `astra_unknown_ref` | Error | Una puerta o una instalación nombra un ID que no existe. |
| `astra_wrong_kind` | Error | La referencia existe pero es de otro tipo (puerta → objeto). |
| `astra_door_endpoint` | Error | Los dos extremos de la puerta son el mismo sector. |
| `astra_asset_missing` | Error / Warn (greybox) | El `asset` del objeto no es una pieza del catálogo. |
| `astra_scene_missing` | Error | El `res://` del sector no existe bajo `tools.godot_project`. |
| `astra_manifest_missing` | Warn | `godot_manifest` está configurado pero el fichero no existe (indica el comando). |
| `astra_manifest_syntax` | Error | El JSON del manifiesto no parsea. |
| `astra_manifest_mismatch` | Error | Un ID está en un lado y no en el otro, la escena de un sector no coincide, o el `connects` de una puerta difiere. |

### Manifiesto y exportador

- En la escena de Godot, los nodos llevan metadatos: `astra_id` y, en las
  puertas, `astra_connects` (array de dos IDs). No hace falta un plugin ni
  una clase propia.
- `kiln astra-manifest --project DIR [--config FILE]`: lee las escenas de
  los `Sector` del RON y ejecuta
  `godot --headless --path <godot_project> --script astra_manifest.gd -- in.json out.json`.
  Escribe en `godot_manifest`:

  ```json
  { "version": 1, "scenes": [ { "scene": "res://astra/warehouse.tscn",
      "nodes": [ { "id": "sector:warehouse" },
                 { "id": "door:warehouse_yard", "connects": ["sector:warehouse", "sector:yard"] } ] } ] }
  ```

- El script vive en `kiln-cli/godot/astra_manifest.gd`, embebido con
  `include_str!` igual que `measure.gd`.

### Fixture `examples/astra/`

`kiln.toml` con la sección activa, `data/astra/world.ron`, un proyecto
Godot mínimo (`godot/project.godot`, `godot/astra/warehouse.tscn`,
`yard.tscn`), un kit con una pieza y su `LICENSE.md`, y el manifiesto ya
exportado y versionado.

## Política aplicada

- **Datos personales**: ninguno. Contenido de juego ficticio.
- **Datos de negocio sensibles**: ninguno (no hay costes, márgenes, tarifas
  ni stock).
- **Credenciales**: ninguna.
- **Decisiones sobre personas / alto riesgo**: no.

## Fuera de alcance

- Lo que el encargo excluye en su §6: NPC, economía, contratos, memoria,
  SQLite, editor de topología, framework de plugins, servidor Rust,
  generador de mundo de T-029 y Tarea 003.
- **Validación incremental o selectiva** (§3.8 del encargo, "cuando se
  pueda"): no entra en esta iteración. La fixture se valida en
  milisegundos y no hay un caso que la pida. Si se quiere un
  `kiln check --only astra`, se abre como intent aparte.
- Mostrar los hallazgos ASTRA en la página de `kiln serve` con una vista
  propia. Ya salen en la lista general de hallazgos, porque llegan por el
  mismo `Catalog`.
- Detectar un manifiesto desfasado por fechas de modificación (el mtime no
  es fiable tras un checkout de git).

## Riesgos

| Riesgo | Impacto | Mitigación |
|--------|---------|------------|
| El esquema RON de producción cambiará | Hay que rehacer la fixture y el enum | Se marca como provisional. El enum está aislado en `astra.rs`. |
| Godot no está en CI | El exportador no se prueba automáticamente | Test `#[ignore]` que lo ejecuta en local; el manifiesto queda versionado. La validación no necesita Godot. |
| Una sección `[integrations]` mal escrita en `kiln.toml` | Un proyecto no ASTRA deja de abrir | Sin `deny_unknown_fields`. Test de R1 con el ejemplo actual. |
| Mezclar K001 con la entrega 10 sin commitear | Diff y PR difíciles de revisar | Rama propia desde `main` (la crea Aarón). |

---
Siguiente etapa: Build lee este fichero y escribe `plan.md`.
Requiere el visto bueno de Aarón.
