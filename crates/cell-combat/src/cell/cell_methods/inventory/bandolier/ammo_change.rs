//! Per-slot ammo-type change (`requestAmmoChange`). Validates the chosen
//! subtype against the weapon's whitelist, persists via
//! `BandolierAmmoUpdate`, and refreshes the client's ammo-type indicator
//! when the changed slot is the active weapon.
//!
//! Every refusal logs `event="ammo_type_change_rejected"` on target `ammo`
//! with a `reason=` from `cimmeria_entity::ammo_telemetry::reasons`, and
//! sends the player one `CHAN_FEEDBACK` line, the route known to render
//! (the shipped client has no Lua consumer for `onErrorCode`, AT-E1). A
//! refusal sends nothing else: no `BandolierAmmoUpdate`, no
//! `onEntityProperty`.

use std::collections::HashMap;

use cimmeria_entity::ammo_telemetry::reasons;
use cimmeria_entity::cell_entity::{BandolierItem, PlayerIdentity};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WeaponDef;

use super::super::constants::{build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID};

/// Why a `requestAmmoChange` was refused. `reason()` is the log field,
/// `text()` the line the player is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AmmoChangeRefusal {
    /// `ammo_type <= 0`: 0 is the "no choice" sentinel and the DB column
    /// has `CHECK (cur_ammo_type >= 0)`.
    NonPositiveAmmoType,
    /// No bandolier slot holds the requested weapon instance.
    ItemNotInBandolier,
    /// More than one bandolier slot claims the instance id. The id is the
    /// `sgw_inventory` primary key, so this is corrupt cell state, never a
    /// legitimate request.
    AmbiguousSlot,
    /// The weapon's design id has no `WeaponDef`. `load_item_defs` caches
    /// every weapon with `clip_size > 0`, so a miss is either not an
    /// ammo-bearing weapon or a broken cache load. Fail closed (#448).
    WeaponDefCacheMiss { design_id: i32 },
    /// The weapon's `ammo_types` does not list the type.
    NotInAllowedTypes { design_id: i32 },
}

impl AmmoChangeRefusal {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::NonPositiveAmmoType => reasons::NON_POSITIVE_AMMO_TYPE,
            Self::ItemNotInBandolier => reasons::ITEM_NOT_IN_BANDOLIER,
            Self::AmbiguousSlot => reasons::AMBIGUOUS_SLOT,
            Self::WeaponDefCacheMiss { .. } => reasons::WEAPON_DEF_CACHE_MISS,
            Self::NotInAllowedTypes { .. } => reasons::NOT_IN_ALLOWED_TYPES,
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::NonPositiveAmmoType | Self::NotInAllowedTypes { .. } => {
                "That weapon cannot use that ammo type."
            }
            Self::ItemNotInBandolier => "That weapon is not in your bandolier.",
            Self::AmbiguousSlot => "That weapon could not be found in one bandolier slot.",
            Self::WeaponDefCacheMiss { .. } => "That weapon cannot change ammo type.",
        }
    }

    fn design_id(self) -> Option<i32> {
        match self {
            Self::WeaponDefCacheMiss { design_id } | Self::NotInAllowedTypes { design_id } => {
                Some(design_id)
            }
            _ => None,
        }
    }
}

/// The slot a valid request changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmmoChangeTarget {
    pub(crate) slot: i32,
    /// The weapon's design id (`resources.items.item_id`).
    pub(crate) design_id: i32,
    /// `cur_ammo_type` before the change. AM-02's switch-return reads it.
    pub(crate) prev_ammo_type: i32,
}

/// Decide a `requestAmmoChange(instance_id, ammo_type)`.
///
/// `instance_id` is the weapon's **instance** id (`InvItem.id`, the
/// `sgw_inventory.item_id` PK): the client sends `item+0x0C` from
/// `FUN_00e1ee10`, which the item ctor `FUN_00d21750` fills from `id`, not
/// `dbid` (#534; `inventory-wire-formats.md` § requestAmmoChange). The
/// whitelist is looked up by the matched slot's **design** id, never by the
/// wire value, so an instance id that happens to equal some design id can't
/// validate against the wrong weapon.
///
/// The weapon's default ammo type is always allowed, so a player can switch
/// back from a special. `WeaponDef.allowed_ammo_types` is `resources.items.
/// ammo_types` converted to ordinals at load time.
pub(crate) fn validate_ammo_change(
    bandolier: &HashMap<i32, BandolierItem>,
    item_defs: &HashMap<i32, WeaponDef>,
    instance_id: i32,
    ammo_type: i32,
) -> Result<AmmoChangeTarget, AmmoChangeRefusal> {
    if ammo_type <= 0 {
        return Err(AmmoChangeRefusal::NonPositiveAmmoType);
    }
    // Resolve the slot before the cache lookup: a forged id then lands on
    // `item_not_in_bandolier`, and `weapon_def_cache_miss` stays an ops
    // signal about weapons the player really holds.
    let mut matches = bandolier
        .iter()
        .filter(|(_, item)| item.instance_id == instance_id);
    let (slot, item) = match (matches.next(), matches.next()) {
        (Some((slot, item)), None) => (*slot, item),
        (None, _) => return Err(AmmoChangeRefusal::ItemNotInBandolier),
        (Some(_), Some(_)) => return Err(AmmoChangeRefusal::AmbiguousSlot),
    };
    let design_id = item.item_id;
    let Some(def) = item_defs.get(&design_id) else {
        return Err(AmmoChangeRefusal::WeaponDefCacheMiss { design_id });
    };
    if ammo_type != def.default_ammo_type && !def.allowed_ammo_types.contains(&ammo_type) {
        return Err(AmmoChangeRefusal::NotInAllowedTypes { design_id });
    }
    Ok(AmmoChangeTarget {
        slot,
        design_id,
        prev_ammo_type: item.cur_ammo_type,
    })
}

