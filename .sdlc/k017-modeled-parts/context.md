# Context: k017-modeled-parts

Lo leen primero todas las tareas. Lo escribe solo la sesión principal.

## Decisiones

- Fuente nueva `source = { kind = "model" }`; la forma va en `[[parts.shapes]]`.
  Vocabulario cerrado: `box`, `para`, `tube`. Cada forma lleva `zone`.
- Coordenadas de la ficha en metros, ejes de glTF: Y arriba, frente de la
  pieza hacia +Z. `assemble.py` las pasa a Blender con `to_blender`
  (`(x, y, z) -> (x, -z, y)`).
- Triángulos: `box` y `para` 12; `tube` `4·segments − 4` (siempre con tapas).
- Límites (R6): 1..=256 formas por pieza, `segments` 3..=32, coordenadas
  finitas con |c| ≤ 100 m, `radius`/`radius_to` en (0, 50] m; formas sin
  volumen rechazadas (`box` con algún lado < 1 mm, `para` con |det(u,v,w)|
  < 1e-9 m³, `tube` con longitud < 1 mm).
- Códigos en `spec::check` (`crates/kiln-core/src/spec.rs:352`), severidad
  Error: `model_shape` (forma rota o fuera de límites; también `shapes` en una
  pieza que no es `model`, o `model` sin `shapes`), `model_zone` (zona sin
  material: ni en `materials` ni `material`), `model_size` (`size_m` dado que
  no casa con la caja de las formas a ±1 mm). Los triángulos de las formas por
  `count` se suman a `parts_tris` y avisan con el `budget` que ya existe.
- `size_m` de una pieza `model` sale de `bounds()` si la ficha no lo da.
- API de `crates/kiln-core/src/model.rs`: `Shape { zone, form: Form }`,
  `Form::{Box([V3;2]), Para(Para), Tube(Tube)}`, `tris()`, `bounds()`,
  `problems()` (mensajes «forma N: …»; lista vacía o > 256 sin prefijo),
  `tube_rims()`, `VOCABULARY` (sin título). `Shape` se lee con `try_from`:
  una clave desconocida o dos formas en una entrada son error de lectura.
- Vértices del tubo (Kiln y `assemble.py` iguales, para que `bounds()` sea la
  caja real): `d` = eje unitario; `a` = eje del mundo menos alineado con `d`
  (índice menor si empatan); `e1 = unit(d × a)`, `e2 = d × e1`; vértice k en
  `c + r(cos θ·e1 + sin θ·e2)`, θ = 2πk/segments; tapas en abanico (n−2
  triángulos cada una).
- `[assets] uv_per_m`: opcional, > 0, 1.0 si falta. Va en `Plan.uv_per_m`.
- Blender no ejecuta código del agente: `plan.json` solo lleva números y
  nombres de zona.

## Convenciones

- Código e identificadores en inglés; mensajes al usuario en castellano, como
  los que ya hay en `spec::check`.
- Campos nuevos serializados con `skip_serializing_if` para que los planes y
  fichas sin piezas `model` salgan idénticos (R9).
- Pruebas: como mucho 3 nuevas por tarea; errores en una tabla.
- Comandos: `cargo test -p kiln-core <filtro>` mientras se itera; la batería
  entera la corre la sesión principal.
