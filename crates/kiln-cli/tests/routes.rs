//! Smoke test of every route of `kiln serve`: pins today's status codes and
//! error messages so the logic can move without changing what the page gets.

mod common;
use common::*;

const FORBIDDEN: &str = "solo desde la pagina de Kiln";

fn project() -> (TempDir, Server) {
    let dir = temp_project("");
    let server = Server::start(&dir);
    (dir, server)
}

/// POST as the page and expect a 400 with exactly this error.
fn bad(s: &Server, path: &str, body: &str, error: &str) {
    let (code, b) = s.post(path, body, true);
    assert_eq!(code, 400, "{path}: {}", String::from_utf8_lossy(&b));
    assert_eq!(error_of(&b).as_deref(), Some(error), "{path}");
}

#[test]
fn gets_answer_200() {
    let (_d, s) = project();
    for path in ["/", "/api/catalog", "/api/context", "/api/new", "/api/chat", "/vendor/three.js"] {
        let (code, body) = s.get(path);
        assert_eq!(code, 200, "{path}");
        assert!(!body.is_empty(), "{path}");
    }
    assert_eq!(json_of(&s.get("/api/new").1)["step"], 0);
    assert!(json_of(&s.get("/api/catalog").1)["specs"].is_array());
    assert!(json_of(&s.get("/api/context").1)["context"]["claims"].is_array());
    assert!(json_of(&s.get("/api/chat").1)["messages"].is_array());
}

#[test]
fn unknown_things_are_404() {
    let (_d, s) = project();
    let (code, b) = s.get("/api/jobs/999");
    assert_eq!(code, 404);
    assert_eq!(error_of(&b).as_deref(), Some("no existe ese trabajo"));
    let (code, b) = s.get("/api/jobs/abc");
    assert_eq!(code, 404);
    assert_eq!(error_of(&b).as_deref(), Some("no existe ese trabajo"));
    for path in ["/nope", "/api/nope", "/files/../../etc/passwd", "/files/%2e%2e/%2e%2e/etc/passwd.png", "/files/../kiln.toml", "/files/nothing.glb"] {
        let (code, b) = s.get(path);
        assert_eq!(code, 404, "{path}");
        assert_eq!(b, b"no existe", "{path}");
    }
}

#[test]
fn files_serves_a_model_under_the_roots() {
    let (_d, s) = project();
    let (code, b) = s.get("/files/godot/kits/harbor/beam.glb");
    assert_eq!(code, 200);
    assert!(!b.is_empty());
}

#[test]
fn posts_without_the_header_are_403() {
    let (_d, s) = project();
    for path in [
        "/api/ask", "/api/build", "/api/generate", "/api/review", "/api/godot", "/api/search", "/api/import", "/api/bind",
        "/api/material", "/api/validate", "/api/split", "/api/paint", "/api/new", "/api/settle", "/api/chat",
        "/api/context/sort", "/api/context/refine", "/api/context/text", "/api/context/intent",
        "/api/context/answer", "/api/context/claim", "/api/unknown",
    ] {
        let (code, b) = s.post(path, "{}", false);
        assert_eq!(code, 403, "{path}");
        assert_eq!(error_of(&b).as_deref(), Some(FORBIDDEN), "{path}");
    }
}

