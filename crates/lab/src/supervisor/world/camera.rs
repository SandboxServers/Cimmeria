//! `client_camera`: mouse-look, zoom, and face-a-point, with real input.
//!
//! Mouse-look is DirectInput relative motion (the bridge feeds the game's
//! mouse device); zoom is the wheel. "Face" turns the camera until the
//! point projects near the screen centre, closed loop on the game's own
//! `worldToPixel`, so it needs no knowledge of the camera's field of view
//! or the mouse sensitivity (both learned while turning).
//!
//! After any face, an absolute pitch (`pitch_deg`) and yaw
//! (`yaw_offset_deg`) are closed loops on the camera readout and the camera
//! actor's yaw, so a spec lands on the same pose from any start.

use rmcp::schemars;
use serde_json::{json, Value};

use super::find::{find_entities, FindRequest};
use super::geometry::{bearing, wrap_pi, Pose, Screen, Vec3};
use super::io::WorldIo;
use super::memory;
use super::steer::{FaceConfig, FaceController, FaceStep, TurnModel};
use super::{PointArg, Steps, TargetArg, WorldError};

/// Frames for a mouse-look motion to show up in the next projection.
pub const LOOK_SETTLE_MS: u64 = 120;
/// Default face iterations.
pub const DEFAULT_FACE_STEPS: u32 = 10;
/// Wheel units per notch.
pub const WHEEL_NOTCH: i32 = 120;
/// How close an absolute pitch or yaw must land, degrees.
pub const ABS_TOLERANCE_DEG: f64 = 0.5;
/// Looks an absolute pitch or yaw may take before it fails.
pub const ABS_MAX_LOOKS: u32 = 4;

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
    /// Mouse-wheel notches (positive = wheel forward = zoom in).
    #[serde(default)]
    pub zoom_notches: Option<i32>,
    /// Zoom to this camera distance (100..775, 30 per notch) instead.
    #[serde(default)]
    pub zoom_to: Option<f32>,
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
    /// Absolute pitch after any face, readout degrees (D-SV4).
    #[serde(default)]
    pub pitch_deg: Option<f64>,
    /// Yaw after any face, degrees of camera world yaw in -180..=180; the
    /// +yaw_counts direction on a camera without yaw inversion (flag 0x4).
    #[serde(default)]
    pub yaw_offset_deg: Option<f64>,
    /// Camera distance; the same as `zoom_to` (an error to give both).
    #[serde(default)]
    pub zoom: Option<f32>,
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
                let level = io
                    .look(dx, dy, 0)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
                steps.used(level);
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

/// A readout gain worth dividing by: a camera read before the game loads its
/// settings reports 0 (the same guard as `click::restore_pitch_counts`).
fn usable_gain(g: f64) -> bool {
    g.is_finite() && g.abs() >= 1e-3
}

/// Closed loop on the readout's pitch_offset_deg (D-SV4).
async fn abs_pitch<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    want: f64,
) -> Result<Value, WorldError> {
    let t0 = io.now_ms();
    let mut looks = 0;
    // The last look's start pitch and counts, to learn the real gain from.
    let mut last: Option<(f64, i32)> = None;
    let mut learned: Option<f64> = None;
    loop {
        let r = io
            .camera_readout()
            .await
            .map_err(|e| steps.fail(io.now_ms(), "abs_pitch", e))?;
        let (Some(got), Some(read_gain)) = (
            r["pitch_offset_deg"].as_f64(),
            r["pitch_gain"].as_f64().filter(|g| usable_gain(*g)),
        ) else {
            return Err(steps.fail_with(
                io.now_ms(),
                "abs_pitch",
                "camera readout has no pitch_offset_deg or a usable pitch_gain",
                r,
            ));
        };
        let err = want - got;
        if err.abs() <= ABS_TOLERANCE_DEG {
            let out = json!({ "want": want, "got": got, "looks": looks });
            steps.record("abs_pitch", t0, io.now_ms(), out.clone());
            return Ok(out);
        }
        if looks == ABS_MAX_LOOKS {
            return Err(steps.fail_with(
                io.now_ms(),
                "abs_pitch",
                format!("the camera stopped at {got:.2} deg, not {want} deg, after {looks} looks"),
                r,
            ));
        }
        // Rotator units per count as measured by the last look, when it moved
        // the camera the way the readout's gain says (else the readout's gain).
        if let Some((from, sent)) = last {
            let units = (got - from) * 65536.0 / 360.0;
            let g = units / f64::from(sent);
            if (got - from).abs() > 0.05 && usable_gain(g) && g.signum() == read_gain.signum() {
                learned = Some(g);
            }
        }
        let gain = learned.unwrap_or(read_gain);
        // Rotator units: the readout's degrees, so the clamp sees the same offset.
        let pitch = (got * 65536.0 / 360.0) as i32;
        let counts = (err * 65536.0 / 360.0 / gain).round() as f32;
        let counts = memory::clamped_pitch_counts(pitch, gain as f32, counts).round() as i32;
        if counts == 0 {
            // The pitch limit allows no further move; more looks would not help.
            return Err(steps.fail_with(
                io.now_ms(),
                "abs_pitch",
                format!("the camera stopped at {got:.2} deg at its pitch limit, not {want} deg"),
                r,
            ));
        }
        let level = io
            .look(0, counts, 0)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
        steps.used(level);
        looks += 1;
        last = Some((got, counts));
        io.sleep(LOOK_SETTLE_MS).await;
    }
}

