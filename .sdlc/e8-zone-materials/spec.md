# Spec: Materiales por zona (entrega 8)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: Aarón, 2026-11-13

Reescrito a partir del traspaso de la entrega 7 y del código de `main`
(`7fab775`). El documento original no estaba disponible.

## Cómo está hoy

- La ficha tiene un solo `material` por pieza. `assemble.py` lo pone en
  **todas** las ranuras, así que tapa las texturas de un modelo traído de fuera.
- Sin `material`, cada ranura toma el vínculo del catálogo
  (`kiln/bindings.toml`, por pieza de kit) o se queda con el suyo.
- En el GLB montado los materiales se llaman como la clave de la paleta: no se
  sabe a qué pieza ni a qué zona pertenece cada uno.
- Kiln no escribe nada para Godot sobre materiales.

## Requisitos

| # | Requisito | Viene de (intent) | Criterio de aceptación |
|---|-----------|-------------------|------------------------|
| R1 | Una pieza de la ficha lleva un material por zona: `materials = { zona = "clave" }` | «Un material de la paleta por zona en la ficha» | Una ficha con dos zonas se lee, se comprueba y se monta con dos materiales |
| R2 | El montaje nombra cada material `parte/zona` | «conserva las zonas por nombre» | El GLB montado lista `post/wood`, `post/iron`…; las copias de una pieza comparten nombres |
| R3 | Cambiar un material en la vista 3D se ve al instante y se guarda | «en la vista 3D, al instante» | Sin llamar a Blender; la ficha queda cambiada en disco |
| R4 | Godot toma el material nuevo sin volver a montar | «en Godot, mediante los vínculos del `.import`» | El `.import` del GLB apunta cada `parte/zona` a su material; Godot lo reimporta bien |
| R5 | «Aplicar materiales» actualiza el GLB reasignando, sin recolocar | «en el GLB, con "Aplicar materiales"» | Mismas posiciones y triángulos antes y después; solo cambian los materiales |
| R6 | Una zona sin elegir conserva el material del modelo | «tapa las texturas del modelo» | Una pieza traída de fuera con una sola zona elegida conserva las demás texturas |
| R7 | Una pieza de una sola ranura se puede partir en zonas | «Partir en zonas una pieza de una sola ranura» | Una pieza de una ranura con dos trozos sueltos sale con dos zonas elegibles |

| R8 | Un asset tiene variantes de color: los mismos modelos con otros materiales | Pregunta abierta 3 del intent, decidida «en esta entrega» | Una ficha con una variante produce un segundo GLB sin volver a montar, y se elige en la vista 3D |
| R9 | Validar una elección la deja como valor por defecto de la pieza en el catálogo | Pregunta abierta 1 del intent | Tras validar, otra ficha que reutiliza la pieza sale con ese material sin pedirlo |

## Diseño

**Zonas.** Una zona es una ranura de material de la pieza, con el nombre que
trae el modelo (`glb.rs` ya los lee). Una caja de relleno tiene una zona,
`body`.

**La ficha** (`spec.rs`). `Part` gana `materials: BTreeMap<String, String>`.
`material` se queda como atajo de «todas las zonas» para no romper las fichas
que hay. Orden al resolver una zona:

1. `materials[zona]` de la ficha
2. `material` de la ficha
3. el vínculo del catálogo para esa ranura
4. el material del propio modelo

`check` avisa de una zona que la pieza no tiene y de una clave que no está en
la paleta. `set_material(parte, zona, clave)` escribe la ficha, igual que
`set_source`.

**El plan** (`build.rs`). `Item` cambia `material` + `slots` por un solo mapa
`zones: zona → Paint`, ya resuelto con el orden de arriba. El plan escribe
además `materials.json` junto al GLB: `{ "parte/zona": "clave" }`.

**El montaje** (`assemble.py`). Por cada pieza y zona crea (o reutiliza) un
material de Blender llamado `parte/zona`, con el color o la textura de la
paleta, o copia del material del modelo si la zona no tiene elección.

