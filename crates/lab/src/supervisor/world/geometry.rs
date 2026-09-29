//! World geometry for the world tools: client (UE3) vs server (BigWorld)
//! coordinates, bearings, and the screen rectangle.
//!
//! The client keeps actor positions in UE3 units with Z up. The server and
//! the GM console (`.location`, `.gotoxyz`, `server_entity_get`) use
//! BigWorld metres with Y up. The mapping (docs/engine/ue3-package-format.md
//! § Coordinate system, and 100 UE3 units per metre from
//! docs/engine/navmesh-build-pipeline.md §1):
//!
//! ```text
//! client (X, Y, Z) = server (z, x, y) * 100
//! ```
//!
//! Yaw is the actor's `FRotator` yaw: 65536 units per turn, measured from
//! +X toward +Y, so the facing vector is `(cos yaw, sin yaw)` in client X/Y.

use std::f64::consts::{PI, TAU};

use serde_json::{json, Value};

/// UE3 units per BigWorld metre.
pub const UU_PER_METRE: f64 = 100.0;

/// A point or vector. Which space it is in is the caller's business; the
/// helpers below say which one they take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn to_json(self) -> Value {
        let r = |v: f64| (v * 100.0).round() / 100.0;
        json!({ "x": r(self.x), "y": r(self.y), "z": r(self.z) })
    }
}

/// Server (BigWorld metres, Y up) to client (UE3 units, Z up).
pub fn server_to_client(p: Vec3) -> Vec3 {
    Vec3::new(p.z * UU_PER_METRE, p.x * UU_PER_METRE, p.y * UU_PER_METRE)
}

/// Client (UE3 units, Z up) to server (BigWorld metres, Y up).
pub fn client_to_server(p: Vec3) -> Vec3 {
    Vec3::new(p.y / UU_PER_METRE, p.z / UU_PER_METRE, p.x / UU_PER_METRE)
}

/// Horizontal (client X/Y) distance in metres between two client points.
pub fn horizontal_m(a: Vec3, b: Vec3) -> f64 {
    let d = b.sub(a);
    (d.x * d.x + d.y * d.y).sqrt() / UU_PER_METRE
}

/// Angle wrapped into (-PI, PI].
pub fn wrap_pi(a: f64) -> f64 {
    let mut r = a.rem_euclid(TAU);
    if r > PI {
        r -= TAU;
    }
    r
}

/// Rotator units (65536 per turn) to radians in (-PI, PI].
pub fn rotator_to_rad(units: i32) -> f64 {
    wrap_pi(units as f64 * TAU / 65536.0)
}

/// Bearing from `from` to `to` in client X/Y, radians, same convention as
/// the actor yaw.
pub fn bearing(from: Vec3, to: Vec3) -> f64 {
    (to.y - from.y).atan2(to.x - from.x)
}

/// How far `yaw` must turn to face `to` from `from` (positive = increase yaw).
pub fn yaw_error(from: Vec3, yaw: f64, to: Vec3) -> f64 {
    wrap_pi(bearing(from, to) - yaw)
}

/// Where an actor is and which way it faces: client position (UE3 units)
/// and rotator pitch/yaw in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub pos: Vec3,
    pub yaw: f64,
    pub pitch: f64,
}

impl Pose {
    pub fn to_json(self) -> Value {
        json!({
            "client": self.pos.to_json(),
            "server": client_to_server(self.pos).to_json(),
            "yaw_deg": (self.yaw.to_degrees() * 10.0).round() / 10.0,
        })
    }
}

/// The game view in UI pixels (the CEGUI root window).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub w: f64,
    pub h: f64,
}

impl Screen {
    pub fn centre(self) -> (f64, f64) {
        (self.w / 2.0, self.h / 2.0)
    }

    /// Inside the screen with `margin` pixels to spare on every side.
    pub fn contains(self, (x, y): (f64, f64), margin: f64) -> bool {
        x >= margin && y >= margin && x <= self.w - margin && y <= self.h - margin
    }

    /// Horizontal offset from the centre as a fraction of the half width
    /// (-1 left edge, +1 right edge).
    pub fn x_error(self, (x, _): (f64, f64)) -> f64 {
        let half = (self.w / 2.0).max(1.0);
        (x - half) / half
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_client_round_trip_swizzles_and_scales() {
        let server = Vec3::new(-295.407, 68.511, -169.726);
        let client = server_to_client(server);
        // HUD (X, Y, Z) -> world (Z, X, Y) * 100, as the UE3 package doc says.
        assert!((client.x - -16972.6).abs() < 1e-6);
        assert!((client.y - -29540.7).abs() < 1e-6);
        assert!((client.z - 6851.1).abs() < 1e-6);
        let back = client_to_server(client);
        assert!((back.x - server.x).abs() < 1e-9);
        assert!((back.y - server.y).abs() < 1e-9);
        assert!((back.z - server.z).abs() < 1e-9);
    }

    #[test]
    fn horizontal_distance_ignores_height_and_is_in_metres() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(300.0, 400.0, 9000.0);
        assert!((horizontal_m(a, b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn rotator_units_wrap_to_signed_radians() {
        assert!((rotator_to_rad(16384) - PI / 2.0).abs() < 1e-9);
        assert!((rotator_to_rad(49152) + PI / 2.0).abs() < 1e-9);
        assert!((rotator_to_rad(65536 + 16384) - PI / 2.0).abs() < 1e-9);
        assert!((rotator_to_rad(-16384) + PI / 2.0).abs() < 1e-9);
    }

    #[test]
    fn yaw_error_takes_the_short_way_round() {
        let o = Vec3::new(0.0, 0.0, 0.0);
        // Facing +X (yaw 0), target at +Y: turn +90 degrees.
        let e = yaw_error(o, 0.0, Vec3::new(0.0, 10.0, 0.0));
        assert!((e - PI / 2.0).abs() < 1e-9);
        // Facing 170 degrees, target at -170 degrees: +20, not -340.
        let yaw = 170f64.to_radians();
        let t = Vec3::new(
            (-170f64).to_radians().cos(),
            (-170f64).to_radians().sin(),
            0.0,
        );
        assert!((yaw_error(o, yaw, t).to_degrees() - 20.0).abs() < 1e-6);
    }

    #[test]
    fn screen_contains_honours_the_margin() {
        let s = Screen {
            w: 1024.0,
            h: 768.0,
        };
        assert!(s.contains((512.0, 384.0), 20.0));
        assert!(!s.contains((10.0, 384.0), 20.0));
        assert!(!s.contains((512.0, 760.0), 20.0));
        assert!((s.x_error((768.0, 0.0)) - 0.5).abs() < 1e-9);
    }
}
