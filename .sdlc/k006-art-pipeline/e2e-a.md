# K006, entrega A: recorrido por MCP (2026-11-17)

Copia temporal de `examples` con `[publish] dir = "godot/assets"` y un `project.godot` mínimo.
Cliente: guion JSON-RPC por stdio contra `kiln mcp`. Blender 5.2.3, Godot 4.7.1, glTF Validator 2.0.0-dev.3.10.
El mismo recorrido por CLI lo hace `crates/kiln-cli/tests/godot_verify.rs`.

```
## tools
  blender: 5.2.3 ok
  godot: 4.7.1 ok
  gltf_validator: 2.0.0 ok
  gltf-transform: - no
  claude: 2.1.326 ok
  curl: 8.7.1 ok
## publish {"asset": "hoist"}
{"job":1,"next":"llama a `job` con este id"}
  | Error: verifica antes el asset: kiln verify --asset hoist
  => {'id': 1, 'asked': 'publicar hoist', 'status': 'failed', 'next': 1, 'file': None, 'reply': None}
## build {"asset": "hoist"}
{"job":2,"next":"llama a `job` con este id"}
  | Plan de montaje: 9 copias de 7 piezas
  | windlass: caja en lugar de un modelo por buscar: wooden windlass crank
  | pulley: caja en lugar de un modelo por buscar: wooden pulley block
  | rope: retocar: la soga del kit está enrollada; se estira en un tramo recto
  | Montando en Blender (blender)
  | built <proyecto>/kiln/build/hoist/hoist.glb 108 tris
  | Aplicando materiales en Blender (blender)
  | painted <proyecto>/kiln/build/hoist/hoist--weathered.glb 8 materials
  => {'id': 2, 'asked': 'montar hoist', 'status': 'done', 'next': 8, 'file': 'kiln/build/hoist/hoist.glb', 'reply': None}
## verify {"asset": "hoist"}
{"job":3,"next":"llama a `job` con este id"}
  | Kiln comprueba el GLB
  | glTF Validator comprueba el GLB (gltf_validator)
  | Godot importa e instancia el GLB (godot)
  | kiln: pasa
  | gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
  | godot_import: pasa (proyecto de prueba en kiln/cache/verify/hoist)
  => {'id': 3, 'asked': 'verificar hoist', 'status': 'done', 'next': 6, 'file': 'kiln/build/hoist/validation.json', 'reply': None}
## publish {"asset": "hoist"}
{"job":4,"next":"llama a `job` con este id"}
  | «hoist» publicado en godot/assets/hoist (versión 1)
  | Godot importa lo publicado (godot)
  => {'id': 4, 'asked': 'publicar hoist', 'status': 'done', 'next': 2, 'file': 'godot/assets/hoist/asset.json', 'reply': None}
## spec_get hoist
  contract outputs: [(None, 'hoist.glb', '2828a7494f72'), ('weathered', 'hoist--weathered.glb', '2828a7494f72')]
  validation: True [('kiln', 'passed', 0, 11), ('gltf_validator', 'passed', 0, 0), ('godot_import', 'passed', 0, 0)]
## published: ['asset.json', 'hoist--weathered.glb', 'hoist--weathered.glb.import', 'hoist.glb', 'hoist.glb.import'] version 1
```

Notas:
- Publicar antes de verificar se rechaza.
- Los 11 avisos de `kiln` son las piezas pendientes de `hoist` (cajas por buscar) y materiales sin clave de la paleta: avisos, no errores.
- `verify` comprueba el GLB base; las variantes entran en el contrato con su hash y se publican, pero no pasan por las tres comprobaciones.