**Vista 3D** (`ui/index.html`). En el panel del asset, un desplegable por
`parte/zona`. Al cambiar, three.js cambia el material con ese nombre en el
momento y un POST a `/api/material` guarda la ficha.

**Aplicar materiales** (`repaint.py`, nuevo, y un botón). Abre el `.blend` del
montaje, vuelve a vestir los materiales por nombre desde `materials.json` y
exporta el GLB. No lee el plan de colocación.

**Godot** (`godot.rs`). Kiln escribe en el `.import` del GLB montado la sección
`_subresources.materials`, con cada `parte/zona` apuntando al material de Godot
de su clave. Hace falta saber dónde está ese material: una plantilla nueva en
`kiln.toml`, `[materials] godot = "res://materials/{key}.tres"`. Sin ella, o si
el GLB montado queda fuera del proyecto de Godot, este paso se salta con un
aviso. Comprobado con Godot 4.7.1: un material del GLB llamado `post/wood`
con `"use_external/enabled": true` y `"use_external/path"` en
`_subresources.materials` se importa con el material externo, y Godot conserva
la entrada al reimportar (añade `use_external/fallback_path`).

**Variantes** (`spec.rs`, `repaint.py`). La ficha gana
`[[variants]]` con `id` y `materials = { "parte/zona" = "clave" }`, que solo
lista lo que cambia respecto al asset base. Cada variante sale como
`<asset>--<variante>.glb` con «Aplicar materiales» (sin montar otra vez), con
su `materials.json` y su `.import`. La vista 3D tiene un selector de variante.

**Validar** (`bindings.rs`). Un botón por pieza copia sus elecciones de zona
al catálogo como vínculos por defecto de la pieza de origen y las quita de la
ficha. Solo vale para piezas que vienen del catálogo (`reuse`, `variant`).

**Partir en zonas.** Se guarda por pieza del catálogo en `kiln/zones.toml`
(`["kit/pieza"] split = "loose"`), y el montaje parte la malla al cargarla.
En esta entrega, dos maneras:

- **por trozos sueltos**: cada isla de la malla es una zona (`part_1`…);
- **por textura de origen**: los triángulos se agrupan por el color dominante
  de la textura bajo sus UV.

Por reglas y a mano quedan para la siguiente tanda.

## Decisiones tomadas (Aarón, 13-11-2026)

1. **Dónde se guarda la elección**: en la ficha mientras se prueba; al
   **validar**, pasa al catálogo (`kiln/bindings.toml`) como valor por defecto
   de esa pieza.
2. **Godot**: Kiln escribe el `.import`.
3. **Variantes de color**: en esta entrega.
4. **Material de Godot por clave**: plantilla `[materials] godot` en `kiln.toml`.
5. **Partir en zonas**: trozos sueltos y textura ahora; reglas y a mano después.

## Política aplicada

- **Datos personales**: ninguno.
- **Datos comerciales sensibles**: ninguno.
- **Credenciales**: ninguna.
- **Decisiones sobre personas / alto riesgo**: no.

## Fuera de alcance

- Partir en zonas por reglas y a mano.
- Proponer vínculos de material con SigLIP 2.

## Riesgos

| Riesgo | Impacto | Mitigación |
|--------|---------|------------|
| El `.import` no existe hasta que Godot importa el GLB por primera vez | R4 no tiene dónde escribir | Kiln lanza `godot --import` si falta, y solo toca `_subresources.materials` |
| Otras versiones de Godot guardan los vínculos con otro formato | R4 falla fuera de 4.7 | Solo se ha probado 4.7.1; aviso si la sección no tiene la forma esperada |
| Agrupar por textura da zonas ruidosas en modelos con atlas | Zonas inútiles | Umbral de tamaño mínimo por zona; se puede volver a una sola zona |
| Fichas antiguas con `material` | Dejan de montar igual | `material` sigue valiendo como «todas las zonas»; test con la ficha del ejemplo |

---
Siguiente etapa: Build lee este fichero y escribe `plan.md`.
