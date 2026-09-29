//! `client_move_to`: walk to a point or an entity with the movement keys.
//!
//! Forward is the `W` key held down; turning is mouse-look (DirectInput
//! motion), exactly what a player does. The loop reads the player's actor
//! pose every tick (one memory read), steers with [`MoveController`], and
//! ends on arrival, on a snag it cannot get out of, or on the timeout.
//! Every exit releases the keys it pressed.

use rmcp::schemars;
use serde_json::{json, Value};

use super::find::{find_entities, FindRequest};
use super::geometry::{horizontal_m, Pose, Vec3};
use super::io::WorldIo;
use super::steer::{MoveConfig, MoveController, MoveOutcome, TurnModel, Unstick};
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

pub const FORWARD_KEY: &str = "W";
pub const DEFAULT_TICK_MS: u64 = 100;
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Default arrival radius for an entity target (stop next to it, not in it).
pub const ENTITY_ARRIVAL_M: f64 = 2.5;
/// A jump this large in one tick is a teleport or a zone change, not a walk.
pub const TELEPORT_M: f64 = 30.0;
/// How long a strafe unstick holds its key.
const STRAFE_MS: u64 = 600;
/// Path samples kept in the result.
const PATH_SAMPLE_MS: u64 = 1000;

/// `client_move_to` arguments.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct MoveRequest {
    /// Walk to this entity (it may move; the goal follows it) ...
    #[serde(default)]
    pub entity_id: Option<u32>,
    /// ... or the nearest entity with this name ...
    #[serde(default)]
    pub name: Option<String>,
    /// ... or this point.
    #[serde(default)]
    pub point: Option<PointArg>,
    /// Points to pass through first, in order (same space rules as `point`).
    #[serde(default)]
    pub waypoints: Vec<PointArg>,
    /// Arrival radius, metres (default 1.5 for a point, 2.5 for an entity).
    #[serde(default)]
    pub arrival_m: Option<f64>,
    /// Give up after this long, ms (default 60000, max 600000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// A leg's goal.
#[derive(Debug, Clone, Copy)]
enum Goal {
    Point(Vec3),
    Actor { id: u32, actor: u32, last: Vec3 },
}

impl Goal {
    async fn position<W: WorldIo>(&mut self, io: &mut W) -> Vec3 {
        match self {
            Goal::Point(p) => *p,
            Goal::Actor { actor, last, .. } => {
                if let Ok(p) = io.actor_pose(*actor).await {
                    *last = p.pos;
                }
                *last
            }
        }
    }

    fn describe(&self) -> Value {
        match self {
            Goal::Point(p) => json!({ "point": p.to_json() }),
            Goal::Actor { id, last, .. } => json!({ "entity_id": id, "at": last.to_json() }),
        }
    }
}

/// The player's actor, re-resolved when a read fails.
async fn player_actor<W: WorldIo>(io: &mut W) -> Result<(u32, Pose), String> {
    let snap = io.snapshot().await?;
    let p = snap
        .player()
        .ok_or_else(|| format!("player entity {} is not in the entity map", snap.player_id))?;
    let pose = p
        .pose
        .ok_or_else(|| format!("player entity {} has no actor", snap.player_id))?;
    Ok((p.actor, pose))
}

/// Release what the walk may be holding. Errors are ignored on purpose: a
/// cleanup failure must not mask the walk's own result.
async fn release<W: WorldIo>(io: &mut W) {
    for k in [FORWARD_KEY, "A", "D"] {
        let _ = io.key(k, false).await;
    }
}

/// `client_move_to`.
pub async fn run<W: WorldIo>(io: &mut W, req: MoveRequest) -> Result<Value, WorldError> {
    let r = walk(io, &req).await;
    release(io).await;
    r
}

