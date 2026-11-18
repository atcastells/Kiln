# K008: recorrido real del humanoide de prueba (2026-11-18)

Rama `k008-rigged-characters`. Blender 5.2.3, Godot 4.7.1, glTF Validator
2.0.0-dev.3.10. Proyecto: copia de `examples` fuera del repo, con el kit del
humanoide en `kits_src/crew/` (fuera de `godot/`) y `[publish] dir =
"godot/assets/kiln"`. El humanoide sale de
`crates/kiln-cli/tests/fixtures/rig_humanoid.py`: 6 huesos, 72 triángulos,
24 vértices con dos huesos, acciones `idle` y `walk`.

Ficha `specs/crew_member.toml`: una pieza `reuse` de `crew/humanoid` en el
origen y `animations = ["idle", "walk"]`.

## En positivo

```
== build
Blender exporta crew/humanoid (blender)
Plan de montaje: 1 copias de 1 piezas
Montando en Blender (blender)
built <p>/kiln/build/crew_member/crew_member.glb 72 tris
== verify
kiln: pasa
gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
godot_import: pasa (proyecto de prueba en kiln/cache/verify/crew_member)
== publish
«crew_member» publicado en godot/assets/kiln/crew_member (versión 1)
```

- GLB publicado: 1 `skin` de 6 huesos, `JOINTS_0` y `WEIGHTS_0`,
  animaciones `idle` y `walk` (18 canales cada una).
- `asset.json`: `rig = { bones: 6, animations: [idle 2,5 s, walk 1,0 s] }`.
- `validation.json`: válido. Dos avisos: el material `body/Suit` no lleva
  clave de la paleta (el humanoide de prueba no usa la paleta) y
  `NODE_SKINNED_MESH_NON_ROOT` del glTF Validator (la malla cuelga de la
  armadura, como la exporta Blender; Godot la importa bien).
- `godot_import` sin avisos: las dos animaciones mueven huesos.

## En negativo

El mismo `.blend` con las acciones borradas, misma ficha:

```
== build
built <p>/kiln/build/crew_member/crew_member.glb 72 tris
== verify
kiln: falla
gltf_validator: pasa (glTF Validator 2.0.0-dev.3.10)
godot_import: falla (proyecto de prueba en kiln/cache/verify/crew_member)
la verificación de «crew_member» no pasa: la animación 'idle' está declarada en el spec y no está en el GLB
== publish
la verificación de «crew_member» no pasó: corrige y vuelve a verificar
```

- Hallazgos de error: `rig_animation_missing` (`idle`, `walk`) y
  `godot_animation_missing` (`idle`, `walk`).
- Lo publicado no cambia: sigue la versión 1, con sus animaciones.

## Regresiones

- `cargo test --workspace`: en verde (268 en `kiln-core`, 87 unitarios en
  `kiln-cli`, más los de integración). Clippy con `-D warnings`: limpio.
- `cargo test --workspace -- --ignored`: 12 en verde con Blender y Godot
  reales (`blender_rig` 3, `godot_verify` 4, y los 5 anteriores).
- Hash estructural con el Kiln de `main` (`23f0eb7`) y con el de esta rama,
  montando lo mismo: idéntico en `hoist` (`examples`) y en `workbench`,
  `door_industrial_leaf` y `corridor_wall_doorway` (copia de ASTRA).
- `kiln check` sobre la copia de ASTRA con el binario nuevo: 23 piezas,
  0 errores.

## Verificación independiente (T12)

Ámbar, con un hallazgo importante ya corregido: una pieza con esqueleto y
`turn` distinto de cero hacía fallar `verify` con un `glb_size` falso. Kiln
aplicaba la transformación del nodo a la caja de una malla con piel, cuyos
vértices ya están en el espacio del esqueleto (glTF ignora ese nodo). Test
rojo `glb::tests::a_skinned_mesh_ignores_its_node_transform` y, con
herramientas reales, `godot_verify::a_moved_and_turned_rig_still_verifies`.
Los tests ignorados son ahora 13.

Tras las correcciones de la revisión (ver *Review* en `plan.md`) se repitió
todo: `cargo test --workspace` 423 en verde, Clippy limpio, 14 tests
ignorados en verde con Blender y Godot reales, y los cuatro hashes
estructurales frente a `main` siguen idénticos.

El verificador montó también una variante de color de un asset con
esqueleto (`variant--alt.glb`): conserva piel y animaciones. No hay test.

El verificador confirmó además: el recorrido en positivo y en negativo, el
personaje sin `animations` declaradas (conserva el esqueleto), animaciones
declaradas sobre una pieza estática (`rig_no_skeleton`), dos copias y cambio
de tamaño (el montaje se detiene), una animación quieta (aviso), GLB mal
formados, y que `hoist` y `kiln index` dan lo mismo que con el binario de
`main`. Sin confirmar por él: la parada con `split` y `animation_empty`
(esta, cubierta por test unitario).

## Observado

- `rig_count` salta para cualquier pieza con `count` mayor que 1 cuando la
  ficha declara animaciones, también una estática: la ficha no sabe cuál
  trae el esqueleto. Queda documentado como límite.
- `import_check.gd` mide el tamaño aplicando el nodo a las mallas con piel;
  ese tamaño es informativo y `verify` no lo compara.

- La raíz de la pieza con esqueleto toma el nombre de la pieza en el
  montaje (`body`); una malla que se llamara igual pasa a `body_mesh`.
- `cargo fmt --check` ya da diferencias en `main` en muchos ficheros; no es
  una puerta de este repo y no se ha tocado.
