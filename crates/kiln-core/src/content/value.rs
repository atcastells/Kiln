//! The parsed form of a RON file: structs keep their name, fields keep their order.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Struct { name: Option<String>, fields: Vec<(String, Value)>, line: usize },
}

impl Value {
    pub fn field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Struct { fields, .. } => fields.iter().find(|(key, _)| key == name).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Same words as the `type` of a schema field.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Str(_) => "string",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Ident(_) => "ident",
            Value::List(_) => "list",
            Value::Tuple(_) => "tuple",
            Value::Struct { .. } => "struct",
        }
    }

    /// A single value, a tuple or a list as its items.
    pub fn scalars(&self) -> Vec<&Value> {
        match self {
            Value::List(items) | Value::Tuple(items) => items.iter().collect(),
            other => vec![other],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_cover_every_variant() {
        let s = Value::Struct {
            name: None,
            fields: vec![("a".into(), Value::Str("x".into())), ("b".into(), Value::Int(1))],
            line: 1,
        };
        assert_eq!(s.field("a").and_then(Value::as_str), Some("x"));
        assert_eq!(s.field("zz"), None);
        assert_eq!(Value::Int(1).field("a"), None);
        assert_eq!(Value::Int(1).as_str(), None);
        let names: Vec<_> = [
            Value::Str(String::new()),
            Value::Int(0),
            Value::Float(0.0),
            Value::Bool(true),
            Value::Ident(String::new()),
            Value::List(vec![]),
            Value::Tuple(vec![]),
            s,
        ]
        .iter()
        .map(Value::type_name)
        .collect();
        assert_eq!(names, ["string", "int", "float", "bool", "ident", "list", "tuple", "struct"]);
    }

    #[test]
    fn scalars_flattens_one_level() {
        let one = Value::Str("a".into());
        assert_eq!(one.scalars(), vec![&one]);
        let tuple = Value::Tuple(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(tuple.scalars().len(), 2);
        assert_eq!(Value::List(vec![]).scalars().len(), 0);
    }
}
