//! Missionary interface ClientMethods (indices 80–84).

/// Mission status update (accepted/completed/failed).
pub const ON_MISSION_UPDATE: u16 = 80;
/// Mission step status update.
pub const ON_STEP_UPDATE: u16 = 81;
/// Mission objective status update.
pub const ON_OBJECTIVE_UPDATE: u16 = 82;
/// Mission task progress update.
pub const ON_TASK_UPDATE: u16 = 83;
/// Offer to share a mission with the player.
pub const OFFER_SHARED_MISSION: u16 = 84;
