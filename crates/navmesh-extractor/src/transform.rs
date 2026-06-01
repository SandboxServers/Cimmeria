//! Actor-to-world transform math.
//!
//! UE3 stores per-actor placement as four properties:
//!
//! - `Location` (FVector) — world-space cm translation.
//! - `Rotation` (FRotator) — packed integer Euler angles where 65536 units
//!   == 360°. The components are named `pitch` (Y-axis), `yaw` (Z-axis),
//!   `roll` (X-axis); applied in **Roll → Pitch → Yaw** (i.e., R_yaw *
//!   R_pitch * R_roll) order against an X-forward, Y-right, Z-up basis.
//! - `DrawScale` (Float) — uniform scale, default 1.
//! - `DrawScale3D` (FVector) — per-axis scale, default (1, 1, 1).
//!
//! The final world position of a mesh-local vertex `v` is:
//!
//! ```text
//! v_world = Location + R_yaw * R_pitch * R_roll * (v * DrawScale * DrawScale3D)
//! ```
//!
//! This module exposes that as a single [`ActorTransform::apply`] call so
//! the StaticMesh walker can apply it once per vertex without growing
//! special-case knowledge of UE3 conventions.

use std::f32::consts::PI;

/// Decoded per-actor transform — exactly what the StaticMesh extractor
/// pulls out of the actor's tagged-property block.
///
/// Defaults match UE3's cook-omits-default-value behaviour: a vector at
/// the origin, no rotation, uniform scale.
#[derive(Debug, Clone, Copy)]
pub struct ActorTransform {
    pub location: [f32; 3],
    /// Raw UE3 rotator (pitch, yaw, roll) — 65536 == 360°.
    pub rotation: [i32; 3],
    pub draw_scale: f32,
    pub draw_scale_3d: [f32; 3],
}

impl Default for ActorTransform {
    fn default() -> Self {
        Self {
            location: [0.0; 3],
            rotation: [0; 3],
            draw_scale: 1.0,
            draw_scale_3d: [1.0; 3],
        }
    }
}

impl ActorTransform {
    /// Convert a UE3 rotator integer to radians.
    fn rot_to_radians(raw: i32) -> f32 {
        // 65536 raw == 2π. Use `wrapping_*` semantics: rotators are
        // intentionally allowed to overflow past 360° (UE3 stores them
        // mod 2^32, but for rotation math any multiple-of-2π offset
        // collapses anyway).
        (raw as f32) * (2.0 * PI / 65536.0)
    }

    /// Apply this transform to a mesh-local vertex, returning the
    /// world-space position.
    ///
    /// Rotation order matches `URotator::Quaternion()` (Engine/Src/UnMath.cpp):
    ///   `q = q_yaw * q_pitch * q_roll`, applied to a vector that lives
    ///   in an X-forward, Y-right, Z-up basis.
    ///
    /// We compose the three axis-angle rotations as plain 3x3 matrices
    /// and apply them in the same order. The math is direct and easy to
    /// audit; the performance cost is irrelevant since we're processing
    /// at-most a few hundred thousand vertices per map at build time.
    pub fn apply(&self, v: [f32; 3]) -> [f32; 3] {
        // 1. Scale (uniform * per-axis).
        let s = self.draw_scale;
        let ds3 = self.draw_scale_3d;
        let scaled = [v[0] * s * ds3[0], v[1] * s * ds3[1], v[2] * s * ds3[2]];

        // 2. Rotation. UE3 rotator components: rotation[0] = pitch (Y),
        // rotation[1] = yaw (Z), rotation[2] = roll (X).
        let pitch = Self::rot_to_radians(self.rotation[0]);
        let yaw = Self::rot_to_radians(self.rotation[1]);
        let roll = Self::rot_to_radians(self.rotation[2]);

        let rotated = rotate_yaw_pitch_roll(scaled, yaw, pitch, roll);

        // 3. Translate.
        [
            rotated[0] + self.location[0],
            rotated[1] + self.location[1],
            rotated[2] + self.location[2],
        ]
    }
}

/// Apply roll (X), then pitch (Y), then yaw (Z) — composed as
/// `R_yaw * R_pitch * R_roll * v` so the X-axis rotation runs first
/// against the input vector.
fn rotate_yaw_pitch_roll(v: [f32; 3], yaw: f32, pitch: f32, roll: f32) -> [f32; 3] {
    // Pre-compute sines/cosines once.
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sr, cr) = roll.sin_cos();

    // R_roll (around X): (x, y, z) → (x, cr*y - sr*z, sr*y + cr*z)
    let x1 = v[0];
    let y1 = cr * v[1] - sr * v[2];
    let z1 = sr * v[1] + cr * v[2];

    // R_pitch (around Y): (x, y, z) → (cp*x + sp*z, y, -sp*x + cp*z)
    let x2 = cp * x1 + sp * z1;
    let y2 = y1;
    let z2 = -sp * x1 + cp * z1;

    // R_yaw (around Z): (x, y, z) → (cy*x - sy*y, sy*x + cy*y, z)
    let x3 = cy * x2 - sy * y2;
    let y3 = sy * x2 + cy * y2;
    let z3 = z2;

    [x3, y3, z3]
}

