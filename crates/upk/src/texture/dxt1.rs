//! DXT1 (BC1) decode and encode, integer-only.
//!
//! The encoder exists so a launcher can rebuild a texture on the player's
//! machine and get the same bytes as the machine that pinned the result hash.
//! That rules out a third-party encoder (its output can change between
//! versions) and anything floating point. The fit is a plain bounding-box
//! range fit: per channel min and max, inset by 1/16 of the range, packed to
//! RGB565, then each pixel takes the nearest of the four palette colours by
//! squared RGB distance, ties to the lowest index.

/// An RGB image, row-major, three bytes a pixel.
pub type Rgb = Vec<u8>;

fn expand565(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

fn pack565(c: [u8; 3]) -> u16 {
    (((c[0] >> 3) as u16) << 11) | (((c[1] >> 2) as u16) << 5) | (c[2] >> 3) as u16
}

/// The four palette colours of a block. Colour order decides the mode, as the
/// format defines: `c0 > c1` gives two interpolated colours a third and two
/// thirds of the way, otherwise one midpoint and black.
fn palette(c0: u16, c1: u16) -> [[u8; 3]; 4] {
    let (a, b) = (expand565(c0), expand565(c1));
    let mix = |wa: u32, wb: u32, div: u32| {
        let f = |i: usize| ((wa * a[i] as u32 + wb * b[i] as u32) / div) as u8;
        [f(0), f(1), f(2)]
    };
    if c0 > c1 {
        [a, b, mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        [a, b, mix(1, 1, 2), [0, 0, 0]]
    }
}

/// Decode `w` x `h` pixels (both multiples of 4) from DXT1 `data`.
pub fn decode(data: &[u8], w: usize, h: usize) -> Result<Rgb, String> {
    if !w.is_multiple_of(4) || !h.is_multiple_of(4) || data.len() != w / 4 * (h / 4) * 8 {
        return Err(format!("{} DXT1 bytes do not fill {w}x{h}", data.len()));
    }
    let mut out = vec![0u8; w * h * 3];
    for (bi, block) in data.as_chunks::<8>().0.iter().enumerate() {
        let c0 = u16::from_le_bytes([block[0], block[1]]);
        let c1 = u16::from_le_bytes([block[2], block[3]]);
        let bits = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
        let pal = palette(c0, c1);
        let (bx, by) = (bi % (w / 4) * 4, bi / (w / 4) * 4);
        for p in 0..16 {
            let idx = ((bits >> (2 * p)) & 3) as usize;
            let at = ((by + p / 4) * w + bx + p % 4) * 3;
            out[at..at + 3].copy_from_slice(&pal[idx]);
        }
    }
    Ok(out)
}

/// Encode `w` x `h` pixels (both multiples of 4) to DXT1.
pub fn encode(rgb: &[u8], w: usize, h: usize) -> Vec<u8> {
    assert!(w.is_multiple_of(4) && h.is_multiple_of(4) && rgb.len() == w * h * 3);
    let mut out = Vec::with_capacity(w / 4 * (h / 4) * 8);
    for by in (0..h).step_by(4) {
        for bx in (0..w).step_by(4) {
            let mut px = [[0u8; 3]; 16];
            for (p, slot) in px.iter_mut().enumerate() {
                let at = ((by + p / 4) * w + bx + p % 4) * 3;
                slot.copy_from_slice(&rgb[at..at + 3]);
            }
            out.extend_from_slice(&encode_block(&px));
        }
    }
    out
}

fn encode_block(px: &[[u8; 3]; 16]) -> [u8; 8] {
    let mut lo = [255u8; 3];
    let mut hi = [0u8; 3];
    for p in px {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let mut a = hi;
    let mut b = lo;
    for i in 0..3 {
        let inset = (hi[i] - lo[i]) >> 4;
        a[i] = hi[i] - inset;
        b[i] = lo[i] + inset;
    }
    let (mut c0, mut c1) = (pack565(a), pack565(b));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let mut bits = 0u32;
    if c0 != c1 {
        let pal = palette(c0, c1);
        for (p, pixel) in px.iter().enumerate() {
            let mut best = (u32::MAX, 0u32);
            for (i, c) in pal.iter().enumerate() {
                let d: u32 = (0..3)
                    .map(|k| {
                        let e = pixel[k] as i32 - c[k] as i32;
                        (e * e) as u32
                    })
                    .sum();
                if d < best.0 {
                    best = (d, i as u32);
                }
            }
            bits |= best.1 << (2 * p);
        }
    }
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..8].copy_from_slice(&bits.to_le_bytes());
    out
}
