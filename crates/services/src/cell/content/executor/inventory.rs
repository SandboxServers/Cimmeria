//! Inventory action handlers: `Action::GrantItem` (with bandolier seeding for
//! weapons) and `Action::RemoveItem` (with by-instance vs by-type fork).

use std::collections::HashMap;

use serde_json::Value;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Determine the inventory container for an item from the DB-loaded map.
/// Falls back to INV_Main (1) if the item has no explicit container_sets entry.
pub(in crate::cell::content) fn item_container(
    item_id: i32,
    item_containers: &HashMap<i32, i32>,
) -> i32 {
    *item_containers.get(&item_id).unwrap_or(&1)
}

/// Return the clip size and default ammo type for a granted weapon item.
///
/// Reads from the `space_mgr.item_defs` cache loaded at startup from
/// `resources.items` (see `spawner::load_item_defs`). Returns `None` for
/// non-weapon items (clip_size IS NULL in DB) or when the cache wasn't
/// populated (e.g. tests without a DB pool) — callers skip the bandolier
/// seeding in that case, and the player can still receive the item normally.
fn weapon_stats(
    item_id: i32,
    item_defs: &HashMap<i32, crate::cell::spawner::WeaponDef>,
) -> Option<(i32, i32)> {
    item_defs
        .get(&item_id)
        .map(|d| (d.clip_size, d.default_ammo_type))
}

/// `Action::GrantItem` — add an item to the player's inventory; for weapons
/// (container 3) also seeds the bandolier slot and clears the ammo stat so
/// the client renders an empty mag until the player reloads.
#[allow(clippy::too_many_arguments)]
pub(super) async fn grant(
    item_id: i32,
    count: i32,
    container_id: Option<i32>,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        item_id,
        count,
        chain_id,
        "Content: granting item"
    );
    let cid = container_id
        .filter(|&c| c > 0)
        .unwrap_or_else(|| item_container(item_id, &space_mgr.item_containers));

    // If this is a weapon (bandolier), set ammo state on the entity.
    // Weapons start unloaded — the player must press R to reload.
    //
    // Stage C: insert a `BandolierItem` for the granted slot and seed
    // the AmmoSlot{N} stat to (0, 0, clip_size) so subsequent fire /
    // reload paths (which now read through `active_ammo()` and
    // `set_slot_ammo`) operate on a valid clamp range. We also send
    // an `onStatUpdate` so the client renders the empty mag for the
    // new weapon without waiting for the next fire.
    let mut ammo_stat_payload: Option<Vec<u8>> = None;
    if cid == 3 {
        if let Some((clip, default_ammo_type)) = weapon_stats(item_id, &space_mgr.item_defs) {
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                // The weapon-grant chain doesn't tell us which slot
                // the base will assign — content engine grants
                // implicitly fill the active bandolier slot.
                let slot_id = entity.active_bandolier_slot;
                entity.bandolier_items.insert(
                    slot_id,
                    cimmeria_entity::cell_entity::BandolierItem {
                        item_id,
                        clip_size: clip,
                        default_ammo_type,
                        current_ammo: 0,
                        cur_ammo_type: default_ammo_type,
                    },
                );
                entity.bandolier_ammo_dirty.insert(slot_id);
                let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                if let Some(stat) = entity.stats.get_mut(stat_id) {
                    stat.update(0, 0, clip);
                    let payload = entity.stats.serialize_dirty();
                    entity.stats.clear_dirty();
                    ammo_stat_payload = Some(payload);
                }
                tracing::info!(entity_id, item_id, slot_id, clip, "Weapon granted unloaded");
            }
        }
    }

    if let Err(e) = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            container_id: cid,
            count,
        })
        .await
    {
        tracing::error!(
            entity_id, player_id, item_id, container_id = cid,
            count, chain_id, error = %e,
            "GrantItem send to base failed -- item not persisted to inventory"
        );
    }

    if let Some(payload) = ammo_stat_payload {
        if !payload.is_empty() {
            crate::cell::abilities::send_entity_method(entity_id, 20, payload, tx, space_mgr).await;
        }
    }
}

/// `Action::RemoveItem` — consume the originating inventory `instance_id`
/// from chain context if present (set by `fire_item_use` — the OnItemUse
/// dispatch path) for "consume the slappack the player clicked", otherwise
/// fall back to by-type resolution.
pub(super) async fn remove(
    item_id: i32,
    count: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    params: &HashMap<String, Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    // Prefer the originating inventory `instance_id` if the
    // chain context carries one (set by `fire_item_use` —
    // the OnItemUse dispatch path). This is the difference
    // between "consume the slappack the player clicked" and
    // "consume the leftmost slappack of that type": when a
    // player has two stacks of the same item and clicks the
    // second one, the first stack must NOT be silently
    // consumed instead.
    //
    // For chains fired by other paths (mission events,
    // ambernol vial consumption via `enter_region`, etc.)
    // there's no instance_id in context, so we fall back to
    // the by-type resolution that picks the player's first
    // matching instance. Both paths converge on the same
    // wire-update sequence on the base side.
    let instance_id = params
        .get("instance_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .filter(|&v| v != 0);

    let send_result = match instance_id {
        Some(instance) => {
            tracing::info!(
                entity_id,
                player_id,
                instance,
                type_id = item_id,
                count,
                chain_id,
                "Content: RemoveItem → RemoveInventoryItem (by instance from context)"
            );
            tx.send(CellToBaseMsg::RemoveInventoryItem {
                entity_id,
                player_id,
                item_id: instance,
                quantity: count,
            })
            .await
        }
        None => {
            tracing::info!(
                entity_id,
                player_id,
                type_id = item_id,
                count,
                chain_id,
                "Content: RemoveItem → RemoveInventoryItemByType"
            );
            tx.send(CellToBaseMsg::RemoveInventoryItemByType {
                entity_id,
                player_id,
                type_id: item_id,
                count,
            })
            .await
        }
    };

    if let Err(e) = send_result {
        // Saturated/closed channel — the consume silently
        // skips otherwise. Surface it loudly so missions
        // that depend on the removal (e.g., FindAmbernol
        // chain 1034 consumes the vial) don't silently
        // strand the player with the item still in their
        // bag while the chain reports completion.
        tracing::error!(
            entity_id, player_id, type_id = item_id, count, chain_id,
            error = %e,
            "Content: RemoveItem cell→base channel send failed — item NOT removed"
        );
    }
}
