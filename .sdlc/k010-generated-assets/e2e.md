# K010: recorrido real de un asset generado (2026-11-18)

Rama `k010-generated-assets`. Godot 4.7.1, Blender 5.x solo para las
capturas. Proyecto: copia de `examples` (Puerto Niebla) fuera del repo.

## La petición al agente

`POST /api/ask` con «crear un asteroide», por la página (`claude -p` con la
skill `kiln-asset`). El trabajo terminó `done` y dejó `specs/asteroid.toml`:

```toml
[asset]
id = "asteroid"
name = "Piedra caída del cielo"
asked = "crear un asteroide"
budget_tris = 600
status = "layout"

[generator]
seed = 42
material = "basalt"

[[generator.ops]]
op = "sphere"
radius_m = [0.5, 1.5]
segments = 16

[[generator.ops]]
op = "stretch"
x = [0.9, 1.5]
y = [0.6, 0.9]
z = [0.8, 1.3]

[[generator.ops]]
op = "noise"
amplitude = [0.15, 0.35]
size = 0.8

[[generator.ops]]
op = "noise"
amplitude = [0.05, 0.12]
size = 0.25

[[generator.ops]]
op = "facet"
```

- Eligió receta y no piezas, solo con operaciones del vocabulario.
- Tradujo la petición al mundo del proyecto: en un pueblo de pescadores sin
  espacio exterior, un meteorito caído en la costa. Lo dice en `notes` y
  dejó una pregunta nueva en el contexto.
- 16 segmentos dan 288 triángulos, dentro de su presupuesto de 600.

## Lo que falló y se arregló

El agente propuso el material `basalt`, que no está en la paleta del
proyecto, como le dice la skill para las piezas. La receta lo trataba como
error (`generator_material`) y `generate` se negaba: el trabajo acabó
`failed`. Con las piezas un material fuera de la paleta es un aviso. Se
cambió a aviso, con test rojo primero, y generar sigue adelante diciéndolo.

## Generar

```
$ kiln generate --asset asteroid --seed 42 --project <p>
Se resuelve la receta de «asteroid» con la semilla 42
GLB generado en kiln/cache/generated/asteroid/42.glb
$ kiln check --project <p>
aviso  asset:asteroid   generator_material   el material «basalt» no está en la paleta
4 piezas, 0 errores, 1 avisos
```

| Semilla | Medidas (m) | Triángulos |
|---|---|---|
| 42 | 2,10 × 1,51 × 2,03 | 288 (los calculados) |
| 43 | 3,08 × 1,92 × 2,32 | 288 (los calculados) |

Capturas de los dos GLB (Blender sin ventana, mismo encuadre relativo):
`shots/asteroid_42.png` y `shots/asteroid_43.png`.

## Lo que comprobó el verificador, con Godot de verdad

Sobre otra copia de `examples`, con la receta de la spec (64 segmentos):

- La semilla 42 dos veces da el mismo `glb_sha256`; la 43, otro, con otras
  medidas. 4 224 triángulos, los calculados. 0,39 s por ejecución de
  `kiln generate`.
- `gltf_validator`: 0 errores, 0 avisos. Normales unitarias; el material
  lleva el nombre de la clave.
- `build`, `verify`, `publish`, `paint` y `review` rechazan el asset
  generado sin escribir nada.
- La ruta: 403 sin la cabecera de la página, 400 con una semilla o una
  receta inválidas (la ficha no cambia), 200 y el GLB servido por `/files/`.
- Ids con `..` o barras se rechazan en la orden y en la ruta.
- Montar y verificar `hoist`, un asset de piezas, sigue pasando.

## Sin comprobar

- La misma semilla en otra máquina o con otra versión de Godot.
- La herramienta MCP `generate` con Godot de verdad (sus tests usan un Godot
  que no existe o comprueban rechazos).
- El botón Aplicar con Godot de verdad en un navegador: la prueba de la
  página usa un Godot de pega que copia un GLB ya hecho.
