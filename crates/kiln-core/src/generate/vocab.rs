//! The closed vocabulary of recipe operations: the one table that checks, the
//! page controls, the agent's text and the Godot script read or mirror.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Param {
    pub id: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub integer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct OpDef {
    pub id: &'static str,
    pub label: &'static str,
    pub params: &'static [Param],
    pub repeatable: bool,
}

const fn axis(id: &'static str, label: &'static str) -> Param {
    Param { id, label, unit: "", min: 0.2, max: 5.0, default: 1.0, integer: false }
}

pub const OPS: &[OpDef] = &[
    OpDef {
        id: "sphere",
        label: "Esfera",
        params: &[
            Param { id: "radius_m", label: "Radio", unit: "m", min: 0.1, max: 10000.0, default: 10.0, integer: false },
            Param { id: "segments", label: "Segmentos", unit: "", min: 8.0, max: 256.0, default: 32.0, integer: true },
        ],
        repeatable: false,
    },
    OpDef {
        id: "stretch",
        label: "Estirar",
        params: &[axis("x", "Eje X"), axis("y", "Eje Y"), axis("z", "Eje Z")],
        repeatable: false,
    },
    OpDef {
        id: "noise",
        label: "Ruido",
        params: &[
            Param { id: "amplitude", label: "Amplitud", unit: "", min: 0.0, max: 1.0, default: 0.2, integer: false },
            Param { id: "size", label: "Tamaño", unit: "", min: 0.01, max: 4.0, default: 0.5, integer: false },
        ],
        repeatable: true,
    },
    OpDef { id: "facet", label: "Facetado", params: &[], repeatable: false },
];

pub fn op(id: &str) -> Option<&'static OpDef> {
    OPS.iter().find(|o| o.id == id)
}

impl OpDef {
    pub fn param(&self, id: &str) -> Option<&'static Param> {
        self.params.iter().find(|p| p.id == id)
    }
}

pub fn sphere_tris(segments: u64) -> u64 {
    2 * segments * (segments / 2 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(op: &str, param: &str) -> (f64, f64, f64, bool) {
        let p = super::op(op).unwrap().param(param).unwrap();
        (p.min, p.max, p.default, p.integer)
    }

    #[test]
    fn the_four_ops_exist_in_order() {
        let ids: Vec<&str> = OPS.iter().map(|o| o.id).collect();
        assert_eq!(ids, ["sphere", "stretch", "noise", "facet"]);
        assert_eq!(limits("sphere", "radius_m"), (0.1, 10000.0, 10.0, false));
        assert_eq!(op("sphere").unwrap().param("radius_m").unwrap().unit, "m");
        assert_eq!(limits("sphere", "segments"), (8.0, 256.0, 32.0, true));
        for axis in ["x", "y", "z"] {
            assert_eq!(limits("stretch", axis), (0.2, 5.0, 1.0, false));
        }
        assert_eq!(limits("noise", "amplitude"), (0.0, 1.0, 0.2, false));
        assert_eq!(limits("noise", "size"), (0.01, 4.0, 0.5, false));
        assert!(op("facet").unwrap().params.is_empty());
        assert!(op("sphere").unwrap().param("nope").is_none());
    }

    #[test]
    fn unknown_op_is_none() {
        assert!(op("crater").is_none());
    }

    #[test]
    fn only_noise_is_repeatable() {
        let rep: Vec<&str> = OPS.iter().filter(|o| o.repeatable).map(|o| o.id).collect();
        assert_eq!(rep, ["noise"]);
    }

    #[test]
    fn sphere_triangles() {
        assert_eq!(sphere_tris(64), 4224);
    }

    #[test]
    fn table_serializes_as_an_array() {
        let v = serde_json::to_value(OPS).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 4);
        assert_eq!(v[0]["id"], "sphere");
    }
}
