# Revisión (K007)

Un verifier y tres pasadas independientes (bugs, seguridad, conformidad) sobre el cambio sin commit de la rama `k007-ui`, el 2026-11-17. Kiln no tiene `REVIEW.md`; se usaron las tres pasadas de la plantilla. Las correcciones se hicieron en dos encargos (A y B), con prueba en rojo o mutación, y no las ha vuelto a ver otro revisor.

## Verifier: ámbar

Comprobado en ejecución: `check` sobre `examples` y sobre una copia de ASTRA, `status` por MCP (12/15), los contrastes, los casos provocados (glifo, tema ausente), los niveles `error` y `off`, los ciclos, un tema mal formado, fuentes basura, y los flujos vecinos (`content.station`, `catalog` de otros tipos).

| Hallazgo | Gravedad | Resolución |
|---|---|---|
| Una fuente manipulada (cmap con 2000 grupos `0..0x10FFFF`) cuelga `check` y el MCP | IMPORTANT | Arreglado (A): presupuesto de `0x110000 × 2` en los formatos 4 y 12; prueba con la fuente bomba |
| Una regla en `off` y un glob vacío dan el criterio por cumplido | IMPORTANT (a decidir) | Arreglado (A y B): el glob vacío da `ui-files`, que deja sin cumplir los criterios `kiln:*`; con la regla en `off`, el criterio queda pendiente con un problema. Enmienda 2 de la spec |
| Una fuente ilegible da siempre error | NIT | Arreglado (A): sigue el nivel de `glyphs` (`ui-files`) |
| Un color mal formado en el `.tres` se descarta sin avisar | NIT | Aceptado: Godot siempre escribe `Color(r, g, b, a)` completo |
| `.TTF` en mayúsculas se salta | NIT | Arreglado (A) |
| Cadenas de varias líneas sin escanear | NIT | Arreglado (A) |
| `BUILTIN` corto: `OptionButton`, `Tree` dan falso `ui-variation-missing` | NIT | Arreglado (A): 39 clases |
| `catalog what=ui` incompleto para R1 | NIT | Arreglado (B): `uses`, `fonts`, `colors`. `kiln index` sin UI, desviación aceptada (enmienda 2) |
| `[ui.glypths]` con errata se ignora (`deny_unknown_fields`) | NIT | Aceptado: el resto de `kiln.toml` se comporta igual |

## Pasada de bugs

| Hallazgo | Gravedad | Resolución |
|---|---|---|
| Claves sin `/` (`default_font`) y estilos de tipo (`Button/styles/normal`) se toman como variaciones y salen como huérfanas | IMPORTANT | Arreglado (A): solo cuenta como variación una entrada con `base_type` |
| `text = "…"` de varias líneas en una escena no se escanea | IMPORTANT | Arreglado (A) |
| `theme_type_variation = _pick("kind")` cuenta «kind» como uso | NIT | Arreglado (A): solo si el lado derecho empieza por una cadena literal |
| `add_theme_stylebox_override` no está en R5, y las marcas se buscan también dentro de cadenas | NIT | Arreglado (A): se mira solo el código; R5 se amplía (enmienda 2) |
| Regla desconocida o `status` inválido sin problema | NIT | Arreglado (B) |
| 4,46 se muestra «4,5:1 < 4,5:1» | NIT | Arreglado (A): se trunca |
| Subtablas `(0,3)` antes de `(0,4)` | Sin confirmar | Anotado |
| Colores fuera de 0..1 dan NaN | Sin confirmar | Arreglado (A): se acotan a 0..1 |

Verificado correcto: `cmap` coincide con `fontTools` `getBestCmap` en 7 fuentes del sistema, y la fórmula WCAG, el alfa y la herencia son correctos.

## Pasada de seguridad

| Hallazgo | Gravedad | Resolución |
|---|---|---|
| Globs y fuentes `res://` sin confinar: `scripts = ["../outside/*.gd"]` leía fuera del proyecto | IMPORTANT | Arreglado (A): `ui::inside`, con la misma convención que `asset.rs`; los enlaces que salen de la raíz se saltan |
| `ui.theme` y `ui.target` sin confinar; el error de TOML de `target` devolvía por MCP una línea de un fichero de fuera | IMPORTANT | Arreglado (A y B): rutas confinadas; los mensajes no citan contenido |
| Fuente bomba (cmap) | IMPORTANT | Arreglado (A), igual que en el verifier |
| Lecturas sin límite de tamaño ni de número (`**/*`) | Sin confirmar | Anotado: los globs ya no salen del proyecto. El límite de tamaño queda para otra tarea |

Limpio: no hay pánicos alcanzables, no escribe en disco, no hay XSS (la página escapa), la licencia OFL va junto a cada copia de Plex, y no hay secretos ni datos personales.

## Pasada de conformidad

| Hallazgo | Gravedad | Resolución |
|---|---|---|
| La spec no recogía la regla del peor de negro y blanco | IMPORTANT | Arreglado: enmienda 1 de la spec |
| R1 parcial (`catalog` y `kiln index`) | IMPORTANT | Arreglado en parte (B); `kiln index` queda como desviación (enmienda 2) |
| R4: los tipos de Godot salían como huérfanos | IMPORTANT | Arreglado (A) |
| Claves sin `/` crean variaciones fantasma | IMPORTANT | Arreglado (A) |
| R8: las pruebas de contraste llevaban colores de ASTRA | IMPORTANT | Arreglado (B): valores neutros; `grep` de ASTRA en `crates/kiln-core/src/ui` vacío. Los nombres de ASTRA de K002 en `content/` siguen; anotado en la enmienda 2 |
| `examples/kiln/ui_target.toml` está ignorado por git | IMPORTANT | Arreglado (B): pasa a `examples/ui_target.toml` |
| R5 tenía una sexta marca | IMPORTANT | Aceptado y escrito en la spec (enmienda 2) |
| `ui-theme` siempre error frente a R7 | IMPORTANT (juicio) | Aceptado: es un error de configuración, no una regla (enmienda 2) |
| Espaciado de `const SPEC` en `check.rs` | NIT | Arreglado (B) |
| Escaneo de usos repetido (`rules.rs`, y `variation_uses` en `target.rs`) | NIT | Abierto: unificar en `rules.rs` en otra tarea |
| Fuente OFL de 136 KB en dos sitios | NIT | Aceptado (plan) |

## Abierto

- Unificar el escaneo de usos de variaciones (`rules.rs` y `target.rs`).
- Límite de tamaño y de número de ficheros en las lecturas de UI.
- `split_writes_the_zones_file` (servidor HTTP) falló una vez en la batería y pasó 3 de 3 aislada y en las baterías siguientes. Prueba inestable, ajena a K007.