/// Closed loop on the camera actor's world yaw (D-SV4). Without a camera
/// pose the first look is the whole move, reported as `open_loop`.
async fn abs_yaw<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    want: f64,
) -> Result<Value, WorldError> {
    let t0 = io.now_ms();
    if want.abs() <= ABS_TOLERANCE_DEG {
        let out = json!({ "want": want, "got": 0.0, "looks": 0 });
        steps.record("abs_yaw", t0, io.now_ms(), out.clone());
        return Ok(out);
    }
    let r = io
        .camera_readout()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "abs_yaw", e))?;
    let Some(gain) = r["yaw_gain"].as_f64().filter(|g| usable_gain(*g)) else {
        return Err(steps.fail_with(
            io.now_ms(),
            "abs_yaw",
            "camera readout has no usable yaw_gain",
            r,
        ));
    };
    let y0 = io.camera_pose().await.ok().map(|p| p.yaw);
    // Signed gain: an inverted camera (0x4) turns the other way.
    let mut counts = (want * 65536.0 / 360.0 / gain).round() as i32;
    // A correction never exceeds four times the first move, so a look the
    // client swallowed (a tiny measured turn) cannot ask for a huge spin.
    let max_step = counts.saturating_abs().max(64).saturating_mul(4);
    let mut sent: i32 = 0;
    let mut looks = 0;
    let mut got = None;
    let open_loop = loop {
        let level = io
            .look(counts, 0, 0)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
        steps.used(level);
        looks += 1;
        sent = sent.saturating_add(counts);
        io.sleep(LOOK_SETTLE_MS).await;
        let turned = match y0 {
            Some(y0) => io
                .camera_pose()
                .await
                .ok()
                .map(|p| wrap_pi(p.yaw - y0).to_degrees()),
            None => None,
        };
        let Some(d) = turned else {
            break true;
        };
        got = Some(d);
        let err = want - d;
        if err.abs() <= ABS_TOLERANCE_DEG {
            break false;
        }
        if looks == ABS_MAX_LOOKS {
            return Err(steps.fail_with(
                io.now_ms(),
                "abs_yaw",
                format!("the camera stopped at {d:.2} deg, not {want} deg, after {looks} looks"),
                json!({ "want": want, "got": d, "looks": looks }),
            ));
        }
        // Scale the counts sent so far by the measured turn; the readout's
        // gain while the camera has not moved.
        let step = if d != 0.0 {
            err * f64::from(sent) / d
        } else {
            err * 65536.0 / 360.0 / gain
        };
        counts = (step.round() as i32).clamp(-max_step, max_step);
    };
    let mut out = json!({
        "want": want,
        "got": got,
        "looks": looks,
    });
    if open_loop {
        out["open_loop"] = json!(true);
    }
    steps.record("abs_yaw", t0, io.now_ms(), out.clone());
    Ok(out)
}

/// `client_camera`.
pub async fn run<W: WorldIo>(io: &mut W, req: CameraRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_camera", io.now_ms());
    if req.zoom.is_some() && req.zoom_to.is_some() {
        return Err(steps.fail(io.now_ms(), "arguments", "give zoom or zoom_to, not both"));
    }
    if req
        .yaw_offset_deg
        .is_some_and(|y| !(-180.0..=180.0).contains(&y))
    {
        return Err(steps.fail(
            io.now_ms(),
            "arguments",
            "yaw_offset_deg must be within -180..=180",
        ));
    }
    let zoom_to = req.zoom_to.or(req.zoom);
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
    let readout_before = io.camera_readout().await;

    let (dx, dy) = (req.yaw_counts.unwrap_or(0), req.pitch_counts.unwrap_or(0));
    let notches = match (zoom_to, &readout_before) {
        (Some(to), Ok(r)) => r["zoom"]
            .as_f64()
            .map(|from| memory::zoom_notches(from as f32, to))
            .unwrap_or(0),
        (Some(_), Err(e)) => {
            return Err(steps.fail(io.now_ms(), "zoom_to", format!("no camera readout: {e}")))
        }
        (None, _) => req.zoom_notches.unwrap_or(0),
    };
    let wheel = notches * WHEEL_NOTCH;
    if dx != 0 || dy != 0 || wheel != 0 {
        let t0 = io.now_ms();
        let level = io
            .look(dx, dy, wheel)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
        steps.used(level);
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

    let mut absolute = serde_json::Map::new();
    if let Some(want) = req.pitch_deg {
        absolute.insert("pitch".into(), abs_pitch(io, &mut steps, want).await?);
    }
    if let Some(want) = req.yaw_offset_deg {
        absolute.insert("yaw".into(), abs_yaw(io, &mut steps, want).await?);
    }

    let camera_after = io.camera_pose().await.ok();
    let readout = |r: Result<Value, String>| r.unwrap_or_else(|e| json!({ "error": e }));
    let mut out = json!({
        "camera_before": camera_before.map(|p| p.to_json()),
        "camera_after": camera_after.map(|p| p.to_json()),
        "view_before": readout(readout_before),
        "view_after": readout(io.camera_readout().await),
        "target": resolved,
        "face": face_json,
    });
    if !absolute.is_empty() {
        out["absolute"] = Value::Object(absolute);
    }
    Ok(steps.finish(io.now_ms(), out))
}
