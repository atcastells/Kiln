//! From the GLB arrays (`f32` positions, `u32` indices) to a canonical
//! geometry: quantize, join equal integer points, drop triangles that repeat
//! a vertex and check the sphere counts (S1 §2.2, §5.5). No winding flip
//! (decision 001: the GLB already faces outwards).

use super::{Geometry, Rejection};
use std::collections::HashMap;

/// `round(f64(x) × 1000)`, halves away from zero.
pub fn quantize(x: f32) -> Result<i32, Rejection> {
    let q = (x as f64 * 1000.0).round();
    if q.is_finite() && q >= i32::MIN as f64 && q <= i32::MAX as f64 {
        Ok(q as i32)
    } else {
        Err(Rejection::new("canonical_quantize", None, format!("{x} m does not fit an i32 of quanta")))
    }
}

/// Canonical geometry of a sphere-vocabulary mesh with `segments` segments.
/// Vertices are numbered by first appearance in `positions`; triangles keep
/// the index order and their own vertex order.
pub fn build(positions: &[[f32; 3]], indices: &[u32], segments: u32) -> Result<Geometry, Rejection> {
    let mut ids: HashMap<[i32; 3], u32> = HashMap::new();
    let mut vertices = Vec::new();
    let mut remap = Vec::with_capacity(positions.len());
    for (i, p) in positions.iter().enumerate() {
        let mut q = [0; 3];
        for (k, c) in p.iter().enumerate() {
            q[k] = quantize(*c).map_err(|r| Rejection { element: Some(i), ..r })?;
        }
        let id = *ids.entry(q).or_insert_with(|| {
            vertices.push(q);
            (vertices.len() - 1) as u32
        });
        remap.push(id);
    }
    if !indices.len().is_multiple_of(3) {
        let detail = format!("{} indices is not a whole number of triangles", indices.len());
        return Err(Rejection::new("canonical_count", None, detail));
    }
    let mut triangles = Vec::with_capacity(indices.len() / 3);
    for (t, tri) in indices.chunks_exact(3).enumerate() {
        let mut v = [0; 3];
        for (k, i) in tri.iter().enumerate() {
            v[k] = *remap.get(*i as usize).ok_or_else(|| {
                Rejection::new("I2_index_out_of_range", Some(t), format!("index {i} of {} positions", remap.len()))
            })?;
        }
        if v[0] != v[1] && v[1] != v[2] && v[0] != v[2] {
            triangles.push(v);
        }
    }
    let s = segments as u64;
    let (want_v, want_t) = (2 + s * s / 2, s * s);
    if (vertices.len() as u64, triangles.len() as u64) != (want_v, want_t) {
        let detail = format!(
            "{segments} segments expect {want_v} vertices and {want_t} triangles, got {} and {}",
            vertices.len(),
            triangles.len()
        );
        return Err(Rejection::new("canonical_count", None, detail));
    }
    Ok(Geometry { vertices, triangles })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same arrays as `kiln-cli` `verify_recipe::tests::sphere_glb`.
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
    fn canonical_sphere_8_gives_34_and_64() {
        let (pos, idx) = sphere(8);
        assert_eq!((pos.len(), idx.len() / 3), (54, 80));
        let g = build(&pos, &idx, 8).unwrap();
        assert_eq!((g.vertices.len(), g.triangles.len()), (34, 64));
        assert_eq!(g.vertices[0], [0, 1000, 0]);
        // Second vertex is the first one of ring 1 (first appearance order).
        assert_eq!(g.vertices[1], [quantize(pos[9][0]).unwrap(), quantize(pos[9][1]).unwrap(), 0]);
        // Triangles keep index order and vertex order: (b, d, c) of the first quad of ring 1.
        assert_eq!(g.triangles[0], [0, 2, 1]);
        let b = super::super::to_bytes(&g);
        assert_eq!(b.len(), 32 + 12 * 34 + 12 * 64);
    }

    #[test]
    fn canonical_sphere_64_gives_2050_and_4096() {
        let (pos, idx) = sphere(64);
        let g = build(&pos, &idx, 64).unwrap();
        assert_eq!((g.vertices.len(), g.triangles.len()), (2050, 4096));
    }

    #[test]
    fn canonical_count_rejects_wrong_segments() {
        let (pos, idx) = sphere(8);
        let r = build(&pos, &idx, 10).unwrap_err();
        assert_eq!(r.code, "canonical_count");
        assert!(r.detail.contains("52") && r.detail.contains("34"), "{}", r.detail);
    }

    #[test]
    fn canonical_build_rejects_bad_indices() {
        let (pos, mut idx) = sphere(8);
        idx[7] = pos.len() as u32;
        let r = build(&pos, &idx, 8).unwrap_err();
        assert_eq!((r.code, r.element), ("I2_index_out_of_range", Some(2)));
        idx.truncate(idx.len() - 1);
        assert_eq!(build(&pos, &idx, 8).unwrap_err().code, "canonical_count");
    }

    #[test]
    fn canonical_build_rejects_unquantizable_position() {
        let (mut pos, idx) = sphere(8);
        pos[3][1] = f32::NAN;
        let r = build(&pos, &idx, 8).unwrap_err();
        assert_eq!((r.code, r.element), ("canonical_quantize", Some(3)));
    }

    #[test]
    fn canonical_quantize_halves_away_from_zero() {
        assert_eq!(quantize(0.0005), Ok(1));
        assert_eq!(quantize(-0.0005), Ok(-1));
        assert_eq!(quantize(0.0004999), Ok(0));
        assert_eq!(quantize(-0.0004999), Ok(0));
        assert_eq!(quantize(1.0), Ok(1000));
        assert_eq!(quantize(2_000_000.0), Ok(2_000_000_000));
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 3_000_000.0, -3_000_000.0] {
            assert_eq!(quantize(bad).unwrap_err().code, "canonical_quantize");
        }
    }
}
