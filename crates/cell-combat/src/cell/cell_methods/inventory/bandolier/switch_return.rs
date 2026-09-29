//! Ammo-type switch returns unfired special rounds to the bags (ammo
//! campaign AM-02, issue #1026). Owned by AM-02; created empty by AM-F so
//! AM-02 and AM-03 (`ammo_change.rs`) never edit this directory's `mod.rs`
//! at the same time.
//!
//! AM-02 fills this in: on `requestAmmoChange`, the clip's unfired rounds of
//! the previous type, if special, go back through the base's
//! `inventory::ammo_reserve::return_rounds`; the `remainder` that does not
//! fit stays in the clip (D-AM05). AM-03's handler calls in here.
//!
//! AM-03 stub: the enum and the signature are the contract agreed with
//! AM-02. Until AM-02 lands, every switch `Proceed`s, which is today's
//! behavior; AM-02 replaces this file whole.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// What `requestAmmoChange` does after the switch-return hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwitchReturn {
    /// The base returns the rounds and persists the new type; the cell
    /// finishes the switch when the base answers.
    Deferred,
    /// Switch now, as before the ammo campaign.
    Proceed,
}

/// Start the switch-return for `slot` changing to `ammo_type`. The stub
/// always proceeds.
pub(crate) async fn begin_switch_return(
    _entity_id: u32,
    _slot: i32,
    _ammo_type: i32,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> SwitchReturn {
    SwitchReturn::Proceed
}
