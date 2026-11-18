# Spec: personajes con esqueleto y animaciones a través de Kiln (K008)

- **Derived from**: `intent.md`
- **Status**: approved
- **Approved by**: product owner (Aaron Castells, 2026-11-18)

## Lo medido en la Fase 1 (2026-11-18)

Humanoide mínimo hecho con `bpy`: 6 huesos, una malla de 72 triángulos con
pesos (24 de sus 48 vértices repartidos entre dos huesos), acciones `idle` y
`walk`. Blender 5.2.3, Godot 4.7.1, glTF Validator
2.0.0-dev.3.10, Kiln en `23f0eb7`. Todo en una copia de `examples` fuera del
repo.

| Recorrido | `skins` | animaciones | `JOINTS_0`/`WEIGHTS_0` | Godot sin ventana |
|---|---|---|---|---|
| Blender → GLB directo (línea base) | 1 (6 huesos) | `idle`, `walk` | sí | `Skeleton3D` de 6 huesos, `AnimationPlayer`, `walk` mueve `leg_l` |
| GLB → Blender → GLB (ida y vuelta, exportador por defecto) | 1 (6 huesos) | `idle`, `walk` | sí | igual que la línea base |
| `kiln build` (pieza `.blend` del kit) | 0 | ninguna | no | una malla estática |
| `kiln verify` sobre ese GLB | — | — | — | **pasa** las tres comprobaciones |

Conclusiones:

1. **El formato es compatible**: GLB con esqueleto y animaciones entra en
   Godot 4.7.1 sin ajustes. El trabajo artístico no depende de cambiar de
   formato.
2. **Kiln bloquea**: pierde esqueleto, pesos y animaciones, en dos puntos.
   `export_piece.py` selecciona solo mallas y exporta con
   `export_animations=False`. `assemble.py` (`import_model`) hornea la
   jerarquía en la malla, borra lo que no es malla y une; luego
   `base_to_origin` mueve vértices.
3. **La pérdida es silenciosa**: `validate.rs` y `glb.rs` no leen `skins` ni
   `animations`, el hash estructural no los cubre e `import_check.gd` solo
   cuenta nodos y mallas. `verify` da por bueno un personaje roto.
4. **La ida y vuelta por Blender es fiel**, así que el montaje puede seguir
   pasando por `assemble.py`. Los huesos y pesos de cada vértice son
   idénticos (48 de 48). La matriz de deformación de cada hueso, medida en
   Godot en 9 instantes de cada animación (108 muestras), difiere como
   mucho 0,000009 m con un movimiento de hasta 0,62 m. El importador crea
   un objeto auxiliar con la forma de los huesos (`Icosphere`), que no llegó al GLB exportado; el importador tiene la
   opción `disable_bone_shape` para no crearlo (existe, sin probar aún).
5. `publish` funciona con el kit fuera del proyecto de Godot y hoy publica
   el GLB ya sin esqueleto. Con el `.blend` del kit dentro de `godot/`,
   Godot intenta importarlo y `publish` falla (comprobado en los dos
   sentidos).
6. Godot importa las animaciones sin bucle (`loop_mode` 0).

## Requirements

| # | Requirement | Traces to (intent) | Acceptance criterion |
|---|-------------|--------------------|----------------------|
| R1 | Una pieza `.blend` con esqueleto llega al catálogo con su esqueleto, sus pesos y sus acciones | «Kiln conserva el esqueleto y las animaciones al ejecutar build» | El GLB de la pieza del humanoide tiene 1 `skin` de 6 huesos, `JOINTS_0`/`WEIGHTS_0` y las animaciones `idle` y `walk` |
| R2 | `build` monta una pieza con esqueleto sin hornearla, unirla, recentrarla ni escalarla | ídem | El GLB montado conserva lo mismo que R1, con los mismos huesos y pesos por vértice y la misma deformación que el GLB de la pieza (diferencia menor de 0,0001 m en 9 instantes de cada animación, medida en Godot); una pieza sin esqueleto se monta como hasta ahora (hash estructural de `hoist` idéntico) |
| R3 | Kiln lee del GLB los esqueletos (huesos) y las animaciones (nombre, canales, duración), y el hash estructural los cubre | «Validación de esqueletos, pesos y animaciones GLB» | Quitar una animación o la piel cambia el hash; dos montajes iguales dan el mismo; un GLB sin esqueleto ni animaciones da el hash de hoy |
| R4 | La comprobación propia de Kiln avisa de un esqueleto o una animación mal formados | ídem | Un hallazgo de error, con su test, para: primitiva con piel sin `JOINTS_0` o `WEIGHTS_0`; hueso que apunta a un nodo inexistente; animación sin canales; canal que apunta a un nodo inexistente. La normalización de pesos queda en el glTF Validator, que ya la comprueba |
| R5 | La ficha declara las animaciones esperadas y `verify` falla si el GLB montado no las trae o no tiene esqueleto | «Un GLB que las pierde hace fallar verify» | Ficha con `animations = ["idle", "walk"]`: GLB sin `walk` o sin `skins` da error en `kiln`; ficha sin `animations` se comporta como hoy |
| R6 | La importación de prueba en Godot comprueba esqueleto y animaciones | «Godot sin ventana lo instancia con Skeleton3D y AnimationPlayer y las animaciones mueven huesos» | `import_check.gd` informa de huesos, animaciones y de si cada una mueve algún hueso (9 instantes); `godot_import` falla si falta el esqueleto o una animación declarada, y avisa sin fallar si una declarada no mueve ningún hueso (un reposo quieto es legítimo) |
| R7 | El contrato `asset.json` recoge huesos y animaciones, y `publish` entrega el GLB con ellos | «build → verify → publish» | `asset.json` lista huesos (número) y animaciones (nombre, duración); el GLB publicado del humanoide conserva lo de R1 |
| R8 | El recorrido completo queda demostrado y documentado | «demostrar primero el pipeline de personajes animados en Kiln» | `e2e.md` con el recorrido del humanoide de prueba, en positivo y con un GLB sin animaciones que falla; README y `skills/asset` describen la ficha y sus límites |

