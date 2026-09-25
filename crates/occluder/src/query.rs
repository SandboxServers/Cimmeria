//! The segment test: walk the cells a segment crosses and check its height
//! in each against the solid spans there.
//!
//! The walk is Amanatides-Woo over XZ, per layer. In each cell the segment
//! occupies a parameter range `[t0, t1]`. For each span there, that range is
//! clipped to the span's sub-cell rectangle; the segment's Y over what is
//! left is exact (the segment is straight), and it is blocked when that
//! meets the span's Y range. Spans and rectangles are rounded outwards, so
//! the test can call a ray blocked that grazes within a sub-cell (1/16 of a
//! cell) of a wall, but it cannot see through one.
//!
//! **Endpoint clearance.** [`Occluder::sight_with_clearance`] can leave the
//! first and last metres (horizontally) of the segment untested. The default
//! is zero. The sub-cell rectangles already keep a wall out of the part of a
//! cell where a unit stands. On the NA27 Castle sweep a 0.3 m clearance
//! removed no false blocks and added 3 false clears, where an endpoint stood
//! just behind a wall's end.

use crate::grid::{Cell, Layer, LayerKind, Occluder, SubRect, SUB};

/// Slack on a span rectangle's world edges, metres: covers the rounding
/// between the builder's cell bounds and the loaded layer's.
const RECT_EPS: f32 = 1e-3;

/// What the occluder says about a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sight {
    /// No solid span meets the segment.
    Clear,
    /// The segment meets solid geometry; `at` is where it entered the first
    /// blocking cell, and `layer` which layer blocked it.
    Blocked { at: [f32; 3], layer: LayerKind },
    /// An endpoint is outside the occluder's coverage: the grid has no
    /// information about this segment.
    OffGrid,
}

impl Sight {
    /// Stable label for logs: `clear`, `blocked`, `off_grid`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Blocked { .. } => "blocked",
            Self::OffGrid => "off_grid",
        }
    }
}

impl Occluder {
    /// Horizontal metres at each end of a segment that are not tested; see
    /// the module docs.
    pub const DEFAULT_CLEARANCE: f32 = 0.0;

    /// Whether the segment `from -> to` (BigWorld metres, Y up) is free of
    /// solid geometry, with [`Self::DEFAULT_CLEARANCE`] at each end.
    pub fn sight(&self, from: [f32; 3], to: [f32; 3]) -> Sight {
        self.sight_with_clearance(from, to, Self::DEFAULT_CLEARANCE)
    }

    /// [`Self::sight`] with an explicit endpoint clearance.
    pub fn sight_with_clearance(&self, from: [f32; 3], to: [f32; 3], clearance: f32) -> Sight {
        if !(from.iter().chain(&to).all(|v| v.is_finite())) {
            return Sight::OffGrid;
        }
        if !self.covers(from[0], from[2]) || !self.covers(to[0], to[2]) {
            return Sight::OffGrid;
        }
        let len = ((to[0] - from[0]).powi(2) + (to[2] - from[2]).powi(2)).sqrt();
        let s = if len > 1e-4 {
            clearance.max(0.0) / len
        } else {
            0.0
        };
        if 2.0 * s >= 1.0 {
            return Sight::Clear;
        }
        match self.hit_between(from, to, s, 1.0 - s) {
            Some((at, layer)) => Sight::Blocked { at, layer },
            None => Sight::Clear,
        }
    }

    /// Where the segment `a -> b`, restricted to parameters `[t0, t1]`,
    /// first meets solid geometry, and which layer. No coverage check: a
    /// part of the segment outside this occluder's cells sees nothing. The
    /// paged occluder runs each page over its own stretch of the segment.
    pub(crate) fn hit_between(
        &self,
        a: [f32; 3],
        b: [f32; 3],
        t0: f32,
        t1: f32,
    ) -> Option<([f32; 3], LayerKind)> {
        for layer in &self.layers {
            if let Some(at) = self.first_hit(layer, a, b, t0, t1) {
                return Some((at, layer.kind));
            }
        }
        let hf = self.heightfield.as_ref()?;
        hf.first_hit(a, b, t0, t1)
            .map(|at| (at, LayerKind::Terrain))
    }

