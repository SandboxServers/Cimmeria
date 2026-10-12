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

use super::find::{find_entities, FindRequest};
use super::geometry::{horizontal_m, Pose, Vec3};
use super::io::WorldIo;
use super::steer::{MoveConfig, MoveController, MoveOutcome, TurnCmd, TurnKey, TurnRate, Unstick};
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

pub const FORWARD_KEY: &str = "W";
/// Backward key, for the blocked-vs-dead probe retry.
const BACKWARD_KEY: &str = "S";

/// The names of the turn keys this walk may press, for pressing and for
/// the release on every exit.
#[derive(Debug, Clone)]
struct TurnKeys {
    left: String,
    right: String,
}

impl Default for TurnKeys {
    fn default() -> Self {
        Self {
            left: "Q".into(),
            right: "E".into(),
        }
    }
}

impl TurnKeys {
    fn name(&self, k: TurnKey) -> &str {
        match k {
            TurnKey::Left => &self.left,
            TurnKey::Right => &self.right,
        }
    }
}

/// Put the turn keys in the state `cmd` wants. `held` is the key currently
/// down. Returns whether a turn key was used.
async fn apply_turn<W: WorldIo>(
    io: &mut W,
    keys: &TurnKeys,
    held: &mut Option<TurnKey>,
    cmd: TurnCmd,
) -> Result<bool, String> {
    let want = match cmd {
        TurnCmd::Off => None,
        TurnCmd::Hold(k) | TurnCmd::Pulse(k, _) => Some(k),
    };
    if *held != want {
        if let Some(old) = held.take() {
            io.key(keys.name(old), false).await?;
        }
        if let Some(k) = want {
            io.key(keys.name(k), true).await?;
            *held = Some(k);
        }
    }
    if let TurnCmd::Pulse(_, ms) = cmd {
        io.sleep(ms).await;
        if let Some(k) = held.take() {
            io.key(keys.name(k), false).await?;
        }
    }
    Ok(want.is_some())
}

