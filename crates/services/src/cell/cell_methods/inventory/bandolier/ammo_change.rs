//! Per-slot ammo-type change (`requestAmmoChange`). Validates the chosen
//! subtype against the weapon's whitelist, persists via
//! `BandolierAmmoUpdate`, and refreshes the client's ammo-type indicator
//! when the changed slot is the active weapon.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::super::constants::{build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID};

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
        // `instance_id` (sgw_inventory.item_id PK) is the persist TOCTOU guard;
        // `item_id` (design id) is carried only for log context.
        let instance_id_for_persist = item.instance_id;
        let item_id_for_log = item.item_id;
        let current_ammo = item.current_ammo;
        entity.bandolier_ammo_dirty.insert(slot);
        let is_active = slot == entity.active_bandolier_slot;
        let player_id = entity.player_id;
        (
            player_id,
            (
                slot,
                instance_id_for_persist,
                item_id_for_log,
                current_ammo,
                ammo_type,
            ),
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
        let (slot_id, expected_instance_id, item_id_for_log, current_ammo, cur_ammo_type) = persist;
        match tx
            .send(CellToBaseMsg::BandolierAmmoUpdate {
                player_id,
                slot_id,
                expected_instance_id,
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
                    entity_id, player_id, slot_id, expected_instance_id,
                    item_id = item_id_for_log, cur_ammo_type,
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
