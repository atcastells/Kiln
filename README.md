# Kiln

Motor local de assets para proyectos de juego. Describes un asset y un grafo
de nodos lo desgrana en piezas, reutiliza lo que el juego ya tiene y liga cada
modelo a la paleta de materiales del proyecto. Trabaja con Godot, Blender y
Rust.

Diseño: [Motor de creación de assets](https://claude.ai/artifact/Au2dgBc6AtrwVevdstwL2z) ·
[Proyectos parecidos](https://claude.ai/artifact/AUiJFz821qrR9Z5pKgvWne) ·
[maqueta del grafo](https://claude.ai/artifact/QrShQzVqXJ1VoVwzvn4q5f).

## Entrega 1: el grafo de lo que ya hay

Kiln lee un proyecto descrito por su `kiln.toml` y monta el catálogo: cada
kit, cada pieza con su medida, triángulos, ranuras de material y licencia, y
los materiales de la paleta. Pasa las comprobaciones que fallan sin que nadie
lo vea (licencias, crédito CC-BY, escala, base en el suelo, coste) y lo
enseña como grafo en el navegador.

```bash
cargo build --release
# rune-world con el kiln.toml de ejemplo (o copia examples/rune-world.kiln.toml a la raíz del juego)
target/release/kiln serve --project ../rune-world --config examples/rune-world.kiln.toml   # http://127.0.0.1:4330
target/release/kiln check --project ../rune-world --config examples/rune-world.kiln.toml   # sale con 1 si hay errores
target/release/kiln index --project ../rune-world --config examples/rune-world.kiln.toml --out catalog.json
```

Sin dependencias fuera de Rust; el servidor solo escucha en `127.0.0.1`.

## `kiln.toml`

```toml
[project]
name = "Rune World"

[assets]
kits = ["godot/kit"]           # cada subcarpeta es un kit: modelos, JSON de manifiesto, LICENSE.md

[materials]
tables = ["data/buildings/materials.toml"]   # tablas TOML con filas que llevan `id`
textures = ["godot/textures"]                # texturas `<clave>_albedo.<ext>`

[rules]
licenses = ["CC0", "CC-BY"]
max_tris = 20000
```

## Entrega 2: pedir un asset

En el grafo, escribe la petición («crear un ascensor») y pulsa Desglosar.
Kiln lanza Claude Code sin ventana (`claude -p`, con la skill
`skills/asset/SKILL.md`) en el proyecto; el agente lee el proyecto y el
catálogo y escribe la ficha en la carpeta `[assets] specs` de `kiln.toml`.
El grafo sigue lo que hace y, al terminar, despliega la ficha: petición,
asset, piezas, de dónde sale cada modelo (reutilizar, variante, buscar,
retocar), material de la paleta y validación. `examples/specs/hoist.toml` es
una ficha tal como salió para rune-world.

- Necesita `claude` en el PATH (o su ruta en `KILN_CLAUDE`) con sesión iniciada.
- El agente solo puede leer y escribir ficheros (sin terminal) y no baja nada:
  las piezas «buscar» esperan tu permiso.
- Solo la propia página puede lanzar al agente: la petición lleva una cabecera
  propia y el servidor solo escucha en `127.0.0.1`.

## Lo que viene

- Entrega 3: producción y montaje con Blender y Godot, vista 3D en cada nodo, vínculo de ranuras con la paleta y app de escritorio (Tauri).
- Entrega 4: el primer asset de punta a punta.
- Entrega 5: componer escenas con módulos de piezas (un pasillo de suelo, techo, muros y luces).
- El grafo pasará a Rete.js cuando los nodos se puedan editar.

Los `.res` y escenas de Godot no se miden todavía: lo hará Godot sin ventana en la entrega 3.
