//! Checks of a recipe against the vocabulary.

use super::vocab::{self, Param};
use super::{Generator, Value};
use crate::catalog::{Finding, Severity};

fn whole(v: f64) -> bool {
    v.fract() == 0.0
}

/// Problem with a value against its parameter, if any.
fn range_problem(p: &Param, v: &Value) -> Option<String> {
    let ends: Vec<f64> = match v {
        Value::Fixed(x) => vec![*x],
        Value::Range(r) => r.to_vec(),
    };
    if ends.iter().any(|x| !x.is_finite()) {
        return Some("no es un número finito".into());
    }
    if p.integer && ends.iter().any(|x| !whole(*x)) {
        return Some("debe ser un número entero".into());
    }
    if let Value::Range([a, b]) = v
        && a > b
    {
        return Some(format!("el rango [{a}, {b}] tiene el mínimo por encima del máximo"));
    }
    if ends.iter().any(|x| *x < p.min || *x > p.max) {
        return Some(format!("queda fuera de {} a {}", p.min, p.max));
    }
    None
}

pub fn check(
    asset_id: &str,
    g: &Generator,
    budget_tris: Option<u64>,
    palette: &[&str],
) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut push = |code: &str, message: String| {
        out.push(Finding {
            severity: Severity::Error,
            subject: format!("asset:{asset_id}"),
            code: code.into(),
            message,
        })
    };
    if g.ops.first().is_none_or(|o| o.op != "sphere") {
        push("generator_order", "la receta debe empezar por una esfera".into());
    }
    let mut segments_ok = true;
    let mut worst_segments = None;
    for (i, o) in g.ops.iter().enumerate() {
        if o.op == "sphere" && i > 0 {
            push("generator_order", "la esfera solo puede ir al principio de la receta".into());
        }
        let Some(def) = vocab::op(&o.op) else {
            push("generator_op", format!("la operación «{}» no existe", o.op));
            continue;
        };
        if !def.repeatable && o.op != "sphere" && g.ops[..i].iter().any(|p| p.op == o.op) {
            push("generator_op", format!("la operación «{}» no se puede repetir", o.op));
        }
        for (name, v) in &o.params {
            let Some(p) = def.param(name) else {
                push("generator_param", format!("«{}» no tiene el parámetro «{name}»", o.op));
                continue;
            };
            if let Some(why) = range_problem(p, v) {
                push("generator_range", format!("{}.{name}: {why}", o.op));
                if o.op == "sphere" && name == "segments" {
                    segments_ok = false;
                }
            }
        }
        if o.op == "sphere" && i == 0 {
            worst_segments = Some(match o.params.get("segments") {
                Some(Value::Fixed(x)) => *x,
                Some(Value::Range([a, b])) => a.max(*b),
                None => vocab::op("sphere")
                    .and_then(|d| d.param("segments"))
                    .map_or(0.0, |p| p.default),
            });
        }
    }
    if let (Some(budget), Some(s), true) = (budget_tris, worst_segments, segments_ok)
        && vocab::sphere_tris(s as u64) > budget
    {
        let tris = vocab::sphere_tris(s as u64);
        push(
            "generator_budget",
            format!("con {s} segmentos la esfera llega a {tris} triángulos, por encima de {budget}"),
        );
    }
    if let Some(m) = &g.material
        && !palette.contains(&m.as_str())
    {
        push("generator_material", format!("el material «{m}» no está en la paleta"));
        if let Some(f) = out.last_mut() {
            f.severity = Severity::Warn;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec;

    const PALETTE: &[&str] = &["basalt"];

    fn recipe(ops: &str, material: &str) -> Generator {
        let text = format!(
            "[asset]\nid = \"a\"\nname = \"A\"\n[generator]\n{material}\n{ops}"
        );
        spec::parse(&text).unwrap().generator.unwrap()
    }

    fn codes(ops: &str, budget: Option<u64>) -> Vec<String> {
        let g = recipe(ops, "material = \"basalt\"");
        check("a", &g, budget, PALETTE).into_iter().map(|f| f.code).collect()
    }

    fn sphere(params: &str) -> String {
        format!("[[generator.ops]]\nop = \"sphere\"\n{params}\n")
    }

    #[test]
    fn the_example_recipe_has_no_findings() {
        let ops = format!(
            "{}[[generator.ops]]\nop = \"stretch\"\nx = [1.0, 1.8]\n\
             [[generator.ops]]\nop = \"noise\"\namplitude = [0.2, 0.5]\nsize = 0.6\n\
             [[generator.ops]]\nop = \"noise\"\namplitude = [0.05, 0.15]\nsize = 0.1\n\
             [[generator.ops]]\nop = \"facet\"\n",
            sphere("radius_m = [8, 40]\nsegments = 64")
        );
        let g = recipe(&ops, "material = \"basalt\"");
        assert!(check("a", &g, Some(6000), PALETTE).is_empty());
    }

    #[test]
    fn findings_name_the_asset() {
        let f = check("rock", &recipe("", ""), None, PALETTE);
        assert_eq!(f[0].subject, "asset:rock");
    }

    #[test]
    fn order_needs_a_first_sphere_and_no_other() {
        assert_eq!(codes("", None), ["generator_order"]);
        assert_eq!(codes("[[generator.ops]]\nop = \"facet\"\n", None), ["generator_order"]);
        let twice = format!("{}{}", sphere(""), sphere(""));
        assert_eq!(codes(&twice, None), ["generator_order"]);
    }

    #[test]
    fn unknown_or_repeated_op() {
        let ops = format!("{}[[generator.ops]]\nop = \"crater\"\n", sphere(""));
        assert_eq!(codes(&ops, None), ["generator_op"]);
        let ops = format!("{}[[generator.ops]]\nop = \"facet\"\n[[generator.ops]]\nop = \"facet\"\n", sphere(""));
        assert_eq!(codes(&ops, None), ["generator_op"]);
    }

    #[test]
    fn unknown_param() {
        assert_eq!(codes(&sphere("depth = 1"), None), ["generator_param"]);
    }

    #[test]
    fn range_problems() {
        assert_eq!(codes(&sphere("segments = 4"), None), ["generator_range"]);
        assert_eq!(codes(&sphere("segments = 12.5"), None), ["generator_range"]);
        assert_eq!(codes(&sphere("segments = [8, 12.5]"), None), ["generator_range"]);
        assert_eq!(codes(&sphere("radius_m = [40, 8]"), None), ["generator_range"]);
        assert_eq!(codes(&sphere("radius_m = [8, 20000]"), None), ["generator_range"]);
        assert_eq!(codes(&sphere("radius_m = nan"), None), ["generator_range"]);
    }

    #[test]
    fn budget_uses_the_worst_case() {
        assert_eq!(codes(&sphere("segments = 64"), Some(1000)), ["generator_budget"]);
        assert_eq!(codes(&sphere("segments = [8, 64]"), Some(1000)), ["generator_budget"]);
        assert!(codes(&sphere("segments = [8, 64]"), Some(4224)).is_empty());
        assert_eq!(codes(&sphere(""), Some(100)), ["generator_budget"]);
        assert_eq!(codes(&sphere("segments = 4"), Some(10)), ["generator_range"]);
    }

    #[test]
    fn material_must_be_in_the_palette() {
        let g = recipe(&sphere(""), "material = \"gold\"");
        let found = check("a", &g, None, PALETTE);
        let c: Vec<&str> = found.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(c, ["generator_material"]);
        // like a part's material outside the palette: it warns, it does not block
        assert_eq!(found[0].severity, Severity::Warn);
        assert!(check("a", &recipe(&sphere(""), ""), None, PALETTE).is_empty());
    }

    #[test]
    fn spec_check_runs_the_recipe_checks() {
        let text = format!(
            "[asset]\nid = \"a\"\nname = \"A\"\n[generator]\n{}",
            sphere("depth = 1")
        );
        let mut cat = crate::Catalog::default();
        spec::check(&spec::parse(&text).unwrap(), &Default::default(), &mut cat);
        assert!(cat.findings.iter().any(|f| f.code == "generator_param"));
    }
}
