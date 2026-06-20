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

mod lifecycle;
mod progression;
mod resend;

pub use lifecycle::{abandon_mission, accept_mission};
pub use progression::{advance_step, complete_mission_direct, complete_objective};
pub use resend::resend_missions;

// ── Method indices for mission client methods ────────────────────────────────
// Missionary interface: flat indices 80-84

/// onMissionUpdate(INT32 missionId, INT8 status, INT32 giverName)
pub(super) const ON_MISSION_UPDATE: u16 = 80;
/// onStepUpdate(INT32 stepId, INT8 status)
pub(super) const ON_STEP_UPDATE: u16 = 81;
/// onObjectiveUpdate(INT32 objectiveId, INT8 status, INT8 hidden, INT8 optional)
pub(super) const ON_OBJECTIVE_UPDATE: u16 = 82;
