---
task: k013-canonical-geometry/T5
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core canonical && cargo clippy -p kiln-core --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/canonical/radial.rs, crates/kiln-core/src/canonical/mod.rs]
evidence: [crates/kiln-core/src/canonical/radial.rs:23, crates/kiln-core/src/canonical/radial.rs:40, crates/kiln-core/src/canonical/radial.rs:57, crates/kiln-core/src/canonical/radial.rs:76, crates/kiln-core/src/canonical/radial.rs:110, crates/kiln-core/src/canonical/mod.rs:5, /tmp/claudio-test-k013-canonical-geometry-T5.log, /tmp/claudio-clippy-k013-canonical-geometry-T5.log]
---

## Summary

- `radial::check_i8_i9(&Geometry) -> Result<(), Rejection>` (radial.rs:23) comprueba primero I8 y luego I9. El doc comment declara la precondición: la geometría ya cumple I1–I7. Si no la cumple, puede entrar en pánico o dar un veredicto sin sentido. Son 85 líneas sin tests, de un máximo de 100. Lo hice yo, sin delegarlo. En mod.rs solo he añadido `pub mod radial;` (mod.rs:5).
- I8 (radial.rs:40): calcula det(a,b,c) en i128 para cada triángulo; si da ≤ 0, devuelve `I8_not_radial` y como elemento el primer triángulo que falla.
- I9 (radial.rs:57): no reintenta direcciones. Usa la dirección simbólica d = m + εb + ε²c, con m = a+b+c del triángulo 0. Esa d está dentro del triángulo 0 y no puede caer sobre el borde de ningún cono. El signo de cada det se decide por el primer valor no nulo de det(m,·,·), det(b,·,·) y det(c,·,·) (radial.rs:76). Siempre termina y no tiene tope. La cota |det| < 2^95 está en el comentario (radial.rs:31).
- El elemento de I9 es el primer triángulo distinto del 0 cuyo cono contiene d. Hay códigos fijos de `I9_covers_a_direction_2_times` a `_9_times`; a partir de 10 el código es `I9_covers_a_direction_10_or_more_times` y el recuento va en el detalle, porque `Rejection.code` es `&'static str`. **Elección revocable por la sesión principal:** ese código de respaldo no está en M§8 ni en el patrón `<n>`.
- El detalle de I8 y de I9 incluye la constante exacta que T6 puede comparar: "a well-formed shape that v0 does not admit (not radial), not a corrupt file" (radial.rs:18). **Elección revocable por la sesión principal:** el texto va en inglés, como el resto de detalles de `Rejection`, aunque R2 lo cita en castellano («forma correcta que v0 no admite»).
- Primero las 5 pruebas en rojo: 4 fallaron por el stub y la fixture ya daba 226/448. Después, `cargo test -p kiln-core canonical` pasa 30/30 (radial, build, mod y topology de T3). Clippy con `-D warnings` no da avisos.
- La doble vuelta de M§8 está portada de ASTRA `a7_extra.py` (S=16, R=1e6). Da 226 V y 448 T, pasa I8 y cae en `I9_covers_a_direction_2_times`. Learned: no se ha comprobado aquí que la fixture pase I1–I7 con el `topology.rs` de T3, que aún estaba en curso; puede hacerlo T6.

Learned: el criterio de geo.py (det ≥ 0, conos cerrados) puede contar dos veces una dirección que cae sobre una arista compartida; la perturbación simbólica lo evita sin tolerancias.
Learned: el triángulo 0 siempre se cuenta a sí mismo, así que I9 nunca da 0 y los códigos empiezan en 2.
Learned: para quien escriba el validador, m,b,c son independientes porque det(m,b,c) = det(a,b,c) > 0, así que la regla lexicográfica nunca da un 0 con entradas que cumplan I8.

## Detail

- Pruebas (radial.rs:138–194): tetraedro centrado y octaedro → Ok; tetraedro desplazado +3000 en x → I8 en el elemento 2 (los triángulos 0 y 1 siguen positivos; comprobado a mano), con detalle «v0 does not admit / not a corrupt file»; vértice 3 en (100,100,100) → I8 en el elemento 1; doble vuelta → 226/448, `check_i8` Ok, I9 de 2 con elemento > 0 y el detalle; además `check_i9(octaedro)` da Ok. Una prueba del clasificador: 5 direcciones exactamente sobre el rayo de un vértice (+z), sobre una arista compartida (+x+y) o en el interior, cada una con su perturbación, caen en exactamente 1 cara del octaedro. Otra dirección, con m y b en el plano y = 0, solo la resuelve el término ε² y cae en la cara 0.
- Cotas: |c| < 2^30 → cada producto de det(a,b,c) es < 2^90 y el det < 2^93; |m_i| < 3·2^30 < 2^32 → det(m,u,v) < 2^95. Cabe de sobra en i128.
- Alternativa considerada y descartada: d_k = a + k·b + k²·c con k creciente. Termina, pero necesita un tope de k; con mallas grandes ese tope rompe la cota de i128 o deja casos sin resolver. La perturbación simbólica no tiene ninguno de los dos problemas.
