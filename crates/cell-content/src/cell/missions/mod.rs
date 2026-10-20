//! Mission operations for the CellService.
//!
//! Handles mission accept, abandon, complete, and objective updates.
//! Sends wire-format mission state to the client.
//!
//! Reference: `python/cell/MissionManager.py`
//!
//! Split along lifecycle seams:
//! - [`resend`] — resend active mission state on map load.
//! - [`lifecycle`] — accept / abandon.
//! - [`progression`] — advance step, complete objective.
//! - [`complete_direct`] — a chain action completes the whole mission.
//! - [`persist`] — serialize the live instance into the base-side
//!   `MissionUpdate` that UPSERTs `sgw_mission`.
//! - [`state_lock`] — the unreadable-saved-missions lock (TG-DB-12): every
//!   mutation and persist above refuses while it holds.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;

use crate::cell::messages::CellToBaseMsg;

mod complete_direct;
mod letter_turn_in;
mod lifecycle;
mod persist;
mod progression;
mod resend;
pub mod state_lock;

#[cfg(test)]
mod abandon_persist_tests;
#[cfg(test)]
mod hidden_frames_tests;
#[cfg(test)]
mod letter_turn_in_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod progression_tests;
#[cfg(test)]
mod state_lock_tests;

pub use complete_direct::complete_mission_direct;
pub(crate) use letter_turn_in::{
    announce_letter_turn_in, install_letter_turn_in, valid_letter_postimage,
};
pub use lifecycle::{abandon_mission, accept_mission};
pub use persist::{mission_update_msg, send_mission_update};
pub use progression::{advance_step, complete_objective};
pub use resend::resend_missions;

/// Hidden-mission client-frame gate (#715).
///
/// The reference `MissionManager.py` guards every client-facing mission
/// frame with `if not mission.mission.isHidden:` — accept (:636), resend
/// (:565), complete (:704), fail (:724), objective complete/fail
/// (:747/:769), abandon (:792), clear (:812) and advance (:851). Hidden
/// missions (the Hallway0N controllers 682-686 and the Prison Boot gate
/// 689) are server-side bookkeeping: their state, persistence and content
/// events run normally, the client just never hears about them.
///
/// Returns `true` when the caller must skip its `onMissionUpdate` /
/// `onStepUpdate` / `onObjectiveUpdate` sends, and logs one DEBUG event
/// per suppressed call site so SigNoz can confirm the gate fired
/// (`reason=hidden_mission`, `site=` names the call).
///
/// Every send site in this module goes through here; so does the GM
/// `.missionfail` frame in `cell::console`. The login / respawn resend
/// needs no call: `MissionManager::serialize_resend` iterates
/// `active_missions()`, which already filters hidden missions out.
pub fn suppress_hidden_mission_frames(
    is_hidden: bool,
    entity_id: u32,
    player_id: Option<i32>,
    mission_id: i32,
    site: &'static str,
) -> bool {
    if is_hidden {
        // The callers hold the entity mutably, so no name is in reach here;
        // each caller's own line just before this one names the player.
        tracing::debug!(
            entity_id, // nt:id-only caller holds the entity mutably; its adjacent line names it
            player_id, // nt:id-only caller holds the entity mutably; its adjacent line names it
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            site,
            reason = "hidden_mission",
            "mission client frames suppressed: mission is hidden (reference parity, #715)"
        );
    }
    is_hidden
}

/// Queue one client mission frame (`onMissionUpdate`, `onStepUpdate` or
/// `onObjectiveUpdate`) to the base.
///
/// A closed base channel means the client never hears about this journal
/// change, so it WARNs with the frame name and the player's identity instead
/// of discarding the send. The entity's name is the character name, the same
/// as the `player_name` in `who`.
pub(super) async fn send_mission_frame(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    frame: &'static str,
    mission_id: i32,
    who: &PlayerIdentity,
) {
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    };
    if let Err(e) = tx.send(msg).await {
        tracing::warn!(
            event = "mission_frame_dropped",
            reason = "base_channel_closed",
            frame,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            error = %e,
            "Mission journal frame not queued: the base channel is closed"
        );
    }
}

// ── Method indices for mission client methods ────────────────────────────────
// Missionary interface: flat indices 80-84

/// onMissionUpdate(INT32 missionId, INT8 status, INT32 giverName)
pub(super) const ON_MISSION_UPDATE: u16 = 80;
/// onStepUpdate(INT32 stepId, INT8 status)
pub(super) const ON_STEP_UPDATE: u16 = 81;
/// onObjectiveUpdate(INT32 objectiveId, INT8 status, INT8 hidden, INT8 optional)
pub(super) const ON_OBJECTIVE_UPDATE: u16 = 82;