async fn walk<W: WorldIo>(io: &mut W, req: &MoveRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_move_to", io.now_ms());
    let target = TargetArg {
        entity_id: req.entity_id,
        name: req.name.clone(),
        point: req.point,
    };
    steps.subject(target.describe());
    if target.is_empty() && req.waypoints.is_empty() {
        return Err(steps.fail(
            io.now_ms(),
            "arguments",
            "give entity_id, name, point or waypoints",
        ));
    }
    io.ensure_focus()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "focus", e))?;

    // Legs: waypoints, then the target.
    let mut legs: Vec<(Goal, f64)> = req
        .waypoints
        .iter()
        .map(|w| (Goal::Point(w.to_client()), req.arrival_m.unwrap_or(1.5)))
        .collect();
    if let Some(p) = target.point {
        legs.push((Goal::Point(p.to_client()), req.arrival_m.unwrap_or(1.5)));
    } else if target.entity_id.is_some() || target.name.is_some() {
        let fr = FindRequest {
            entity_id: target.entity_id,
            name: target.name.clone(),
            rendered_only: true,
            limit: Some(1),
            project: Some(false),
            ..FindRequest::default()
        };
        let r = find_entities(io, &fr, &mut steps).await?;
        let Some(m) = r.matches.into_iter().next() else {
            return Err(steps.fail(
                io.now_ms(),
                "resolve_target",
                "no rendered entity matches (see client_entity_find)",
            ));
        };
        let Some(pose) = m.pose else {
            return Err(steps.fail(
                io.now_ms(),
                "resolve_target",
                format!("entity {} has no pose", m.id),
            ));
        };
        steps.subject(json!({ "entity_id": m.id, "name": m.name() }));
        legs.push((
            Goal::Actor {
                id: m.id,
                actor: m.actor,
                last: pose.pos,
            },
            req.arrival_m.unwrap_or(ENTITY_ARRIVAL_M),
        ));
    }

    let (mut actor, start) = player_actor(io)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_player", e))?;
    let timeout = req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).min(600_000);
    let t_start = io.now_ms();
    let mut turn = TurnModel::default();
    let mut path = vec![json!({ "t_ms": 0, "at": start.pos.to_json() })];
    let mut last_sample = t_start;
    let mut leg_reports = Vec::new();
    let mut prev = start;
    let mut forward = false;

    for (i, (mut goal, arrival)) in legs.into_iter().enumerate() {
        let t_leg = io.now_ms();
        let mut ctl = MoveController::new(
            MoveConfig {
                arrival_m: arrival,
                ..MoveConfig::default()
            },
            turn,
        );
        let mut unsticks = Vec::new();
        let outcome = loop {
            let now = io.now_ms();
            if now.saturating_sub(t_start) > timeout {
                let goal_pos = goal.position(io).await;
                return Err(steps.fail_with(
                    now,
                    &format!("leg_{i}"),
                    format!("timed out after {timeout} ms"),
                    json!({
                        "at": prev.to_json(),
                        "goal": goal.describe(),
                        "distance_m": horizontal_m(prev.pos, goal_pos),
                        "turn_gain": ctl.turn.to_json(),
                        "unsticks": unsticks,
                    }),
                ));
            }
            let pose = match io.actor_pose(actor).await {
                Ok(p) => p,
                Err(_) => {
                    let (a, p) = player_actor(io)
                        .await
                        .map_err(|e| steps.fail(io.now_ms(), "read_player", e))?;
                    actor = a;
                    p
                }
            };
            let jumped = horizontal_m(prev.pos, pose.pos);
            if jumped > TELEPORT_M {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!("the player moved {jumped:.1} m in one tick (teleport or zone change)"),
                    json!({ "from": prev.to_json(), "to": pose.to_json() }),
                ));
            }
            prev = pose;
            if now.saturating_sub(last_sample) >= PATH_SAMPLE_MS {
                path.push(json!({ "t_ms": now - t_start, "at": pose.pos.to_json() }));
                last_sample = now;
            }
            let goal_pos = goal.position(io).await;
            let s = ctl.step(now, pose, goal_pos);
            if let Some(o) = s.outcome {
                break o;
            }
            if let Some(u) = s.unstick {
                unsticks.push(u.name());
                match u {
                    Unstick::Jump => {
                        io.key(u.key(), true)
                            .await
                            .map_err(|e| steps.fail(io.now_ms(), "unstick", e))?;
                        io.sleep(80).await;
                        io.key(u.key(), false)
                            .await
                            .map_err(|e| steps.fail(io.now_ms(), "unstick", e))?;
                    }
                    Unstick::StrafeRight | Unstick::StrafeLeft => {
                        io.key(u.key(), true)
                            .await
                            .map_err(|e| steps.fail(io.now_ms(), "unstick", e))?;
                        io.sleep(STRAFE_MS).await;
                        io.key(u.key(), false)
                            .await
                            .map_err(|e| steps.fail(io.now_ms(), "unstick", e))?;
                    }
                }
            }
            io.look(s.dx, 0, 0)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "mouse_look", e))?;
            if s.forward != forward {
                io.key(FORWARD_KEY, s.forward)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "forward_key", e))?;
                forward = s.forward;
            }
            if s.dx != 0 || s.forward {
                steps.used(NativeLevel::RealInput);
            }
            io.sleep(DEFAULT_TICK_MS).await;
        };
        turn = ctl.turn;
        let report = json!({
            "goal": goal.describe(),
            "arrival_m": arrival,
            "ms": io.now_ms() - t_leg,
            "unsticks": unsticks,
            "standing_turn": ctl.standing_turn,
            "outcome": match outcome {
                MoveOutcome::Arrived { distance_m } => json!({ "arrived": true, "distance_m": distance_m }),
                MoveOutcome::Stuck { distance_m, attempts } => json!({ "arrived": false, "stuck": true, "distance_m": distance_m, "attempts": attempts }),
            },
        });
        steps.record(&format!("leg_{i}"), t_leg, io.now_ms(), report.clone());
        leg_reports.push(report);
        if let MoveOutcome::Stuck {
            distance_m,
            attempts,
        } = outcome
        {
            return Err(steps.fail_with(
                io.now_ms(),
                &format!("leg_{i}"),
                format!("stuck {distance_m:.1} m short after {attempts} unstick attempts"),
                json!({ "at": prev.to_json(), "goal": goal.describe() }),
            ));
        }
    }
    if forward {
        io.key(FORWARD_KEY, false)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "forward_key", e))?;
    }
    path.push(json!({ "t_ms": io.now_ms() - t_start, "at": prev.pos.to_json() }));
    let out = json!({
        "arrived": true,
        "start": start.to_json(),
        "end": prev.to_json(),
        "legs": leg_reports,
        "turn_gain": turn.to_json(),
        "path": path,
    });
    Ok(steps.finish(io.now_ms(), out))
}
