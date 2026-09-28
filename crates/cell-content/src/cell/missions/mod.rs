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
//! - [`progression`] — advance step, complete objective, complete direct.
//! - [`persist`] — serialize the live instance into the base-side
//!   `MissionUpdate` that UPSERTs `sgw_mission`.

mod lifecycle;
mod persist;
mod progression;
mod resend;

#[cfg(test)]
mod hidden_frames_tests;
#[cfg(test)]
mod progression_tests;

pub use lifecycle::{abandon_mission, accept_mission};
pub use persist::{mission_update_msg, send_mission_update};
pub use progression::{advance_step, complete_mission_direct, complete_objective};
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
        tracing::debug!(
            entity_id,
            player_id,
            mission_id,
            site,
            reason = "hidden_mission",
            "mission client frames suppressed: mission is hidden (reference parity, #715)"
        );
    }
    is_hidden
}

// ── Method indices for mission client methods ────────────────────────────────
// Missionary interface: flat indices 80-84

/// onMissionUpdate(INT32 missionId, INT8 status, INT32 giverName)
pub(super) const ON_MISSION_UPDATE: u16 = 80;
/// onStepUpdate(INT32 stepId, INT8 status)
pub(super) const ON_STEP_UPDATE: u16 = 81;
/// onObjectiveUpdate(INT32 objectiveId, INT8 status, INT8 hidden, INT8 optional)
pub(super) const ON_OBJECTIVE_UPDATE: u16 = 82;
