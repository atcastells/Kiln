//! Measures of the meshes a recipe generated: triangle counts and how well the
//! faces follow the sphere the recipe started from.

use super::resolve::Resolved;
use super::vocab;
use crate::catalog::{Finding, Severity};
use crate::mesh::Mesh;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Measures {
    pub tris: u64,
    pub degenerate: u64,
    pub flipped: u64,
    /// Fraction of triangles whose normal is far from the centroid direction.
    pub steep: f64,
    pub size_m: [f64; 3],
}

type V = [f64; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `stretch` is the total stretch of the resolved recipe per axis (1.0 each when there is none).
pub fn measure(meshes: &[Mesh], stretch: [f64; 3]) -> Measures {
    let k = stretch.map(|s| if s.is_finite() && s != 0.0 { s } else { 1.0 });
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    let (mut tris, mut degenerate, mut flipped, mut steep) = (0u64, 0u64, 0u64, 0u64);
    for mesh in meshes {
        for p in &mesh.positions {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        let at = |i: u32| {
            let p = mesh.positions[i as usize];
            [0, 1, 2].map(|j| p[j] as f64 / k[j])
        };
        for t in mesh.indices.chunks_exact(3) {
            let (a, b, c) = (at(t[0]), at(t[1]), at(t[2]));
            tris += 1;
            let n = cross(sub(b, a), sub(c, a));
            let cen = [0, 1, 2].map(|j| (a[j] + b[j] + c[j]) / 3.0);
            let (ln, lc) = (dot(n, n).sqrt(), dot(cen, cen).sqrt());
            if ln < 1e-9 * lc * lc || lc == 0.0 {
                degenerate += 1;
                continue;
            }
            let d = dot(n, cen) / (ln * lc);
            if d < 0.0 {
                flipped += 1;
            }
            if d.abs() < 0.5 {
                steep += 1;
            }
        }
    }
    let size_m = if lo[0].is_finite() {
        [0, 1, 2].map(|i| hi[i] - lo[i])
    } else {
        [0.0; 3]
    };
    let steep = if tris == 0 { 0.0 } else { steep as f64 / tris as f64 };
    Measures { tris, degenerate, flipped, steep, size_m }
}

/// Product, per axis, of every `stretch` op of a resolved recipe.
pub fn total_stretch(resolved: &Resolved) -> [f64; 3] {
    let mut total = [1.0; 3];
    for op in resolved.ops.iter().filter(|o| o.op == "stretch") {
        for (i, axis) in ["x", "y", "z"].iter().enumerate() {
            total[i] *= op.params.get(*axis).copied().unwrap_or(1.0);
        }
    }
    total
}

/// The `segments` param of the `sphere` op of a resolved recipe (0 when absent).
pub fn segments_of(resolved: &Resolved) -> u64 {
    resolved
        .ops
        .iter()
        .find(|o| o.op == "sphere")
        .and_then(|o| o.params.get("segments"))
        .map_or(0, |s| *s as u64)
}

/// What a seed of a recipe must respect.
pub struct Limits {
    pub segments: u64,
    pub budget_tris: Option<u64>,
    pub max_tris: Option<u64>,
    pub steep_max: f64,
}

pub fn judge(asset_id: &str, seed: u32, m: &Measures, l: &Limits) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut add = |severity, code: &str, message: String| {
        out.push(Finding {
            severity,
            subject: format!("asset:{asset_id}"),
            code: code.to_string(),
            message: format!("semilla {seed}: {message}"),
        });
    };
    if m.flipped > 0 {
        add(
            Severity::Error,
            "generated_flipped",
            format!("{} caras miran hacia dentro: la malla se cruza consigo misma", m.flipped),
        );
    }
    let poles = 2 * l.segments;
    if m.degenerate > poles {
        add(
            Severity::Error,
            "generated_degenerate",
            format!(
                "{} triángulos degenerados, de los cuales {poles} son de los polos de la esfera",
                m.degenerate
            ),
        );
    }
    let expected = vocab::sphere_tris(l.segments);
    let tris_problem = if m.tris != expected {
        Some(format!("{} triángulos y la esfera de la receta tiene {expected}", m.tris))
    } else if let Some(b) = l.budget_tris.filter(|b| m.tris > *b) {
        Some(format!("{} triángulos superan el presupuesto de la ficha ({b})", m.tris))
    } else {
        l.max_tris
            .filter(|x| m.tris > *x)
            .map(|x| format!("{} triángulos superan el límite de la regla max_tris ({x})", m.tris))
    };
    if let Some(message) = tris_problem {
        add(Severity::Error, "generated_tris", message);
    }
    if m.steep > l.steep_max {
        add(
            Severity::Warn,
            "generated_steep",
            format!(
                "{:.1}% de las caras son empinadas, por encima del {:.1}% permitido",
                m.steep * 100.0,
                l.steep_max * 100.0
            ),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::resolve::ResolvedOp;
    use super::*;
    use std::collections::BTreeMap;

    fn tetra(reverse_first: bool) -> Mesh {
        let positions = vec![
            [1.0, 1.0, 1.0],
            [1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
        ];
        let mut indices = vec![1, 3, 2, 0, 2, 3, 0, 3, 1, 0, 1, 2];
        if reverse_first {
            indices.swap(1, 2);
        }
        Mesh { positions, indices }
    }

    fn octahedron(scale: [f32; 3]) -> Mesh {
        let base = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let positions = base.iter().map(|p| [0, 1, 2].map(|i| p[i] * scale[i])).collect();
        let mut indices = Vec::new();
        for sx in [0u32, 1] {
            for sy in [2u32, 3] {
                for sz in [4u32, 5] {
                    let odd = (sx + (sy - 2) + (sz - 4)) % 2 == 1;
                    if odd {
                        indices.extend([sx, sz, sy]);
                    } else {
                        indices.extend([sx, sy, sz]);
                    }
                }
            }
        }
        Mesh { positions, indices }
    }

    const ONE: [f64; 3] = [1.0; 3];

    #[test]
    fn a_clean_tetrahedron_has_nothing_wrong() {
        let m = measure(&[tetra(false)], ONE);
        assert_eq!((m.tris, m.degenerate, m.flipped, m.steep), (4, 0, 0, 0.0));
    }

    #[test]
    fn a_reversed_face_is_flipped() {
        let m = measure(&[tetra(true)], ONE);
        assert_eq!((m.tris, m.flipped), (4, 1));
    }

    #[test]
    fn a_collapsed_triangle_is_degenerate_only() {
        let mut t = tetra(false);
        t.indices.extend([0, 0, 1]);
        let m = measure(&[t], ONE);
        assert_eq!((m.tris, m.degenerate, m.flipped), (5, 1, 0));
    }

    #[test]
    fn unstretching_makes_a_stretched_octahedron_regular() {
        let o = octahedron([5.0, 0.2, 1.0]);
        let raw = measure(&[octahedron([5.0, 0.2, 1.0])], ONE);
        assert!(raw.steep > 0.4, "{}", raw.steep);
        let near = |s: [f64; 3]| s.iter().zip([10.0, 0.4, 2.0]).all(|(a, b)| (a - b).abs() < 1e-6);
        assert!(near(raw.size_m), "{:?}", raw.size_m);
        let m = measure(&[o], [5.0, 0.2, 1.0]);
        assert_eq!((m.tris, m.flipped, m.steep), (8, 0, 0.0));
        assert!(near(m.size_m), "{:?}", m.size_m);
    }

    #[test]
    fn a_spike_is_steep_not_flipped() {
        let mut t = tetra(false);
        t.positions[0] = [10.0, 10.0, 10.0];
        let m = measure(&[t], ONE);
        assert!(m.steep > 0.0, "{}", m.steep);
        assert_eq!(m.flipped, 0);
    }

    #[test]
    fn nothing_to_measure() {
        let m = measure(&[], ONE);
        assert_eq!((m.tris, m.steep, m.size_m), (0, 0.0, [0.0; 3]));
    }

    fn stretch(params: &[(&str, f64)]) -> ResolvedOp {
        ResolvedOp {
            op: "stretch".into(),
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            noise_seed: None,
        }
    }

    #[test]
    fn stretches_multiply_per_axis() {
        let resolved = Resolved {
            seed: 1,
            material: None,
            ops: vec![
                stretch(&[("x", 2.0)]),
                ResolvedOp { op: "facet".into(), params: BTreeMap::new(), noise_seed: None },
                stretch(&[("x", 1.5), ("y", 0.5)]),
            ],
        };
        assert_eq!(total_stretch(&resolved), [3.0, 0.5, 1.0]);
    }

    fn clean() -> Measures {
        Measures { tris: 4224, degenerate: 128, flipped: 0, steep: 0.01, size_m: [1.0; 3] }
    }

    fn limits() -> Limits {
        Limits { segments: 64, budget_tris: Some(6000), max_tris: Some(15000), steep_max: 0.10 }
    }

    fn codes(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|x| x.code.as_str()).collect()
    }

    #[test]
    fn a_clean_seed_has_no_findings() {
        assert!(judge("rock", 7, &clean(), &limits()).is_empty());
    }

    #[test]
    fn flipped_faces_are_an_error() {
        let m = Measures { flipped: 54, ..clean() };
        let f = judge("rock", 7, &m, &limits());
        assert_eq!(codes(&f), ["generated_flipped"]);
        assert_eq!(f[0].severity, Severity::Error);
        assert_eq!(f[0].subject, "asset:rock");
        assert!(f[0].message.contains("54"), "{}", f[0].message);
    }

    #[test]
    fn degenerate_triangles_beyond_the_poles_are_an_error() {
        let m = Measures { degenerate: 129, ..clean() };
        let f = judge("rock", 7, &m, &limits());
        assert_eq!(codes(&f), ["generated_degenerate"]);
        assert_eq!(f[0].severity, Severity::Error);
        assert!(f[0].message.contains("129") && f[0].message.contains("128"), "{}", f[0].message);
        let m = Measures { degenerate: 128, ..clean() };
        assert!(judge("rock", 7, &m, &limits()).is_empty());
    }

    #[test]
    fn a_triangle_count_that_is_not_the_sphere_is_an_error() {
        let m = Measures { tris: 4000, ..clean() };
        let f = judge("rock", 7, &m, &limits());
        assert_eq!(codes(&f), ["generated_tris"]);
        assert_eq!(f[0].severity, Severity::Error);
        assert!(f[0].message.contains("4000") && f[0].message.contains("4224"), "{}", f[0].message);
    }

    #[test]
    fn the_budget_and_the_rule_limit_the_triangles() {
        let over_budget = Limits { budget_tris: Some(4000), ..limits() };
        let f = judge("rock", 7, &clean(), &over_budget);
        assert_eq!(codes(&f), ["generated_tris"]);
        assert!(f[0].message.contains("presupuesto") && f[0].message.contains("4000"), "{}", f[0].message);
        let over_rule = Limits { budget_tris: None, max_tris: Some(4000), ..limits() };
        let f = judge("rock", 7, &clean(), &over_rule);
        assert_eq!(codes(&f), ["generated_tris"]);
        assert!(f[0].message.contains("regla") && f[0].message.contains("4000"), "{}", f[0].message);
        let none = Limits { budget_tris: None, max_tris: None, ..limits() };
        assert!(judge("rock", 7, &clean(), &none).is_empty());
    }

    #[test]
    fn steep_faces_are_only_a_warning() {
        let m = Measures { steep: 0.35, ..clean() };
        let f = judge("rock", 7, &m, &limits());
        assert_eq!(codes(&f), ["generated_steep"]);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("35.0%") && f[0].message.contains("10.0%"), "{}", f[0].message);
    }

    #[test]
    fn every_message_names_the_seed() {
        let m = Measures { tris: 4000, degenerate: 129, flipped: 54, steep: 0.35, size_m: [1.0; 3] };
        let f = judge("rock", 7, &m, &limits());
        assert_eq!(f.len(), 4);
        assert!(f.iter().all(|x| x.message.starts_with("semilla 7: ")));
    }

    #[test]
    fn segments_come_from_the_sphere_op() {
        let sphere = ResolvedOp {
            op: "sphere".into(),
            params: [("segments".to_string(), 64.0)].into(),
            noise_seed: None,
        };
        let r = Resolved { seed: 1, material: None, ops: vec![stretch(&[("x", 2.0)]), sphere] };
        assert_eq!(segments_of(&r), 64);
        let r = Resolved { seed: 1, material: None, ops: vec![] };
        assert_eq!(segments_of(&r), 0);
    }
}
