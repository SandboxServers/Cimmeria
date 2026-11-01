//! `client_move_to`: walk to a point or an entity with the movement keys.
//!
//! Forward is the `W` key held down; turning is the client's bound turn keys
//! (TurnLeft / TurnRight, Q / E by default), because the pawn's heading ignores
//! mouse-look. The loop reads the player's actor pose every tick (one memory
//! read), steers with [`MoveController`], and ends on arrival, on a snag it
//! cannot get out of, or on the timeout. Every exit releases the keys it
//! pressed.

use rmcp::schemars;
use serde_json::{json, Value};

use super::coast::CoastGlue;
use super::find::{find_entities, FindRequest};
use super::geometry::{horizontal_m, Pose, Vec3};
use super::io::WorldIo;
use super::ledge::{self, Guard};
use super::move_probe::input_probe;
use super::move_proof::{arrival_checks, server_end_json, stop_report, tried_list};
use super::steer::{MoveConfig, MoveController, MoveOutcome, TurnKey, TurnRate};
use super::turn_state::{apply_turn, TurnKeys};
use super::unstick::run_unstick;
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};
use crate::proof::stop::{StopKind, StopReport};
use crate::proof::Evidence;

pub const FORWARD_KEY: &str = "W";
/// Backward key, for the blocked-vs-dead probe retry.
pub(super) const BACKWARD_KEY: &str = "S";

