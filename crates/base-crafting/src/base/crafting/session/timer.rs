//! The induction bar: `onTimerUpdate` with `Type = TIMER_CRAFT_INDUCTION`.
//!
//! The client draws the crafting bar only from this timer (SGW.exe
//! `0x00e47800`): it reads `SourceID`, `TotalTime` and
//! `BigWorldTimeComplete`, computes `remaining = complete - client clock`,
//! and draws the bar when the source is the player. There is no separate
//! "craft started" or "craft finished" message. `BigWorldTimeComplete` is
//! absolute, in the one server-wide game clock the client follows.

use cimmeria_cell_catalog::crafting::TIMER_CRAFT_INDUCTION;
use cimmeria_entity::abilities::serialize_timer_update;

use super::{InductionEnv, INDUCTION_SECS};
use crate::base::crafting::telemetry::{send_timer, JobIds};
use crate::mercury::game_clock::game_time_secs;

/// The `onTimerUpdate` arguments for an induction that starts at game time
/// `now_game_secs`: `ID = timer_id`, `Type = 16`, `SourceID` = the player,
/// `SecondaryID = 0`, `TotalTime = 3.0`, `BigWorldTimeComplete = now + 3.0`.
pub fn induction_timer_args(entity_id: u32, timer_id: i32, now_game_secs: f32) -> Vec<u8> {
    serialize_timer_update(
        timer_id,
        TIMER_CRAFT_INDUCTION as i8,
        entity_id as i32,
        0,
        INDUCTION_SECS,
        now_game_secs + INDUCTION_SECS,
    )
}

/// Send the induction bar to the player's own client, expiring
/// [`INDUCTION_SECS`] from now on the game clock. Returns the
/// `BigWorldTimeComplete` sent (the `expires_at` event field).
pub async fn send_induction_timer(env: &InductionEnv, ids: &JobIds, timer_id: i32) -> f32 {
    let now = game_time_secs();
    let args = induction_timer_args(ids.entity_id, timer_id, now);
    send_timer(env, ids, &args).await;
    now + INDUCTION_SECS
}
