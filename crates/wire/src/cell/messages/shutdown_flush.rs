//! The shutdown flush (TG-DB-02, D-TG15): the cell hands the base every
//! in-world player's position and dirty bandolier ammo when the server stops,
//! and the base writes them in parallel before the database pool goes away.
//!
//! Carried by `CellToBaseMsg::ShutdownFlush`. The base runs the writes until
//! [`ShutdownFlushRequest::deadline`] and answers with the `player_id`s whose
//! every write landed; the cell reports the rest as
//! `player_state_not_persisted`.

use std::time::Duration;

/// How long the shutdown flush may run before the server stops anyway
/// (D-TG15). The container stop grace (`stop_grace_period` and
/// `WATCHTOWER_TIMEOUT` in `docker/compose.yml`) is set well above it.
pub const SHUTDOWN_FLUSH_DEADLINE: Duration = Duration::from_secs(10);

/// One dirty bandolier slot to persist, the same payload as
/// `CellToBaseMsg::BandolierAmmoUpdate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownAmmoSlot {
    pub slot_id: i32,
    /// `sgw_inventory.item_id` of the slot's weapon: the TOCTOU guard.
    pub expected_instance_id: i32,
    pub current_ammo: i32,
    pub cur_ammo_type: i32,
}

/// Everything one player needs saved at shutdown.
#[derive(Debug, Clone, PartialEq)]
pub struct ShutdownPlayerSave {
    pub player_id: i32,
    /// The world the player stands in, written to `world_location`.
    pub world_name: String,
    pub position: [f32; 3],
    /// The slots in `bandolier_ammo_dirty`, possibly none.
    pub ammo: Vec<ShutdownAmmoSlot>,
}

/// The request: save these players by `deadline`, then answer.
#[derive(Debug)]
pub struct ShutdownFlushRequest {
    pub saves: Vec<ShutdownPlayerSave>,
    /// Writes still running at this instant are abandoned.
    pub deadline: tokio::time::Instant,
    /// The `player_id`s whose position and every ammo slot were written.
    pub reply: tokio::sync::oneshot::Sender<Vec<i32>>,
}
