use kiln_core::canonical::{build, validate};
fn sphere(segments: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let rings = segments / 2;
    let mut pos = Vec::new();
    for r in 0..=rings + 1 {
        let theta = std::f64::consts::PI * r as f64 / (rings + 1) as f64;
        let ring = if r == 0 || r == rings + 1 { 0.0 } else { theta.sin() };
        for s in 0..=segments {
            let phi = std::f64::consts::TAU * s as f64 / segments as f64;
            pos.push([(ring * phi.cos()) as f32, theta.cos() as f32, (ring * phi.sin()) as f32]);
        }
    }
    let mut idx = Vec::new();
    for r in 0..=rings {
        for s in 0..segments {
            let a = r * (segments + 1) + s;
            let c = a + segments + 1;
            idx.extend([a, a + 1, c, a + 1, c + 1, c]);
        }
    }
    (pos, idx)
}
#[test]
fn odd() {
    for s in [8u32, 9, 10, 11, 15] {
        let (p, i) = sphere(s);
        match build(&p, &i, s) {
            Ok(g) => eprintln!("RESULT S={s}: ok V={} T={} validate={:?}", g.vertices.len(), g.triangles.len(), validate(&g).map_err(|r| r.code)),
            Err(r) => eprintln!("RESULT S={s}: {} {}", r.code, r.detail),
        }
    }
}
