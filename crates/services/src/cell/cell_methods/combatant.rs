//! SGWCombatant interface exposed CellMethods (indices 5–7).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Set crouched state.
pub const SET_CROUCHED: u16 = 5;
/// Toggle heal debug overlay.
pub const TOGGLE_HEAL_DEBUG: u16 = 6;
/// Request holster/unholster weapon.
pub const REQUEST_HOLSTER_WEAPON: u16 = 7;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_CROUCHED => {
            if args.len() >= 1 {
                let crouched = args[0] as i8;
                tracing::debug!(entity_id, crouched, "setCrouched");
            }
            true
        }
        TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, "toggleHealDebug (stub)");
            true
        }
        REQUEST_HOLSTER_WEAPON => {
            if args.len() >= 1 {
                let holstered = args[0] as i8;
                tracing::debug!(entity_id, holstered, "requestHolsterWeapon");
            }
            true
        }
        _ => false,
    }
}
