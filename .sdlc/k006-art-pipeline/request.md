# Encargo original (literal)

Texto pegado por el originador el 2026-11-17. `intent.md` y `spec.md` citan
sus fases (A–E) y sus pruebas (K2-01…K2-09) desde aquí.

---

# ASTRA V2 — K002: preparación de Kiln para producción artística

**Estado:** propuesta de tarea previa a T010
**Repositorio principal:** Kiln
**Repositorio consumidor:** ASTRA V2
**Dependencias:** K001 y flujo de Godot existente
**Objetivo posterior:** T010 — Primer entorno visual y jugable Neo-PSX

## 1. Objetivo

Preparar Kiln como punto central de producción, integración y validación de recursos artísticos de ASTRA.

El flujo deberá permitir producir un asset editable, revisarlo, exportarlo, validarlo e incorporarlo a Godot de forma reproducible, utilizando Blender y otras herramientas externas cuando aporten capacidades necesarias.

Kiln coordina el contenido. Blender realiza las operaciones de modelado que correspondan. Godot presenta los recursos. Rust sigue siendo autoritativo para entidades, interacciones, navegación, permisos y estado persistente.

**K002 prepara las herramientas; T010 producirá el entorno artístico.**

## 2. Fase A — Auditoría de Kiln y del MCP

Antes de desarrollar nuevas capacidades:

1. Identificar la rama, SHA, versión y pruebas actuales del repositorio Kiln.
2. Inventariar las herramientas expuestas por el MCP y sus equivalentes CLI.
3. Comprobar las integraciones existentes con Blender, Godot y exportadores glTF.
4. Identificar funciones duplicadas, incompletas o no verificadas.
5. Revisar las capacidades reales de K001 para ASTRA.
6. Documentar las dependencias y versiones necesarias.

**Entregable:** matriz de capacidades con tres estados: disponible y probado, disponible pero pendiente de verificación, o ausente.

No se reimplementarán funciones que Kiln ya proporcione correctamente.

## 3. Fase B — Integración con Blender

Kiln deberá permitir ejecutar un flujo automatizado de producción utilizando Blender.

Capacidades mínimas:

- Detectar Blender y comprobar la compatibilidad de su versión.
- Lanzar operaciones reproducibles mediante una integración existente o mediante Blender en modo background.
- Crear o modificar un recurso de prueba con parámetros explícitos.
- Conservar el archivo `.blend` y los scripts o configuraciones utilizados.
- Exportar GLB con geometría, transformaciones, jerarquía y materiales compatibles.
- Devolver errores y registros de ejecución estructurados a Kiln.
- Repetir una exportación sin intervención manual.

Se priorizará reutilizar un puente Blender/MCP existente si supera las pruebas. No se construirá otro servidor MCP para Blender por defecto.

Las operaciones automatizadas tendrán directorios de trabajo controlados, tiempos máximos y permisos limitados. No se expondrá ejecución arbitraria de scripts a clientes no autorizados.

## 4. Fase C — Herramientas complementarias

Evaluar e integrar, preferentemente mediante adaptadores opcionales:

| Herramienta | Función | Prioridad |
|---|---|---|
| Blender | Modelado, geometría procedural, materiales y fuentes editables | Obligatoria |
| Khronos glTF Validator | Detectar GLB inválidos y problemas de compatibilidad | Alta |
| glTF-Transform | Inspección, estadísticas y optimización controlada | Media |
| Godot headless | Verificar la importación real en el proyecto | Alta |
| Herramientas adicionales | UV, texturas, LOD, rigging u otras, si existe una necesidad demostrada | Condicional |

Una herramienta nueva solo se incorporará cuando:

1. Resuelva una carencia concreta.
2. Tenga licencia y condiciones de distribución compatibles.
3. Permita ejecución reproducible.
4. Pueda integrarse sin dependencias obligatorias innecesarias.
5. Supere una prueba de funcionamiento sobre un asset real.

Las optimizaciones se aplicarán únicamente si no destruyen pivotes, nombres, materiales, animaciones o conectores que ASTRA necesita.

No se exige instalar todas las herramientas candidatas.

## 5. Fase D — Contrato de assets para ASTRA

Establecer un contrato mínimo de producción que permita identificar cada recurso sin confundirlo con una entidad persistente del mundo.

Cada recurso deberá conservar, según corresponda:

