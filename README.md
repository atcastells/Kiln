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
# tu juego con el kiln.toml de ejemplo (o copia examples/kiln.toml a la raíz del juego)
target/release/kiln serve --project ../mi-juego --config examples/kiln.toml   # http://127.0.0.1:4330
target/release/kiln check --project ../mi-juego --config examples/kiln.toml   # sale con 1 si hay errores
target/release/kiln index --project ../mi-juego --config examples/kiln.toml --out catalog.json
```

Sin dependencias fuera de Rust; el servidor solo escucha en `127.0.0.1`.

## `kiln.toml`

```toml
[project]
name = "Puerto Niebla"

[assets]
kits = ["godot/kit"]           # cada subcarpeta es un kit: modelos, JSON de manifiesto, LICENSE.md

[materials]
tables = ["data/buildings/materials.toml"]   # tablas TOML con filas que llevan `id`
textures = ["godot/textures"]                # texturas `<clave>_albedo.<ext>`

[rules]
licenses = ["CC0", "CC-BY"]
max_tris = 20000
```

## Lo que viene

- Entrega 2: pedir un asset desde el grafo; Claude Code (`claude -p`) escribe la ficha y los nodos aparecen. El grafo pasa a Rete.js.
- Entrega 3: producción y montaje con Blender y Godot, vista 3D en cada nodo, vínculo de ranuras con la paleta y app de escritorio (Tauri).
- Entrega 4: el primer asset de punta a punta.

Los `.res` y escenas de Godot no se miden todavía: lo hará Godot sin ventana en la entrega 3.
