//! Handlers for `BaseToCellMsg::UpdateBandolierItem` and
//! `BaseToCellMsg::SyncBandolierItems` — weapon equip/unequip display logic,
//! ammo stat seeding, and OOC holster timer management.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::BandolierItem;

/// Processes a `UpdateBandolierItem` message — inserts the weapon into the
/// bandolier, optionally draws it, arms the OOC holster timer, and pushes the
/// dirty stat packet to the client.
pub(in crate::cell::service) async fn handle_update_bandolier_item(
    entity_id: u32,
    slot_id: i32,
    item: BandolierItem,
    make_active: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Display the newly-equipped weapon and arm the OOC holster
    // timer so it re-holsters after `OOC_HOLSTER_DELAY`. Mirrors
    // `python/cell/SGWPlayer.py:onItemEquipped` which fires
    // `playSequence(Item_Equip)` whenever an item lands in the
    // bandolier. Gated on `make_active` so only the *active*
    // weapon's equip plays the animation (non-active slots are
    // wire-invisible anyway).
    //
    // In-combat equip: weapon is already drawn,
    // `combat_exit_at` stays None (set by `exit_player_combat`
    // when the fight ends). We skip the timer arming here to
    // avoid stamping a timer while threatened_mobs is non-empty,
    // which the holster scan doesn't (yet) gate on combat state.
    let (play_equip_anim, drew_weapon, was_in_combat, entity_state, anim_path) =
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            entity.bandolier_items.insert(slot_id, item);
            let is_player = entity.is_player;
            let player_id = entity.player_id;
            // Fire the equip-display animation when the inserted
            // slot is (or just became) the player's *active*
            // slot. Three cases land here:
            //
            // 1. `make_active=true` — base's bandolier_slot
            //    UPDATE flipped to the new slot; we own
            //    `active_bandolier_slot` from here.
            // 2. `make_active=false` BUT `slot_id` already
            //    equals `active_bandolier_slot` — this is the
            //    initial-grant case: the
            //    base's SQL doesn't flip bandolier_slot when
            //    the INSERT already lands at the player's
            //    default selection (the NOT EXISTS guard finds
            //    the row we just inserted), so
            //    `bandolier_became_active` is false even
            //    though the new weapon IS the active one.
            //    Without this branch the first weapon a
            //    player ever gets never plays its equip
            //    animation or attaches its mesh.
            // 3. `make_active=false` AND `slot_id` !=
            //    `active_bandolier_slot` — the player got a
            //    second/third weapon into a non-active slot.
            //    Mesh stays wherever the active slot is. Skip.
            let slot_is_active = make_active || entity.active_bandolier_slot == slot_id;
            let path = if make_active {
                "make_active=true"
            } else if slot_is_active {
                "initial-grant (slot matches active)"
            } else {
                "non-active slot — skipping"
            };
            if slot_is_active {
                entity.active_bandolier_slot = slot_id;
                let in_combat = !entity.threatened_mobs.is_empty();
                let drew = if !in_combat {
                    entity.set_weapon_holstered(false);
                    entity.combat_exit_at = Some(std::time::Instant::now());
                    // Cancel any in-flight holster Phase 2 — the
                    // player just (re-)equipped, the mesh should
                    // STAY attached; a stale Phase 2 would yank it
                    // away mid-equip.
                    entity.holster_animation_complete_at = None;
                    true
                } else {
                    false
                };
                (true, drew, in_combat, (is_player, player_id), path)
            } else {
                (false, false, false, (is_player, player_id), path)
            }
        } else {
            (false, false, false, (false, None), "entity missing")
        };
    tracing::info!(
        entity_id,
        slot_id,
        make_active,
        play_equip_anim,
        drew_weapon,
        was_in_combat,
        is_player = entity_state.0,
        player_id = ?entity_state.1,
        anim_path,
        "UpdateBandolierItem: equip-display decision"
    );
    if play_equip_anim {
        // Appearance refresh first so the weapon mesh is socket-
        // attached when the `Item_Equip` animation plays. Same
        // ordering principle as combat-enter.
        crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
        crate::cell::cell_methods::player::world::fire_item_sequence(
            entity_id,
            crate::cell::spawner::EVENT_ITEM_EQUIP,
            tx,
            space_mgr,
        )
        .await;
    }
}