    fn first_hit(&self, l: &Layer, a: [f32; 3], b: [f32; 3], t0: f32, t1: f32) -> Option<[f32; 3]> {
        let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        let len = (dx * dx + dz * dz).sqrt();
        let at = |t: f32| [a[0] + dx * t, a[1] + dy * t, a[2] + dz * t];
        let sub = l.cell / SUB as f32;
        let hits = |cx: i64, cz: i64, ta: f32, tb: f32| -> bool {
            let Cell::Spans(spans) = l.cell(cx, cz) else {
                return false;
            };
            let x0 = l.origin[0] + cx as f32 * l.cell;
            let z0 = l.origin[1] + cz as f32 * l.cell;
            spans.iter().any(|&s| {
                let r = SubRect::unpack(s.rect);
                let rx = (
                    x0 + r.x0 as f32 * sub - RECT_EPS,
                    x0 + (r.x1 + 1) as f32 * sub + RECT_EPS,
                );
                let rz = (
                    z0 + r.z0 as f32 * sub - RECT_EPS,
                    z0 + (r.z1 + 1) as f32 * sub + RECT_EPS,
                );
                let Some((u0, u1)) =
                    slab(a[0], dx, rx, (ta, tb)).and_then(|range| slab(a[2], dz, rz, range))
                else {
                    return false;
                };
                let (ya, yb) = (a[1] + dy * u0, a[1] + dy * u1);
                let (lo, hi) = self.span_metres(s);
                lo <= ya.max(yb) && hi >= ya.min(yb)
            })
        };

        let p0 = at(t0);
        let p1 = at(t1);
        let (mut cx, mut cz) = l.cell_of(p0[0], p0[2]);
        let (ex, ez) = l.cell_of(p1[0], p1[2]);
        if len <= 1e-4 {
            return hits(cx, cz, t0, t1).then_some(p0);
        }
        // Parameter step per cell, and the parameter of the next boundary.
        let (sx, tdx, mut tmx) = axis(dx, a[0], l.origin[0], l.cell, cx);
        let (sz, tdz, mut tmz) = axis(dz, a[2], l.origin[1], l.cell, cz);
        let mut t = t0;
        // Two spare steps: rounding can put one extra boundary crossing
        // between the end cells; the `next >= t1` exit ends the walk.
        let steps = (ex - cx).abs() + (ez - cz).abs() + 2;
        for _ in 0..=steps {
            let next = tmx.min(tmz).min(t1);
            if hits(cx, cz, t, next) {
                return Some(at(t));
            }
            if next >= t1 {
                break;
            }
            if tmx < tmz {
                cx += sx;
                t = tmx;
                tmx += tdx;
            } else {
                cz += sz;
                t = tmz;
                tmz += tdz;
            }
        }
        None
    }
}

/// The part of the parameter range `t` where the coordinate `p + d * t`
/// lies inside `bounds`, or `None`.
fn slab(p: f32, d: f32, bounds: (f32, f32), t: (f32, f32)) -> Option<(f32, f32)> {
    if d.abs() < 1e-12 {
        return (p >= bounds.0 && p <= bounds.1).then_some(t);
    }
    let (mut u0, mut u1) = ((bounds.0 - p) / d, (bounds.1 - p) / d);
    if u0 > u1 {
        std::mem::swap(&mut u0, &mut u1);
    }
    let (lo, hi) = (t.0.max(u0), t.1.min(u1));
    (lo <= hi).then_some((lo, hi))
}

/// Amanatides-Woo set-up for one axis: the step direction, the parameter
/// length of one cell, and the parameter of the first boundary crossing.
fn axis(d: f32, a: f32, origin: f32, cell: f32, c: i64) -> (i64, f32, f32) {
    if d > 0.0 {
        let boundary = origin + (c + 1) as f32 * cell;
        (1, cell / d, (boundary - a) / d)
    } else if d < 0.0 {
        let boundary = origin + c as f32 * cell;
        (-1, cell / -d, (boundary - a) / d)
    } else {
        (0, f32::INFINITY, f32::INFINITY)
    }
}
