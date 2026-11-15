//! Reader for the RON subset the content files use. The `ron` crate loses struct
//! names, bare identifiers and field order, so this keeps them.

use super::value::Value;

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

const MAX_DEPTH: usize = 64;

type R<T> = Result<T, ParseError>;

pub fn parse(text: &str) -> Result<Value, ParseError> {
    let mut parser = Parser { s: text, pos: 0, depth: 0 };
    let value = parser.value()?;
    parser.skip()?;
    if parser.pos < text.len() {
        return Err(parser.err("contenido inesperado tras el valor"));
    }
    Ok(value)
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn err(&self, message: &str) -> ParseError {
        self.err_at(self.pos, message)
    }

    // 1-based line and column (in characters) of a byte offset on a char boundary.
    fn err_at(&self, pos: usize, message: &str) -> ParseError {
        let before = &self.s[..pos];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        ParseError { line, column, message: message.to_string() }
    }

    fn line(&self) -> usize {
        self.s[..self.pos].matches('\n').count() + 1
    }

    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    fn skip(&mut self) -> R<()> {
        loop {
            let rest = &self.s[self.pos..];
            if let Some(c) = rest.chars().next().filter(|c| c.is_whitespace()) {
                self.pos += c.len_utf8();
            } else if rest.starts_with("//") {
                self.pos += rest.find('\n').unwrap_or(rest.len());
            } else if rest.starts_with("/*") {
                let end = rest.find("*/").ok_or_else(|| self.err("comentario sin cerrar"))?;
                self.pos += end + 2;
            } else {
                return Ok(());
            }
        }
    }

    fn at(&mut self, c: char) -> R<bool> {
        self.skip()?;
        Ok(self.peek() == Some(c))
    }

    fn eat(&mut self, c: char) -> R<bool> {
        let hit = self.at(c)?;
        if hit {
            self.pos += c.len_utf8();
        }
        Ok(hit)
    }

    fn ident(&mut self) -> Option<String> {
        let rest = &self.s[self.pos..];
        let n = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
        if n == 0 || rest.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.pos += n;
        Some(rest[..n].to_string())
    }

    fn value(&mut self) -> R<Value> {
        self.skip()?;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.err("anidamiento excesivo"));
        }
        let start = self.pos;
        let line = self.line();
        let value = match self.peek() {
            None => Err(self.err("fin de fichero inesperado")),
            Some('"') => self.string().map(Value::Str),
            Some('[') => self.items(']').map(Value::List),
            Some('(') => self.group(None, line),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => self.number(),
            Some(_) => match self.ident() {
                Some(w) if w == "true" || w == "false" => Ok(Value::Bool(w == "true")),
                Some(w) if self.at('(')? => self.group(Some(w), line),
                Some(w) => Ok(Value::Ident(w)),
                None => Err(self.err_at(start, "carácter inesperado")),
            },
        };
        self.depth -= 1;
        value
    }

    // Consumes the opener, then `item (',' item)* ','?` up to the closer.
    fn items(&mut self, close: char) -> R<Vec<Value>> {
        self.pos += 1;
        let mut items = Vec::new();
        while !self.eat(close)? {
            items.push(self.value()?);
            if !self.eat(',')? && !self.at(close)? {
                return Err(self.err(&format!("se esperaba ',' o '{close}'")));
            }
        }
        Ok(items)
    }

    // `(` opens a struct when named or when it holds `ident :`; else a tuple.
    fn group(&mut self, name: Option<String>, line: usize) -> R<Value> {
        let save = self.pos;
        self.pos += 1;
        self.skip()?;
        let is_struct = name.is_some() || (self.ident().is_some() && self.at(':')?);
        self.pos = save + 1;
        if !is_struct {
            self.pos = save;
            return self.items(')').map(Value::Tuple);
        }
        let mut fields = Vec::new();
        while !self.eat(')')? {
            self.skip()?;
            let key = self.ident().ok_or_else(|| self.err("se esperaba un nombre de campo"))?;
            if !self.eat(':')? {
                return Err(self.err("se esperaba ':'"));
            }
            fields.push((key, self.value()?));
            if !self.eat(',')? && !self.at(')')? {
                return Err(self.err("se esperaba ',' o ')'"));
            }
        }
        Ok(Value::Struct { name, fields, line })
    }

    fn string(&mut self) -> R<String> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            let at = self.pos;
            self.pos += c.len_utf8();
            if c == '"' {
                return Ok(out);
            }
            if c != '\\' {
                out.push(c);
                continue;
            }
            let escaped = match self.peek() {
                Some(e @ ('"' | '\\')) => e,
                Some('n') => '\n',
                Some('t') => '\t',
                _ => return Err(self.err_at(at, "escape desconocido")),
            };
            out.push(escaped);
            self.pos += 1;
        }
        Err(self.err_at(start, "cadena sin cerrar"))
    }

    fn number(&mut self) -> R<Value> {
        let rest = &self.s[self.pos..];
        let n = rest.find(|c: char| !(c.is_ascii_alphanumeric() || "+-._".contains(c))).unwrap_or(rest.len());
        let tok = rest[..n].replace('_', "");
        let parsed = tok.parse().map(Value::Int).or_else(|_| match tok.parse::<f64>() {
            Ok(f) if f.is_finite() && tok.contains(|c: char| c.is_ascii_digit()) => Ok(Value::Float(f)),
            _ => Err(()),
        });
        match parsed {
            Ok(v) => {
                self.pos += n;
                Ok(v)
            }
            Err(()) => Err(self.err("número no válido")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: &str = include_str!("../../../../examples/content/data/content/world.ron");

    const LIBRARY: &str = r#"// Library
(
    modules: [
        (
            key: "access",
            kind: Access,
            size: (3, 2),
            connectors: [
                (name: "east", side: East, offset: 0),
                (name: "north", side: North, offset: 1),
            ],
            anchors: [
                (name: "start", cell: (0, 0)),
            ],
            scene: "res://modules/access.tscn",
        ),
        (
            key: "workshop",
            kind: Workshop,
            size: (3, 3),
            connectors: [
                (name: "west", side: West, offset: 1),
            ],
            anchors: [
                (name: "bench", cell: (0, 0)),
            ],
            scene: "res://modules/workshop.tscn",
        ),
    ],
)
"#;

    fn err(text: &str) -> (usize, usize, String) {
        let e = parse(text).expect_err("must fail");
        (e.line, e.column, e.message)
    }

    #[test]
    fn world_fixture_keeps_names_and_tuples() {
        let Ok(Value::List(items)) = parse(WORLD) else { panic!("not a list") };
        let names: Vec<_> = items
            .iter()
            .map(|v| match v {
                Value::Struct { name, .. } => name.clone().unwrap_or_default(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(names, ["Sector", "Sector", "Door", "Object", "Installation"]);
        let connects = items[2].field("connects");
        assert_eq!(
            connects,
            Some(&Value::Tuple(vec![
                Value::Str("sector:warehouse".into()),
                Value::Str("sector:yard".into())
            ]))
        );
    }

    #[test]
    fn library_keeps_order_kinds_tuples_and_lines() {
        let root = parse(LIBRARY).expect("library parses");
        let Value::Struct { name: None, fields, line: 2 } = &root else { panic!("root: {root:?}") };
        assert_eq!(fields[0].0, "modules");
        let Value::List(modules) = &fields[0].1 else { panic!("modules") };
        let line_of = |v: &Value| if let Value::Struct { line, .. } = v { *line } else { 0 };
        assert_eq!((line_of(&modules[0]), line_of(&modules[1])), (4, 17));
        let keys: Vec<_> = modules[0].fields_order();
        assert_eq!(keys, ["key", "kind", "size", "connectors", "anchors", "scene"]);
        assert_eq!(modules[0].field("kind"), Some(&Value::Ident("Access".into())));
        assert_eq!(modules[0].field("size"), Some(&Value::Tuple(vec![Value::Int(3), Value::Int(2)])));
        let Some(Value::List(conns)) = modules[0].field("connectors") else { panic!("conns") };
        assert_eq!(conns.len(), 2);
        assert!(matches!(&conns[1], Value::Struct { name: None, line: 10, .. }));
        assert_eq!(conns[1].field("side"), Some(&Value::Ident("North".into())));
    }

    impl Value {
        fn fields_order(&self) -> Vec<&str> {
            match self {
                Value::Struct { fields, .. } => fields.iter().map(|(k, _)| k.as_str()).collect(),
                _ => vec![],
            }
        }
    }

    #[test]
    fn syntax_errors_have_exact_positions() {
        let deep = "[".repeat(65);
        let cases: Vec<(&str, &str, (usize, usize))> = vec![
            ("missing colon", "(a: 1,\n  b 2)", (2, 5)),
            ("unexpected char", "[1,\n @]", (2, 2)),
            ("unclosed string", "[\n  \"abc", (2, 3)),
            ("unclosed comment", "[1] /* x", (1, 5)),
            ("missing paren", "(a: 1", (1, 6)),
            ("trailing content", "[1]\n2", (2, 1)),
            ("too deep", &deep, (1, 65)),
            ("char literal", "'a'", (1, 1)),
            ("map", "{}", (1, 1)),
            ("hex", "0x1F", (1, 1)),
            ("tuple struct", "Door(1)", (1, 6)),
        ];
        for (what, text, pos) in cases {
            let (line, column, message) = err(text);
            assert_eq!((line, column), pos, "{what}: {message}");
            assert!(!message.is_empty(), "{what}");
        }
    }

    #[test]
    fn depth_limit_is_inclusive() {
        let ok = format!("{}{}", "[".repeat(64), "]".repeat(64));
        assert!(parse(&ok).is_ok());
    }

    #[test]
    fn valid_edge_cases() {
        let s = |t: &str| Value::Str(t.into());
        let table: Vec<(&str, Value)> = vec![
            ("-5", Value::Int(-5)),
            ("2.5", Value::Float(2.5)),
            ("-0.5", Value::Float(-0.5)),
            ("1e3", Value::Float(1000.0)),
            ("true", Value::Bool(true)),
            ("false // c", Value::Bool(false)),
            ("/* c */ Access", Value::Ident("Access".into())),
            (r#""a\"b\\c\nd\te""#, s("a\"b\\c\nd\te")),
            (r#""ñandú""#, s("ñandú")),
            ("[]", Value::List(vec![])),
            ("()", Value::Tuple(vec![])),
            ("[1, 2,]", Value::List(vec![Value::Int(1), Value::Int(2)])),
            ("(1, 2,)", Value::Tuple(vec![Value::Int(1), Value::Int(2)])),
        ];
        for (text, want) in table {
            assert_eq!(parse(text), Ok(want), "{text}");
        }
        assert_eq!(
            parse("Unit()"),
            Ok(Value::Struct { name: Some("Unit".into()), fields: vec![], line: 1 })
        );
        assert_eq!(
            parse("(a: 1,)"),
            Ok(Value::Struct { name: None, fields: vec![("a".into(), Value::Int(1))], line: 1 })
        );
    }

    #[test]
    fn never_panics() {
        for (i, _) in WORLD.char_indices().chain([(WORLD.len(), ' ')]) {
            let _ = parse(&WORLD[..i]);
        }
        for (i, _) in LIBRARY.char_indices() {
            let _ = parse(&LIBRARY[..i]);
        }
        for text in [
            "é", "\"é", "\"\\é", "[é", "/*é", "/é", "(é: 1)", "日本語(", "-é", "0x1F", "'a'", "{}", "r\"x\"",
            "é(a: 1)", "(a: é)", "\u{0}", "\r\n", "1__", "-", "+", "--1", "9999999999999999999999", "Name(", "[,]",
        ] {
            let _ = parse(text);
        }
    }
}
