//! Area and axis arithmetic for the census.
//!
//! Split out of the walk so the "how much of this mesh is floor" answer
//! can be checked against a hand-built triangle without opening a
//! package. Nothing here knows what an archetype is.

use cimmeria_navmesh_extractor::floor_probe::recast_up;
use cimmeria_navmesh_extractor::transform::ActorTransform;

/// Recast's walkable-slope cut-off at the 45° NavBuilder is configured
/// with: cos 45° == 1/sqrt(2). Same constant `floor_probe` defaults to.
pub const MIN_UP: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Centimetres per BigWorld unit.
pub const CM_PER_BW: f32 = 100.0;

/// `bw = (ue.Y, ue.Z, ue.X) / 100` — the calibrated Castle mapping.
pub fn ue3_to_bw(v: [f32; 3]) -> [f32; 3] {
    [v[1] / CM_PER_BW, v[2] / CM_PER_BW, v[0] / CM_PER_BW]
}

/// Footprint and walkable-facing area of one instance, in BigWorld m².
///
/// Footprint is the XZ-projected triangle area — the shadow the mesh
/// casts on the ground plane, which is what "how much of the map does
/// this thing cover" means for a navmesh. Walkable is the *true* area
/// of the triangles Recast would accept as floor, which is the quantity
/// that decides whether the missing geometry is a surface you can stand
/// on.
///
/// `recast_up`, not the right-hand normal: NavBuilder reverses the
/// winding on load, so a floor's emitted-order normal points down.
pub fn areas(local_tris: &[[[f32; 3]; 3]], xf: &ActorTransform) -> (f64, f64) {
    let mut footprint = 0.0f64;
    let mut walkable = 0.0f64;
    for t in local_tris {
        let bw = [
            ue3_to_bw(xf.apply(t[0])),
            ue3_to_bw(xf.apply(t[1])),
            ue3_to_bw(xf.apply(t[2])),
        ];
        footprint += xz_area(&bw) as f64;
        if recast_up(&bw) >= MIN_UP {
            walkable += area3(&bw) as f64;
        }
    }
    (footprint, walkable)
}

/// Area of the triangle's shadow on the XZ plane.
pub fn xz_area(t: &[[f32; 3]; 3]) -> f32 {
    let (ax, az) = (t[1][0] - t[0][0], t[1][2] - t[0][2]);
    let (bx, bz) = (t[2][0] - t[0][0], t[2][2] - t[0][2]);
    0.5 * (ax * bz - az * bx).abs()
}

/// True 3D area of the triangle.
pub fn area3(t: &[[f32; 3]; 3]) -> f32 {
    let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
}