/// Apply a transform to every vertex of every triangle in a list.
pub fn transform_triangles(triangles: &[[[f32; 3]; 3]], xf: &ActorTransform) -> Vec<[[f32; 3]; 3]> {
    triangles
        .iter()
        .map(|t| [xf.apply(t[0]), xf.apply(t[1]), xf.apply(t[2])])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn approx_vec(a: [f32; 3], b: [f32; 3]) -> bool {
        approx_eq(a[0], b[0]) && approx_eq(a[1], b[1]) && approx_eq(a[2], b[2])
    }

    #[test]
    fn identity_transform_returns_input() {
        let xf = ActorTransform::default();
        let v = xf.apply([100.0, 200.0, 300.0]);
        assert!(approx_vec(v, [100.0, 200.0, 300.0]));
    }

    #[test]
    fn translation_only() {
        let xf = ActorTransform {
            location: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let v = xf.apply([1.0, 2.0, 3.0]);
        assert!(approx_vec(v, [11.0, 22.0, 33.0]));
    }

    #[test]
    fn uniform_scale_applied_before_translate() {
        let xf = ActorTransform {
            location: [10.0, 0.0, 0.0],
            draw_scale: 2.0,
            ..Default::default()
        };
        // Local (1, 1, 1) * 2 = (2, 2, 2), + (10, 0, 0) = (12, 2, 2).
        let v = xf.apply([1.0, 1.0, 1.0]);
        assert!(approx_vec(v, [12.0, 2.0, 2.0]));
    }

    #[test]
    fn non_uniform_drawscale_3d() {
        let xf = ActorTransform {
            draw_scale_3d: [2.0, 3.0, 4.0],
            ..Default::default()
        };
        let v = xf.apply([1.0, 1.0, 1.0]);
        assert!(approx_vec(v, [2.0, 3.0, 4.0]));
    }

    #[test]
    fn yaw_90_rotates_x_to_y() {
        // 90° yaw (rotation around Z) maps +X → +Y.
        // Raw UE3 rotator value for 90° = 65536/4 = 16384.
        let xf = ActorTransform {
            rotation: [0, 16384, 0],
            ..Default::default()
        };
        let v = xf.apply([1.0, 0.0, 0.0]);
        assert!(
            approx_vec(v, [0.0, 1.0, 0.0]),
            "got {v:?}, expected ~(0, 1, 0)"
        );
    }

    #[test]
    fn pitch_90_rotates_x_to_negative_z() {
        // 90° pitch (rotation around Y) maps +X → -Z (right-handed Y).
        let xf = ActorTransform {
            rotation: [16384, 0, 0],
            ..Default::default()
        };
        let v = xf.apply([1.0, 0.0, 0.0]);
        assert!(
            approx_vec(v, [0.0, 0.0, -1.0]),
            "got {v:?}, expected ~(0, 0, -1)"
        );
    }

    #[test]
    fn roll_90_rotates_y_to_z() {
        // 90° roll (rotation around X) maps +Y → +Z.
        let xf = ActorTransform {
            rotation: [0, 0, 16384],
            ..Default::default()
        };
        let v = xf.apply([0.0, 1.0, 0.0]);
        assert!(
            approx_vec(v, [0.0, 0.0, 1.0]),
            "got {v:?}, expected ~(0, 0, 1)"
        );
    }

    #[test]
    fn scale_rotate_translate_compose() {
        // Combined: scale (1, 2, 1), yaw 90°, translate (10, 20, 30).
        // Vertex (1, 0, 0) → scale → (1, 0, 0) → yaw 90° → (0, 1, 0) →
        //   translate → (10, 21, 30).
        let xf = ActorTransform {
            location: [10.0, 20.0, 30.0],
            rotation: [0, 16384, 0],
            draw_scale: 1.0,
            draw_scale_3d: [1.0, 2.0, 1.0],
        };
        let v = xf.apply([1.0, 0.0, 0.0]);
        assert!(approx_vec(v, [10.0, 21.0, 30.0]), "got {v:?}");
    }

    #[test]
    fn transform_triangles_applies_to_all_vertices() {
        let xf = ActorTransform {
            location: [5.0, 0.0, 0.0],
            ..Default::default()
        };
        let tris = vec![[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
        let out = transform_triangles(&tris, &xf);
        assert_eq!(out.len(), 1);
        assert!(approx_vec(out[0][0], [5.0, 0.0, 0.0]));
        assert!(approx_vec(out[0][1], [6.0, 0.0, 0.0]));
        assert!(approx_vec(out[0][2], [5.0, 1.0, 0.0]));
    }

    #[test]
    fn transform_triangles_empty_input() {
        let xf = ActorTransform::default();
        let out = transform_triangles(&[], &xf);
        assert!(out.is_empty());
    }
}
