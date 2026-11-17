# K006, entrega B: recorrido en ASTRA (2026-11-17)

Proyecto: worktree `astra-k002b-door-pilot` de ASTRA, con `[publish] dir = "godot/assets/kiln"`.
Blender 5.2.3, Godot 4.7.1, glTF Validator 2.0.0-dev.3.10. Godot siempre sin ventana.

## B4 por MCP: build, verify y publish de los dos assets

Cliente: guion JSON-RPC por stdio contra `kiln mcp`.

```
## tools
  blender: 5.2.3 ok
  godot: 4.7.1 ok
  gltf_validator: 2.0.0 ok
  gltf-transform: - no
  claude: 2.1.326 ok
  curl: 8.7.1 ok
## build {"asset": "door_industrial_frame"}
{"job":1,"next":"llama a `job` con este id"}
  | Plan de montaje: 1 copias de 1 piezas
  | Montando en Blender (blender)
  | built <astra>/kiln/build/door_industrial_frame/door_industrial_frame.glb 84 tris
  | Aplicando materiales en Blender (blender)
  | painted <astra>/kiln/build/door_industrial_frame/door_industrial_frame--restricted.glb 4 materials
  => {'id': 1, 'asked': 'montar door_industrial_frame', 'status': 'done', 'next': 5, 'file': 'kiln/build/door_industrial_frame/door_industrial_frame.glb', 'reply': None}
## verify {"asset": "door_industrial_frame"}
{"job":2,"next":"llama a `job` con este id"}
  | Kiln comprueba el GLB
  | glTF Validator comprueba el GLB (gltf_validator)
  | Godot importa e instancia el GLB (godot)
  | kiln: pasa
  | gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
  | godot_import: pasa (proyecto de prueba en kiln/cache/verify/door_industrial_frame)
  => {'id': 2, 'asked': 'verificar door_industrial_frame', 'status': 'done', 'next': 6, 'file': 'kiln/build/door_industrial_frame/validation.json', 'reply': None}
## publish {"asset": "door_industrial_frame"}
{"job":3,"next":"llama a `job` con este id"}
  | «door_industrial_frame» copiado a godot/assets/kiln/door_industrial_frame
  | Godot importa lo publicado (godot)
  | «door_industrial_frame» publicado en godot/assets/kiln/door_industrial_frame (versión 1, sin cambios)
  => {'id': 3, 'asked': 'publicar door_industrial_frame', 'status': 'done', 'next': 3, 'file': 'godot/assets/kiln/door_industrial_frame/asset.json', 'reply': None}
## spec_get door_industrial_frame
  outputs: [(None, 'door_industrial_frame.glb', '1f8450990641'), ('restricted', 'door_industrial_frame--restricted.glb', '1f8450990641')]
  validation: True [('kiln', 'passed', 0, 0), ('gltf_validator', 'passed', 0, 0), ('godot_import', 'passed', 0, 0)]
  published: ['asset.json', 'door_industrial_frame--restricted.glb', 'door_industrial_frame--restricted.glb.import', 'door_industrial_frame.glb', 'door_industrial_frame.glb.import'] version 1
## build {"asset": "door_industrial_leaf"}
{"job":4,"next":"llama a `job` con este id"}
  | Plan de montaje: 1 copias de 1 piezas
  | Montando en Blender (blender)
  | built <astra>/kiln/build/door_industrial_leaf/door_industrial_leaf.glb 84 tris
  => {'id': 4, 'asked': 'montar door_industrial_leaf', 'status': 'done', 'next': 3, 'file': 'kiln/build/door_industrial_leaf/door_industrial_leaf.glb', 'reply': None}
## verify {"asset": "door_industrial_leaf"}
{"job":5,"next":"llama a `job` con este id"}
  | Kiln comprueba el GLB
  | glTF Validator comprueba el GLB (gltf_validator)
  | Godot importa e instancia el GLB (godot)
  | kiln: pasa
  | gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
  | godot_import: pasa (proyecto de prueba en kiln/cache/verify/door_industrial_leaf)
  => {'id': 5, 'asked': 'verificar door_industrial_leaf', 'status': 'done', 'next': 6, 'file': 'kiln/build/door_industrial_leaf/validation.json', 'reply': None}
## publish {"asset": "door_industrial_leaf"}
{"job":6,"next":"llama a `job` con este id"}
  | «door_industrial_leaf» copiado a godot/assets/kiln/door_industrial_leaf
  | Godot importa lo publicado (godot)
  | «door_industrial_leaf» publicado en godot/assets/kiln/door_industrial_leaf (versión 1, sin cambios)
  => {'id': 6, 'asked': 'publicar door_industrial_leaf', 'status': 'done', 'next': 3, 'file': 'godot/assets/kiln/door_industrial_leaf/asset.json', 'reply': None}
## spec_get door_industrial_leaf
  outputs: [(None, 'door_industrial_leaf.glb', 'b97451adb2b1')]
  validation: True [('kiln', 'passed', 0, 0), ('gltf_validator', 'passed', 0, 0), ('godot_import', 'passed', 0, 0)]
  published: ['asset.json', 'door_industrial_leaf.glb', 'door_industrial_leaf.glb.import'] version 1
```

## B4 por CLI

De esta pasada solo se guardó la salida de `publish`; `build` y `verify` por CLI quedan
transcritos en B6.

```
$ kiln publish --asset door_industrial_frame
«door_industrial_frame» copiado a godot/assets/kiln/door_industrial_frame
Godot importa lo publicado (godot)
«door_industrial_frame» publicado en godot/assets/kiln/door_industrial_frame (versión 1)
<astra>/godot/assets/kiln/door_industrial_frame
$ kiln publish --asset door_industrial_leaf
«door_industrial_leaf» copiado a godot/assets/kiln/door_industrial_leaf
Godot importa lo publicado (godot)
«door_industrial_leaf» publicado en godot/assets/kiln/door_industrial_leaf (versión 1)
<astra>/godot/assets/kiln/door_industrial_leaf
```

## B6 por CLI: cambia la banda de la hoja y se publica la versión 2

Cambio en `assets/kits/industrial/src/door.py`: la banda de la hoja pasa de z 1,02–1,16 a 0,98–1,20.
El marco no se toca y sigue en versión 1. El sha256 de `godot/station/door.tscn` es el mismo antes y después.

```
$ kiln build --project . --asset door_industrial_leaf
Blender exporta industrial/door_leaf (blender)
Plan de montaje: 1 copias de 1 piezas
Montando en Blender (blender)
built <astra>/kiln/build/door_industrial_leaf/door_industrial_leaf.glb 84 tris
<astra>/kiln/build/door_industrial_leaf/door_industrial_leaf.glb
exit 0
$ kiln verify --project . --asset door_industrial_leaf
Kiln comprueba el GLB
glTF Validator comprueba el GLB (gltf_validator)
Godot importa e instancia el GLB (godot)
kiln: pasa
gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
godot_import: pasa (proyecto de prueba en kiln/cache/verify/door_industrial_leaf)
<astra>/kiln/build/door_industrial_leaf/validation.json
exit 0
$ kiln publish --project . --asset door_industrial_leaf
«door_industrial_leaf» copiado a godot/assets/kiln/door_industrial_leaf
Godot importa lo publicado (godot)
«door_industrial_leaf» publicado en godot/assets/kiln/door_industrial_leaf (versión 2)
<astra>/godot/assets/kiln/door_industrial_leaf
exit 0
$ kiln check --project .
2 piezas, 0 errores, 0 avisos
exit 0
```