#[tracing::instrument(
    name = "ammo.ammo_change",
    level = "info",
    skip_all,
    fields(entity_id, args_len = args.len()),
)]
/// Handle `requestAmmoChange(ItemId, AmmoType)`, the player's per-slot
/// ammo-type swap. `ItemId` is the weapon's instance id (see
/// [`validate_ammo_change`]).
///
/// The bandolier's per-slot `cur_ammo_type` carries an `EAmmoType` ordinal.
/// A valid swap persists via `BandolierAmmoUpdate` to
/// `sgw_inventory.cur_ammo_type` so it survives relog and, when the slot is
/// the **active** weapon, sends `onEntityProperty(GENERICPROPERTY_AMMO_TYPE_ID,
/// ammo_type)` so the client's ammo-type indicator refreshes.
pub async fn handle_request_ammo_change(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 8 {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            args_len = args.len(),
            reason = "truncated_args",
            "requestAmmoChange: truncated args"
        );
        return;
    }
    let instance_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    tracing::debug!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        instance_id, // nt:id-only weapon inventory instance, no name of its own
        ammo_type,
        "requestAmmoChange"
    );

    let (id, verdict) = {
        let Some(entity) = space_mgr.get_entity(entity_id) else {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                instance_id, // nt:id-only weapon inventory instance, no name of its own
                ammo_type,
                reason = "entity_not_found",
                "requestAmmoChange: no cell entity for the sender"
            );
            return;
        };
        (
            entity.identity(),
            validate_ammo_change(
                &entity.bandolier_items,
                &space_mgr.item_defs,
                instance_id,
                ammo_type,
            ),
        )
    };
    // The telemetry contract's `item_id` is the ammo item's design id
    // (`ammo_item_types`); a free type has none.
    let ammo_item_id = space_mgr.ammo_catalog.item_id_for(ammo_type);

    let target = match verdict {
        Ok(target) => target,
        Err(refusal) => {
            reject(
                entity_id,
                id,
                instance_id,
                ammo_type,
                ammo_item_id,
                refusal,
                tx,
            )
            .await;
            return;
        }
    };

    // AM-02 hook (switch-return, D-AM05): after validation, so a refused
    // type never returns rounds, and before the slot mutation. `Deferred`
    // means the base returns the previous type's unfired rounds, persists
    // the new type, and the cell finishes the switch when the base answers.
    let slot = target.slot;
    if super::switch_return::begin_switch_return(entity_id, slot, ammo_type, tx, space_mgr).await
        == super::switch_return::SwitchReturn::Deferred
    {
        return;
    }

    // Phase 1: mutate the slot, capture the BandolierAmmoUpdate payload and
    // the active-slot flag, drop the mutable borrow.
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    let Some(item) = entity.bandolier_items.get_mut(&target.slot) else {
        return;
    };
    item.cur_ammo_type = ammo_type;
    // `instance_id` (sgw_inventory.item_id PK) is the persist TOCTOU guard.
    let expected_instance_id = item.instance_id;
    let current_ammo = item.current_ammo;
    // The dirty marker stays set until the send below is accepted; if it
    // fails the next flush picks the change up.
    entity.bandolier_ammo_dirty.insert(target.slot);
    let is_active = target.slot == entity.active_bandolier_slot;

    // Phase 2: persist + (if active) refresh the client's indicator.
    // `persistence` separates "enqueued" from "send failed" from "no
    // player_id", so an operator can explain a swap that reverts on relog.
    let slot_id = target.slot;
    let persistence: &'static str = if let Some(player_id) = id.player_id {
        match tx
            .send(CellToBaseMsg::BandolierAmmoUpdate {
                player_id,
                slot_id,
                expected_instance_id,
                current_ammo,
                cur_ammo_type: ammo_type,
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
                    entity_id,
                    entity_name = id.player_name,
                    player_id,
                    player_name = id.player_name,
                    slot_id, // nt:id-only bandolier slot index, not a named object
                    expected_instance_id, // nt:id-only weapon inventory instance, no name of its own
                    item_id = target.design_id,
                    item_name = cimmeria_cell_world::cell::effects::content_names::item_name(target.design_id),
                    cur_ammo_type = ammo_type,
                    error = %e,
                    "BandolierAmmoUpdate (ammo change) send failed; dirty marker preserved for retry"
                );
                "send_failed"
            }
        }
    } else {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            reason = "no_player_id",
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

    // Stable `target: "bandolier"` so SigNoz can `groupBy=event` across the
    // bandolier family (matches the active_slot_change emitter).
    tracing::info!(
        target: "bandolier",
        event = "ammo_type_change",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        item_id = target.design_id,
        item_name = cimmeria_cell_world::cell::effects::content_names::item_name(target.design_id),
        instance_id, // nt:id-only weapon inventory instance, no name of its own
        ammo_type,
        prev_ammo_type = target.prev_ammo_type,
        is_active_slot = is_active,
        persistence,
        "Bandolier ammo type changed"
    );
}

