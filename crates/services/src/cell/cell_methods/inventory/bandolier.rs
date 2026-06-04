//! Bandolier slot operations: persistence flush, active-slot swap, and
//! per-slot ammo-type change. These handlers interleave reads and mutations
//! on the cell entity, so each carefully scopes its `&mut` borrows around
//! await points.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::CellEntity;
use tokio::sync::mpsc;

use super::constants::{build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID};

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
    // Snapshot dirty slot IDs without draining; we only clear each marker
    // after its send succeeds, so a transient channel failure leaves the
    // marker in place and the next flush retries it.
    let dirty: Vec<i32> = entity.bandolier_ammo_dirty.iter().copied().collect();
    for slot_id in dirty {
        let (item_id, current_ammo, cur_ammo_type) = match entity.bandolier_items.get(&slot_id) {
            Some(item) => (item.item_id, item.current_ammo, item.cur_ammo_type),
            None => {
                // Dirty slot with no item -- nothing to persist. Drop marker.
                entity.bandolier_ammo_dirty.remove(&slot_id);
                continue;
            }
        };
        match tx
            .send(CellToBaseMsg::BandolierAmmoUpdate {
                player_id,
                slot_id,
                expected_item_id: item_id,
                current_ammo,
                cur_ammo_type,
            })
            .await
        {
            Ok(()) => {
                entity.bandolier_ammo_dirty.remove(&slot_id);
            }
            Err(e) => {
                tracing::warn!(
                    player_id, slot_id, item_id, current_ammo, cur_ammo_type,
                    error = %e,
                    "BandolierAmmoUpdate send failed; leaving slot dirty for retry"
                );
                // Subsequent sends will likely also fail; preserve ordering
                // by retrying the whole set on the next flush.
                break;
            }
        }
    }
}

