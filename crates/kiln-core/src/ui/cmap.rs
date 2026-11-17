//! Minimal TrueType/OpenType `cmap` reader: which code points a font covers.

use std::collections::BTreeSet;

fn u16_at(d: &[u8], o: usize) -> Result<u32, String> {
    let b = d.get(o..o.checked_add(2).ok_or("offset overflow")?);
    b.map(|b| u16::from_be_bytes([b[0], b[1]]) as u32)
        .ok_or_else(|| "truncated font".to_string())
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, String> {
    let b = d.get(o..o.checked_add(4).ok_or("offset overflow")?);
    b.map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "truncated font".to_string())
}

/// Code points covered by the best Unicode `cmap` subtable (format 4 or 12).
pub fn codepoints(font: &[u8]) -> Result<BTreeSet<u32>, String> {
    let tables = u16_at(font, 4)? as usize;
    let mut cmap = None;
    for i in 0..tables {
        let rec = 12 + i * 16;
        if font.get(rec..rec + 4).ok_or("truncated font")? == b"cmap" {
            cmap = Some(u32_at(font, rec + 8)? as usize);
        }
    }
    let base = cmap.ok_or("no cmap table")?;
    let mut best: Option<(u8, usize)> = None;
    for i in 0..u16_at(font, base + 2)? as usize {
        let rec = base + 4 + i * 8;
        let (plat, enc) = (u16_at(font, rec)?, u16_at(font, rec + 2)?);
        let rank = match (plat, enc) {
            (3, 10) => 3,
            (3, 1) => 2,
            (0, _) => 1,
            _ => continue,
        };
        let off = base + u32_at(font, rec + 4)? as usize;
        let format = u16_at(font, off)?;
        if (format == 4 || format == 12) && best.is_none_or(|(r, _)| rank > r) {
            best = Some((rank, off));
        }
    }
    let (_, off) = best.ok_or("no supported Unicode cmap subtable")?;
    let mut set = BTreeSet::new();
    // Overlapping ranges could otherwise make a tiny file cost unbounded work.
    let (budget, mut spent) = (0x110000u64 * 2, 0u64);
    let mut spend = |n: u64| -> Result<(), String> {
        spent += n;
        if spent > budget {
            return Err("cmap with too many (or overlapping) ranges".into());
        }
        Ok(())
    };
    if u16_at(font, off)? == 12 {
        for g in 0..u32_at(font, off + 12)? as usize {
            let r = off + 16 + g * 12;
            let (start, end) = (u32_at(font, r)?, u32_at(font, r + 4)?);
            let glyph = u32_at(font, r + 8)?;
            if start > end || end > 0x10FFFF {
                return Err("invalid format 12 group".into());
            }
            spend(u64::from(end - start) + 1)?;
            // Glyph 0 is .notdef; a start glyph of 0 maps nothing at the start.
            set.extend((start..=end).filter(|c| glyph != 0 || *c != start));
        }
    } else {
        let segs = u16_at(font, off + 6)? as usize / 2;
        let (ends, starts) = (off + 14, off + 16 + segs * 2);
        let (deltas, ranges) = (starts + segs * 2, starts + segs * 4);
        for s in 0..segs {
            let (end, start) = (u16_at(font, ends + s * 2)?, u16_at(font, starts + s * 2)?);
            let (delta, range) = (u16_at(font, deltas + s * 2)?, u16_at(font, ranges + s * 2)?);
            if start <= end.min(0xFFFE) {
                spend(u64::from(end.min(0xFFFE) - start) + 1)?;
            }
            for c in start..=end.min(0xFFFE) {
                let glyph = if range == 0 {
                    (c + delta) & 0xFFFF
                } else {
                    let o = ranges + s * 2 + range as usize + (c - start) as usize * 2;
                    match u16_at(font, o)? {
                        0 => 0,
                        g => (g + delta) & 0xFFFF,
                    }
                };
                if glyph != 0 {
                    set.insert(c);
                }
            }
        }
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLEX: &[u8] = include_bytes!("../../tests/fixtures/ui/IBMPlexMono-Regular.ttf");

    #[test]
    fn plex_has_check_times_and_latin() {
        let set = codepoints(PLEX).unwrap();
        for c in ['\u{2713}', '\u{00D7}', 'A'] {
            assert!(set.contains(&(c as u32)), "missing U+{:04X}", c as u32);
        }
    }

    #[test]
    fn plex_lacks_warning_and_heavy_x() {
        let set = codepoints(PLEX).unwrap();
        for c in ['\u{26A0}', '\u{2715}'] {
            assert!(!set.contains(&(c as u32)), "unexpected U+{:04X}", c as u32);
        }
    }

    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        assert!(codepoints(&[]).is_err());
        assert!(codepoints(&[0u8; 7]).is_err());
        assert!(codepoints(&PLEX[..200]).is_err());
        // The cmap sits early in the file, so late cuts may still be valid:
        // only require that no cut length panics.
        for n in (0..4000).step_by(7) {
            let _ = codepoints(&PLEX[..n]);
        }
        assert!(codepoints(&[0xFF; 4096]).is_err());
    }

    /// A font with one format 12 subtable made of the given `(start, end, glyph)` groups.
    fn font12(groups: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut sub = Vec::new();
        sub.extend(12u16.to_be_bytes());
        sub.extend(0u16.to_be_bytes());
        sub.extend((16 + groups.len() as u32 * 12).to_be_bytes());
        sub.extend(0u32.to_be_bytes());
        sub.extend((groups.len() as u32).to_be_bytes());
        for g in groups {
            for v in [g.0, g.1, g.2] {
                sub.extend(v.to_be_bytes());
            }
        }
        let mut cmap = Vec::new();
        cmap.extend(0u16.to_be_bytes());
        cmap.extend(1u16.to_be_bytes());
        cmap.extend(3u16.to_be_bytes());
        cmap.extend(10u16.to_be_bytes());
        cmap.extend(12u32.to_be_bytes());
        cmap.extend(sub);
        let mut font = vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        font.extend(b"cmap");
        font.extend(0u32.to_be_bytes());
        font.extend(28u32.to_be_bytes());
        font.extend((cmap.len() as u32).to_be_bytes());
        font.extend(cmap);
        font
    }

    #[test]
    fn a_well_formed_synthetic_format_12_is_read() {
        let set = codepoints(&font12(&[(0x41, 0x43, 1), (0x2713, 0x2713, 2)])).unwrap();
        assert_eq!(set.into_iter().collect::<Vec<_>>(), [0x41, 0x42, 0x43, 0x2713]);
    }

    #[test]
    fn overlapping_groups_that_add_up_to_a_bomb_are_an_error_and_fast() {
        let font = font12(&vec![(0, 0x10FFFF, 1); 2000]);
        let t = std::time::Instant::now();
        assert!(codepoints(&font).is_err());
        assert!(t.elapsed().as_secs() < 10, "took {:?}", t.elapsed());
    }
}
