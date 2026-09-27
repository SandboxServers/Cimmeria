//! Triangle-to-column rasterisation: the Y range a triangle occupies over
//! one cell's XZ square.
//!
//! The triangle is clipped (Sutherland-Hodgman, in 3D, against the four
//! vertical planes of the cell) and the Y range of what is left is the span.
//! The planes are inclusive, so a wall lying exactly on a cell boundary
//! lands in both neighbours: the grid may only ever over-state solid space,
//! never lose a wall between two cells.

/// A triangle in BigWorld metres: `[x, y (up), z]` per vertex.
pub type Triangle = [[f32; 3]; 3];

/// Upper bound on the clipped polygon's vertex count: a triangle cut by four
/// half-planes gains at most one vertex per plane.
const MAX_POLY: usize = 8;

#[derive(Clone, Copy)]
struct Poly {
    v: [[f32; 3]; MAX_POLY],
    n: usize,
}

impl Poly {
    fn from_tri(t: &Triangle) -> Self {
        let mut v = [[0.0; 3]; MAX_POLY];
        v[..3].copy_from_slice(t);
        Self { v, n: 3 }
    }

    /// Keep the part with `sign * p[axis] >= sign * bound` (inclusive).
    fn clip(&self, axis: usize, bound: f32, sign: f32) -> Self {
        let mut out = Self {
            v: [[0.0; 3]; MAX_POLY],
            n: 0,
        };
        if self.n == 0 {
            return out;
        }
        let inside = |p: &[f32; 3]| sign * (p[axis] - bound) >= 0.0;
        for i in 0..self.n {
            let a = self.v[i];
            let b = self.v[(i + 1) % self.n];
            let (ia, ib) = (inside(&a), inside(&b));
            if ia {
                out.push(a);
            }
            if ia != ib {
                let t = (bound - a[axis]) / (b[axis] - a[axis]);
                out.push([
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ]);
            }
        }
        out
    }

    fn push(&mut self, p: [f32; 3]) {
        // A degenerate input can revisit a vertex; dropping the overflow
        // loses nothing, since the Y range is the only output.
        if self.n < MAX_POLY {
            self.v[self.n] = p;
            self.n += 1;
        }
    }
}

/// The part of a triangle over one cell: its bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clipped {
    /// `(min, max)` Y.
    pub y: (f32, f32),
    /// `(min, max)` X.
    pub x: (f32, f32),
    /// `(min, max)` Z.
    pub z: (f32, f32),
}

/// The bounds of the part of `tri` whose XZ projection lies in
/// `[x0, x1] x [z0, z1]`, or `None` when they do not touch.
pub fn clip_to_cell(tri: &Triangle, x0: f32, x1: f32, z0: f32, z1: f32) -> Option<Clipped> {
    let p = Poly::from_tri(tri)
        .clip(0, x0, 1.0)
        .clip(0, x1, -1.0)
        .clip(2, z0, 1.0)
        .clip(2, z1, -1.0);
    if p.n == 0 {
        return None;
    }
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for v in &p.v[..p.n] {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    (lo[1] <= hi[1]).then_some(Clipped {
        y: (lo[1], hi[1]),
        x: (lo[0], hi[0]),
        z: (lo[2], hi[2]),
    })
}

/// [`clip_to_cell`]'s Y range alone.
pub fn clip_y_range(tri: &Triangle, x0: f32, x1: f32, z0: f32, z1: f32) -> Option<(f32, f32)> {
    clip_to_cell(tri, x0, x1, z0, z1).map(|c| c.y)
}

/// Whether the triangle faces up or down within `max_slope_deg` of vertical
/// (either winding): a floor or a ceiling a unit could stand on or under.
/// Used only to build the coverage mask.
pub fn is_floor_like(tri: &Triangle, max_slope_deg: f32) -> bool {
    let u = sub(tri[1], tri[0]);
    let v = sub(tri[2], tri[0]);
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    // A NaN length falls through: the comparison below is then false.
    if len <= 0.0 {
        return false;
    }
    n[1].abs() / len >= max_slope_deg.to_radians().cos()
}

/// Exact segment/triangle intersection (Moller-Trumbore), either face.
pub fn segment_hits_triangle(a: [f32; 3], b: [f32; 3], t: &Triangle) -> bool {
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

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_floor_triangle_is_a_flat_span_where_it_covers_the_cell() {
        let t = [[0.0, 2.0, 0.0], [4.0, 2.0, 0.0], [0.0, 2.0, 4.0]];
        assert_eq!(clip_y_range(&t, 0.5, 1.0, 0.5, 1.0), Some((2.0, 2.0)));
        // Past the hypotenuse.
        assert_eq!(clip_y_range(&t, 3.0, 3.5, 3.0, 3.5), None);
    }

    #[test]
    fn a_vertical_wall_spans_its_height_in_the_cells_it_crosses() {
        // A wall in the plane x = 1.25, from y 0 to 3.
        let t = [[1.25, 0.0, 0.0], [1.25, 3.0, 0.0], [1.25, 0.0, 10.0]];
        let r = clip_y_range(&t, 1.0, 1.5, 0.0, 0.5).unwrap();
        assert_eq!(r.0, 0.0);
        assert!(r.1 > 2.8, "{r:?}");
        assert_eq!(clip_y_range(&t, 1.5, 2.0, 0.0, 0.5), None);
    }

    #[test]
    fn a_wall_on_a_cell_boundary_lands_in_both_cells() {
        let t = [[1.0, 0.0, 0.0], [1.0, 3.0, 0.0], [1.0, 0.0, 10.0]];
        assert!(clip_y_range(&t, 0.5, 1.0, 0.0, 0.5).is_some());
        assert!(clip_y_range(&t, 1.0, 1.5, 0.0, 0.5).is_some());
    }

    #[test]
    fn a_ramp_spans_only_the_heights_over_the_cell() {
        // Rises 1 m per metre along x.
        let t = [[0.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 0.0, 10.0]];
        let (lo, hi) = clip_y_range(&t, 2.0, 3.0, 0.0, 1.0).unwrap();
        assert!(
            (lo - 2.0).abs() < 1e-5 && (hi - 3.0).abs() < 1e-5,
            "{lo} {hi}"
        );
    }

    #[test]
    fn the_clipped_bounds_hug_a_wall_inside_the_cell() {
        let t = [[1.25, 0.0, 0.0], [1.25, 3.0, 0.0], [1.25, 0.0, 10.0]];
        let c = clip_to_cell(&t, 1.0, 1.5, 0.0, 0.5).unwrap();
        assert_eq!(c.x, (1.25, 1.25));
        assert_eq!(c.z, (0.0, 0.5));
    }

    #[test]
    fn floor_like_accepts_both_windings_and_rejects_walls() {
        let up = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
        let down = [up[0], up[2], up[1]];
        let wall = [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        assert!(is_floor_like(&up, 45.0));
        assert!(is_floor_like(&down, 45.0));
        assert!(!is_floor_like(&wall, 45.0));
        assert!(!is_floor_like(&[[0.0; 3]; 3], 45.0));
    }
}