#[tracing::instrument(
    name = "bandolier.request_active_slot_change",
    level = "info",
    skip_all,
    fields(entity_id, args_len = args.len()),
)]
pub(crate) async fn handle_request_active_slot_change(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 8 {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "requestActiveSlotChange: truncated args"
        );
        return;
    }
    let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let wire_slot_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    // Wire slot IDs are 1-indexed (matching the legacy convention in
    // `Bag.py:369` `onActiveSlotUpdate(self.bagId, self.activeSlotId + 1)`
    // and `SGWPlayer.py:2192` `updateActiveSlot(bagId, slotId - 1)`).
    // Translate to the 0-indexed server slot before any further work.
    // Saturating subtraction so a forged `i32::MIN` doesn't panic in debug
    // builds — the resulting `i32::MIN` still fails the range check below.
    let slot_id = wire_slot_id.saturating_sub(1);
    tracing::debug!(
        entity_id,
        bag_id,
        wire_slot_id,
        slot_id,
        "requestActiveSlotChange"
    );
    // Only bandolier (container 3) carries an active slot. Validate this
    // before the slot range check so a forged non-bandolier request gets
    // logged with the original bag_id rather than a misleading "out of
    // range" message.
    if bag_id != 3 {
        tracing::warn!(
            entity_id,
            bag_id,
            wire_slot_id,
            "requestActiveSlotChange: ignoring non-bandolier container"
        );
        return;
    }
    // Reject out-of-range slots before any mutation. Bound is `bag_max_slots(3)`
    // (currently 4 → server slots 0..=3, wire slots 1..=4); a forged value
    // would otherwise leave the entity in an impossible local state, cancel
    // any in-flight reload, and propagate via ActiveSlotUpdate to base.
    let max_slots = crate::base::resources::bag_max_slots(bag_id);
    if !(0..max_slots).contains(&slot_id) {
        tracing::warn!(
            entity_id,
            bag_id,
            wire_slot_id,
            slot_id,
            max_slots,
            "requestActiveSlotChange: slot_id out of range, rejecting"
        );
        return;
    }

    // Resolve player_id BEFORE taking the mutable borrow on
    // space_mgr, otherwise the immutable borrow would alias.
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => Some(id),
        None => {
            tracing::warn!(
                entity_id,
                "requestActiveSlotChange dropped: entity has no player_id"
            );
            None
        }
    };

    // Swap-with-choreography case: when both the current and target
    // slots hold weapons, play `Item_Unequip` first, then defer the
    // actual slot change until the holster animation has played out.
    // `pending_slot_swap_tick` re-invokes this handler with the
    // target slot once the animation window elapses; the re-entry
    // is gated by `pending_slot_swap_at.is_some()` so it skips this
    // branch and runs the normal swap path.
    //
    // The choreography fires regardless of combat state. Swapping
    // weapons mid-fight is supposed to cost real time — the
    // animation duration plus the ability lockout (weapon attacks
    // are rejected while `pending_slot_swap_at` is set; see
    // `handle_use_ability`). Without that penalty, weapons would
    // teleport in and out of the player's hands and the swap would
    // be free, defeating the bandolier as a real loadout choice.
    //
    // Skipped when:
    // - Already mid-choreography (re-entry from the tick)
    // - Same slot (no-op or replayed packet)
    // - Source slot empty (no weapon to holster)
    // - Target slot empty (handled by the normal path's empty-slot
    //   branch — Item_Unequip there is the right next step, not the
    //   weapon-to-weapon choreography)
    let needs_holster_first = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.pending_slot_swap_at.is_none()
                && e.active_bandolier_slot != slot_id
                && e.bandolier_items.contains_key(&e.active_bandolier_slot)
                && e.bandolier_items.contains_key(&slot_id)
        }
        None => false,
    };
    if needs_holster_first {
        // Per-weapon holster duration: longarms (P90, AR, LMG) take
        // longer to stow than sidearms (pistol). Using a single
        // constant cuts the longarm animation off mid-flight and
        // the new weapon's draw starts on top of the still-playing
        // holster — the user's playtest glitch on P90→pistol.
        let outgoing_item_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.bandolier_items.get(&e.active_bandolier_slot))
            .map(|item| item.item_id);
        let duration = outgoing_item_id
            .and_then(|id| space_mgr.item_defs.get(&id))
            .map(|d| d.holster_animation_duration)
            .unwrap_or(crate::cell::service::ticks::HOLSTER_ANIMATION_DURATION);
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.pending_slot_swap_at = Some(std::time::Instant::now() + duration);
            e.pending_slot_swap_target = Some(slot_id);
            // Discard any in-flight reload-Phase-A draw or queued attack
            // from the OUTGOING weapon. By committing to the swap the
            // player has abandoned the previous slot — letting the Phase
            // A tick re-enter `handle_reload` after the swap would refill
            // the NEW slot (Phase B pins `reload_slot_id` to the
            // currently-active slot, not the slot reload started on), and
            // letting `pending_attack_tick` re-fire the queued ability
            // would consume the NEW slot's ammo with the OLD slot's
            // ability id (e.g. queued Pistol Shot → swap to Staff →
            // tick fires Pistol Shot off the Staff's ammo bar).
            //
            // `reload_complete_at` / `reload_slot_id` are cancelled below
            // in the immediate-swap path; we don't touch them here
            // because Phase B is gated on `pending_reload_at.is_none()`
            // and stays dormant until the tick fires.
            e.pending_reload_at = None;
            e.pending_attack_at = None;
            e.pending_attack_ability_id = None;
            e.pending_attack_target_id = None;
        }
        tracing::info!(
            entity_id,
            target_slot = slot_id,
            outgoing_item_id,
            duration_ms = duration.as_millis() as u64,
            "requestActiveSlotChange: weapon→weapon swap, firing Item_Unequip + deferring slot change"
        );
        crate::cell::cell_methods::player::world::fire_item_sequence(
            entity_id,
            crate::cell::spawner::EVENT_ITEM_UNEQUIP,
            tx,
            space_mgr,
        )
        .await;
        return;
    }
    // Re-entry from the tick: pending state was kept set so the
    // choreography check above could short-circuit. Now that we're
    // past the gate, clear it — the normal slot-change path below
    // takes over.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        if e.pending_slot_swap_at.is_some() {
            e.pending_slot_swap_at = None;
            e.pending_slot_swap_target = None;
        }
    }

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
    let (prev_persist, new_ammo_type, prev_slot_for_log, auto_cycle_clear_state): (
        Option<(i32, i32, i32, i32)>,
        Option<i32>,
        i32,
        Option<u32>,
    ) = {
        let entity = match space_mgr.get_entity_mut(entity_id) {
            Some(e) => e,
            None => return,
        };
        let prev_slot = entity.active_bandolier_slot;
        let prev_persist = if entity.bandolier_ammo_dirty.contains(&prev_slot) {
            entity.bandolier_items.get(&prev_slot).map(|item| {
                (
                    prev_slot,
                    item.item_id,
                    item.current_ammo,
                    item.cur_ammo_type,
                )
            })
        } else {
            None
        };
        // Leave the dirty marker in place; phase 2 only clears it after the
        // BandolierAmmoUpdate send succeeds, so a failed send doesn't lose
        // pending persistence state.
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
                entity_id,
                prev_slot,
                new_slot = slot_id,
                "active slot change cancelled in-flight reload"
            );
        }
        // Mirror the choreography branch: a real slot change discards
        // any in-flight reload Phase A draw and queued attack from the
        // outgoing weapon. Same-slot calls (no-ops / replayed packets)
        // leave them alone — the player hasn't expressed intent to
        // change weapons, so the queued action should still resolve.
        // This branch catches the no-choreography paths (one slot
        // empty) and the tick re-entry path (which clears
        // `pending_slot_swap_at` above and falls through to here).
        // Weapon swap also clears auto-cycle stash + last-fired stash.
        // The stashed ability ids resolved against the OUTGOING weapon
        // (via `items_event_sets`); firing them on the incoming weapon
        // either silently rejects in `use_ability` (the new weapon
        // doesn't grant the stashed id) or misfires the wrong ability.
        // Live observation: lomiada1 session 2026-06-04 10:04 — auto-
        // cycle fired stale ability 559 (SMG) after swap to a non-SMG
        // weapon. Re-engaging auto-cycle on the new weapon's slot is
        // the player's explicit choice.
        let auto_cycle_clear_state = if slot_id != prev_slot {
            entity.pending_reload_at = None;
            entity.pending_attack_at = None;
            entity.pending_attack_ability_id = None;
            entity.pending_attack_target_id = None;

            let had_auto_cycle = entity.abilities.auto_cycle;
            entity.abilities.auto_cycle = false;
            entity.abilities.auto_cycle_ability_id = None;
            entity.abilities.last_fired_ability_id = None;
            if had_auto_cycle {
                let old = entity.state_field;
                entity.state_field &= !crate::cell::combat::BSF_AUTO_CYCLING;
                (entity.state_field != old).then_some(entity.state_field)
            } else {
                None
            }
        } else {
            None
        };
        entity.active_bandolier_slot = slot_id;
        let new_ammo_type = entity
            .bandolier_items
            .get(&slot_id)
            .map(|i| i.cur_ammo_type);
        (
            prev_persist,
            new_ammo_type,
            prev_slot,
            auto_cycle_clear_state,
        )
    };
    // Observability: log the resolved weapon ability for the new slot so
    // SigNoz can verify Phase 1's per-weapon resolution is firing
    // correctly on swap. Pre- every weapon resolved to ability 592
    // (Pistol Shot hardcode); now the resolution is per-item via
    // items_event_sets. Log carries enough context to spot regressions:
    // "swapped to slot N, item M, resolved to ability A" — a missing
    // ability id (None) flags an unbound item that the right-click
    // fallback would route to 592, which is the canary for a content gap.
    {
        let item_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.bandolier_items.get(&slot_id).map(|b| b.item_id));
        let resolved_ranged_ability = item_id.and_then(|iid| {
            crate::cell::abilities::ability_for_item(
                space_mgr,
                iid,
                crate::cell::spawner::EVENT_ITEM_RANGED,
            )
        });
        tracing::info!(
            target: "bandolier",
            event = "active_slot_change",
            entity_id,
            slot_id,
            prev_slot = prev_slot_for_log,
            item_id = ?item_id,
            resolved_ranged_ability = ?resolved_ranged_ability,
            "Active bandolier slot changed — auto-attack ability resolved"
        );
    }

    // Broadcast BSF_AUTO_CYCLING clear if the swap killed an active
    // auto-cycle. Self-routed (matches the wire rule for BSF transitions
    // driven by use_ability::clear_auto_cycle). Skipped when no
    // transition fired — `Option::is_some` was set above only on a
    // genuine bit drop.
    if let Some(new_state) = auto_cycle_clear_state {
        super::super::super::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            new_state.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // Phase 2: send messages now that the borrow is released. Clear the
    // dirty marker for the previously-active slot only after the persist
    // message is accepted by the channel.
    if let Some(player_id) = player_id {
        if let Some((p_slot, item_id, current_ammo, cur_ammo_type)) = prev_persist {
            match tx
                .send(CellToBaseMsg::BandolierAmmoUpdate {
                    player_id,
                    slot_id: p_slot,
                    expected_item_id: item_id,
                    current_ammo,
                    cur_ammo_type,
                })
                .await
            {
                Ok(()) => {
                    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                        entity.bandolier_ammo_dirty.remove(&p_slot);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        entity_id, player_id, slot_id = p_slot,
                        expected_item_id = item_id,
                        error = %e,
                        "BandolierAmmoUpdate (slot swap) send failed; dirty marker preserved for retry"
                    );
                }
            }
        }
        if let Err(e) = tx
            .send(CellToBaseMsg::ActiveSlotUpdate {
                entity_id,
                player_id,
                slot_id,
            })
            .await
        {
            tracing::warn!(
                entity_id, player_id, slot_id,
                error = %e,
                "ActiveSlotUpdate send failed"
            );
        }
    }

    // Tell the *client* about the new active slot so the bandolier UI
    // updates its highlighted slot indicator. Without this packet, the
    // client UI keeps thinking the previously-selected slot is active —
    // and the LUA `BandolierMod.ActivateBandolierSlotN` keybinds gate on
    // `getActiveSlotForContainer(...) ~= N`, so further keypresses for
    // the slot the UI thinks is selected are silently dropped at the
    // client. That presented as "switching back to the first bandolier
    // slot does not give me my weapon back" during play-testing.
    //
    // Wire format matches `Bag.py:369` `onActiveSlotUpdate(bagId, activeSlotId + 1)`
    // and the existing sites in `grant.rs` / `vendor/helpers.rs` that
    // already use this method index — bag_id then 1-indexed wire slot.
    let mut active_slot_args = Vec::with_capacity(8);
    active_slot_args.extend_from_slice(&bag_id.to_le_bytes());
    active_slot_args.extend_from_slice(&(slot_id + 1).to_le_bytes());
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE,
        active_slot_args,
        tx,
        space_mgr,
    )
    .await;

    // Stage D: refresh the client's ammo-type indicator for the
    // newly-active slot. If the new slot is empty (no item),
    // send 0 to mirror legacy "no weapon equipped" behavior
    // (SGWPlayer.py:522 — `activeItem.ammoType if activeItem else 0`).
    let property_value = new_ammo_type.unwrap_or(0);
    let property_args = build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, property_value);
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
        property_args,
        tx,
        space_mgr,
    )
    .await;

    // Display the newly-active weapon + arm the OOC re-holster timer.
    // Same mechanism as `UpdateBandolierItem` (initial equip): if the
    // player is OOC, draw the weapon (so the wire `ComponentList`
    // includes it) and stamp `combat_exit_at` so the holster timer
    // re-holsters it after `OOC_HOLSTER_DELAY`. The base side's
    // `active_slot_update` calls `refresh_player_appearance` after
    // persisting the slot — by sending `RefreshAppearance` from here
    // FIRST we update the base's cached `weapon_holstered`, so the
    // base's subsequent refresh reads the correct (drawn) state.
    //
    // Three outcomes:
    //
    //  1. New slot has a weapon + OOC: draw it, fire `Item_Equip`,
    //     arm OOC timer.
    //  2. New slot has NO weapon (empty slot): refresh appearance so
    //     the wire `ComponentList` reflects the empty slot (the old
    //     weapon visual goes away), but do NOT fire `Item_Equip` —
    //     there's nothing to equip. Don't arm the OOC timer either;
    //     there's no weapon to re-holster. The base's subsequent
    //     `refresh_player_appearance` from `active_slot_update` will
    //     still re-emit the empty `ComponentList` from the persisted
    //     bandolier_slot, so the visual change lands either way; we
    //     skip the cell-side `request_appearance_refresh` to avoid
    //     a redundant packet.
    //  3. In-combat slot swap: weapon's already drawn, no state
    //     changes needed.
    let new_slot_has_weapon = new_ammo_type.is_some();
    let draw_intent = match space_mgr.get_entity_mut(entity_id) {
        Some(e) if e.threatened_mobs.is_empty() && new_slot_has_weapon => {
            e.set_weapon_holstered(false);
            e.combat_exit_at = Some(std::time::Instant::now());
            e.holster_animation_complete_at = None;
            true
        }
        _ => false,
    };
    if draw_intent {
        crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
        crate::cell::cell_methods::player::world::fire_item_sequence(
            entity_id,
            crate::cell::spawner::EVENT_ITEM_EQUIP,
            tx,
            space_mgr,
        )
        .await;
    }
    // Honour the `reloadOnActivate` client option — F1-F4 slot swap into
    // a partial-clip weapon auto-tops it up. Gated inside the helper on
    // the option being enabled (default off per SystemOptions.xml). Also
    // fires for in-combat swaps where `draw_intent` is false: the weapon
    // is already drawn, but the player still wanted the active slot to
    // be the one with a full clip.
    crate::cell::cell_methods::player::world::maybe_trigger_reload_on_activate(
        entity_id, tx, space_mgr,
    )
    .await;
}

