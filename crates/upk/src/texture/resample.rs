//! Area-average resampling, integer-only.
//!
//! Each destination pixel is the exact average of the source area it covers,
//! weighted by overlap, computed in whole numbers and rounded half up. There
//! is no floating point anywhere, so the output is the same on every machine,
//! which a result hash pinned in a patch recipe depends on.

/// Overlap weights of the source pixels under each destination pixel along
/// one axis: for destination `d`, `(source index, weight)` pairs whose
/// weights sum to `src`. Both axes are scaled to a common unit of
/// `src * dst`, so every boundary is an integer.
fn axis(src: usize, dst: usize) -> Vec<Vec<(usize, u64)>> {
    (0..dst)
        .map(|d| {
            let (start, end) = (d * src, (d + 1) * src);
            let (first, last) = (start / dst, (end - 1) / dst);
            (first..=last)
                .map(|s| {
                    let lo = start.max(s * dst);
                    let hi = end.min((s + 1) * dst);
                    (s, (hi - lo) as u64)
                })
                .collect()
        })
        .collect()
}

/// Resample an RGB image from `sw` x `sh` to `dw` x `dh`.
pub fn area(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    assert!(src.len() == sw * sh * 3 && dw > 0 && dh > 0);
    let (xs, ys) = (axis(sw, dw), axis(sh, dh));
    // Horizontal pass: per source row, sums weighted by overlap.
    let mut wide = vec![0u64; dw * sh * 3];
    for y in 0..sh {
        for (x, taps) in xs.iter().enumerate() {
            for c in 0..3 {
                wide[(y * dw + x) * 3 + c] = taps
                    .iter()
                    .map(|&(s, w)| w * src[(y * sw + s) * 3 + c] as u64)
                    .sum();
            }
        }
    }
    // Both passes weigh by overlap, so the weights of one destination pixel
    // sum to `sw` horizontally and `sh` vertically.
    let total = (sw * sh) as u64;
    let mut out = vec![0u8; dw * dh * 3];
    for (y, taps) in ys.iter().enumerate() {
        for x in 0..dw {
            for c in 0..3 {
                let sum: u64 = taps
                    .iter()
                    .map(|&(s, w)| w * wide[(s * dw + x) * 3 + c])
                    .sum();
                out[(y * dw + x) * 3 + c] = ((sum + total / 2) / total) as u8;
            }
        }
    }
    out
}