pub const DEFAULT_TICK_MS: u64 = 100;
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Default arrival radius for an entity target (stop next to it, not in
/// it). Inside the 3.5 to 4.5 m a right-click or a trainer window needs, so a
/// target already in range is a no-op and a walk ends in range (#1660).
pub const ENTITY_ARRIVAL_M: f64 = 4.0;
/// Navmesh corridor corners need a tighter pass radius than the final goal:
/// cutting one by 1.5 m can put the next straight leg through a wall.
pub const WAYPOINT_ARRIVAL_M: f64 = 0.5;

/// What a failed walk tells the caller to try. The walk is a straight line
/// with no path-finding.
pub(super) const FAIL_HINT: &str = "client_move_to walks a straight line and cannot path-find. Go round the obstacle with waypoints, hold a movement key with a ui_sequence key step (hold_ms), or teleport on the server and click without walking; then stand 3.5 to 4.5 m from the target.";
/// A jump this large in one tick is a teleport or a zone change, not a walk.
pub const TELEPORT_M: f64 = 30.0;
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
    #[serde(default)]
    pub template_id: Option<i32>,
    #[serde(default)]
    pub alive: Option<bool>,
    #[serde(default)]
    pub nearest: Option<bool>,
    /// Zero-based match index after filtering and ordering.
    #[serde(default)]
    pub index: Option<usize>,
    /// ... or this point.
    #[serde(default)]
    pub point: Option<PointArg>,
    /// Points to pass through first, in order (same space rules as `point`).
    #[serde(default)]
    pub waypoints: Vec<PointArg>,
    /// Arrival radius for intermediate waypoints, metres (default 0.5).
    /// The final point or entity still uses `arrival_m`.
    #[serde(default)]
    pub waypoint_arrival_m: Option<f64>,
    /// Walk off ledges too: without it a walk stops before a drop of 2 m (C14).
    #[serde(default)]
    pub allow_drops: Option<bool>,
    /// Arrival radius, metres (default 1.5 for a point, 4.0 for an entity).
    #[serde(default)]
    pub arrival_m: Option<f64>,
    /// Give up after this long, ms (default 60000, max 600000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// A leg's goal.
#[derive(Debug, Clone, Copy)]
pub(super) enum Goal {
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

    pub(super) fn describe(&self) -> Value {
        match self {
            Goal::Point(p) => json!({ "point": p.to_json() }),
            Goal::Actor { id, last, .. } => json!({ "entity_id": id, "at": last.to_json() }),
        }
    }
}

/// The player's actor, re-resolved when a read fails.
async fn player_actor<W: WorldIo>(io: &mut W) -> Result<(u32, Pose, u32), String> {
    let snap = io.snapshot().await?;
    let p = snap
        .player()
        .ok_or_else(|| format!("player entity {} is not in the entity map", snap.player_id))?;
    let pose = p
        .pose
        .ok_or_else(|| format!("player entity {} has no actor", snap.player_id))?;
    Ok((p.actor, pose, snap.player_id))
}

/// The stop report of a failed leg (C8). `elapsed_ms` is since the walk began.
fn stop_of(
    kind: StopKind,
    leg: usize,
    elapsed_ms: u64,
    unsticks: &[&str],
    probed: bool,
    at: Pose,
    here: Pose,
) -> StopReport {
    stop_report(
        kind,
        &format!("leg_{leg}"),
        elapsed_ms as f64 / 1000.0,
        tried_list(unsticks, probed),
        at.pos,
        here.pos,
    )
}

/// Release what the walk may be holding. Errors are ignored on purpose: a
/// cleanup failure must not mask the walk's own result.
async fn release<W: WorldIo>(io: &mut W, turn: &TurnKeys) {
    for k in [
        FORWARD_KEY,
        BACKWARD_KEY,
        "A",
        "D",
        turn.left.as_str(),
        turn.right.as_str(),
    ] {
        let _ = io.key(k, false).await;
    }
}

/// Let go of every key a walk may hold, including the client's bound turn
/// keys (read again, since the cancelled walk's own copy is gone). For a
/// dropped tool future; errors are ignored.
pub async fn release_stray<W: WorldIo>(io: &mut W) {
    let mut keys = TurnKeys::default();
    if let Ok((left, right)) = io.turn_keys().await {
        keys.left = left;
        keys.right = right;
    }
    release(io, &keys).await;
}

/// `client_move_to`.
pub async fn run<W: WorldIo>(io: &mut W, req: MoveRequest) -> Result<Value, WorldError> {
    let mut turn_keys = TurnKeys::default();
    let r = walk(io, &req, &mut turn_keys).await;
    release(io, &turn_keys).await;
    r
}

async fn walk<W: WorldIo>(
    io: &mut W,
    req: &MoveRequest,
    turn_keys: &mut TurnKeys,
) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_move_to", io.now_ms());
    let target = TargetArg {
        entity_id: req.entity_id,
        name: req.name.clone(),
        point: req.point,
        template_id: req.template_id,
        alive: req.alive,
        nearest: req.nearest,
        index: req.index,
    };
    steps.subject(target.describe());
    if target.is_empty() && req.waypoints.is_empty() {
        return Err(steps.fail(
            io.now_ms(),
            "arguments",
            "give entity_id, name, template_id, point or waypoints",
        ));
    }
    if target.point.is_some()
        && (target.entity_id.is_some()
            || target.name.is_some()
            || target.template_id.is_some()
            || target.alive.is_some()
            || target.index.is_some())
    {
        return Err(steps.fail(
            io.now_ms(),
            "arguments",
            "point cannot be combined with entity selectors",
        ));
    }
    if req
        .waypoint_arrival_m
        .is_some_and(|radius| !radius.is_finite() || !(0.2..=1.5).contains(&radius))
    {
        return Err(steps.fail(
            io.now_ms(),
            "arguments",
            "waypoint_arrival_m must be between 0.2 and 1.5 metres",
        ));
    }
    io.ensure_focus()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "focus", e))?;
    match io.turn_keys().await {
        Ok((left, right)) => {
            turn_keys.left = left;
            turn_keys.right = right;
        }
        Err(why) => tracing::debug!(%why, "turn key bindings unreadable; using Q and E"),
    }

    // Legs: waypoints, then the target.
    let mut legs: Vec<(Goal, f64)> = req
        .waypoints
        .iter()
        .map(|w| {
            (
                Goal::Point(w.to_client()),
                req.waypoint_arrival_m.unwrap_or(WAYPOINT_ARRIVAL_M),
            )
        })
        .collect();
    if let Some(p) = target.point {
        legs.push((Goal::Point(p.to_client()), req.arrival_m.unwrap_or(1.5)));
    } else if target.entity_id.is_some() || target.name.is_some() || target.template_id.is_some() {
        let fr = FindRequest {
            entity_id: target.entity_id,
            name: target.name.clone(),
            template_id: target.template_id,
            alive: target.alive,
            nearest: target.nearest,
            rendered_only: true,
            limit: Some(target.index.unwrap_or(0).saturating_add(1)),
            project: Some(false),
            ..FindRequest::default()
        };
        let r = find_entities(io, &fr, &mut steps).await?;
        let Some(m) = r.matches.into_iter().nth(target.index.unwrap_or(0)) else {
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

    let (mut actor, start, player_id) = player_actor(io)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_player", e))?;
    // The server's own view of where the player is, before and after: the
    // client can walk into a wall that the server never agreed to (D-AX11).
    let start_server = io.server_position(player_id).await.ok().flatten();
    let timeout = req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).min(600_000);
    let t_start = io.now_ms();
    let mut rate = TurnRate::default();
    let mut held_turn: Option<TurnKey> = None;
    let mut path = vec![json!({ "t_ms": 0, "at": start.pos.to_json() })];
    let mut last_sample = t_start;
    let mut leg_reports = Vec::new();
    let mut prev = start;
    let mut forward = false;
    let mut guard = if req.allow_drops == Some(true) {
        Guard::Off
    } else {
        Guard::On { last_ms: None }
    };
    ledge::self_check(&mut guard, io, start).await;
    let mut leg_in_range: Vec<bool> = Vec::new();
    let mut probed = false;
    // The last leg's goal (client space) and radius, for the arrival proof.
    let mut final_leg: Option<(Vec3, f64)> = None;
    // The final leg learns how far the pawn coasts after `W` goes up.
    let mut glue = CoastGlue::new(io.coast_model());
    let leg_count = legs.len();

    for (i, (mut goal, arrival)) in legs.into_iter().enumerate() {
        let last_leg = i + 1 == leg_count;
        let t_leg = io.now_ms();
        let mut ctl = MoveController::new(
            MoveConfig {
                arrival_m: arrival,
                ..MoveConfig::default()
            },
            rate,
        );
        let mut unsticks = Vec::new();
        let mut leg_start_m: Option<f64> = None;
        let outcome = loop {
            let now = io.now_ms();
            if now.saturating_sub(t_start) > timeout {
                let goal_pos = goal.position(io).await;
                return Err(steps.fail_with_stop(
                    now,
                    &format!("leg_{i}"),
                    format!("timed out after {timeout} ms"),
                    json!({
                        "at": prev.to_json(),
                        "goal": goal.describe(),
                        "distance_m": horizontal_m(prev.pos, goal_pos),
                        "start_distance_m": leg_start_m,
                        "turn_rate": ctl.rate.to_json(),
                        "unsticks": unsticks,
                        "path": path,
                        "hint": FAIL_HINT,
                    }),
                    stop_of(
                        StopKind::Timeout,
                        i,
                        now - t_start,
                        &unsticks,
                        probed,
                        start,
                        prev,
                    ),
                ));
            }
            let pose = match io.actor_pose(actor).await {
                Ok(p) => p,
                Err(_) => {
                    let (a, p, _) = player_actor(io)
                        .await
                        .map_err(|e| steps.fail(io.now_ms(), "read_player", e))?;
                    actor = a;
                    p
                }
            };
            let jumped = horizontal_m(prev.pos, pose.pos);
            if jumped > TELEPORT_M {
                return Err(steps.fail_with_stop(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!("the player moved {jumped:.1} m in one tick (teleport or zone change)"),
                    json!({ "from": prev.to_json(), "to": pose.to_json() }),
                    stop_of(
                        StopKind::Stalled,
                        i,
                        io.now_ms().saturating_sub(t_start),
                        &unsticks,
                        probed,
                        start,
                        pose,
                    ),
                ));
            }
            prev = pose;
            if now.saturating_sub(last_sample) >= PATH_SAMPLE_MS {
                path.push(json!({ "t_ms": now - t_start, "at": pose.pos.to_json() }));
                last_sample = now;
            }
            let goal_pos = goal.position(io).await;
            if last_leg {
                glue.on_pose(pose.pos, now);
                ctl.set_lead(glue.lead_m(), glue.holdoff_ms());
            }
            let was_coasting = ctl.coasting();
            let s = ctl.step(now, pose, goal_pos);
            if last_leg && !was_coasting && ctl.coasting() {
                glue.on_released(pose.pos);
            } else if last_leg && was_coasting && !ctl.coasting() {
                glue.on_settled(pose.pos);
            }
            if leg_start_m.is_none() {
                leg_start_m = Some(s.distance_m);
            }
            if let Some(o) = s.outcome {
                break o;
            }
            // The first tick that still has to walk: prove the forward key moves the pawn.
            // Placed after the outcome check on purpose: a target already inside
            // `arrival_m` breaks out above, so it never costs a real move. Runs once
            // per walk, not per leg.
            if !probed {
                probed = true;
                let after = input_probe(io, &mut steps, i, actor, &goal, pose, t_start).await?;
                steps.used(NativeLevel::RealInput);
                prev = after;
                ctl.skip_next_learning();
            }
            if let Some(u) = s.unstick {
                ctl.skip_next_learning();
                unsticks.push(u.name());
                run_unstick(io, u)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "unstick", e))?;
            }
            let turned = apply_turn(io, turn_keys, &mut held_turn, s.turn)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "turn_key", e))?;
            if forward && ledge::check(&mut guard, io, now, pose).await {
                release(io, turn_keys).await;
                let state = json!({ "at": pose.to_json(), "goal": goal.describe(), "ledge_guard": guard.label(), "path": path, "hint": FAIL_HINT });
                let stop = stop_of(
                    StopKind::Ledge,
                    i,
                    io.now_ms().saturating_sub(t_start),
                    &unsticks,
                    probed,
                    start,
                    pose,
                );
                return Err(steps.fail_with_stop(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    "ledge ahead: no floor within 2 m".to_string(),
                    state,
                    stop,
                ));
            }
            if s.forward != forward {
                io.key(FORWARD_KEY, s.forward)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "forward_key", e))?;
                forward = s.forward;
            }
            if turned || s.forward {
                steps.used(NativeLevel::RealInput);
            }
            io.sleep(DEFAULT_TICK_MS).await;
        };
        rate = ctl.rate;
        let mut report = json!({
            "goal": goal.describe(),
            "arrival_m": arrival,
            "start_distance_m": leg_start_m,
            "ms": io.now_ms() - t_leg,
            "unsticks": unsticks,
            "outcome": match outcome {
                MoveOutcome::Arrived { distance_m } => json!({ "arrived": true, "distance_m": distance_m }),
                MoveOutcome::Stuck { distance_m, attempts } => json!({ "arrived": false, "stuck": true, "distance_m": distance_m, "attempts": attempts }),
                MoveOutcome::Diverged { distance_m, best_m } => json!({ "arrived": false, "diverged": true, "distance_m": distance_m, "best_m": best_m }),
                MoveOutcome::NoTurn { ticks } => json!({ "arrived": false, "no_turn": true, "ticks": ticks }),
            },
        });
        if last_leg {
            report["coast"] = glue.leg_json();
        }
        steps.record(&format!("leg_{i}"), t_leg, io.now_ms(), report.clone());
        leg_reports.push(report);
        leg_in_range.push(leg_start_m.is_some_and(|d| d <= arrival));
        let stop = |kind| {
            stop_of(
                kind,
                i,
                io.now_ms().saturating_sub(t_start),
                &unsticks,
                probed,
                start,
                prev,
            )
        };
        let evidence = || {
            json!({
                "at": prev.to_json(),
                "goal": goal.describe(),
                "start_distance_m": leg_start_m,
                "unsticks": unsticks,
                "turn_rate": rate.to_json(),
                "path": path,
                "ledge_guard": guard.label(),
                "hint": FAIL_HINT,
            })
        };
        match outcome {
            MoveOutcome::Arrived { .. } => {}
            MoveOutcome::Stuck {
                distance_m,
                attempts,
            } => {
                return Err(steps.fail_with_stop(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!("stuck {distance_m:.1} m short after {attempts} unstick attempts"),
                    evidence(),
                    stop(StopKind::Stuck),
                ));
            }
            MoveOutcome::Diverged { distance_m, best_m } => {
                return Err(steps.fail_with_stop(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!(
                        "walking away from the goal: {distance_m:.1} m now, {best_m:.1} m at the closest; stopped instead of drifting further"
                    ),
                    evidence(),
                    stop(StopKind::Stalled),
                ));
            }
            MoveOutcome::NoTurn { ticks } => {
                return Err(steps.fail_with_stop(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!(
                        "the pawn did not turn in {ticks} ticks of holding the turn keys ({} / {}): something roots it (a dialog, a cast, a stun) or the client binds other keys; compare client_player_state heading_deg before and after a ui_sequence key step with hold_ms",
                        turn_keys.left, turn_keys.right
                    ),
                    evidence(),
                    stop(StopKind::IgnoredInput),
                ));
            }
        }
        final_leg = Some((goal.position(io).await, arrival));
    }
    io.store_coast_model(glue.model);
    if forward {
        io.key(FORWARD_KEY, false)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "forward_key", e))?;
    }
    path.push(json!({ "t_ms": io.now_ms() - t_start, "at": prev.pos.to_json() }));
    // Ask the server too (D-AX11): the pose above is the client's own belief.
    let end_server = io.server_position(player_id).await.ok().flatten();
    if let Some((goal_pos, arrival)) = final_leg {
        steps.judged(Evidence::judge(arrival_checks(
            goal_pos,
            start.pos,
            prev.pos,
            arrival,
            start_server,
            end_server,
        )));
    }
    let out = json!({
        "arrived": true,
        "ledge_guard": guard.label(),
        "server_end": server_end_json(end_server),
        "already_in_range": leg_in_range.iter().all(|b| *b),
        "start": start.to_json(),
        "end": prev.to_json(),
        "legs": leg_reports,
        "turn_rate": rate.to_json(),
        "path": path,
    });
    Ok(steps.finish(io.now_ms(), out))
}
