//! SGWCombatant interface exposed CellMethods (indices 5–7).
//!
//! These methods handle player-initiated state changes (crouch, weapon holster)
//! and must reflect the updated stateField back to the client via
//! `onStateFieldUpdate` so the animation system transitions correctly.
//!
//! Reference: `python/cell/SGWBeing.py:746-770`

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Set crouched state.
pub const SET_CROUCHED: u16 = 5;
/// Toggle heal debug overlay.
pub const TOGGLE_HEAL_DEBUG: u16 = 6;
/// Request holster/unholster weapon.
pub const REQUEST_HOLSTER_WEAPON: u16 = 7;

/// Being State Field bit positions (from Atrea.enums BSF_*).
const BSF_CROUCHING: u32 = 1 << 2;
const BSF_HOLSTER: u32 = 1 << 8;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_CROUCHED => {
            if args.is_empty() {
                return true;
            }
            let crouched = args[0] as i8;
            tracing::debug!(entity_id, crouched, "setCrouched");

            if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                let old = e.state_field;
                if crouched != 0 {
                    e.state_field |= BSF_CROUCHING;
                } else {
                    e.state_field &= !BSF_CROUCHING;
                }
                if e.state_field != old {
                    let new_state = e.state_field;
                    if let Err(e) = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 19, // onStateFieldUpdate
                            args: new_state.to_le_bytes().to_vec(),
                        })
                        .await
                    {
                        tracing::warn!(entity_id, "EntityMethodCall send failed: {e}");
                    }
                    // TODO: also send to witnesses via AoI broadcast
                }
            }
            true
        }
        TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, "toggleHealDebug (stub)");
            true
        }
        REQUEST_HOLSTER_WEAPON => {
            if args.is_empty() {
                return true;
            }
            let holstered = args[0] as i8;
            tracing::debug!(entity_id, holstered, "requestHolsterWeapon");

            if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                let old = e.state_field;
                if holstered != 0 {
                    e.state_field |= BSF_HOLSTER;
                } else {
                    e.state_field &= !BSF_HOLSTER;
                }
                if e.state_field != old {
                    let new_state = e.state_field;
                    if let Err(e) = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 19, // onStateFieldUpdate
                            args: new_state.to_le_bytes().to_vec(),
                        })
                        .await
                    {
                        tracing::warn!(entity_id, "EntityMethodCall send failed: {e}");
                    }
                    // TODO: also send to witnesses via AoI broadcast
                }
            }
            true
        }
        _ => false,
    }
}
