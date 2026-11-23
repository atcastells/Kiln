//! A small PNG writer (RGB, 8 bits) with its own DEFLATE: fixed Huffman
//! codes and LZ77 matches. Enough for flat-shaded previews, which are mostly
//! runs of one colour, and no dependency outside Rust's standard library.

/// Encodes `rgb` (`w * h * 3` bytes, rows top down) as a PNG file.
pub fn encode(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    assert_eq!(rgb.len(), w as usize * h as usize * 3, "rgb size");
    let row = w as usize * 3;
    let mut raw = Vec::with_capacity((row + 1) * h as usize);
    let zero = vec![0u8; row];
    for y in 0..h as usize {
        let cur = &rgb[y * row..(y + 1) * row];
        let up = if y == 0 { &zero[..] } else { &rgb[(y - 1) * row..y * row] };
        // Sub or Up, whichever leaves the smaller sum: the usual heuristic.
        let sub: Vec<u8> = (0..row).map(|i| cur[i].wrapping_sub(if i >= 3 { cur[i - 3] } else { 0 })).collect();
        let upf: Vec<u8> = (0..row).map(|i| cur[i].wrapping_sub(up[i])).collect();
        let cost = |v: &[u8]| v.iter().map(|&b| (b as i8).unsigned_abs() as u64).sum::<u64>();
        if cost(&upf) < cost(&sub) {
            raw.push(2);
            raw.extend(upf);
        } else {
            raw.push(1);
            raw.extend(sub);
        }
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend(kind);
    out.extend(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    b << 16 | a
}

/// A zlib stream: header, one fixed-Huffman DEFLATE block, Adler-32.
pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.extend(deflate(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    /// `len` bits of `v`, least significant first.
    fn put(&mut self, v: u32, len: u32) {
        self.acc |= (v as u64) << self.n;
        self.n += len;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code, which DEFLATE stores most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        let mut r = 0;
        for i in 0..len {
            r |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.put(r, len);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

const LEN_BASE: [u16; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

fn literal(b: &mut Bits, sym: u32) {
    match sym {
        0..=143 => b.code(0x30 + sym, 8),
        144..=255 => b.code(0x190 + sym - 144, 9),
        256..=279 => b.code(sym - 256, 7),
        _ => b.code(0xC0 + sym - 280, 8),
    }
}

fn matched(b: &mut Bits, len: usize, dist: usize) {
    let i = LEN_BASE.iter().rposition(|&l| l as usize <= len).unwrap_or(0);
    literal(b, 257 + i as u32);
    b.put((len - LEN_BASE[i] as usize) as u32, LEN_EXTRA[i] as u32);
    let d = DIST_BASE.iter().rposition(|&x| x as usize <= dist).unwrap_or(0);
    b.code(d as u32, 5);
    b.put((dist - DIST_BASE[d] as usize) as u32, DIST_EXTRA[d] as u32);
}

/// Raw DEFLATE, one final block with the fixed codes; greedy LZ77 over a
/// hash chain of 3-byte prefixes.
pub fn deflate(data: &[u8]) -> Vec<u8> {
    const WINDOW: usize = 32768;
    const HASH: usize = 1 << 15;
    const CHAIN: usize = 48;
    let mut b = Bits { out: Vec::with_capacity(data.len() / 8 + 16), acc: 0, n: 0 };
    b.put(1, 1); // BFINAL
    b.put(1, 2); // BTYPE = fixed
    let mut head = vec![usize::MAX; HASH];
    let mut prev = vec![usize::MAX; data.len()];
    let hash = |i: usize| -> usize {
        ((data[i] as usize) << 10 ^ (data[i + 1] as usize) << 5 ^ data[i + 2] as usize) & (HASH - 1)
    };
    let insert = |i: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if i + 2 < data.len() {
            let h = hash(i);
            prev[i] = head[h];
            head[h] = i;
        }
    };
    let mut i = 0;
    while i < data.len() {
        let (mut best, mut at) = (0usize, 0usize);
        if i + 2 < data.len() {
            let mut cand = head[hash(i)];
            let mut tries = 0;
            let max = (data.len() - i).min(258);
            while cand != usize::MAX && i - cand <= WINDOW && tries < CHAIN {
                let mut l = 0;
                while l < max && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best {
                    best = l;
                    at = cand;
                    if l == max {
                        break;
                    }
                }
                cand = prev[cand];
                tries += 1;
            }
        }
        if best >= 3 {
            matched(&mut b, best, i - at);
            for k in i..i + best {
                insert(k, &mut head, &mut prev);
            }
            i += best;
        } else {
            literal(&mut b, data[i] as u32);
            insert(i, &mut head, &mut prev);
            i += 1;
        }
    }
    literal(&mut b, 256);
    b.finish()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Inflates one fixed-Huffman block: enough to check `deflate`.
    pub fn inflate_fixed(src: &[u8]) -> Vec<u8> {
        let mut pos = 0usize;
        let bit = |pos: &mut usize| -> u32 {
            let v = (src[*pos / 8] >> (*pos % 8)) & 1;
            *pos += 1;
            v as u32
        };
        let bits = |pos: &mut usize, n: u32| -> u32 { (0..n).map(|i| bit(pos) << i).sum() };
        assert_eq!(bits(&mut pos, 1), 1, "final");
        assert_eq!(bits(&mut pos, 2), 1, "fixed");
        let mut out: Vec<u8> = Vec::new();
        loop {
            // Read the literal/length symbol, most significant bit first.
            let mut code = 0u32;
            let mut len = 0;
            let sym = loop {
                code = code << 1 | bit(&mut pos);
                len += 1;
                match len {
                    7 if code <= 0x17 => break code + 256,
                    8 if (0x30..=0xBF).contains(&code) => break code - 0x30,
                    8 if (0xC0..=0xC7).contains(&code) => break code - 0xC0 + 280,
                    9 if (0x190..=0x1FF).contains(&code) => break code - 0x190 + 144,
                    9.. => panic!("bad code"),
                    _ => {}
                }
            };
            match sym {
                0..=255 => out.push(sym as u8),
                256 => return out,
                _ => {
                    let i = (sym - 257) as usize;
                    let l = LEN_BASE[i] as usize + bits(&mut pos, LEN_EXTRA[i] as u32) as usize;
                    let mut d = 0u32;
                    for _ in 0..5 {
                        d = d << 1 | bit(&mut pos);
                    }
                    let d = d as usize;
                    let dist = DIST_BASE[d] as usize + bits(&mut pos, DIST_EXTRA[d] as u32) as usize;
                    for _ in 0..l {
                        out.push(out[out.len() - dist]);
                    }
                }
            }
        }
    }

    #[test]
    fn deflate_round_trips_and_shrinks_runs() {
        let mut data: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        data.extend(vec![0u8; 40_000]);
        data.extend(b"abcabcabcabcabc hola hola hola".iter());
        data.extend((0..3000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8));
        let z = deflate(&data);
        assert_eq!(inflate_fixed(&z), data);
        assert!(z.len() < data.len() / 3, "{} of {}", z.len(), data.len());
        assert_eq!(inflate_fixed(&deflate(b"")), b"");
        assert_eq!(inflate_fixed(&deflate(b"ab")), b"ab");
    }

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_png_has_its_chunks() {
        let rgb = vec![200u8; 4 * 3 * 3];
        let png = encode(4, 3, &rgb);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 4);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // IDAT holds zlib of the filtered rows.
        let idat_len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
        assert_eq!(&png[37..41], b"IDAT");
        let z = &png[41..41 + idat_len];
        let raw = inflate_fixed(&z[2..z.len() - 4]);
        assert_eq!(raw.len(), 3 * (1 + 12));
    }
}
