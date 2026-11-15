# Intent: interfaz de Kiln fácil de usar, ágil y escalable (K003)

- **Originator**: Aaron Castells
- **Date**: 2026-11-15
- **Status**: approved by product owner (2026-11-15)
- **Slug**: k003-ui

## The problem, in the originator's words

> prepara una mejora de UI de Kiln para que se pueda usar facilmente, sea
> agil y escalable.

## Qué pasa hoy (auditoría del 2026-11-15, sobre la rama de K002)

- **Un solo fichero.** `crates/kiln-cli/ui/index.html`: 897 líneas con CSS
  y JS dentro, sin módulos ni pruebas. Todo el estado son variables
  globales.
- **Se rehace todo cada vez.** Cada acción y cada tecla del buscador
  vuelve a pintar la vista entera; cada edición vuelve a pedir el
  catálogo completo, y el servidor relee el proyecto del disco en cada
  petición (`serve.rs:469-482`).
- **No escala.** Los hallazgos se recorren enteros por cada fila
  (`index.html:147, 200, 208, 446`); no hay paginación, orden ni
  virtualización; los materiales se cortan a 80 sin avisar (`223`).
- **Se pierde el sitio.** No hay URL por vista ni enlaces directos:
  recargar vuelve al inicio y deja huérfano un trabajo en curso
  (`150-165`, `788`).
- **Calla cuando está ocupada.** Con un trabajo en marcha, los botones no
  hacen nada y no lo dicen (`467`, `531`); los errores se apilan en el
  panel lateral; el registro se desplaza solo y pelea con el usuario
  (`786`).
- **Primera pantalla sin salida.** En un proyecto como Astra, sin kits,
  se abre un árbol con un nodo y el texto «nadie declara sus hijos
  todavía».
- **Controles poco accesibles.** `prompt()` nativo, enlaces `href="#"`
  como botones, sin `aria-live`, un único punto de corte a 760 px.
- **Un riesgo.** Una URL externa se pone en un `href` sin comprobar su
  esquema (`515`).

No hay medidas de tiempo con proyectos grandes: el único proyecto real a
mano tiene cero piezas.

## Who it affects

Quien use Kiln a diario para preparar y validar los assets y el contenido
de un juego.

## What happens if we do nothing

Cada entrega añade vistas al mismo fichero (K002 y el asistente de e10 lo
han hecho esta semana). Con un proyecto de miles de piezas la interfaz se
vuelve lenta, y cada cambio choca con los demás.

## How we will know it is solved

- **Fácil**: la primera pantalla dice qué hay, qué falla y qué se puede
  hacer; toda acción da respuesta visible; todo se alcanza con teclado.
- **Ágil**: con 5.000 piezas y 2.000 hallazgos, la primera vista aparece
  en menos de 300 ms y filtrar responde en menos de 50 ms; una edición no
  vuelve a descargar el catálogo entero.
- **Escalable**: el código está en módulos con pruebas; añadir una vista
  es un fichero nuevo; recargar conserva la vista y el trabajo en curso.

## Open questions

- Sobre qué base se construye: K002 no está mergeado y e10 es un commit
  en curso sobre el mismo fichero.
- Si se acepta servir varios ficheros de interfaz en lugar de uno, siempre
  sin paso de compilación ni dependencias de Node.
- Qué tamaño de proyecto es el objetivo real.

---
Next stage: Design reads this file and writes `spec.md`.
Requires product owner approval.
