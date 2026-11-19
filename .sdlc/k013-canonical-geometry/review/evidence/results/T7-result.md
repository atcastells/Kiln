---
task: k013-canonical-geometry/T7
from: integrator
kind: result
result: done
data: evidence
cost:
regen: "cargo test -p kiln-core --lib -- canonical transform config:: mesh:: && cargo clippy -p kiln-core --all-targets -- -D warnings"
persist: propose
files: [crates/kiln-core/src/config.rs, crates/kiln-core/src/mesh.rs]
evidence: [crates/kiln-core/src/config.rs:60, crates/kiln-core/src/config.rs:75, crates/kiln-core/src/config.rs:89, crates/kiln-core/src/mesh.rs:25, crates/kiln-core/src/mesh.rs:33, crates/kiln-core/src/glb.rs:98, /tmp/claudio-test-k013-canonical-geometry-T7.log, /tmp/claudio-clippy-k013-canonical-geometry-T7.log]
---

## Summary

- R9: `GenerateSection.canonical: Option<CanonicalClass>` (config.rs:60), enum `CanonicalClass::RadialV0` con `name()` = "radial_v0" (config.rs:75). Sin clave = `None`, lo de hoy. Valor desconocido o no textual = error de parseo «[generate] canonical: valor desconocido …; se admite «radial_v0»» con un deserializador propio (config.rs:89).
- R10: nuevas `mesh::node_transform(path)` / `node_transform_bytes(bytes) -> Result<Option<String>, String>` (mesh.rs:25, :33). Devuelve `Some("nodo N: <clave>")` para el primer nodo que tiene malla o es antepasado de una y lleva `translation`/`rotation`/`scale`/`matrix` distinta de la identidad; un valor mal formado cuenta como transformación. `read`/`read_bytes` sin cambios; un test comprueba que `read` sigue ignorando la transformación.
- Queda para T8: emitir el código de rechazo (p. ej. `canonical_node_transform`) a partir de ese `Some`.
- Rojo observado en la primera ejecución (8 errores de compilación: E0609, E0433, E0425); el log solo conserva las ejecuciones en verde. Final: 19/19 en verde (tests nuevos + `config::` + `mesh::` existentes); Clippy `-D warnings` limpio en kiln-core.
- No corrido: `cargo test --workspace` (le toca a la orquestación).

Learned: `deserialize_with` + `serde::de::Error::custom` deja el mensaje en castellano dentro del error de toml, sin validar después del parseo.
Learned: un nodo con `skin` y transformación se marca igualmente; `glb.rs:98` en cambio ignora la transformación de un nodo con `skin` (sigue a glTF). La geometría canónica es de mallas sin esqueleto, así que marcarlo es conservador; lo deciden T8 o la sesión principal.
Learned: la identidad se compara exacta (f64 ==); un escritor que emita casi-identidad (1e-9) se marcaría. El exportador de Blender omite la identidad, así que no se espera en la práctica.
Learned: `glb.rs` tiene `local(node)` (glb.rs:411), privado, que ya compone la matriz de un nodo; no se reutilizó por estar fuera de los archivos de la tarea.

## Detail

Tests: config.rs `canonical_geometry_is_off_without_the_key`, `reads_the_canonical_class`, `rejects_an_unknown_canonical_class` ("radial_v1", "", 1); mesh.rs `detects_a_transform_on_the_mesh_node` (translation, rotation, scale, matrix, mal formado, padre transformado), `an_identity_or_absent_transform_is_not_detected` (TRS de identidad explícita, matriz identidad, hermano transformado sin malla, sin `nodes`). Los nodos sin malla ni descendientes con malla (cámaras, luces, vacíos) se ignoran. El cálculo de «coloca malla» es un único recorrido ascendente desde los nodos con malla, cada nodo visitado una vez: lineal y sin bucles ante `children` cíclicos en un archivo fabricado; índices de hijo fuera de rango se descartan.