/// Processes a `SyncBandolierItems` message — detects weapon-draw/holster
/// transitions, updates ammo stats, fires content-engine equip/unequip events,
/// and manages the two-phase holster animation timer.
pub(in crate::cell::service) async fn handle_sync_bandolier_items(
    entity_id: u32,
    active_bandolier_slot: i32,
    bandolier_items: Vec<(i32, BandolierItem)>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Detect "weapon manually equipped into active slot" — when
    // a player drags a weapon from main inventory into their
    // bandolier slot, base persists the move and dispatches
    // `SyncBandolierItems` with the full new bandolier set. We
    // need to fire the equip-display chain (draw weapon, mesh
    // attach, Item_Equip animation, OOC re-holster timer) when
    // the active slot just gained a weapon it didn't have
    // before. Compare prev vs new item_id in the active slot.
    //
    // (initial player equip goes through
    // `moveInventoryItem`, not `grantItem`. The earlier
    // `UpdateBandolierItem` fix only covered chain-engine
    // grants; the player-driven drag-to-equip case lands here.)
    // Detect "weapon manually equipped into active slot" — when
    // a player drags a weapon from main inventory into their
    // bandolier slot, base persists the move and dispatches
    // `SyncBandolierItems` with the full new bandolier set. We
    // need to fire the equip-display chain (draw weapon, mesh
    // attach, Item_Equip animation, OOC re-holster timer) when
    // the active slot just gained a weapon it didn't have
    // before. Compare prev vs new item_id in the active slot.
    //
    // (initial player equip goes through
    // `moveInventoryItem`, not `grantItem`. The earlier
    // `UpdateBandolierItem` fix only covered chain-engine
    // grants; the player-driven drag-to-equip case lands here.)
    let (prev_active_item_id, new_active_item_id) =
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            let prev = entity
                .bandolier_items
                .get(&active_bandolier_slot)
                .map(|i| i.item_id);
            entity.active_bandolier_slot = active_bandolier_slot;
            entity.bandolier_items = bandolier_items.into_iter().collect();
            let new = entity
                .bandolier_items
                .get(&active_bandolier_slot)
                .map(|i| i.item_id);

            // Re-seed AmmoSlot{N} stats from the new bandolier set so the
            // client's bandolier UI reflects the actual ammo of any newly-
            // equipped weapon. Without this, post-vendor-buy or post-grant
            // bars show stale stats from the previous weapon.
            //
            // Slots that disappeared from `bandolier_items` get reset to
            // (0, 0, 0) so an empty slot's bar clears.
            let new_states: Vec<(i32, i32, i32)> = (0..5)
                .map(|slot_id| {
                    let (cur, max) = entity
                        .bandolier_items
                        .get(&slot_id)
                        .map_or((0, 0), |item| (item.current_ammo, item.clip_size));
                    (slot_id, cur, max)
                })
                .collect();
            for (slot_id, cur, max) in new_states {
                let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                if let Some(stat) = entity.stats.get_mut(stat_id) {
                    stat.update(0, cur, max);
                }
            }
            (prev, new)
        } else {
            (None, None)
        };

    // Borrow released — push the dirty stats out.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        let payload = entity.stats.serialize_dirty();
        entity.stats.clear_dirty();
        if !payload.is_empty() {
            crate::cell::abilities::send_entity_method(entity_id, 20, payload, tx, space_mgr).await;
        }
    }

    // Re-borrow to make the equip-display decision.
    let active_slot_gained_weapon =
        new_active_item_id.is_some() && new_active_item_id != prev_active_item_id;
    // Player-driven unequip (active slot LOST its weapon):
    // disarm any in-flight OOC re-holster timer so the
    // `Item_Unequip` animation doesn't fire 10s later against
    // a now-empty hand. Also reset `weapon_holstered=true`
    // so the cached state matches the now-empty bandolier
    // (next equip's prev-vs-new check sees a clean baseline).
    let active_slot_lost_weapon = prev_active_item_id.is_some() && new_active_item_id.is_none();
    // Look up the holstered weapon's class-specific animation
    // duration BEFORE the mutable borrow below (can't borrow
    // space_mgr.item_defs and an entity_mut at the same time).
    let holster_duration = prev_active_item_id
        .and_then(|id| space_mgr.item_defs.get(&id))
        .map(|d| d.holster_animation_duration)
        .unwrap_or(crate::cell::service::ticks::HOLSTER_ANIMATION_DURATION);
    let (play_equip_anim, play_holster_anim, drew_weapon, was_in_combat, entity_state, anim_path) =
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            let is_player = entity.is_player;
            let player_id = entity.player_id;
            let in_combat = !entity.threatened_mobs.is_empty();
            let path = if active_slot_lost_weapon {
                // Player-driven unequip — schedule the same
                // two-phase choreography the OOC holster timer
                // uses: fire `Item_Unequip` now,
                // leave the mesh attached (weapon_holstered
                // stays at its current value), and arm a
                // Phase 2 via `holster_animation_complete_at`
                // so `holster_timer_tick` dispatches the
                // eventual `RefreshAppearance(holstered=true)`
                // after the animation plays out.
                //
                // The base-side `sync_bandolier_after_inventory_change`
                // call defers its `refresh_player_appearance`
                // for unequip so the mesh stays during the
                // animation — without that defer, the base
                // yanks the mesh immediately and the user sees
                // no animation.
                entity.combat_exit_at = None;
                entity.holster_animation_complete_at =
                    Some(std::time::Instant::now() + holster_duration);
                "active slot lost weapon (unequip) — fire Item_Unequip + Phase 2"
            } else if !active_slot_gained_weapon {
                if new_active_item_id.is_none() {
                    "active slot is empty — skip"
                } else {
                    "active slot unchanged — skip"
                }
            } else if in_combat {
                "in combat — skip OOC timer arming"
            } else {
                "active slot gained weapon (OOC) — draw + animate"
            };
            if active_slot_gained_weapon && !in_combat {
                entity.set_weapon_holstered(false);
                entity.combat_exit_at = Some(std::time::Instant::now());
                entity.holster_animation_complete_at = None;
                (true, false, true, false, (is_player, player_id), path)
            } else if active_slot_lost_weapon {
                (false, true, false, in_combat, (is_player, player_id), path)
            } else {
                (false, false, false, in_combat, (is_player, player_id), path)
            }
        } else {
            (false, false, false, false, (false, None), "entity missing")
        };
    tracing::info!(
        entity_id,
        active_bandolier_slot,
        play_equip_anim,
        play_holster_anim,
        drew_weapon,
        was_in_combat,
        is_player = entity_state.0,
        player_id = ?entity_state.1,
        anim_path,
        "SyncBandolierItems: equip-display decision"
    );
    if play_equip_anim {
        crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
        crate::cell::cell_methods::player::world::fire_item_sequence(
            entity_id,
            crate::cell::spawner::EVENT_ITEM_EQUIP,
            tx,
            space_mgr,
        )
        .await;
    } else if play_holster_anim {
        // Fire `Item_Unequip` (event 4001) — the bandolier
        // take-off animation. Phase 2 of `holster_timer_tick`
        // will broadcast the mesh removal once
        // `HOLSTER_ANIMATION_DURATION` has elapsed.
        crate::cell::cell_methods::player::world::fire_item_sequence(
            entity_id,
            crate::cell::spawner::EVENT_ITEM_UNEQUIP,
            tx,
            space_mgr,
        )
        .await;
    }
}