/// Log the refusal and send the player its feedback line. Nothing else is
/// sent.
async fn reject(
    entity_id: u32,
    id: PlayerIdentity,
    instance_id: i32,
    ammo_type: i32,
    ammo_item_id: Option<i32>,
    refusal: AmmoChangeRefusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    tracing::warn!(
        target: "ammo",
        event = "ammo_type_change_rejected",
        reason = refusal.reason(),
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        item_id = ammo_item_id,
        item_name = cimmeria_cell_world::cell::effects::content_names::item_name(ammo_item_id),
        ammo_type,
        weapon_instance_id = instance_id, // nt:id-only weapon inventory instance, no name of its own
        weapon_item_id = refusal.design_id(),
        weapon_item_name = cimmeria_cell_world::cell::effects::content_names::item_name(refusal.design_id()),
        "requestAmmoChange refused; the slot is unchanged"
    );
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, refusal.text());
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "ammo",
            event = "ammo_feedback_send_failed",
            reason = "base_channel_closed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            refusal = refusal.reason(),
            "requestAmmoChange: refusal feedback line could not be queued"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn slot(instance_id: i32, design_id: i32, cur_ammo_type: i32) -> BandolierItem {
        BandolierItem {
            instance_id,
            item_id: design_id,
            clip_size: 30,
            default_ammo_type: 1,
            current_ammo: 20,
            cur_ammo_type,
        }
    }

    fn defs() -> HashMap<i32, WeaponDef> {
        HashMap::from([(
            42,
            WeaponDef {
                clip_size: 30,
                default_ammo_type: 1,
                allowed_ammo_types: vec![1, 3, 5],
                holster_animation_duration: Duration::from_millis(600),
            },
        )])
    }

    #[test]
    fn listed_and_default_types_are_accepted() {
        let bag = HashMap::from([(0, slot(4242, 42, 3))]);
        for ammo_type in [1, 3, 5] {
            assert_eq!(
                validate_ammo_change(&bag, &defs(), 4242, ammo_type),
                Ok(AmmoChangeTarget {
                    slot: 0,
                    design_id: 42,
                    prev_ammo_type: 3
                })
            );
        }
    }

    #[test]
    fn every_refusal_maps_to_its_reason() {
        let bag = HashMap::from([(0, slot(4242, 42, 1)), (1, slot(5000, 77, 1))]);
        let cases = [
            (4242, 0, AmmoChangeRefusal::NonPositiveAmmoType),
            (4242, -3, AmmoChangeRefusal::NonPositiveAmmoType),
            (9999, 3, AmmoChangeRefusal::ItemNotInBandolier),
            // The design id is not the key: 42 names no instance.
            (42, 3, AmmoChangeRefusal::ItemNotInBandolier),
            (
                5000,
                3,
                AmmoChangeRefusal::WeaponDefCacheMiss { design_id: 77 },
            ),
            (
                4242,
                2,
                AmmoChangeRefusal::NotInAllowedTypes { design_id: 42 },
            ),
        ];
        for (instance_id, ammo_type, want) in cases {
            assert_eq!(
                validate_ammo_change(&bag, &defs(), instance_id, ammo_type),
                Err(want),
                "instance {instance_id} ammo {ammo_type}"
            );
        }
        let dup = HashMap::from([(0, slot(4242, 42, 1)), (1, slot(4242, 42, 1))]);
        assert_eq!(
            validate_ammo_change(&dup, &defs(), 4242, 3),
            Err(AmmoChangeRefusal::AmbiguousSlot)
        );
        assert_eq!(
            AmmoChangeRefusal::WeaponDefCacheMiss { design_id: 1 }.reason(),
            "weapon_def_cache_miss"
        );
        assert_eq!(
            AmmoChangeRefusal::NotInAllowedTypes { design_id: 1 }.reason(),
            "not_in_allowed_types"
        );
    }
}
