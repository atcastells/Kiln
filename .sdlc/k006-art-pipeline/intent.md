# Intent: Kiln como cadena de producción artística para ASTRA (K006)

- **Originator**: Aarón (encargo ASTRA V2, tarea de herramientas «K002»,
  previa a T010)
- **Date**: 2026-11-17
- **Status**: approved (Aaron Castells, 2026-11-17)
- **Slug**: k006-art-pipeline
- **Fuente**: «ASTRA V2 — K002: preparación de Kiln para producción
  artística» (pegado en la sesión). En Kiln el identificador K002 ya está
  usado (`.sdlc/k002-content-schema/`), por eso aquí se numera K006; el
  nombre del encargo en ASTRA no cambia.

## El problema, en palabras del originador

> Preparar Kiln como punto central de producción, integración y validación
> de recursos artísticos de ASTRA.
>
> El flujo deberá permitir producir un asset editable, revisarlo, exportarlo,
> validarlo e incorporarlo a Godot de forma reproducible, utilizando Blender
> y otras herramientas externas cuando aporten capacidades necesarias.
>
> Kiln coordina el contenido. Blender realiza las operaciones de modelado que
> correspondan. Godot presenta los recursos. Rust sigue siendo autoritativo
> para entidades, interacciones, navegación, permisos y estado persistente.
>
> **K002 prepara las herramientas; T010 producirá el entorno artístico.**

Criterio de cierre, también en sus palabras:

> K002 se considerará completada cuando exista un recorrido reproducible y
> documentado desde una fuente editable hasta una puerta visual utilizable en
> ASTRA, coordinado por Kiln y validado en Godot.
>
> **Resultado final:** Kiln preparado para producir el primer kit artístico
> Neo-PSX de ASTRA sin depender de una cadena manual diferente para cada
> recurso.

El encargo trae además fases (auditoría, Blender, herramientas
complementarias, contrato de assets, asset piloto), una tabla de pruebas
K2-01…K2-09 y exclusiones. Son propuesta de solución: el diseño las lee en
`spec.md`, no se fijan aquí.

## Qué hay hoy (comprobado, sin auditar a fondo)

- Blender sin ventana ya monta, pinta, divide y renderiza:
  `crates/kiln-cli/blender/{assemble,paint,repaint,split,render}.py`
  (README, «Entrega 4»). Blender 5.2.3 y Godot 4.7.1 están en el `PATH` de
  esta máquina.
- Godot ya mide lo que solo Godot abre: `crates/kiln-cli/godot/measure.gd`
  (herramienta MCP `godot_measure`).
- Kiln ya se sirve por MCP (K004, `crates/kiln-cli/src/mcp/`) con `build`,
  `paint`, `review`, `job`, `jobs`, entre otras.
- K001/K002 de Kiln validan contenido de juego desde un esquema por proyecto.
- No hay Khronos glTF Validator ni glTF-Transform instalados ni referenciados
  en el código.
- Solo un test lanza Blender (`crates/kiln-cli/tests/blender_turn.rs`, K005).

La matriz de capacidades que pide la Fase A es trabajo de este encargo; lo
anterior solo sirve para que el diseño no parta de cero.

## Who it affects

Quien produzca el kit Neo-PSX de T010 (el originador y los agentes que
trabajen por MCP), y cada recurso visual de ASTRA a partir de ahí.

## What happens if we do nothing

Cada recurso de T010 seguiría una cadena manual distinta: sin informe de
validación, sin hash ni procedencia, y sin garantía de que una reexportación
respete pivotes, nombres y puntos de montaje que ASTRA necesita. Los fallos se
verían en Godot o en el juego, donde cuesta más localizarlos.

## How we will know it is solved

- Una puerta industrial modular recorre fuente editable → Blender → GLB →
  validadores → importación en Godot → instancia sobre una puerta existente de
  ASTRA, sin pasos a mano.
- Una modificación artística de la puerta se reexporta y vuelve a entrar sin
  rehacer la integración, y no cambia el estado autoritativo de Rust.
- Las pruebas K2-01…K2-09 del encargo tienen resultado registrado, con
  capturas reales de Godot para K2-09.
- Un proyecto de Kiln sin configuración ASTRA se comporta como hoy (K2-08).

## Open questions

Respondida al aprobar: ASTRA está en `/Users/acastells/.t3/projects/astra`,
con los detalles de estilo en el propio repo (pregunta 1). Las demás pasan a
`spec.md` como decisiones D1–D9.

1. **Repositorio de ASTRA**: no sé dónde está en esta máquina ni qué rama
   usar. La prueba K2-06 (instanciar sobre una puerta existente) lo necesita.
2. **Puerta existente**: qué entidad de ASTRA representa la puerta piloto y
   qué estados visuales expone la simulación (abierta, cerrada, bloqueada…).
3. **Spec de la puerta**: dimensiones y estilo Neo-PSX los aporta el
   originador o los propone el diseño para su aprobación.
4. **Validadores externos**: si se acepta instalar Khronos glTF Validator
   (Apache-2.0) y glTF-Transform (MIT, requiere Node) como adaptadores
   opcionales, o solo se detectan si ya están.
5. **Tamaño**: el encargo es grande. Si se parte en varias entregas (p. ej.
   auditoría + contrato primero, piloto después) o va en una.
6. **Tests con Blender/Godot**: si entran en la batería normal o siguen
   aparte, como el de K005.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