#[tracing::instrument(
    name = "bandolier.request_ammo_change",
    level = "info",
    skip_all,
    fields(entity_id, args_len = args.len()),
)]
/// Handle `requestAmmoChange(item_id, ammo_type)` — the player's per-slot
/// ammo-type swap.
///
/// The bandolier's per-slot `cur_ammo_type` field carries an **ammo subtype
/// id**, which drives damage-type variation (different ammo types do
/// different damage profiles against different enemy types). Players swap
/// to optimize their loadout against the current encounter; ammo is NOT
/// consumed from inventory — the clip-based ammo model (refill on reload)
/// is decoupled from ammo type.
///
/// Implementation contract (closes Phase 6 of):
/// - Validates `ammo_type` is in the weapon's `allowed_ammo_types`
///   whitelist (`crates/entity/src/inventory.rs:81`) so a forged
///   request can't persist an arbitrary subtype the client UI can't
///   render.
/// - Rejects ambiguous matches when the player has the same item_id in
///   multiple bandolier slots — the wire request doesn't carry a slot
///   id, so guessing would mis-attribute the swap.
/// - Persists via `BandolierAmmoUpdate` to `sgw_inventory.cur_ammo_type`
///   so the swap survives relog.
/// - If the swapped slot is the **active** weapon, broadcasts
///   `onEntityProperty(GENERICPROPERTY_AMMO_TYPE_ID, ammo_type)` so the
///   client's ammo-type indicator UI refreshes immediately.
pub(super) async fn handle_request_ammo_change(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 8 {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "requestAmmoChange: truncated args"
        );
        return;
    }
    let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    tracing::debug!(entity_id, item_id, ammo_type, "requestAmmoChange");

    // Loose validation — the legacy is literally `pass`. Reject
    // anything non-positive: 0 is the "no choice" sentinel, and
    // the DB column has CHECK (cur_ammo_type >= 0), so a negative
    // value would mutate cell + client state then fail the DB
    // write later, leaving them ahead of persistence. The proper
    // whitelist lives on the item template's `ammo_types`
    // (crates/entity/src/inventory.rs:81).
    // TODO: validate against item.ammo_types whitelist
    //       (see crates/entity/src/inventory.rs:81 — `Item.ammo_types`).
    if ammo_type <= 0 {
        tracing::warn!(
            entity_id,
            item_id,
            ammo_type,
            "requestAmmoChange: rejecting non-positive ammo_type"
        );
        return;
    }

    // Snapshot the weapon's allowed-ammo whitelist BEFORE taking
    // the mutable entity borrow — both read from `space_mgr` and
    // can't coexist. `None` means the cache had no entry (custom
    // item the loader skipped), in which case we accept any
    // positive ammo_type to match the legacy `pass` semantics
    // for unknown weapons.
    let weapon_def = space_mgr.item_defs.get(&item_id).cloned();

    // Phase 1: locate the slot, mutate, capture the BandolierAmmoUpdate
    // payload + active-slot flag, drop the mutable borrow.
    let (player_id, persist, is_active) = {
        let entity = match space_mgr.get_entity_mut(entity_id) {
            Some(e) => e,
            None => return,
        };
        // Match all slots holding this item_id. The wire request
        // doesn't carry a slot/instance id, so duplicate weapons
        // are ambiguous — reject rather than guess. A future
        // protocol revision should add slot id to the message.
        let matches: Vec<i32> = entity
            .bandolier_items
            .iter()
            .filter(|(_, item)| item.item_id == item_id)
            .map(|(s, _)| *s)
            .collect();
        let slot = match matches.as_slice() {
            [s] => *s,
            [] => {
                tracing::warn!(
                    entity_id,
                    item_id,
                    "requestAmmoChange: item not in bandolier"
                );
                return;
            }
            ambiguous => {
                tracing::warn!(
                    entity_id,
                    item_id,
                    slot_count = ambiguous.len(),
                    "requestAmmoChange: ambiguous — multiple slots hold this item_id, rejecting"
                );
                return;
            }
        };
        // Validate ammo_type against the weapon's allowed
        // subtypes whitelist (snapshotted into `weapon_def`
        // above). Without this an attacker could persist an
        // arbitrary ammo subtype that the client UI can't
        // render. Falls through when the cache entry is
        // missing — see comment at the snapshot site.
        if let Some(def) = weapon_def.as_ref() {
            let allowed =
                ammo_type == def.default_ammo_type || def.allowed_ammo_types.contains(&ammo_type);
            if !allowed {
                tracing::warn!(
                    entity_id, item_id, ammo_type,
                    allowed = ?def.allowed_ammo_types,
                    default_ammo = def.default_ammo_type,
                    "requestAmmoChange: ammo_type not in weapon's allowed list, rejecting"
                );
                return;
            }
        }
        // Mutate the slot and mark it dirty. The dirty marker stays set
        // until phase 2 confirms BandolierAmmoUpdate was accepted by the
        // channel; if the send fails the next flush picks the change up.
        let item = entity.bandolier_items.get_mut(&slot).unwrap();
        item.cur_ammo_type = ammo_type;
        let item_id_for_persist = item.item_id;
        let current_ammo = item.current_ammo;
        entity.bandolier_ammo_dirty.insert(slot);
        let is_active = slot == entity.active_bandolier_slot;
        let player_id = entity.player_id;
        (
            player_id,
            (slot, item_id_for_persist, current_ammo, ammo_type),
            is_active,
        )
    };

    // Phase 2: persist + (if active) refresh the client's ammo-type indicator.
    //
    // `persistence` tracks the outcome so the observability log below
    // distinguishes "enqueued for persist" from "send failed" from
    // "no player_id, skipped." Without this distinction the operator
    // sees "ammo type changed" and assumes the slot stuck.
    let persistence: &'static str = if let Some(player_id) = player_id {
        let (slot_id, expected_item_id, current_ammo, cur_ammo_type) = persist;
        match tx
            .send(CellToBaseMsg::BandolierAmmoUpdate {
                player_id,
                slot_id,
                expected_item_id,
                current_ammo,
                cur_ammo_type,
            })
            .await
        {
            Ok(()) => {
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.bandolier_ammo_dirty.remove(&slot_id);
                }
                "enqueued"
            }
            Err(e) => {
                tracing::warn!(
                    entity_id, player_id, slot_id, expected_item_id, cur_ammo_type,
                    error = %e,
                    "BandolierAmmoUpdate (ammo change) send failed; dirty marker preserved for retry"
                );
                "send_failed"
            }
        }
    } else {
        tracing::warn!(
            entity_id,
            "requestAmmoChange: entity has no player_id — skipping persist"
        );
        "skipped_no_player_id"
    };

    if is_active {
        let property_args = build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, ammo_type);
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
            property_args,
            tx,
            space_mgr,
        )
        .await;
    }

    // Structured observability event for the swap. Stable
    // `target: "bandolier"` so SigNoz can `groupBy=event` across the
    // bandolier event family (matches the active_slot_change emitter).
    // Operators can pivot on item_id + ammo_type to see which subtypes
    // players are picking per weapon — useful data for balance + content
    // gap detection.
    //
    // `persistence` field distinguishes the path: `enqueued` when the
    // BandolierAmmoUpdate send to base succeeded, `skipped_no_player_id`
    // when the entity has no player_id (NPC/non-persisting), or
    // `send_failed` when the channel send returned Err. Without this,
    // operators see "ammo type changed" and assume it stuck, then can't
    // explain why post-relog the slot reverts.
    tracing::info!(
        target: "bandolier",
        event = "ammo_type_change",
        entity_id,
        item_id,
        ammo_type,
        is_active_slot = is_active,
        persistence,
        "Bandolier ammo type changed"
    );
}