## Design

Se enseña al camino existente a conservar el esqueleto; no se añade un
segundo camino. Así el personaje sigue teniendo `.blend` de montaje,
materiales de la paleta, informe y contrato, como cualquier asset.

- **`export_piece.py`**: si la escena tiene armaduras, las selecciona junto
  con las mallas y exporta con animaciones (modo `ACTIONS`) y pieles. Sin
  armaduras, exporta exactamente como hoy.
- **`assemble.py`**: `import_model` detecta la armadura. Con ella, conserva la
  jerarquía y no hornea, une, recentra ni escala. La pieza es el objeto
  raíz (la armadura): se coloca en `at` y se gira con `turn`. El origen del
  personaje es el que trae su fuente (pies en el origen, por convenio). Los
  materiales se visten sobre cada malla hija, y los triángulos y el tamaño
  de `report.json` salen de la suma y la caja de esas mallas. Se importa
  con `disable_bone_shape`.
- **Límites de una pieza con esqueleto**: una sola copia (`count` 1) y sin
  ajuste de tamaño. La ficha que declara `animations` lo comprueba al
  leerse. Si no las declara, la ficha no sabe que la pieza trae esqueleto:
  entonces es `assemble.py` quien detiene el montaje con un mensaje claro,
  en vez de montar un personaje roto.
- **`glb.rs`**: `Structure` gana los esqueletos y las animaciones; entran en
  `structure_hash`.
- **`validate.rs`**: hallazgos de R4 en `glb_findings`; los de R5 en
  `asset_findings`.
- **`spec.rs`**: `[asset] animations = [...]`, opcional.
- **`import_check.gd`** y su lectura en Rust: huesos, animaciones, duración y
  «mueve algún hueso» (compara la pose en 9 instantes).
- **`asset.rs`**: bloque `rig` en `asset.json`, ausente si no hay esqueleto.

No se mueven datos fuera de la máquina: todo son ficheros locales del
proyecto.

### Para la cadena de ASTRA (fases 2–4), comprobado en su worktree

No es parte de este spec; se anota para que no se descubra tarde.

- El técnico se distingue sin tocar Rust: `ActorView.name` vale
  `technician`, el mismo mecanismo que `CONTAINER_PIECES` en `world_view.gd`.
- `npc.gd` gira el nodo `mesh`, le pone `material_override` para la señal
  «te habla» y da por hecho el frente en −Z; el frente de glTF es +Z. El
  modelo tiene que conservar nodo, orientación y señal: se mide, no se supone.
- Reposo o caminar salen del desplazamiento de la propia vista (`_path` no
  vacío), no de una acción nueva. El bucle lo fija ASTRA al cargar.
- Sus tareas encadenan ramas: T010-C1 parte de
  `task-010b-access-workshop-identity`.

## Policy applied

- **Personal data**: none
- **Sensitive business data**: none
- **Credentials**: none
- **Decisions about people / high risk**: no

## Out of scope

- Modelar el técnico e integrarlo en Godot (fases 2–4: cadena de ASTRA).
- Variantes pintadas de un asset con esqueleto (`paint`, `repaint.py`): no
  se han probado; quedan como límite documentado.
- Varias copias o cambio de tamaño de una pieza con esqueleto.
- Formas clave (expresiones faciales), cinemática inversa, redireccionado de
  animaciones entre esqueletos, fijar el bucle al publicar.
- Que publicar falle cuando hay un `.blend` dentro del proyecto de Godot
  (ASTRA guarda sus kits fuera de `godot/`; el humanoide de prueba también).

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| La ida y vuelta por Blender altera los huesos de un personaje real (ejes, pose de reposo) más que los del humanoide de prueba | El técnico se deforma al animarse | R6 comprueba en Godot que cada animación mueve huesos; la fase 2 de ASTRA empieza publicando un boceto del técnico antes del modelo final |
| El cambio de `assemble.py` altera el montaje de piezas sin esqueleto | Regresión en los kits de ASTRA | R2: hash estructural de `hoist` idéntico; `scripts/kiln_check.sh` de ASTRA con el binario nuevo |
| El hash estructural cambia para assets ya publicados | Informes viejos dejan de coincidir | Los bloques nuevos solo entran en el hash si el GLB tiene esqueleto o animaciones |

---
Next stage: Build reads this file and writes `plan.md`.
