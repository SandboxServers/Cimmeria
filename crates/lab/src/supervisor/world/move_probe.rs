//! The forward-key liveness probe of `client_move_to`: proves that `W` moves
//! the pawn before the walk trusts its steering.

use serde_json::json;

use super::geometry::{horizontal_m, Pose};
use super::io::WorldIo;
use super::movement::{Goal, BACKWARD_KEY, FAIL_HINT, FORWARD_KEY};
use super::{Steps, WorldError};

/// Forward-key liveness probe: hold the forward key this long ...
const PROBE_HOLD_MS: u64 = 300;
/// ... let the pose settle this long after the release ...
const PROBE_SETTLE_MS: u64 = 100;
/// ... and expect at least this much horizontal travel, metres. A live pawn
/// covers about 1.5 m in 300 ms; `horizontal_m` is in metres, like `TELEPORT_M`.
const PROBE_MIN_M: f64 = 0.1;

/// Holds W, then S if W did nothing. `Ok` is the pose after a moving W hold;
/// `Err` says the way ahead is blocked (S moved the pawn) or the input is dead.
pub(super) async fn input_probe<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    leg: usize,
    actor: u32,
    goal: &Goal,
    before: Pose,
) -> Result<Pose, WorldError> {
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
    if moved_m >= PROBE_MIN_M {
        return Ok(after);
    }
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
            &format!("leg_{leg}"),
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
    Err(steps.fail_with(
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
    ))
}
