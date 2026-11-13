# Intent: Materiales por zona (entrega 8)

- **Originator**: Aarón
- **Date**: 2026-11-13
- **Status**: draft
- **Slug**: e8-zone-materials

## El problema, en palabras del originador

Tomado del traspaso de la entrega 7 (el documento original
`e8-materiales-por-zona.md` no está en esta máquina):

> Un material de la paleta por zona en la ficha
> (`materials = { zona = "clave" }`), con un montaje que conserva las zonas por
> nombre (`parte/zona`). Cambiar un material es gratis: en la vista 3D, al
> instante; en Godot, mediante los vínculos del `.import`; en el GLB, con
> «Aplicar materiales», que solo reasigna.
>
> Partir en zonas una pieza de una sola ranura: por trozos sueltos, por textura
> de origen, por reglas o a mano.

Detalle conocido que esto resuelve:

> Si una pieza traída de fuera lleva `material` en la ficha, tapa las texturas
> del modelo.

## A quién afecta

A quien monta assets con Kiln: hoy una pieza lleva un solo material de la
paleta en la ficha, que se aplica a todas sus ranuras.

## Qué pasa si no se hace

Cambiar un material obliga a montar otra vez, y una pieza con varias zonas
(madera y herrajes) no se puede vestir desde la ficha.

## Cómo sabremos que está resuelto

- Una ficha con dos zonas en una pieza se monta con dos materiales distintos.
- Cambiar uno en la vista 3D se ve al instante y queda guardado.
- Godot muestra el material nuevo sin volver a montar.
- «Aplicar materiales» actualiza el GLB sin recolocar nada.

## Preguntas abiertas

1. ¿La elección se guarda en la ficha, con el catálogo como valor por defecto?
2. ¿Kiln escribe el `.import` de Godot, o se usa un script de importación con
   `materials.json`?
3. ¿Las variantes de color van después?
4. ¿Dónde vive en Godot el material de cada clave de la paleta? Hoy `kiln.toml`
   no lo dice.

Respondidas el 13-11-2026; las respuestas están en `spec.md`, «Decisiones
tomadas».

---
Siguiente etapa: Design lee este fichero y escribe `spec.md`.
Requiere el visto bueno de Aarón.