#[test]
fn invalid_posts_are_400_with_their_message() {
    let (_d, s) = project();
    bad(&s, "/api/ask", r#"{"text":""}"#, "la peticion va vacia o pasa de 500 caracteres");
    bad(&s, "/api/ask", "{}", "la peticion va vacia o pasa de 500 caracteres");
    bad(&s, "/api/ask", &format!(r#"{{"text":"{}"}}"#, "a".repeat(501)), "la peticion va vacia o pasa de 500 caracteres");
    for path in ["/api/build", "/api/review", "/api/paint"] {
        bad(&s, path, "{}", "falta el asset");
    }
    bad(&s, "/api/import", r#"{"asset":"hoist"}"#, "falta el asset, la pieza, el sitio o el modelo");
    bad(&s, "/api/bind", "{}", "falta la pieza o el hueco");
    bad(&s, "/api/bind", r#"{"piece":"harbor/beam"}"#, "falta la pieza o el hueco");
    bad(&s, "/api/bind", r#"{"slot":"Wood"}"#, "falta la pieza o el hueco");
    bad(&s, "/api/material", r#"{"asset":"hoist","part":"post","zone":"Wood","material":"nope"}"#, "«nope» no está en la paleta");
    bad(&s, "/api/material", r#"{"asset":"nope","part":"post","zone":"Wood","material":"oak"}"#, "no hay ficha con id «nope»");
    bad(&s, "/api/validate", r#"{"asset":"hoist","part":"nope"}"#, "la ficha no tiene la pieza «nope»");
    bad(&s, "/api/validate", r#"{"asset":"nope","part":"post"}"#, "no hay ficha con id «nope»");
    bad(&s, "/api/split", r#"{"piece":"nope","mode":"loose"}"#, "«nope» no está en el catálogo");
    bad(&s, "/api/search", r#"{"asset":"nope","part":"post"}"#, "no hay ficha con id «nope»");
    bad(&s, "/api/search", r#"{"asset":"hoist","part":"nope"}"#, "la ficha no tiene la pieza «nope»");
    bad(&s, "/api/new", "{}", "esta carpeta ya tiene kiln.toml");
    bad(&s, "/api/context/nope", "{}", "no existe");
    bad(&s, "/api/chat", r#"{"step":"9","message":"hola"}"#, "paso desconocido: 9");
    let (code, b) = s.post("/api/chat", "{}", true);
    assert_eq!(code, 400);
    assert!(error_of(&b).unwrap().starts_with("petición mal formada: "));
    bad(&s, "/api/chat", r#"{"step":"1","message":"   "}"#, &format!("el mensaje debe tener entre 1 y {} caracteres", kiln_core::chat::MAX_TEXT_CHARS));
    // page header with a foreign host is still refused
    let (code, _) = s.post("/api/unknown", "{}", true);
    assert_eq!(code, 404);
}

#[test]
fn generate_refuses_a_bad_seed() {
    let (_d, s) = project();
    bad(&s, "/api/generate", r#"{"asset":"rock","seed":"nope"}"#, "la semilla debe ser un número entero entre 0 y 4294967295");
    bad(&s, "/api/generate", r#"{"asset":"rock","seed":4294967296}"#, "la semilla debe ser un número entero entre 0 y 4294967295");
}

#[test]
fn generate_rejects_an_unknown_op_and_writes_nothing() {
    let (d, s) = project();
    let recipe = "[asset]\nid = \"rock\"\nname = \"Rock\"\n\n[generator]\nseed = 1\n\n[[generator.ops]]\nop = \"crater\"\n";
    let body = serde_json::json!({ "asset": "rock", "recipe": recipe }).to_string();
    let (code, b) = s.post("/api/generate", &body, true);
    assert_eq!(code, 400, "{}", String::from_utf8_lossy(&b));
    assert!(error_of(&b).unwrap().contains("generator_op"), "{}", String::from_utf8_lossy(&b));
    assert!(!d.exists("specs/rock.toml"));
}

#[test]
fn bind_writes_the_bindings_file() {
    let (d, s) = project();
    assert!(!d.exists("kiln/bindings.toml"));
    let (code, b) = s.post("/api/bind", r#"{"piece":"harbor/beam","slot":"Wood","material":"oak"}"#, true);
    assert_eq!(code, 200, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json_of(&b)["ok"], true);
    let text = d.read("kiln/bindings.toml");
    assert!(text.contains("harbor/beam") && text.contains("Wood") && text.contains("oak"), "{text}");
    // an empty material unbinds
    let (code, _) = s.post("/api/bind", r#"{"piece":"harbor/beam","slot":"Wood","material":""}"#, true);
    assert_eq!(code, 200);
    assert!(!d.read("kiln/bindings.toml").contains("oak"));
}

#[test]
fn bind_with_an_empty_piece_writes_before_it_refuses() {
    // today's behaviour (serve.rs: the write runs before the emptiness check)
    let (d, s) = project();
    let (code, b) = s.post("/api/bind", r#"{"slot":"Wood","material":"oak"}"#, true);
    assert_eq!(code, 400);
    assert_eq!(error_of(&b).as_deref(), Some("falta la pieza o el hueco"));
    assert!(d.exists("kiln/bindings.toml"), "bindings file was not written");
    assert!(d.read("kiln/bindings.toml").contains("oak"));
}

#[test]
fn material_changes_the_spec() {
    let (d, s) = project();
    let (code, b) = s.post("/api/material", r#"{"asset":"hoist","part":"post","zone":"Wood","material":"oak"}"#, true);
    assert_eq!(code, 200, "{}", String::from_utf8_lossy(&b));
    let spec: toml::Table = d.read("specs/hoist.toml").parse().unwrap();
    let post = spec["parts"].as_array().unwrap().iter().find(|p| p["id"].as_str() == Some("post")).unwrap();
    assert_eq!(post["materials"]["Wood"].as_str(), Some("oak"));
}

#[test]
fn split_writes_the_zones_file() {
    let (d, s) = project();
    let (code, b) = s.post("/api/split", r#"{"piece":"harbor/beam","mode":"loose"}"#, true);
    assert_eq!(code, 200, "{}", String::from_utf8_lossy(&b));
    let text = d.read("kiln/zones.toml");
    assert!(text.contains("harbor/beam") && text.contains("loose"), "{text}");
    let (code, b) = s.post("/api/split", r#"{"piece":"harbor/beam","mode":"nope"}"#, true);
    assert_eq!(code, 400, "{}", String::from_utf8_lossy(&b));
    assert!(error_of(&b).is_some());
}

#[test]
fn context_edits_change_the_context_file() {
    let (d, s) = project();
    let (code, b) = s.post("/api/context/claim", r#"{"action":"add","text":"Mareas de dos metros","layer":"world"}"#, true);
    assert_eq!(code, 200, "{}", String::from_utf8_lossy(&b));
    assert!(d.read("context.toml").contains("Mareas de dos metros"));
    let (code, b) = s.post("/api/context/claim", r#"{"action":"add","text":"","layer":"world"}"#, true);
    assert_eq!(code, 400);
    assert_eq!(error_of(&b).as_deref(), Some("falta el texto o la capa"));
    bad(&s, "/api/context/claim", r#"{"action":"pin","id":"zz"}"#, "no hay afirmación «zz»");
    let (code, _) = s.post("/api/context/text", r#"{"text":"Un texto nuevo del juego"}"#, true);
    assert_eq!(code, 200);
    let ctx = d.read("context.toml");
    assert!(ctx.contains("Un texto nuevo del juego") && !ctx.contains("Un pueblo de pescadores"), "{ctx}");
    assert_eq!(json_of(&s.get("/api/context").1)["context"]["text"], "Un texto nuevo del juego");
}

#[test]
fn settle_on_examples_lists_what_is_missing() {
    // the example context has nothing pinned per layer yet: today it refuses
    let (d, s) = project();
    let before = d.read("context.toml");
    let (code, b) = s.post("/api/settle", "{}", true);
    assert_eq!(code, 400);
    assert_eq!(
        error_of(&b).as_deref(),
        Some("falta el párrafo de qué juego es; «Mundo» no tiene nada fijado; «Escala» no tiene nada fijado; «Juego» no tiene nada fijado; «Técnica» no tiene nada fijado")
    );
    assert_eq!(d.read("context.toml"), before);
}

#[test]
fn without_a_project_only_the_wizard_answers() {
    let d = empty_dir();
    let s = Server::start(&d);
    let (code, b) = s.get("/api/new");
    assert_eq!(code, 200);
    assert_eq!(json_of(&b)["step"], 1);
    assert_eq!(s.get("/").0, 200);
    let (code, b) = s.post("/api/settle", "{}", true);
    assert_eq!(code, 409);
    assert_eq!(error_of(&b).as_deref(), Some("todavia no hay proyecto: termina el primer paso"));
    assert_eq!(s.get("/api/catalog").0, 409);
    let (code, b) = s.post("/api/new", "{}", false);
    assert_eq!(code, 403);
    assert_eq!(error_of(&b).as_deref(), Some(FORBIDDEN));
    assert_eq!(s.get("/api/jobs/1").0, 404);
    assert!(!d.exists("kiln.toml"));
}
