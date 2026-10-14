//! Interaction poses in UAT specs: stand coordinates, aim points and TOML lines.

use serde::Deserialize;

/// How a pose aims the camera at its target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AimMode {
    /// Hover-verified on the entity (`Unit.MouseOver` is the target).
    #[default]
    Entity,
    /// A world point `aim_dy_m` above the target's origin; for bodies that
    /// never answer an entity hover (README F13).
    Point,
}

/// A place to stand next to a tagged target, and how to aim from there.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseSpec {
    /// Row-unique, `[a-z0-9-]+`; actions name it with `pose = "<id>"`.
    pub id: String,
    /// `spawnlist.tag` of the target (seed-checked, SV-04).
    pub tag: String,
    /// Horizontal stand-off from the target, metres (1.0 to 8.0).
    pub radius_m: f64,
    /// From the target, server x/z plane, 0 = +x, 90 = +z (0 <= b < 360).
    pub bearing_deg: f64,
    /// Stand height above the target's origin, metres (default 0.1).
    #[serde(default = "default_dy_m")]
    pub dy_m: f64,
    /// Absolute camera pitch after facing, readout degrees (D-SV4).
    #[serde(default)]
    pub pitch_deg: Option<f64>,
    /// Yaw after facing, degrees in the +yaw_counts direction (D-SV4).
    #[serde(default)]
    pub yaw_offset_deg: Option<f64>,
    /// Camera distance, 100 to 775.
    #[serde(default)]
    pub zoom: Option<f64>,
    #[serde(default)]
    pub aim: AimMode,
    /// `aim = "point"` only: metres above the target's origin.
    #[serde(default)]
    pub aim_dy_m: Option<f64>,
    /// Provenance, written by `lab calibrate` ("2026-10-12 5/5 b6a00db78")
    /// or by `lab record` ("recorded 2026-10-12").
    #[serde(default)]
    pub calibrated: Option<String>,
}

fn default_dy_m() -> f64 {
    0.1
}

impl PoseSpec {
    /// D-SV3: `target + radius * (cos b, 0, sin b)`, `y = target.y + dy_m`.
    pub fn stand_point(&self, target: [f64; 3]) -> [f64; 3] {
        let b = self.bearing_deg.to_radians();
        [
            target[0] + self.radius_m * b.cos(),
            target[1] + self.dy_m,
            target[2] + self.radius_m * b.sin(),
        ]
    }

    /// The aim point for `aim = "point"`: target with `y + aim_dy_m`.
    pub fn aim_point(&self, target: [f64; 3]) -> Option<[f64; 3]> {
        match (self.aim, self.aim_dy_m) {
            (AimMode::Point, Some(dy)) => Some([target[0], target[1] + dy, target[2]]),
            _ => None,
        }
    }

    /// One line, keys in this order, `None` fields left out:
    /// id, tag, radius_m (2 dp), bearing_deg (1 dp), dy_m (2 dp),
    /// pitch_deg (1 dp), yaw_offset_deg (1 dp), zoom (1 dp), aim (only
    /// when "point"), aim_dy_m (2 dp), calibrated.
    /// Shape: `{ id = "frost", tag = "...", radius_m = 4.16, ... }`.
    /// Values are rounded first and kept inside the validation ranges, so a
    /// line it writes always loads (359.96 deg writes as 0.0, a pitch of
    /// -78.75 as -78.7); control characters in `calibrated` become spaces,
    /// so the pose stays one line (D-SV2).
    pub fn to_inline_toml(&self) -> String {
        let bearing = match round_to(self.bearing_deg, 1) {
            b if b >= 360.0 => 0.0,
            b => b,
        };
        let mut keys = vec![
            format!("id = {}", toml_string(&self.id)),
            format!("tag = {}", toml_string(&self.tag)),
            format!(
                "radius_m = {:.2}",
                round_to(self.radius_m, 2).clamp(1.0, 8.0)
            ),
            format!("bearing_deg = {bearing:.1}"),
            format!("dy_m = {:.2}", round_to(self.dy_m, 2).clamp(-3.0, 3.0)),
        ];
        if let Some(p) = self.pitch_deg {
            keys.push(format!(
                "pitch_deg = {:.1}",
                round_to(p, 1).clamp(-78.7, 78.7)
            ));
        }
        if let Some(y) = self.yaw_offset_deg {
            keys.push(format!(
                "yaw_offset_deg = {:.1}",
                round_to(y, 1).clamp(-180.0, 180.0)
            ));
        }
        if let Some(z) = self.zoom {
            keys.push(format!("zoom = {:.1}", round_to(z, 1).clamp(100.0, 775.0)));
        }
        if self.aim == AimMode::Point {
            keys.push("aim = \"point\"".into());
        }
        if let Some(d) = self.aim_dy_m {
            keys.push(format!("aim_dy_m = {:.2}", round_to(d, 2).clamp(-3.0, 3.0)));
        }
        if let Some(c) = &self.calibrated {
            let one_line: String = c
                .chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch })
                .collect();
            keys.push(format!("calibrated = {}", toml_string(&one_line)));
        }
        format!("{{ {} }}", keys.join(", "))
    }
}

/// `v` rounded to `places` decimals, the value a `{:.N}` pose field holds.
fn round_to(v: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (v * k).round() / k
}

/// A string as TOML writes it, escapes included.
fn toml_string(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}