pub const DEFAULT_TICK_MS: u64 = 100;
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Default arrival radius for an entity target (stop next to it, not in
/// it). Inside the 3.5 to 4.5 m a right-click or a trainer window needs, so a
/// target already in range is a no-op and a walk ends in range (#1660).
pub const ENTITY_ARRIVAL_M: f64 = 4.0;

/// What a failed walk tells the caller to try. The walk is a straight line
/// with no path-finding.
const FAIL_HINT: &str = "client_move_to walks a straight line and cannot path-find. Go round the obstacle with waypoints, hold a movement key with a ui_sequence key step (hold_ms), or teleport on the server and click without walking; then stand 3.5 to 4.5 m from the target.";
/// A jump this large in one tick is a teleport or a zone change, not a walk.
pub const TELEPORT_M: f64 = 30.0;
/// How long a strafe unstick holds its key.
const STRAFE_MS: u64 = 600;
/// Path samples kept in the result.
const PATH_SAMPLE_MS: u64 = 1000;
/// Forward-key liveness probe: hold the forward key this long ...
const PROBE_HOLD_MS: u64 = 300;
/// ... let the pose settle this long after the release ...
const PROBE_SETTLE_MS: u64 = 100;
/// ... and expect at least this much horizontal travel, metres. A live pawn
/// covers about 1.5 m in 300 ms; `horizontal_m` is in metres, like `TELEPORT_M`.
const PROBE_MIN_M: f64 = 0.1;

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
    /// Arrival radius, metres (default 1.5 for a point, 4.0 for an entity).
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
    let mut rate = TurnRate::default();
    let mut held_turn: Option<TurnKey> = None;
    let mut path = vec![json!({ "t_ms": 0, "at": start.pos.to_json() })];
    let mut last_sample = t_start;
    let mut leg_reports = Vec::new();
    let mut prev = start;
    let mut forward = false;
    let mut leg_in_range: Vec<bool> = Vec::new();
    let mut probed = false;

    for (i, (mut goal, arrival)) in legs.into_iter().enumerate() {
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
                return Err(steps.fail_with(
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
                let before = pose;
                io.key(FORWARD_KEY, true)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                io.sleep(PROBE_HOLD_MS).await;
                io.key(FORWARD_KEY, false)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                io.sleep(PROBE_SETTLE_MS).await;
                let after = io
                    .actor_pose(actor)
                    .await
                    .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                let moved_m = horizontal_m(before.pos, after.pos);
                if moved_m < PROBE_MIN_M {
                    // A pawn pressed against a wall also ignores W. Back up
                    // once: if S moves it, the way ahead is blocked (geometry),
                    // not the input dead.
                    io.key(BACKWARD_KEY, true)
                        .await
                        .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                    io.sleep(PROBE_HOLD_MS).await;
                    io.key(BACKWARD_KEY, false)
                        .await
                        .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                    io.sleep(PROBE_SETTLE_MS).await;
                    let back = io
                        .actor_pose(actor)
                        .await
                        .map_err(|e| steps.fail(io.now_ms(), "input_probe", e))?;
                    let back_m = horizontal_m(after.pos, back.pos);
                    if back_m >= PROBE_MIN_M {
                        return Err(steps.fail_with(
                            io.now_ms(),
                            &format!("leg_{i}"),
                            format!(
                                "blocked ahead: the avatar did not move for a {PROBE_HOLD_MS} ms forward hold ({moved_m:.2} m) but moved {back_m:.1} m backward, so movement input works and something solid is in front of it"
                            ),
                            json!({
                                "before": before.to_json(),
                                "at": back.to_json(),
                                "moved_m": moved_m,
                                "backward_m": back_m,
                                "goal": goal.describe(),
                                "hint": FAIL_HINT,
                            }),
                        ));
                    }
                    return Err(steps.fail_with(
                        io.now_ms(),
                        "input_probe",
                        format!(
                            "the avatar did not move for a {PROBE_HOLD_MS} ms forward key hold ({moved_m:.2} m) nor a backward one ({back_m:.2} m): movement input is not reaching the pawn (UI keys can still work); see client_input_status.held and the world state"
                        ),
                        json!({
                            "before": before.to_json(),
                            "at": back.to_json(),
                            "moved_m": moved_m,
                            "backward_m": back_m,
                            "hold_ms": PROBE_HOLD_MS,
                            "hint": "W and S both did nothing, so it is not a wall. Try a ui_sequence key step with hold_ms and compare server_entity_get; if the pawn still ignores W, teleport on the server and click the target without walking. Keys are posted window messages, so client_input_status keyboard counters say nothing about it.",
                        }),
                    ));
                }
                steps.used(NativeLevel::RealInput);
                prev = after;
                ctl.skip_next_learning();
            }
            if let Some(u) = s.unstick {
                ctl.skip_next_learning();
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
            let turned = apply_turn(io, turn_keys, &mut held_turn, s.turn)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "turn_key", e))?;
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
        let report = json!({
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
        steps.record(&format!("leg_{i}"), t_leg, io.now_ms(), report.clone());
        leg_reports.push(report);
        leg_in_range.push(leg_start_m.is_some_and(|d| d <= arrival));
        let evidence = || {
            json!({
                "at": prev.to_json(),
                "goal": goal.describe(),
                "start_distance_m": leg_start_m,
                "unsticks": unsticks,
                "turn_rate": rate.to_json(),
                "path": path,
                "hint": FAIL_HINT,
            })
        };
        match outcome {
            MoveOutcome::Arrived { .. } => {}
            MoveOutcome::Stuck {
                distance_m,
                attempts,
            } => {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!("stuck {distance_m:.1} m short after {attempts} unstick attempts"),
                    evidence(),
                ));
            }
            MoveOutcome::Diverged { distance_m, best_m } => {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!(
                        "walking away from the goal: {distance_m:.1} m now, {best_m:.1} m at the closest; stopped instead of drifting further"
                    ),
                    evidence(),
                ));
            }
            MoveOutcome::NoTurn { ticks } => {
                return Err(steps.fail_with(
                    io.now_ms(),
                    &format!("leg_{i}"),
                    format!(
                        "the pawn did not turn in {ticks} ticks of holding the turn keys ({} / {}): something roots it (a dialog, a cast, a stun) or the client binds other keys; compare client_player_state heading_deg before and after a ui_sequence key step with hold_ms",
                        turn_keys.left, turn_keys.right
                    ),
                    evidence(),
                ));
            }
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
        "already_in_range": leg_in_range.iter().all(|b| *b),
        "start": start.to_json(),
        "end": prev.to_json(),
        "legs": leg_reports,
        "turn_rate": rate.to_json(),
        "path": path,
    });
    Ok(steps.finish(io.now_ms(), out))
}
