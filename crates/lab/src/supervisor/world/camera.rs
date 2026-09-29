//! `client_camera`: mouse-look, zoom, and face-a-point, with real input.
//!
//! Mouse-look is DirectInput relative motion (the bridge feeds the game's
//! mouse device); zoom is the wheel. "Face" turns the camera until the
//! point projects near the screen centre, closed loop on the game's own
//! `worldToPixel`, so it needs no knowledge of the camera's field of view
//! or the mouse sensitivity (both learned while turning).

use rmcp::schemars;
use serde_json::{json, Value};

use super::find::{find_entities, FindRequest};
use super::geometry::{bearing, wrap_pi, Pose, Screen, Vec3};
use super::io::WorldIo;
use super::steer::{FaceConfig, FaceController, FaceStep, TurnModel};
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

/// Frames for a mouse-look motion to show up in the next projection.
pub const LOOK_SETTLE_MS: u64 = 120;
/// Default face iterations.
pub const DEFAULT_FACE_STEPS: u32 = 10;
/// Wheel units per notch.
pub const WHEEL_NOTCH: i32 = 120;

/// `client_camera` arguments.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct CameraRequest {
    /// Raw mouse-look motion, DirectInput counts (+ right on this client
    /// unless the result's learned gain says otherwise).
    #[serde(default)]
    pub yaw_counts: Option<i32>,
    /// Raw vertical mouse-look motion, DirectInput counts.
    #[serde(default)]
    pub pitch_counts: Option<i32>,
    /// Mouse-wheel notches (positive = wheel forward).
    #[serde(default)]
    pub zoom_notches: Option<i32>,
    /// Face this entity id ...
    #[serde(default)]
    pub face_entity_id: Option<u32>,
    /// ... or the nearest entity with this name ...
    #[serde(default)]
    pub face_name: Option<String>,
    /// ... or this world point.
    #[serde(default)]
    pub face_point: Option<PointArg>,
    /// Max look iterations while facing (default 10).
    #[serde(default)]
    pub max_steps: Option<u32>,
}

/// What a face run did.
#[derive(Debug, Clone)]
pub struct FaceReport {
    pub centred: bool,
    pub screen: Option<Screen>,
    pub pixel: Option<(f64, f64)>,
    pub looks: u32,
    pub searches: u32,
    pub yaw_model: TurnModel,
}

impl FaceReport {
    pub fn to_json(&self) -> Value {
        json!({
            "centred": self.centred,
            "screen": self.screen.map(|s| json!([s.w, s.h])),
            "pixel": self.pixel.map(|(x, y)| json!([x.round(), y.round()])),
            "looks": self.looks,
            "searches_behind_camera": self.searches,
            "yaw_gain": self.yaw_model.to_json(),
        })
    }
}

/// Where the point to face is now: an entity's actor (re-read each step,
/// since NPCs move) or a fixed point.
#[derive(Debug, Clone, Copy)]
pub enum Aim {
    Actor { actor: u32, last: Vec3 },
    Point(Vec3),
}

impl Aim {
    pub async fn position<W: WorldIo>(&mut self, io: &mut W) -> Vec3 {
        match self {
            Aim::Point(p) => *p,
            Aim::Actor { actor, last } => {
                if let Ok(p) = io.actor_pose(*actor).await {
                    *last = p.pos;
                }
                *last
            }
        }
    }
}

/// Yaw the camera must turn to look at `to`, from the camera actor when the
/// chain resolves, else from the player's facing.
async fn yaw_hint<W: WorldIo>(io: &mut W, player: Option<Pose>, to: Vec3) -> Option<f64> {
    let from = match io.camera_pose().await {
        Ok(c) => c,
        Err(_) => player?,
    };
    Some(wrap_pi(bearing(from.pos, to) - from.yaw))
}

