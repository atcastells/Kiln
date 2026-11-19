//! Canonical geometry v0 (ASTRA AST-ESP-001-S1): quantized integer mesh,
//! its byte container (the exact bytes hashed, S1 §6.2) and its hash.

mod build;
pub mod radial;
pub mod topology;

pub use build::{build, quantize};

/// Container format version.
pub const FORMAT: u32 = 0;
/// Class `radial` (spec D3: radial = 0, general = 1).
pub const CLASS_RADIAL: u32 = 0;
/// Quanta per metre: one quantum is a millimetre.
pub const QUANTA_PER_METRE: u32 = 1000;
/// Centre of a radial geometry, in quanta.
pub const CENTRE: [i32; 3] = [0, 0, 0];
/// Header bytes before the arrays.
pub const HEADER_LEN: usize = 32;

/// Quantized vertices and triangles that index them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub vertices: Vec<[i32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// Why a geometry is not canonical: a stable code, the offending element
/// (vertex or triangle index) when there is one, and a readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub code: &'static str,
    pub element: Option<usize>,
    pub detail: String,
}

impl Rejection {
    pub fn new(code: &'static str, element: Option<usize>, detail: impl Into<String>) -> Self {
        Self { code, element, detail: detail.into() }
    }
}

/// I1–I9 (S1 §3.1) in order; the first failure wins. The only entry point:
/// the later checks assume the earlier ones hold.
pub fn validate(g: &Geometry) -> Result<(), Rejection> {
    topology::check_i1_i7(g)?;
    radial::check_i8_i9(g)
}

/// Little-endian container (spec D3): header, then vertices, then triangles.
pub fn to_bytes(g: &Geometry) -> Vec<u8> {
    let mut b = Vec::with_capacity(HEADER_LEN + 12 * (g.vertices.len() + g.triangles.len()));
    for w in [FORMAT, CLASS_RADIAL, QUANTA_PER_METRE] {
        b.extend(w.to_le_bytes());
    }
    b.extend(CENTRE.iter().flat_map(|c| c.to_le_bytes()));
    b.extend((g.vertices.len() as u32).to_le_bytes());
    b.extend((g.triangles.len() as u32).to_le_bytes());
    b.extend(g.vertices.iter().flatten().flat_map(|c| c.to_le_bytes()));
    b.extend(g.triangles.iter().flatten().flat_map(|i| i.to_le_bytes()));
    b
}

fn word(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

/// Reads a container written by `to_bytes`; any other size or header is rejected.
pub fn from_bytes(bytes: &[u8]) -> Result<Geometry, Rejection> {
    let size = |detail: String| Rejection::new("canonical_size", None, detail);
    if bytes.len() < HEADER_LEN {
        return Err(size(format!("{} bytes, shorter than the header", bytes.len())));
    }
    let u = |at| u32::from_le_bytes(word(bytes, at));
    let i = |at| i32::from_le_bytes(word(bytes, at));
    let header = [u(0), u(4), u(8)];
    let centre = [i(12), i(16), i(20)];
    if header != [FORMAT, CLASS_RADIAL, QUANTA_PER_METRE] || centre != CENTRE {
        let detail = format!("format/class/quanta {header:?} and centre {centre:?} are not v0 radial");
        return Err(Rejection::new("canonical_header", None, detail));
    }
    let (v, t) = (u(24) as u64, u(28) as u64);
    let want = HEADER_LEN as u64 + 12 * v + 12 * t;
    if bytes.len() as u64 != want {
        return Err(size(format!("{} bytes, {v} vertices and {t} triangles need {want}", bytes.len())));
    }
    let body = &bytes[HEADER_LEN..];
    let triple = |at: usize| [0, 4, 8].map(|k| word(body, at + k));
    let vertices = (0..v as usize).map(|n| triple(12 * n).map(i32::from_le_bytes)).collect();
    let tri_at = 12 * v as usize;
    let triangles = (0..t as usize).map(|n| triple(tri_at + 12 * n).map(u32::from_le_bytes)).collect();
    Ok(Geometry { vertices, triangles })
}

/// Lowercase hex SHA-256 of the container bytes.
pub fn geometry_sha256(g: &Geometry) -> String {
    crate::asset::sha256_hex(&to_bytes(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra() -> Geometry {
        Geometry {
            vertices: vec![[1000, 0, 0], [0, 1000, 0], [0, 0, 1000], [-577, -577, -577]],
            triangles: vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]],
        }
    }

    #[test]
    fn canonical_bytes_have_exact_size_and_header() {
        let g = tetra();
        let b = to_bytes(&g);
        assert_eq!(b.len(), HEADER_LEN + 12 * 4 + 12 * 4);
        assert_eq!(&b[0..4], &FORMAT.to_le_bytes());
        assert_eq!(&b[4..8], &CLASS_RADIAL.to_le_bytes());
        assert_eq!(&b[8..12], &QUANTA_PER_METRE.to_le_bytes());
        assert_eq!(&b[12..24], &[0u8; 12]);
        assert_eq!(&b[24..28], &4u32.to_le_bytes());
        assert_eq!(&b[28..32], &4u32.to_le_bytes());
        assert_eq!(&b[32..36], &1000i32.to_le_bytes());
        assert_eq!(&b[32 + 36..32 + 40], &(-577i32).to_le_bytes());
        assert_eq!(&b[32 + 48 + 12..32 + 48 + 16], &0u32.to_le_bytes());
    }

    #[test]
    fn canonical_bytes_round_trip() {
        let g = tetra();
        assert_eq!(from_bytes(&to_bytes(&g)), Ok(g));
    }

    #[test]
    fn canonical_bytes_reject_wrong_size() {
        let b = to_bytes(&tetra());
        for bad in [&b[..b.len() - 1], &b[..20], &[][..]] {
            assert_eq!(from_bytes(bad).unwrap_err().code, "canonical_size");
        }
        let mut longer = b.clone();
        longer.extend([0u8; 12]);
        assert_eq!(from_bytes(&longer).unwrap_err().code, "canonical_size");
        let mut huge = b.clone();
        huge[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(from_bytes(&huge).unwrap_err().code, "canonical_size");
    }

    #[test]
    fn canonical_bytes_reject_other_header() {
        let b = to_bytes(&tetra());
        for at in [0, 4, 8, 12, 16, 20] {
            let mut bad = b.clone();
            bad[at] = 1;
            assert_eq!(from_bytes(&bad).unwrap_err().code, "canonical_header", "byte {at}");
        }
    }

    #[test]
    fn canonical_hash_is_sha256_of_bytes() {
        let g = tetra();
        let h = geometry_sha256(&g);
        assert_eq!(h, crate::asset::sha256_hex(&to_bytes(&g)));
        assert_eq!(h.len(), 64);
    }
}
