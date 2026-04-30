//! Weapon attack event broadcast helpers.

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Fire a weapon attack event for an equipped item.
/// Called after a ranged weapon attack is launched.
// TODO(combat-pass): Implement weapon swing animations, ammo consumption,
// projectile spawn, and equipped-item event broadcast to witnesses.
pub async fn fire_equipped_weapon_attack_event(
    entity_id: u32,
    target_id: i32,
    is_ranged: bool,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    tracing::warn!(
        entity_id,
        target_id,
        is_ranged,
        "fireEquippedWeaponAttackEvent: stub — weapon events not yet implemented"
    );
}