- Identificador estable de asset.
- Tipo y función visual.
- Fuente editable y procedencia.
- Versión de producción.
- Archivo exportado y hash.
- Dimensiones, escala y orientación.
- Origen, pivotes y puntos de conexión necesarios.
- Materiales y variantes.
- Licencia y atribución.
- Informes de validación.

Los puntos de montaje deberán poder comprobarse mediante reglas de compatibilidad.

Kiln no modificará automáticamente la topología autoritativa de Rust ni utilizará IDs persistentes de entidades como sustitutos de IDs de recursos visuales.

El contrato debe ser pequeño, extensible y compatible con las convenciones ya presentes en Kiln y ASTRA.

## 6. Fase E — Asset piloto

Utilizar una única **puerta industrial modular** como prueba del pipeline completo.

La puerta deberá:

1. Tener especificación dimensional y visual.
2. Producirse o modificarse a través del flujo coordinado por Kiln.
3. Conservar una fuente editable.
4. Exportarse como GLB.
5. Pasar las comprobaciones estructurales.
6. Importarse correctamente en Godot.
7. Instanciarse como representación de una puerta existente en ASTRA.
8. Respetar su posición, orientación y estados visuales compatibles con la simulación.
9. Admitir una modificación artística y una nueva exportación sin reconstruir manualmente toda la integración.

No es necesario incorporar todavía una animación compleja.

La prueba debe demostrar que el recurso es reutilizable y que las modificaciones visuales no alteran las reglas del mundo.

## 7. Interfaces de Kiln

Las capacidades nuevas deberán estar disponibles mediante el flujo de trabajo de Kiln y, cuando corresponda, mediante su MCP y CLI.

El comportamiento observable será:

- Preparar o localizar el recurso.
- Ejecutar la operación necesaria.
- Validar el resultado.
- Consultar el informe.
- Publicar una versión utilizable por ASTRA.

Los nombres concretos de comandos y herramientas MCP se determinarán después de auditar las interfaces existentes.

No se crearán interfaces paralelas con funciones duplicadas.

## 8. Validación

| ID | Prueba | Criterio |
|---|---|---|
| K2-01 | Descubrimiento | Kiln reconoce herramientas disponibles y ausentes |
| K2-02 | Blender | Genera o modifica un asset mediante una ejecución automatizada |
| K2-03 | Reproducibilidad | La misma fuente y parámetros producen un resultado equivalente verificable |
| K2-04 | Integridad | Se detectan exportaciones inválidas, referencias rotas y configuraciones incompatibles |
| K2-05 | Godot | El GLB se importa e instancia en el proyecto sin errores relevantes |
| K2-06 | ASTRA | La puerta visual representa la entidad existente sin cambiar permisos o estado autoritativo |
| K2-07 | MCP/CLI | Las operaciones incorporadas pueden invocarse por sus interfaces documentadas |
| K2-08 | Regresiones | K001 y los proyectos Kiln sin configuración ASTRA conservan su comportamiento |
| K2-09 | Revisión artística | Se obtienen capturas del resultado en Godot y se revisa su adecuación visual |

Las comprobaciones técnicas serán automatizadas siempre que resulte posible.

Las capturas visuales no sustituyen la validación de geometría, y las pruebas headless no sustituyen la revisión artística.

## 9. Exclusiones

Quedan fuera de K002:

- Producir el kit industrial completo de T010.
- Modelar todos los NPC.
- Crear un sistema general de generación de personajes.
- Introducir animaciones complejas o combate.
- Desarrollar un editor visual nuevo.
- Construir un sistema universal de generación 3D por IA.
- Sustituir Blender o Godot.
- Cambiar la arquitectura de Rust.
- Añadir dependencias obligatorias a todos los usuarios de Kiln.
- Descargar automáticamente modelos externos sin comprobar sus licencias.

## 10. Criterio de cierre

K002 se considerará completada cuando exista un recorrido reproducible y documentado desde una fuente editable hasta una puerta visual utilizable en ASTRA, coordinado por Kiln y validado en Godot.

El handoff deberá incluir:

- SHA y rama de Kiln.
- Inventario de herramientas MCP verificadas.
- Integraciones reutilizadas y añadidas.
- Versiones y licencias de las dependencias.
- Archivo fuente y GLB del piloto.
- Resultado de los validadores.
- Pruebas automatizadas y regresiones.
- Capturas reales de Godot.
- Limitaciones y capacidades aplazadas.

**Resultado final:** Kiln preparado para producir el primer kit artístico Neo-PSX de ASTRA sin depender de una cadena manual diferente para cada recurso.
