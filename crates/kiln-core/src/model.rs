//! Modelling vocabulary of a `model` part: the closed set of forms, their
//! limits, triangle cost and bounds. One table, like `generate/vocab.rs`.

use serde::{Deserialize, Serialize};

pub type V3 = [f64; 3];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawShape")]
pub struct Shape {
    pub zone: String,
    #[serde(flatten)]
    pub form: Form,
}

/// What the spec says, before checking that it names exactly one form and
/// nothing else (`flatten` alone would let unknown keys through).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShape {
    zone: String,
    #[serde(rename = "box", default)]
    cuboid: Option<[V3; 2]>,
    #[serde(default)]
    para: Option<Para>,
    #[serde(default)]
    tube: Option<Tube>,
}

impl TryFrom<RawShape> for Shape {
    type Error = String;
    fn try_from(r: RawShape) -> Result<Self, String> {
        let form = match (r.cuboid, r.para, r.tube) {
            (Some(b), None, None) => Form::Box(b),
            (None, Some(p), None) => Form::Para(p),
            (None, None, Some(t)) => Form::Tube(t),
            _ => return Err(format!("la forma de la zona «{}» debe llevar una sola de `box`, `para` o `tube`", r.zone)),
        };
        Ok(Shape { zone: r.zone, form })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    /// Two opposite corners, any order.
    Box([V3; 2]),
    /// A corner and three edge vectors.
    Para(Para),
    Tube(Tube),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Para {
    pub origin: V3,
    pub u: V3,
    pub v: V3,
    pub w: V3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tube {
    pub from: V3,
    pub to: V3,
    pub radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_to: Option<f64>,
    pub segments: u32,
}

pub const MAX_SHAPES: usize = 256;
/// Blender cuts long material names (63 bytes before 4.3), and the material
/// is named `part/zone`: a short zone keeps it whole.
pub const MAX_ZONE: usize = 32;
const MAX_COORD: f64 = 100.0;
const MAX_RADIUS: f64 = 50.0;
const MIN_SIDE: f64 = 0.001;

/// Triangles the shapes cost: box and para 12, tube `4*segments - 4`.
/// Triangles one modelled part may bring with all its copies.
pub const MAX_PART_TRIS: u64 = 200_000;

/// The copies of a part multiply its shapes, and each copy is built and
/// written to the plan: past `MAX_PART_TRIS` the part is refused.
pub fn copies_problem(shapes: &[Shape], count: u32) -> Option<String> {
    let total = tris(shapes).saturating_mul(count as u64);
    (total > MAX_PART_TRIS).then(|| {
        format!("{count} copias de estas formas suman {total} triángulos; el tope de una pieza modelada es {MAX_PART_TRIS}")
    })
}

pub fn tris(shapes: &[Shape]) -> u64 {
    shapes
        .iter()
        .map(|s| match &s.form {
            Form::Box(_) | Form::Para(_) => 12,
            Form::Tube(t) => (4 * t.segments as u64).saturating_sub(4),
        })
        .sum()
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn len(a: V3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn det(u: V3, v: V3, w: V3) -> f64 {
    u[0] * (v[1] * w[2] - v[2] * w[1]) - u[1] * (v[0] * w[2] - v[2] * w[0])
        + u[2] * (v[0] * w[1] - v[1] * w[0])
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: V3) -> V3 {
    let l = len(a);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// The rim vertices of a tube, end by end: `segments` points per end, at
/// angle `2πk/segments` in the frame `e1 = unit(d × a)`, `e2 = d × e1`,
/// where `d` is the unit axis and `a` the world axis least aligned with it
/// (lowest index on ties). `assemble.py` builds the same points, so the
/// bounds here are those of the mesh.
pub fn tube_rims(t: &Tube) -> [Vec<V3>; 2] {
    let d = unit(sub(t.to, t.from));
    let k = (0..3).min_by(|&i, &j| d[i].abs().total_cmp(&d[j].abs())).unwrap_or(0);
    let mut a = [0.0; 3];
    a[k] = 1.0;
    let e1 = unit(cross(d, a));
    let e2 = cross(d, e1);
    let n = t.segments.max(1);
    let rim = |c: V3, r: f64| -> Vec<V3> {
        (0..n)
            .map(|i| {
                let th = std::f64::consts::TAU * i as f64 / n as f64;
                let (s, co) = th.sin_cos();
                [0, 1, 2].map(|j| c[j] + r * (co * e1[j] + s * e2[j]))
            })
            .collect()
    };
    [rim(t.from, t.radius), rim(t.to, t.radius_to.unwrap_or(t.radius))]
}

/// Axis-aligned min/max over every vertex of the mesh the shapes make.
pub fn bounds(shapes: &[Shape]) -> Option<(V3, V3)> {
    if !problems(shapes).is_empty() {
        return None;
    }
    let mut pts: Vec<V3> = Vec::new();
    for s in shapes {
        match &s.form {
            Form::Box([a, b]) => {
                pts.push(*a);
                pts.push(*b);
            }
            Form::Para(p) => {
                for m in 0..8u8 {
                    let mut c = p.origin;
                    for (bit, e) in [p.u, p.v, p.w].iter().enumerate() {
                        if m >> bit & 1 == 1 {
                            for k in 0..3 {
                                c[k] += e[k];
                            }
                        }
                    }
                    pts.push(c);
                }
            }
            Form::Tube(t) if len(sub(t.to, t.from)) > 0.0 => {
                pts.extend(tube_rims(t).into_iter().flatten());
            }
            Form::Tube(t) => pts.push(t.from),
        }
    }
    let first = *pts.first()?;
    let (mut lo, mut hi) = (first, first);
    for p in &pts {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    Some((lo, hi))
}

fn coords_ok(vs: &[V3]) -> bool {
    vs.iter().flatten().all(|c| c.is_finite() && c.abs() <= MAX_COORD)
}

fn radius_ok(r: f64) -> bool {
    r > 0.0 && r <= MAX_RADIUS
}

/// One Spanish message per broken shape, prefixed with its index.
pub fn problems(shapes: &[Shape]) -> Vec<String> {
    let mut out = Vec::new();
    if shapes.is_empty() || shapes.len() > MAX_SHAPES {
        out.push(format!(
            "la pieza lleva {} formas; debe llevar de 1 a {MAX_SHAPES}",
            shapes.len()
        ));
    }
    for (i, s) in shapes.iter().enumerate() {
        let msg = if !crate::spec::plain_id(&s.zone) {
            Some("zona vacía o con caracteres no permitidos (solo letras, cifras, `_` y `-`)".to_string())
        } else if s.zone.chars().count() > MAX_ZONE {
            Some(format!("zona de más de {MAX_ZONE} caracteres"))
        } else {
            match &s.form {
                Form::Box([a, b]) => {
                    if !coords_ok(&[*a, *b]) {
                        Some("coordenadas no finitas o fuera de ±100 m".into())
                    } else if (0..3).any(|k| (a[k] - b[k]).abs() < MIN_SIDE) {
                        Some("`box` sin volumen: algún lado mide menos de 1 mm".into())
                    } else {
                        None
                    }
                }
                Form::Para(p) => {
                    if !coords_ok(&[p.origin, p.u, p.v, p.w]) {
                        Some("coordenadas no finitas o fuera de ±100 m".into())
                    } else if det(p.u, p.v, p.w).abs() < 1e-9 {
                        Some("`para` sin volumen: u, v y w son casi coplanares".into())
                    } else {
                        None
                    }
                }
                Form::Tube(t) => {
                    if !coords_ok(&[t.from, t.to]) {
                        Some("coordenadas no finitas o fuera de ±100 m".into())
                    } else if !radius_ok(t.radius) || !t.radius_to.is_none_or(radius_ok) {
                        Some("`radius` y `radius_to` deben estar en (0, 50] m".into())
                    } else if !(3..=32).contains(&t.segments) {
                        Some("`segments` debe estar entre 3 y 32".into())
                    } else if len(sub(t.to, t.from)) < MIN_SIDE {
                        Some("`tube` sin longitud: from y to distan menos de 1 mm".into())
                    } else {
                        None
                    }
                }
            }
        };
        if let Some(m) = msg {
            out.push(format!("forma {i}: {m}"));
        }
    }
    out
}

/// Text for the agent describing the forms; appended to prompts under its
/// own heading.
pub const VOCABULARY: &str = "\
Una pieza `source = { kind = \"model\" }` se dibuja con `[[parts.shapes]]`: de 1 a 256 formas, \
cada una con `zone` (nombre de zona con material en `materials` o `material`; solo letras, \
cifras, `_` y `-`, hasta 32 caracteres) y exactamente una de estas claves.
- `box = [[x0,y0,z0],[x1,y1,z1]]`: dos esquinas opuestas, en cualquier orden. 12 triángulos.
- `para = { origin = [..], u = [..], v = [..], w = [..] }`: una esquina y tres vectores de arista \
(paralelepípedo). 12 triángulos.
- `tube = { from = [..], to = [..], radius = r, radius_to = r2, segments = n }`: cilindro o cono \
con tapas; `radius_to` es opcional (cono) y `segments` va de 3 a 32. `4*segments - 4` triángulos.
Unidades: metros, ejes de glTF (Y arriba, la cara de la pieza mira a +Z). Coordenadas finitas \
con |c| <= 100; `radius` y `radius_to` en (0, 50]. Se rechazan formas sin volumen: lado de `box` \
o longitud de `tube` menor de 1 mm, `para` con |det(u,v,w)| < 1e-9. Las copias (`count`) multiplican \
los triángulos: una pieza modelada con todas sus copias no pasa de 200000.";

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Wrap {
        shapes: Vec<Shape>,
    }

    fn tube(from: V3, to: V3, radius: f64, segments: u32) -> Form {
        Form::Tube(Tube { from, to, radius, radius_to: None, segments })
    }

    fn sh(form: Form) -> Shape {
        Shape { zone: "Top".into(), form }
    }

    #[test]
    fn parses_the_three_forms_from_toml() {
        let w: Wrap = toml::from_str(
            r#"
[[shapes]]
zone = "Top"
box = [[-0.60, 0.86, -0.42], [0.60, 0.92, 0.42]]        # dos esquinas opuestas

[[shapes]]
zone = "Leg"
para = { origin = [0.52, 0, 0.34], u = [0.06, 0, 0], v = [0, 0.86, 0], w = [0, 0, 0.06] }

[[shapes]]
zone = "Leg"
tube = { from = [0, 0, 0], to = [0, 0.8, 0], radius = 0.03, segments = 6 }
"#,
        )
        .unwrap();
        assert!(matches!(w.shapes[0].form, Form::Box(_)));
        assert!(matches!(w.shapes[1].form, Form::Para(_)));
        assert!(matches!(&w.shapes[2].form, Form::Tube(t) if t.segments == 6 && t.radius_to.is_none()));
        assert_eq!(w.shapes[2].zone, "Leg");
        for bad in [
            "zone = \"A\"\nbox = [[0,0,0],[1,1,1]]\ntube = { from = [0,0,0], to = [0,1,0], radius = 0.1, segments = 6 }",
            "zone = \"A\"\nbox = [[0,0,0],[1,1,1]]\nbevel = 0.01",
            "zone = \"A\"",
        ] {
            let r: Result<Wrap, _> = toml::from_str(&format!("[[shapes]]\n{bad}\n"));
            assert!(r.is_err(), "{bad}");
        }
    }

    #[test]
    fn problems_table() {
        let b = |a: V3, c: V3| sh(Form::Box([a, c]));
        let good = vec![
            b([0.0; 3], [1.0; 3]),
            sh(tube([0.0; 3], [0.0, 1.0, 0.0], 0.1, 6)),
        ];
        assert!(problems(&good).is_empty());
        let flat = Para { origin: [0.0; 3], u: [1.0, 0.0, 0.0], v: [0.0, 1.0, 0.0], w: [1.0, 1.0, 0.0] };
        let cases = vec![
            b([0.0; 3], [1.0, 1.0, 0.0005]),
            b([0.0; 3], [f64::NAN, 1.0, 1.0]),
            b([0.0; 3], [101.0, 1.0, 1.0]),
            sh(Form::Para(flat)),
            sh(tube([0.0; 3], [0.0, 1.0, 0.0], 0.1, 2)),
            sh(tube([0.0; 3], [0.0, 1.0, 0.0], 0.1, 33)),
            sh(tube([0.0; 3], [0.0, 1.0, 0.0], 0.0, 6)),
            sh(tube([0.0; 3], [0.0, 1.0, 0.0], 51.0, 6)),
            sh(tube([0.0; 3], [0.0, 0.0005, 0.0], 0.1, 6)),
            Shape { zone: String::new(), form: Form::Box([[0.0; 3], [1.0; 3]]) },
            Shape { zone: "a b".into(), form: Form::Box([[0.0; 3], [1.0; 3]]) },
            Shape { zone: "z".repeat(MAX_ZONE + 1), form: Form::Box([[0.0; 3], [1.0; 3]]) },
        ];
        for c in cases {
            let p = problems(&[good[0].clone(), c.clone()]);
            assert!(p.len() == 1 && p[0].starts_with("forma 1:"), "{c:?} -> {p:?}");
        }
        assert!(!problems(&[]).is_empty());
        assert!(!problems(&vec![good[0].clone(); 257]).is_empty());
    }

    #[test]
    fn tris_and_bounds() {
        let shapes = vec![
            sh(Form::Box([[1.0, 0.0, 0.0], [0.0, 2.0, 3.0]])),
            sh(Form::Para(Para { origin: [0.0; 3], u: [1.0, 0.0, 0.0], v: [0.0, 5.0, 0.0], w: [0.0, 0.0, 1.0] })),
            sh(tube([0.0; 3], [0.0, 0.8, 0.0], 0.03, 6)),
        ];
        assert_eq!(tris(&shapes), 12 + 12 + 20);
        // A six-sided tube along Y: its rim reaches ±r on Z and ±r·sin 60° on X.
        let rx = 0.03 * (std::f64::consts::PI / 3.0).sin();
        let (lo, hi) = bounds(&shapes).unwrap();
        let near = |a: V3, b: V3| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-12);
        assert!(near(lo, [-rx, 0.0, -0.03]) && near(hi, [1.0, 5.0, 3.0]), "{lo:?} {hi:?}");
        let (lo, hi) = bounds(&shapes[2..]).unwrap();
        assert!(near(lo, [-rx, 0.0, -0.03]) && near(hi, [rx, 0.8, 0.03]), "{lo:?} {hi:?}");
        assert!(bounds(&[]).is_none());
        // Shapes outside the limits are never measured: a huge `segments`
        // would allocate its rims before anyone checks them.
        assert!(bounds(&[sh(tube([0.0; 3], [0.0, 1.0, 0.0], 0.1, 33))]).is_none());
        // Copies multiply the shapes: their triangles have a hard cap.
        let boxes = vec![sh(Form::Box([[0.0; 3], [1.0; 3]])); 256];
        assert!(copies_problem(&boxes, 65).is_none());
        assert!(copies_problem(&boxes, 66).is_some());
        assert!(copies_problem(&boxes, u32::MAX).is_some());
    }
}
