//! The confirming click of a ground-targeted ability, and where it lands.
//!
//! A fixed point is fragile: on the Debug Area plaza the camera sits behind
//! the avatar, and a point straight ahead of it hovers as "your own avatar
//! covers the point" and the click is refused (DR-L2 soldier-1 rerun,
//! 2026-11-02). So the point is an offset from the live avatar, resolved when
//! the click happens, and a point the hover rejects is retried at the next
//! offset.

use rmcp::schemars;
use serde_json::{json, Value};

use crate::supervisor::world::click::{self, ClickRequest, Expect};
use crate::supervisor::world::geometry::{Vec3, UU_PER_METRE};
use crate::supervisor::world::io::{LiveWorld, WorldIo};
use crate::supervisor::world::{PointArg, Space};

/// Default distance from the avatar, metres.
pub const DEFAULT_DISTANCE_M: f64 = 8.0;
/// Farther and nearer rings tried after the first offset is rejected.
const ALT_DISTANCES_M: [f64; 2] = [6.0, 10.0];
/// How far either side of the first bearing the retries turn.
const SIDE_DEG: f64 = 30.0;
/// Each rejected try costs up to the hover budget (~12 s), so stop here.
const MAX_TRIES: usize = 6;
/// Pawn `Location` is the feet (probe B-P2), so the ground point keeps the
/// avatar's z.
const GROUND_DROP_UU: f64 = 0.0;

/// Where to left-click after a ground ability press. Give `x`, `y` and `z`
/// for an absolute point (as `client_world_click`'s `point`), or leave them
/// out for an offset from the avatar.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroundArg {
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub z: Option<f64>,
    /// With `x`, `y`, `z`: `server` metres (default) or `client` UE3 units.
    #[serde(default)]
    pub space: Option<Space>,
    /// Relative form: metres from the avatar along the bearing (default 8,
    /// 1 to 30).
    #[serde(default)]
    pub distance_m: Option<f64>,
    /// Relative form: degrees from the camera's forward direction, positive
    /// turning toward +yaw (default 0). A bearing off the camera axis keeps
    /// the point out from behind the avatar's body.
    #[serde(default)]
    pub bearing_deg: Option<f64>,
}

impl GroundArg {
    /// The absolute point, when `x`, `y` and `z` are all given.
    pub fn absolute(&self) -> Result<Option<PointArg>, String> {
        match (self.x, self.y, self.z) {
            (Some(x), Some(y), Some(z)) => Ok(Some(PointArg {
                x,
                y,
                z,
                space: self.space,
            })),
            (None, None, None) => Ok(None),
            _ => Err(
                "ground needs x, y and z together, or none of them for an offset from the avatar"
                    .into(),
            ),
        }
    }

    /// Reject bad arguments before the flow presses anything, so a typo does
    /// not leave a ground reticle open on the client.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(p) = self.absolute()? {
            if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
                return Err("ground x, y and z must be numbers".into());
            }
        }
        let d = self.distance_m.unwrap_or(DEFAULT_DISTANCE_M);
        if !d.is_finite() || !(1.0..=30.0).contains(&d) {
            return Err(format!("ground distance_m must be 1 to 30, not {d}"));
        }
        if !self.bearing_deg.unwrap_or(0.0).is_finite() {
            return Err("ground bearing_deg must be a number".into());
        }
        Ok(())
    }
}

/// One offset from the avatar and the client-space point it resolves to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub distance_m: f64,
    pub bearing_deg: f64,
    pub point: Vec3,
}

/// The offsets to try, first choice first, resolved against the avatar's
/// client position and the camera yaw (radians).
pub fn plan(player: Vec3, cam_yaw: f64, g: &GroundArg) -> Result<Vec<Candidate>, String> {
    let d = g.distance_m.unwrap_or(DEFAULT_DISTANCE_M);
    let b = g.bearing_deg.unwrap_or(0.0);
    if !d.is_finite() || !(1.0..=30.0).contains(&d) {
        return Err(format!("ground distance_m must be 1 to 30, not {d}"));
    }
    if !b.is_finite() {
        return Err("ground bearing_deg must be a number".into());
    }
    let mut offsets = vec![(d, b), (d, b + SIDE_DEG), (d, b - SIDE_DEG)];
    for alt in ALT_DISTANCES_M {
        offsets.push((alt, b + SIDE_DEG));
        offsets.push((alt, b - SIDE_DEG));
    }
    for alt in ALT_DISTANCES_M {
        offsets.push((alt, b));
    }
    let mut out: Vec<Candidate> = Vec::new();
    for (dist, bear) in offsets {
        if out
            .iter()
            .any(|c| (c.distance_m - dist).abs() < 1e-6 && (c.bearing_deg - bear).abs() < 1e-6)
        {
            continue;
        }
        let a = cam_yaw + bear.to_radians();
        out.push(Candidate {
            distance_m: dist,
            bearing_deg: bear,
            point: Vec3::new(
                player.x + dist * UU_PER_METRE * a.cos(),
                player.y + dist * UU_PER_METRE * a.sin(),
                player.z - GROUND_DROP_UU,
            ),
        });
    }
    out.truncate(MAX_TRIES);
    Ok(out)
}

