# K004: de punta a punta por MCP (T23)

## Hecho por main (2026-11-16)

Sobre una copia de `examples` en el scratchpad, con `kiln mcp` real y un
cliente JSON-RPC mínimo (sin Claude Code):

- `build { asset: "hoist" }` → `{"job": 1}`.
- `job { id: 1, since, wait_s: 30 }` tres veces: `running` (5 líneas:
  piezas por buscar como cajas, Blender), `running` (montado, aplicando
  materiales), `done` con `file = kiln/build/hoist/hoist.glb`.
- Blender montó y pintó `hoist` y su variante `weathered` en 2,3 s; quedan
  `hoist.glb`, `hoist--weathered.glb` y `plan.json`.

## Pendiente con la persona (Claude Code)

1. `cargo build --release` y, sobre una copia de `examples`:
   `claude mcp add kiln -- /Users/acastells/Kiln/target/release/kiln mcp --project <copia>`.
2. En Claude Code, el prompt `kiln-asset` con una petición (por ejemplo,
   «una caseta de pescadores»): el agente lee con `status`, `catalog` y
   `context_get`, escribe con `spec_write` y coloca con `spec_layout`.
3. `build` seguido con `job`; después `review` (lanza el revisor de Kiln y
   gasta uso de la suscripción) y `review_sheets`.
4. Guardar aquí la transcripción y la ruta del GLB.

## Hecho con Claude Code (2026-11-16)

Claude Code 2.1.322 en modo `-p`, con `--mcp-config` (sin `claude mcp add`),
sobre una copia de `examples` en el scratchpad. Transcripción resumida en
[e2e-transcript.md](e2e-transcript.md).

- Conecta y expone las 27 herramientas.
- **Paso 2 falla como slash command.** `/mcp__kiln__kiln-asset "una caseta de
  pescadores"` → `ProtocolError: el asset «"una» no vale`. Kiln anuncia los
  argumentos de `kiln-asset` como `["asset", "request"]` (orden alfabético) y
  Claude Code reparte la línea por espacios en ese orden. Una petición de varias
  palabras no se puede pasar. Para seguir, se pidió el texto con `prompts/get`
  y se pasó como prompt.
- Lectura (`status`, `context_get`, `catalog` ×4), `spec_write` (`fisher_hut`,
  aceptada con avisos), `spec_layout` (11 piezas), `context_propose` (q3, q4).
- `build` → job 1 `done` en la primera llamada a `job`. Salen 204 tris.
- `review` → job 2: 3 vueltas, unos 6 min y unas 50 llamadas a `job`. Se queda
  la vuelta 3 en «revisar» (1 grave, 3 importantes, 3 menores). Después,
  `review_sheets`.
- GLB: `<copia>/kiln/build/fisher_hut/fisher_hut.glb` (scratchpad de la sesión,
  no versionado).
- Total: 66 turnos, 466 s.

### Bug encontrado: el montaje pierde `turn` en piezas importadas

`plan.json` lleva `roof_1` y `roof_2` con `turn = [∓56.3, 0, 0]`, pero en el GLB
ambos nodos salen sin `rotation`. Solo gira `net_rack`, que es una caja.
Causa, comprobada en Blender: el importador glTF deja el objeto con
`rotation_mode = 'QUATERNION'`; `src.copy()` lo hereda y
`o.rotation_euler = …` (assemble.py, `build`) no tiene efecto. Las cajas de
`box()` usan `XYZ` y sí giran. No depende del número de copias, como dedujo
el revisor. Afecta a todo `turn` sobre una pieza con modelo (también a `hoist` y
`cargo_quay`). Arreglo probable: `o.rotation_mode = "XYZ"` antes de asignar el
euler, con un test que compruebe la rotación en el GLB.

## Qué se hizo con lo hallado

- Slash command de `kiln-asset` → T24: `request` es opcional; sin ella el
  agente pregunta. Falta decidir si se quita `asset` (ver plan).
- Unas 50 llamadas a `job` en una revisión → T25: `job` agrupa las líneas
  hasta un segundo de silencio.
- Giro perdido en piezas con modelo → viene de la entrega 4, no de K004:
  cadena nueva en `.sdlc/k005-turn-imported/intent.md`.
