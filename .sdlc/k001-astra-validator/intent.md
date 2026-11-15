# Intent: Validar contenido de ASTRA con Kiln (K001)

- **Originator**: Aarón (encargo ASTRA V2, Tarea de herramientas K001)
- **Date**: 2026-11-15
- **Status**: approved (Aarón, 2026-11-15)
- **Slug**: k001-astra-validator
- **Fuente**: `ASTRA_Tarea_K001_Kiln_Integracion.md` (adjunto de la sesión;
  decisiones T-030, T-031, T-063 y T-067 citadas allí, no revisadas aquí)

## El problema, en palabras del originador

> Añadir un validador ASTRA opcional, aprovechando el CLI y los tipos de
> hallazgos de Kiln. La primera iteración debe poder revisar un pequeño
> conjunto de definiciones de contenido RON y referencias a un manifiesto
> espacial de Godot, señalando identificadores duplicados, referencias
> inexistentes y errores de estructura.
>
> No convertir Kiln en el motor de ASTRA. Kiln seguirá siendo una herramienta
> de preparación de contenido, independiente del runtime Rust `astra_sim` y de
> los guardados SQLite.

Restricciones que el encargo ya fija (no son diseño nuestro):

- Opt-in en `kiln.toml`: un proyecto sin la sección se comporta exactamente
  como antes.
- RON para definiciones, TOML para configuración, recursos nativos de Godot
  para escenas. El JSON solo como manifiesto técnico generado por Godot.
- Las referencias a assets reutilizan el catálogo de Kiln; nada de un
  inventario paralelo.
- El manifiesto sale de Godot por un paso reproducible (`@tool` o script
  headless, como las mediciones que Kiln ya hace).
- Los IDs estáticos (`sector:warehouse`, `asset:door_a`) no son
  `PersistentId(u64)` de instancias vivas.

## A quién afecta

A quien prepara contenido de ASTRA (sectores, puertas, objetos e
instalaciones) en RON y escenas en Godot. Hoy no hay nada que avise de un ID
duplicado, una puerta que apunta a un sector que no existe o una escena que
se ha movido; se descubre al cargar el contenido en el runtime.

## Qué pasa si no se hace

Las incoherencias entre RON, escenas de Godot y catálogo de assets llegan al
runtime de ASTRA, donde son más caras de localizar. Y T-067 eligió Kiln como
herramienta base del flujo de contenido: sin este validador esa decisión no
tiene todavía nada que la respalde.

## Cómo sabremos que está resuelto

Lo dicen los criterios de aceptación del encargo (§5), resumidos:

- Un proyecto de Kiln sin la sección ASTRA da el mismo resultado que hoy en
  `kiln check` y en el resto de comandos (`build`, `paint`, `review`, `serve`).
- `kiln check --project <fixture-astra>` pasa con RON y manifiesto válidos.
- IDs duplicados, referencias rotas y RON mal formado salen con `code`
  estable, elemento y fichero, y la salida es distinta de cero.
- Cambiar una conexión puerta/sector en la escena de Godot, volver a exportar
  y validar muestra la discrepancia.
- Tests unitarios rápidos: un caso válido y al menos cuatro inválidos.
- El README explica cómo configurar, exportar el manifiesto y validar.

## Preguntas abiertas

Respondidas por Aarón el 2026-11-15:

1. **Nombre de la sección**: `[integrations.astra]`.
2. **"Modo greybox"**: un ajuste de esa sección.
3. **Esquema RON**: lo propone esta tarea, como fixture provisional y no
   canónica.

Siguen abiertas, con una propuesta por defecto en la spec:

4. **Dónde vive la fixture**: propuesta `examples/astra/`.
5. **Rama**: la rama actual (`e9-new-game`) tiene la entrega 10 sin
   commitear. Propuesta: K001 en una rama propia desde `main`, que crea Aarón.
6. **Hechos del encargo por contrastar**: `Finding`/`Severity` están en
   `kiln-core/src/catalog.rs`, no en `check.rs`. La línea base (tests y CLI
   con commit registrado) se mide al empezar Build.

---
Siguiente etapa: Design lee este fichero y escribe `spec.md`.
Requiere el visto bueno de Aarón.