/// A refused ground click: which step of the click refused, and why.
#[derive(Debug, Clone)]
pub struct ClickFail {
    pub point: PointArg,
    pub step: String,
    pub message: String,
}

impl ClickFail {
    /// A different point can fix it: the hover saw the avatar or another
    /// entity over the point, or the point was off screen.
    pub fn retryable(&self) -> bool {
        matches!(self.step.as_str(), "hover" | "on_screen")
    }
}

impl std::fmt::Display for ClickFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ground click at ({}, {}, {}) failed at {}: {}",
            self.point.x, self.point.y, self.point.z, self.step, self.message
        )
    }
}

/// What the confirm step needs from the client.
#[allow(async_fn_in_trait)]
pub trait GroundClicker {
    /// The avatar's client position and the camera yaw (radians).
    async fn view(&mut self) -> Result<(Vec3, f64), String>;
    /// Left-click a point; `rotate_camera` lets the click turn the view.
    async fn click(&mut self, p: PointArg, rotate_camera: bool) -> Result<Value, ClickFail>;
}

/// The live client.
pub struct LiveGround<'a>(pub LiveWorld<'a>);

impl GroundClicker for LiveGround<'_> {
    async fn view(&mut self) -> Result<(Vec3, f64), String> {
        let io = &mut self.0;
        let snap = io.snapshot().await?;
        let pose = snap
            .player()
            .and_then(|p| p.pose)
            .ok_or("the avatar has no pose")?;
        // The camera's own yaw is what the player sees; the pawn's is the
        // fallback when the camera chain does not resolve.
        let yaw = io.camera_pose().await.map(|c| c.yaw).unwrap_or(pose.yaw);
        Ok((pose.pos, yaw))
    }

    async fn click(&mut self, p: PointArg, rotate_camera: bool) -> Result<Value, ClickFail> {
        click_ground(&mut self.0, p, rotate_camera).await
    }
}

/// Left-click a ground point through the world-click path (real input).
/// `expect: nothing`: the effect is the ability send, which the observe loop
/// reads, not a target or window change.
pub(crate) async fn click_ground<W: WorldIo>(
    io: &mut W,
    p: PointArg,
    rotate_camera: bool,
) -> Result<Value, ClickFail> {
    let req = ClickRequest {
        point: Some(p),
        button: Some("left".into()),
        expect: Some(Expect::Nothing),
        settle_ms: Some(300),
        rotate_camera: Some(rotate_camera),
        ..ClickRequest::default()
    };
    click::run(io, "client_world_click", req, None)
        .await
        .map_err(|e| ClickFail {
            point: p,
            step: e.step.clone(),
            message: e.message.clone(),
        })
}

fn point_json(p: &PointArg) -> Value {
    json!({
        "x": p.x, "y": p.y, "z": p.z,
        "space": match p.space.unwrap_or(Space::Server) { Space::Server => "server", Space::Client => "client" },
    })
}

/// Click the ground point: the absolute one as given, or the first offset
/// from the avatar the hover accepts. The result names the point used and
/// every offset that was refused first.
pub async fn confirm<C: GroundClicker>(c: &mut C, g: GroundArg) -> Result<Value, String> {
    if let Some(p) = g.absolute()? {
        let v = c.click(p, true).await.map_err(|e| e.to_string())?;
        return Ok(json!({
            "point": point_json(&p), "button": "left", "native_level": v["native_level"],
        }));
    }
    let (pos, yaw) = c
        .view()
        .await
        .map_err(|e| format!("ground offset needs the avatar position and camera yaw: {e}"))?;
    let mut refused = Vec::new();
    for cand in plan(pos, yaw, &g)? {
        let p = PointArg {
            x: cand.point.x,
            y: cand.point.y,
            z: cand.point.z,
            space: Some(Space::Client),
        };
        // The offsets are chosen inside the view; a camera turn would move
        // the view out from under the later ones.
        match c.click(p, false).await {
            Ok(v) => {
                return Ok(json!({
                    "point": point_json(&p),
                    "distance_m": cand.distance_m,
                    "bearing_deg": cand.bearing_deg,
                    "button": "left",
                    "native_level": v["native_level"],
                    "refused": refused,
                }));
            }
            Err(e) if e.retryable() => refused.push(json!({
                "distance_m": cand.distance_m,
                "bearing_deg": cand.bearing_deg,
                "step": e.step,
                "error": e.message,
            })),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(format!(
        "no ground offset around the avatar could be clicked; refused: {}",
        Value::Array(refused)
    ))
}
