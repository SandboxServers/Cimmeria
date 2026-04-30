//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::CellEntity;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const REMOVE_ITEM: u16 = 36;
pub const LIST_ITEMS: u16 = 37;
pub const MOVE_ITEM: u16 = 38;
pub const USE_ITEM: u16 = 39;
pub const REPAIR_ITEM_REQUEST: u16 = 40;
pub const REQUEST_ACTIVE_SLOT_CHANGE: u16 = 41;
pub const REQUEST_AMMO_CHANGE: u16 = 42;

/// `GENERICPROPERTY_AmmoTypeId` from `entities/defs/enumerations.xml`. Used as
/// the property-id arg for `onEntityProperty` ammo-type indicator updates.
const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;

/// Build the `onEntityProperty(propId, value)` arg payload (8 bytes LE).
fn build_entity_property_args(prop_id: i32, value: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&prop_id.to_le_bytes());
    args.extend_from_slice(&value.to_le_bytes());
    args
}

/// Drain `entity.bandolier_ammo_dirty` and emit one `BandolierAmmoUpdate` per
/// slot. Used by the logout / world-transition flush paths.
///
/// Slots that are dirty but no longer have a `BandolierItem` (e.g. swapped out
/// before logout) are silently dropped — there's nothing to persist for them.
pub async fn flush_dirty_bandolier_ammo(
    entity: &mut CellEntity,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    if entity.bandolier_ammo_dirty.is_empty() {
        return;
    }
    // Drain into a Vec so we don't hold the borrow on `entity` across awaits.
    let dirty: Vec<i32> = entity.bandolier_ammo_dirty.drain().collect();
    for slot_id in dirty {
        let (item_id, current_ammo, cur_ammo_type) = match entity.bandolier_items.get(&slot_id) {
            Some(item) => (item.item_id, item.current_ammo, item.cur_ammo_type),
            None => continue,
        };
        let _ = tx.send(CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id: item_id,
            current_ammo,
            cur_ammo_type,
        }).await;
    }
}

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

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    // All inventory ops require a player_id — we refuse to silently default
    // to 0 (would either no-op or hit a sentinel row).
    let resolve_player_id = |op: &str| -> Option<i32> {
        match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
            Some(id) => Some(id),
            None => {
                tracing::warn!(entity_id, op, "inventory op dropped: entity has no player_id");
                None
            }
        }
    };

    match method_index {
        REMOVE_ITEM => {
            if args.len() >= 6 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let quantity = i16::from_le_bytes([args[4], args[5]]) as i32;
                tracing::debug!(entity_id, item_id, quantity, "removeItem");
                if let Some(player_id) = resolve_player_id("removeItem") {
                    let _ = tx.send(CellToBaseMsg::RemoveInventoryItem {
                        entity_id, player_id, item_id, quantity,
                    }).await;
                }
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "removeItem: truncated args");
            }
            true
        }
        LIST_ITEMS => {
            tracing::debug!(entity_id, "listItems");
            if let Some(player_id) = resolve_player_id("listItems") {
                let _ = tx.send(CellToBaseMsg::ListInventoryItems { entity_id, player_id }).await;
            }
            true
        }
        MOVE_ITEM => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_container_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_slot_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let quantity = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, item_id, target_container_id, target_slot_id, quantity, "moveItem");
                if let Some(player_id) = resolve_player_id("moveItem") {
                    let _ = tx.send(CellToBaseMsg::MoveInventoryItem {
                        entity_id, player_id, item_id,
                        target_container_id, target_slot_id, quantity,
                    }).await;
                }
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "moveItem: truncated args");
            }
            true
        }
        USE_ITEM => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, target_id, "useItem");

                if let Some(player_id) = resolve_player_id("useItem") {
                    crate::cell::content::fire_item_use(
                        entity_id, player_id, item_id, engine, tx, space_mgr,
                    ).await;
                }
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "useItem: truncated args");
            }
            true
        }
        REPAIR_ITEM_REQUEST => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let repair_ratio = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, repair_ratio, "UNIMPLEMENTED: repairItemRequest");
            }
            true
        }
        REQUEST_ACTIVE_SLOT_CHANGE => {
            if args.len() >= 8 {
                let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let slot_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, bag_id, slot_id, "requestActiveSlotChange");
                // Only bandolier (container 3) carries an active slot.
                if bag_id == 3 {
                    // Resolve player_id BEFORE taking the mutable borrow on
                    // space_mgr, otherwise the immutable borrow inside
                    // `resolve_player_id` would alias.
                    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                        Some(id) => Some(id),
                        None => {
                            tracing::warn!(entity_id, "requestActiveSlotChange dropped: entity has no player_id");
                            None
                        }
                    };

                    // Phase 1: capture the previous slot's pending-flush state
                    // and the new slot's `cur_ammo_type` while we have a
                    // mutable borrow. Drop the borrow before any awaits.
                    //
                    // Stage D: if the previously-active slot was dirty (fires
                    // since the last flush), emit one BandolierAmmoUpdate for
                    // it before swapping. The reload-completion tick already
                    // drains the active slot when reloads finish; this catches
                    // mid-magazine swaps where the player swaps weapons before
                    // reloading the empty one.
                    let (prev_persist, new_ammo_type): (Option<(i32, i32, i32, i32)>, Option<i32>) = {
                        let entity = match space_mgr.get_entity_mut(entity_id) {
                            Some(e) => e,
                            None => return true,
                        };
                        let prev_slot = entity.active_bandolier_slot;
                        let prev_persist = if entity.bandolier_ammo_dirty.contains(&prev_slot) {
                            entity.bandolier_items.get(&prev_slot).map(|item| {
                                (prev_slot, item.item_id, item.current_ammo, item.cur_ammo_type)
                            })
                        } else {
                            None
                        };
                        if prev_persist.is_some() {
                            entity.bandolier_ammo_dirty.remove(&prev_slot);
                        }
                        // Cancel any in-flight reload only if the swap actually
                        // moves to a different slot. A same-slot "swap" (no-op
                        // from the client, or replayed packet) must not cancel
                        // an in-progress reload. Otherwise the fire-gate would
                        // still block until the deadline, and the tick would
                        // refill the original slot — but the player has moved
                        // on; re-pressing R after swapping back restarts the
                        // reload from scratch.
                        if slot_id != prev_slot && entity.reload_complete_at.is_some() {
                            entity.reload_complete_at = None;
                            entity.reload_slot_id = None;
                            tracing::debug!(
                                entity_id, prev_slot, new_slot = slot_id,
                                "active slot change cancelled in-flight reload"
                            );
                        }
                        entity.active_bandolier_slot = slot_id;
                        let new_ammo_type = entity.bandolier_items
                            .get(&slot_id)
                            .map(|i| i.cur_ammo_type);
                        (prev_persist, new_ammo_type)
                    };

                    // Phase 2: send messages now that the borrow is released.
                    if let Some(player_id) = player_id {
                        if let Some((p_slot, item_id, current_ammo, cur_ammo_type)) = prev_persist {
                            let _ = tx.send(CellToBaseMsg::BandolierAmmoUpdate {
                                player_id,
                                slot_id: p_slot,
                                expected_item_id: item_id,
                                current_ammo,
                                cur_ammo_type,
                            }).await;
                        }
                        let _ = tx.send(CellToBaseMsg::ActiveSlotUpdate { player_id, slot_id }).await;
                    }

                    // Stage D: refresh the client's ammo-type indicator for the
                    // newly-active slot. If the new slot is empty (no item),
                    // send 0 to mirror legacy "no weapon equipped" behavior
                    // (SGWPlayer.py:522 — `activeItem.ammoType if activeItem else 0`).
                    let property_value = new_ammo_type.unwrap_or(0);
                    let property_args = build_entity_property_args(
                        GENERICPROPERTY_AMMO_TYPE_ID,
                        property_value,
                    );
                    crate::cell::abilities::send_entity_method(
                        entity_id,
                        crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                        property_args,
                        tx,
                        space_mgr,
                    ).await;
                } else {
                    tracing::warn!(entity_id, bag_id, slot_id, "requestActiveSlotChange: ignoring non-bandolier container");
                }
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "requestActiveSlotChange: truncated args");
            }
            true
        }
        REQUEST_AMMO_CHANGE => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, item_id, ammo_type, "requestAmmoChange");

                // Loose validation — the legacy is literally `pass`. Reject the
                // obvious-junk case (zero), then accept whatever the client
                // sent. The proper whitelist lives on the item template's
                // `ammo_types` (crates/entity/src/inventory.rs:81).
                // TODO: validate against item.ammo_types whitelist
                //       (see crates/entity/src/inventory.rs:81 — `Item.ammo_types`).
                if ammo_type == 0 {
                    tracing::warn!(entity_id, item_id, "requestAmmoChange: rejecting zero ammo_type");
                    return true;
                }

                // Phase 1: locate the slot, mutate, capture the BandolierAmmoUpdate
                // payload + active-slot flag, drop the mutable borrow.
                let (player_id, persist, is_active) = {
                    let entity = match space_mgr.get_entity_mut(entity_id) {
                        Some(e) => e,
                        None => return true,
                    };
                    // Match all slots holding this item_id. The wire request
                    // doesn't carry a slot/instance id, so duplicate weapons
                    // are ambiguous — reject rather than guess. A future
                    // protocol revision should add slot id to the message.
                    let matches: Vec<i32> = entity.bandolier_items.iter()
                        .filter(|(_, item)| item.item_id == item_id)
                        .map(|(s, _)| *s)
                        .collect();
                    let slot = match matches.as_slice() {
                        [s] => *s,
                        [] => {
                            tracing::warn!(entity_id, item_id, "requestAmmoChange: item not in bandolier");
                            return true;
                        }
                        ambiguous => {
                            tracing::warn!(
                                entity_id, item_id,
                                slot_count = ambiguous.len(),
                                "requestAmmoChange: ambiguous — multiple slots hold this item_id, rejecting"
                            );
                            return true;
                        }
                    };
                    // Mutate the slot. We mark dirty (so any in-flight flush
                    // path sees the change) and immediately drain — the
                    // BandolierAmmoUpdate emitted below persists it now.
                    let item = entity.bandolier_items.get_mut(&slot).unwrap();
                    item.cur_ammo_type = ammo_type;
                    let item_id_for_persist = item.item_id;
                    let current_ammo = item.current_ammo;
                    entity.bandolier_ammo_dirty.insert(slot);
                    entity.bandolier_ammo_dirty.remove(&slot);
                    let is_active = slot == entity.active_bandolier_slot;
                    let player_id = entity.player_id;
                    (player_id, (slot, item_id_for_persist, current_ammo, ammo_type), is_active)
                };

                // Phase 2: persist + (if active) refresh the client's ammo-type indicator.
                if let Some(player_id) = player_id {
                    let (slot_id, expected_item_id, current_ammo, cur_ammo_type) = persist;
                    let _ = tx.send(CellToBaseMsg::BandolierAmmoUpdate {
                        player_id,
                        slot_id,
                        expected_item_id,
                        current_ammo,
                        cur_ammo_type,
                    }).await;
                } else {
                    tracing::warn!(entity_id, "requestAmmoChange: entity has no player_id — skipping persist");
                }

                if is_active {
                    let property_args = build_entity_property_args(
                        GENERICPROPERTY_AMMO_TYPE_ID,
                        ammo_type,
                    );
                    crate::cell::abilities::send_entity_method(
                        entity_id,
                        crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                        property_args,
                        tx,
                        space_mgr,
                    ).await;
                }
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "requestAmmoChange: truncated args");
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::AbilityDef;
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::{AMMO_SLOT_1, AMMO_SLOT_2};

    use crate::cell::abilities::handle_use_ability;

    fn make_test_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    /// Register a no-warmup, ranged ability with `required_ammo = 1` and no
    /// event-set (silences onSequence noise during tests).
    fn register_test_fire_ability(mgr: &mut SpaceManager, ability_id: i32) {
        mgr.ability_defs.insert(ability_id, AbilityDef {
            ability_id,
            name: format!("test_fire_{ability_id}"),
            cooldown: 0.001, // very short so back-to-back fires aren't gated
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        });
    }

    /// Stage E: per-slot ammo must survive an active-slot swap.
    ///
    /// Simplified vs the prompt — instead of running the full handle_use_ability
    /// twice on slot 0 then once on slot 1 (which would require waiting out
    /// cooldowns), we exercise the swap path directly: fire once, swap, fire
    /// once, swap back, and verify both slots' ammo is preserved across the
    /// swap. The slot-swap message order assertion is the load-bearing piece.
    #[tokio::test]
    async fn slot_swap_preserves_per_slot_ammo() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let ability_id = 5001;
        register_test_fire_ability(&mut mgr, ability_id);

        // Two bandolier slots: slot 0 (clip 30, full), slot 1 (clip 12, full).
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.abilities.add_ability(ability_id);
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 10, clip_size: 30, default_ammo_type: 1,
                current_ammo: 30, cur_ammo_type: 1,
            });
            e.bandolier_items.insert(1, BandolierItem {
                item_id: 11, clip_size: 12, default_ammo_type: 7,
                current_ammo: 12, cur_ammo_type: 7,
            });
            e.active_bandolier_slot = 0;
            if let Some(s) = e.stats.get_mut(AMMO_SLOT_1) { s.update(0, 30, 30); s.clear_dirty(); }
            if let Some(s) = e.stats.get_mut(AMMO_SLOT_2) { s.update(0, 12, 12); s.clear_dirty(); }
        }

        let (tx, mut rx) = mpsc::channel(64);

        // Fire two shots on slot 0 → current_ammo == 28 (cooldown is 1ms,
        // so the second fire happens past the deadline).
        handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
        assert_eq!(mgr.get_entity(1).unwrap().bandolier_items[&0].current_ammo, 28);

        // Drain rx so the swap-message assertions below can scan a clean buffer.
        while rx.try_recv().is_ok() {}

        // ── Swap to slot 1 ──────────────────────────────────────────────
        // Args: bag_id=3 (bandolier), slot_id=1.
        let mut swap_args = Vec::with_capacity(8);
        swap_args.extend_from_slice(&3i32.to_le_bytes());
        swap_args.extend_from_slice(&1i32.to_le_bytes());
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &swap_args, &tx, &mut mgr, &engine).await;

        // Stage D message order on swap: BandolierAmmoUpdate(prev=0, ammo=28)
        // → ActiveSlotUpdate(1) → onEntityProperty(AmmoTypeId, slot1.cur_ammo_type=7).
        let m1 = rx.try_recv().expect("expected BandolierAmmoUpdate(prev slot)");
        match m1 {
            CellToBaseMsg::BandolierAmmoUpdate { player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type } => {
                assert_eq!(player_id, 100);
                assert_eq!(slot_id, 0, "first swap msg should flush prev slot 0");
                assert_eq!(expected_item_id, 10, "should carry slot 0's item_id for TOCTOU guard");
                assert_eq!(current_ammo, 28);
                assert_eq!(cur_ammo_type, 1);
            }
            other => panic!("expected BandolierAmmoUpdate, got {other:?}"),
        }
        let m2 = rx.try_recv().expect("expected ActiveSlotUpdate");
        match m2 {
            CellToBaseMsg::ActiveSlotUpdate { player_id, slot_id } => {
                assert_eq!(player_id, 100);
                assert_eq!(slot_id, 1);
            }
            other => panic!("expected ActiveSlotUpdate, got {other:?}"),
        }
        let m3 = rx.try_recv().expect("expected onEntityProperty(AmmoTypeId)");
        match m3 {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
                assert_eq!(entity_id, 1);
                assert_eq!(
                    method_index,
                    crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                    "third swap msg should be onEntityProperty"
                );
                let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(prop_id, GENERICPROPERTY_AMMO_TYPE_ID);
                assert_eq!(value, 7, "ammo type should be slot 1's cur_ammo_type");
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }

        // Fire once on slot 1 → current_ammo == 11.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
        assert_eq!(mgr.get_entity(1).unwrap().bandolier_items[&1].current_ammo, 11);

        // ── Swap back to slot 0 ─────────────────────────────────────────
        let mut swap_back = Vec::with_capacity(8);
        swap_back.extend_from_slice(&3i32.to_le_bytes());
        swap_back.extend_from_slice(&0i32.to_le_bytes());
        dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &swap_back, &tx, &mut mgr, &engine).await;

        // Slot 0's ammo is preserved across the swap.
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(entity.bandolier_items[&0].current_ammo, 28, "slot 0 ammo preserved across swap");
        assert_eq!(entity.bandolier_items[&1].current_ammo, 11, "slot 1 ammo preserved across swap");
        assert_eq!(entity.stats.get(AMMO_SLOT_1).unwrap().cur, 28, "AmmoSlot1 stat still reads 28");
        assert_eq!(entity.active_bandolier_slot, 0);
    }

    /// CodeRabbit #4: starting a reload on slot 0, swapping to slot 1, then
    /// letting the warmup elapse must NOT refill slot 1 with slot 0's clip
    /// size. The fix cancels the in-flight reload on swap, so the tick has
    /// nothing to promote and slot 1's ammo stays where it was.
    #[tokio::test]
    async fn slot_swap_cancels_in_flight_reload() {
        use crate::cell::content::build_engine;
        use cimmeria_entity::cell_entity::BandolierItem;

        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 10, clip_size: 30, default_ammo_type: 1,
                current_ammo: 5, cur_ammo_type: 1,
            });
            e.bandolier_items.insert(1, BandolierItem {
                item_id: 11, clip_size: 12, default_ammo_type: 7,
                current_ammo: 8, cur_ammo_type: 7,
            });
            e.active_bandolier_slot = 0;
            // Simulate a reload of slot 0 currently warming up.
            e.reload_complete_at = Some(
                std::time::Instant::now() + std::time::Duration::from_secs(10),
            );
            e.reload_slot_id = Some(0);
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(64);
        let engine = build_engine(None).await;

        // Swap to slot 1.
        let mut swap = Vec::with_capacity(8);
        swap.extend_from_slice(&3i32.to_le_bytes());
        swap.extend_from_slice(&1i32.to_le_bytes());
        dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &swap, &tx, &mut mgr, &engine).await;

        let entity = mgr.get_entity(1).unwrap();
        assert!(entity.reload_complete_at.is_none(), "swap should cancel in-flight reload");
        assert!(entity.reload_slot_id.is_none(), "swap should clear pinned slot");
        assert_eq!(entity.bandolier_items[&0].current_ammo, 5, "cancelled reload must NOT refill slot 0");
        assert_eq!(entity.bandolier_items[&1].current_ammo, 8, "slot 1 untouched");
        assert_eq!(entity.active_bandolier_slot, 1);

        // Drain the rx so the channel doesn't error if assertions above
        // succeeded (slot-swap emits ActiveSlotUpdate and onEntityProperty).
        while rx.try_recv().is_ok() {}
    }

    /// Stage E: requestAmmoChange must update the slot's `cur_ammo_type`,
    /// emit exactly one BandolierAmmoUpdate (drained immediately, NOT pending
    /// for logout flush), and — when the slot is active — push an
    /// `onEntityProperty(AmmoTypeId)` to the client.
    #[tokio::test]
    async fn request_ammo_change_updates_slot_and_sends_property() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 42, clip_size: 30, default_ammo_type: 1,
                current_ammo: 20, cur_ammo_type: 1,
            });
            e.active_bandolier_slot = 0;
        }

        let (tx, mut rx) = mpsc::channel(16);

        // Args: item_id=42, ammo_type=3 (8 bytes LE).
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&42i32.to_le_bytes());
        args.extend_from_slice(&3i32.to_le_bytes());

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        dispatch(1, REQUEST_AMMO_CHANGE, &args, &tx, &mut mgr, &engine).await;

        // Slot mutated, dirty NOT set (drained immediately by the handler).
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(entity.bandolier_items[&0].cur_ammo_type, 3);
        assert!(!entity.bandolier_ammo_dirty.contains(&0), "dirty flag should be drained immediately");

        // First message: BandolierAmmoUpdate carrying the new type + existing ammo.
        let m1 = rx.try_recv().expect("expected BandolierAmmoUpdate");
        match m1 {
            CellToBaseMsg::BandolierAmmoUpdate { player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type } => {
                assert_eq!(player_id, 100);
                assert_eq!(slot_id, 0);
                assert_eq!(expected_item_id, 42, "should carry the slot's item_id for TOCTOU guard");
                assert_eq!(current_ammo, 20);
                assert_eq!(cur_ammo_type, 3);
            }
            other => panic!("expected BandolierAmmoUpdate, got {other:?}"),
        }

        // Second message: onEntityProperty(AmmoTypeId, 3) since slot is active.
        let m2 = rx.try_recv().expect("expected onEntityProperty");
        match m2 {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
                assert_eq!(entity_id, 1);
                assert_eq!(
                    method_index,
                    crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                );
                let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(prop_id, GENERICPROPERTY_AMMO_TYPE_ID);
                assert_eq!(value, 3);
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }

        // No additional messages.
        assert!(rx.try_recv().is_err(), "no further rx messages expected");
    }

    /// Stage E: ammo_type=0 is rejected with a warn, no rx messages emitted.
    /// Matches the deviation Stage D introduced to filter obvious-junk requests.
    #[tokio::test]
    async fn request_ammo_change_rejects_zero() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.bandolier_items.insert(0, BandolierItem {
                item_id: 42, clip_size: 30, default_ammo_type: 1,
                current_ammo: 20, cur_ammo_type: 1,
            });
            e.active_bandolier_slot = 0;
        }

        let (tx, mut rx) = mpsc::channel(8);

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&42i32.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes());

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let handled = dispatch(1, REQUEST_AMMO_CHANGE, &args, &tx, &mut mgr, &engine).await;

        // Handler still returns true (claims the index), but no side effects.
        assert!(handled, "REQUEST_AMMO_CHANGE should be claimed by the inventory dispatch");
        assert!(rx.try_recv().is_err(), "no rx messages should be emitted for ammo_type=0");
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(entity.bandolier_items[&0].cur_ammo_type, 1, "cur_ammo_type should be unchanged");
        assert!(!entity.bandolier_ammo_dirty.contains(&0));
    }
}
