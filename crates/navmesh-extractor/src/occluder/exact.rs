//! Ground truth for occluder accuracy: an exact segment-vs-triangle test
//! over the raw collision triangles.
//!
//! Triangles are bucketed on a coarse XZ grid; a segment walks the buckets
//! it crosses and runs Moller-Trumbore against each triangle once. Slow
//! next to the occluder (it is the thing the occluder approximates), fast
//! enough for a few thousand pairs over a couple of million triangles.

use std::collections::HashMap;

use super::BwTriangle;

/// Bucket edge, metres.
const BUCKET: f32 = 2.0;

/// Triangles indexed for segment queries.
pub struct ExactScene {
    tris: Vec<BwTriangle>,
    buckets: HashMap<(i64, i64), Vec<u32>>,
    /// Per-triangle query stamp, so a triangle in several buckets is tested
    /// once per query.
    stamp: std::cell::RefCell<(Vec<u32>, u32)>,
}

impl ExactScene {
    /// Index `tris`.
    pub fn new(tris: Vec<BwTriangle>) -> Self {
        let mut buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, t) in tris.iter().enumerate() {
            let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for v in t {
                x0 = x0.min(v[0]);
                x1 = x1.max(v[0]);
                z0 = z0.min(v[2]);
                z1 = z1.max(v[2]);
            }
            for bz in (z0 / BUCKET).floor() as i64..=(z1 / BUCKET).floor() as i64 {
                for bx in (x0 / BUCKET).floor() as i64..=(x1 / BUCKET).floor() as i64 {
                    buckets.entry((bx, bz)).or_default().push(i as u32);
                }
            }
        }
        let n = tris.len();
        Self {
            tris,
            buckets,
            stamp: std::cell::RefCell::new((vec![0; n], 0)),
        }
    }

    /// Triangles indexed.
    pub fn len(&self) -> usize {
        self.tris.len()
    }

    /// Whether no triangles are indexed.
    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// Whether the segment `a -> b` meets any triangle.
    pub fn blocked(&self, a: [f32; 3], b: [f32; 3]) -> bool {
        let mut guard = self.stamp.borrow_mut();
        let (stamps, cur) = &mut *guard;
        *cur = cur.wrapping_add(1);
        if *cur == 0 {
            stamps.iter_mut().for_each(|s| *s = 0);
            *cur = 1;
        }
        let cur = *cur;
        let mut hit = false;
        walk_buckets(a, b, |key| {
            let Some(list) = self.buckets.get(&key) else {
                return false;
            };
            for &i in list {
                if stamps[i as usize] == cur {
                    continue;
                }
                stamps[i as usize] = cur;
                if segment_hits_triangle(a, b, &self.tris[i as usize]) {
                    hit = true;
                    return true;
                }
            }
            false
        });
        hit
    }
}

/// Visit every bucket the XZ projection of `a -> b` crosses, until `visit`
/// returns true. A vertical segment visits its one bucket.
fn walk_buckets(a: [f32; 3], b: [f32; 3], mut visit: impl FnMut((i64, i64)) -> bool) {
    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
    let (mut cx, mut cz) = (
        (a[0] / BUCKET).floor() as i64,
        (a[2] / BUCKET).floor() as i64,
    );
    let (ex, ez) = (
        (b[0] / BUCKET).floor() as i64,
        (b[2] / BUCKET).floor() as i64,
    );
    let axis = |d: f32, p: f32, c: i64| -> (i64, f32, f32) {
        if d > 0.0 {
            (1, BUCKET / d, ((c + 1) as f32 * BUCKET - p) / d)
        } else if d < 0.0 {
            (-1, BUCKET / -d, (c as f32 * BUCKET - p) / d)
        } else {
            (0, f32::INFINITY, f32::INFINITY)
        }
    };
    let (sx, tdx, mut tmx) = axis(dx, a[0], cx);
    let (sz, tdz, mut tmz) = axis(dz, a[2], cz);
    let steps = (ex - cx).abs() + (ez - cz).abs() + 2;
    for _ in 0..=steps {
        if visit((cx, cz)) {
            return;
        }
        if tmx.min(tmz) > 1.0 {
            return;
        }
        if tmx < tmz {
            cx += sx;
            tmx += tdx;
        } else {
            cz += sz;
            tmz += tdz;
        }
    }
}

/// Exact segment/triangle intersection (Moller-Trumbore), both faces.
pub fn segment_hits_triangle(a: [f32; 3], b: [f32; 3], t: &BwTriangle) -> bool {
    let sub = |p: [f32; 3], q: [f32; 3]| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    let cross = |p: [f32; 3], q: [f32; 3]| {
        [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ]
    };
    let dot = |p: [f32; 3], q: [f32; 3]| p[0] * q[0] + p[1] * q[1] + p[2] * q[2];
    let d = sub(b, a);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return false;
    }
    let inv = 1.0 / det;
    let s = sub(a, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let w = dot(e2, q) * inv;
    (0.0..=1.0).contains(&w)
}