/// Turn the camera until `aim` is near the screen centre. Counts as real
/// input when it moved the camera.
pub async fn face<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    aim: &mut Aim,
    player: Option<Pose>,
    model: TurnModel,
    max_steps: u32,
) -> Result<FaceReport, WorldError> {
    let t0 = io.now_ms();
    let mut ctl = FaceController::new(FaceConfig::default(), model);
    let mut report = FaceReport {
        centred: false,
        screen: None,
        pixel: None,
        looks: 0,
        searches: 0,
        yaw_model: model,
    };
    for _ in 0..=max_steps {
        let pos = aim.position(io).await;
        let pr = io
            .project(&[pos])
            .await
            .map_err(|e| steps.fail(io.now_ms(), "project", e))?;
        let pixel = pr.points.first().copied().flatten();
        report.screen = Some(pr.screen);
        report.pixel = pixel;
        let hint = if pixel.is_none() {
            yaw_hint(io, player, pos).await
        } else {
            None
        };
        match ctl.step(pr.screen, pixel, hint) {
            FaceStep::Done => {
                report.centred = true;
                break;
            }
            FaceStep::Look { dx, dy } => {
                if report.looks == max_steps {
                    break;
                }
                io.look(dx, dy, 0)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
                steps.used(NativeLevel::RealInput);
                report.looks += 1;
                io.sleep(LOOK_SETTLE_MS).await;
            }
        }
    }
    report.searches = ctl.searches;
    report.yaw_model = ctl.yaw;
    steps.record("face", t0, io.now_ms(), report.to_json());
    Ok(report)
}

/// Resolve a face target to an [`Aim`].
pub async fn resolve_aim<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    target: &TargetArg,
) -> Result<(Aim, Value), WorldError> {
    if let Some(p) = target.point {
        let c = p.to_client();
        return Ok((Aim::Point(c), json!({ "point": c.to_json() })));
    }
    let req = FindRequest {
        entity_id: target.entity_id,
        name: target.name.clone(),
        rendered_only: true,
        limit: Some(1),
        project: Some(false),
        ..FindRequest::default()
    };
    let found = find_entities(io, &req, steps).await?;
    let Some(m) = found.matches.into_iter().next() else {
        return Err(steps.fail(
            io.now_ms(),
            "resolve_target",
            "no rendered entity matches (see client_entity_find)",
        ));
    };
    let pos = m.pose.map(|p| p.pos).ok_or_else(|| {
        steps.fail(
            io.now_ms(),
            "resolve_target",
            format!("entity {} has no pose", m.id),
        )
    })?;
    Ok((
        Aim::Actor {
            actor: m.actor,
            last: pos,
        },
        m.to_json(),
    ))
}

/// `client_camera`.
pub async fn run<W: WorldIo>(io: &mut W, req: CameraRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_camera", io.now_ms());
    let target = TargetArg {
        entity_id: req.face_entity_id,
        name: req.face_name.clone(),
        point: req.face_point,
    };
    steps.subject(target.describe());
    io.ensure_focus()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "focus", e))?;
    let camera_before = io.camera_pose().await.ok();

    let (dx, dy) = (req.yaw_counts.unwrap_or(0), req.pitch_counts.unwrap_or(0));
    let wheel = req.zoom_notches.unwrap_or(0) * WHEEL_NOTCH;
    if dx != 0 || dy != 0 || wheel != 0 {
        let t0 = io.now_ms();
        io.look(dx, dy, wheel)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
        steps.used(NativeLevel::RealInput);
        io.sleep(LOOK_SETTLE_MS).await;
        steps.record(
            "look",
            t0,
            io.now_ms(),
            json!({ "dx": dx, "dy": dy, "wheel": wheel }),
        );
    }

    let mut face_json = Value::Null;
    let mut resolved = Value::Null;
    if !target.is_empty() {
        let (mut aim, r) = resolve_aim(io, &mut steps, &target).await?;
        resolved = r;
        let player = io
            .snapshot()
            .await
            .ok()
            .and_then(|s| s.player().and_then(|p| p.pose));
        let rep = face(
            io,
            &mut steps,
            &mut aim,
            player,
            TurnModel::default(),
            req.max_steps.unwrap_or(DEFAULT_FACE_STEPS),
        )
        .await?;
        if !rep.centred {
            return Err(steps.fail_with(
                io.now_ms(),
                "face",
                "could not bring the target to the screen centre",
                rep.to_json(),
            ));
        }
        face_json = rep.to_json();
    }

    let camera_after = io.camera_pose().await.ok();
    let out = json!({
        "camera_before": camera_before.map(|p| p.to_json()),
        "camera_after": camera_after.map(|p| p.to_json()),
        "target": resolved,
        "face": face_json,
    });
    Ok(steps.finish(io.now_ms(), out))
}
