//! The abilities a bandolier weapon grants, and their swap when the active
//! slot changes (split out of `active_slot` along that seam).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// The abilities the weapon in `entity_id`'s bandolier `slot_id` grants:
/// its `items_event_sets` bindings for `EVENT_ITEM_RANGED`,
/// `EVENT_ITEM_MELEE` and `EVENT_ITEM_USE_ABILITY`. Empty for an empty
/// slot. The active-slot swap and the GM ability reset
/// (`gmResetAbilities`, AB-N2) both reconcile against it.
pub fn weapon_ability_set(
    space_mgr: &SpaceManager,
    entity_id: u32,
    slot_id: i32,
) -> std::collections::HashSet<i32> {
    use crate::cell::spawner::{EVENT_ITEM_MELEE, EVENT_ITEM_RANGED, EVENT_ITEM_USE_ABILITY};
    let item_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.bandolier_items.get(&slot_id).map(|b| b.item_id));
    let Some(item_id) = item_id else {
        return Default::default();
    };
    [EVENT_ITEM_RANGED, EVENT_ITEM_MELEE, EVENT_ITEM_USE_ABILITY]
        .into_iter()
        .filter_map(|event_id| {
            space_mgr
                .item_event_set_abilities
                .get(&(item_id, event_id))
                .copied()
        })
        .collect()
}

/// Compute the new weapon's `items_event_sets` bindings, hand them to
/// [`cimmeria_entity::abilities::AbilityManager::swap_weapon_granted_abilities`],
/// and broadcast `onKnownAbilitiesUpdate` if the set changed.
///
/// Probes `EVENT_ITEM_RANGED`, `EVENT_ITEM_MELEE`, and
/// `EVENT_ITEM_USE_ABILITY` — the three weapon-relevant event ids the
/// resolve.rs lookup also walks. A weapon often grants one ability per
/// event id (e.g., pistol: 579 ranged + 708 melee); all of them go
/// into the new set so any of them is fireable while the weapon is
/// active.
///
/// An empty slot (no item bound) produces an empty new set — every
/// previously-tracked weapon ability is revoked.
pub(super) async fn swap_weapon_granted_abilities_for_slot(
    entity_id: u32,
    slot_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // 1. New weapon's bindings — read item_id from the now-active slot,
    //    look up every relevant event_id in items_event_sets.
    let item_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.bandolier_items.get(&slot_id).map(|b| b.item_id));
    let new_set = weapon_ability_set(space_mgr, entity_id, slot_id);

    // 2. Hand the new set to the diff helper; capture (removed, added).
    let (removed, added) = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e.abilities.swap_weapon_granted_abilities(new_set),
        None => return,
    };
    if removed.is_empty() && added.is_empty() {
        return; // No change — skip the broadcast.
    }
    // A revoked ability's held entries (a toggle left on, a passive) would
    // otherwise outlive the ability with nothing to take them off.
    if !removed.is_empty() {
        crate::cell::effects::strip_timed_effects(
            entity_id,
            crate::cell::effects::stat_buff::StatBuffRemoval::Removed,
            |b| b.expires_at.is_none() && removed.contains(&b.ability_id),
            tx,
            space_mgr,
        )
        .await;
    }

    // 3. Build + broadcast `onKnownAbilitiesUpdate` (method 101) — the
    //    bulk known-abilities replace. Same wire shape the login burst
    //    sends in `base_messages/player_init.rs::send_known_abilities_update`
    //    (we don't call that helper directly because it's `pub(super)`
    //    and the import-chain reshape isn't worth it for one extra
    //    call site).
    //
    //    Wire format: `ARRAY<INT32> AbilityData` → `u32 count` +
    //    N × `i32 ability_id`. Read straight from the post-swap
    //    `known_ability_ids()` so the broadcast matches the entity's
    //    current state exactly.
    let ability_ids: Vec<i32> = match space_mgr.get_entity(entity_id) {
        Some(e) => e.abilities.known_ability_ids(),
        None => return,
    };
    let mut args = Vec::with_capacity(4 + ability_ids.len() * 4);
    args.extend_from_slice(&(ability_ids.len() as u32).to_le_bytes());
    for id in &ability_ids {
        args.extend_from_slice(&id.to_le_bytes());
    }
    // Through the wire ledger (AB-C7): the swap's hotbar replace is
    // accounted for like every other `onKnownAbilitiesUpdate`.
    crate::cell::abilities::send_entity_method_ledgered(
        entity_id,
        crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE,
        args,
        crate::cell::abilities::WireRoute::EntityDefault,
        crate::cell::abilities::WireCtx::new("weapon_swap"),
        tx,
        space_mgr,
    )
    .await;

    tracing::info!(
        target: "bandolier",
        event = "weapon_ability_swap",
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        slot_id, // nt:id-only inventory slot index, not a named object
        item_id = ?item_id,
        item_name = cimmeria_cell_world::cell::effects::content_names::item_name(item_id),
        removed = ?removed,
        added = ?added,
        new_known_count = ability_ids.len(),
        "Per-weapon ability grant — broadcast onKnownAbilitiesUpdate"
    );
}
