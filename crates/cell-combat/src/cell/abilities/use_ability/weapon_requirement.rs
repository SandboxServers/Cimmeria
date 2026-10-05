//! The player weapon-moniker requirement (Class Start v6 CS-07, OD-CS11).
//!
//! An ability whose `item_monikers` is non-empty fires for a player only if
//! the active bandolier item carries at least one of those monikers. No
//! requirement means no check, an empty active slot fails any requirement,
//! and NPC casts (pets included) are never checked. Python did the same in
//! `AbilityInstance.canUse` (`deprecated/python/cell/AbilityManager.py:543`,
//! `SGWPlayer.hasItemMoniker`) and answered `onErrorCode(
//! ERRORCODE_SYSTEM_Ability, abilityId, CONDITION_FEEDBACK_WrongWeaponType)`
//! (`SGWPlayer.py:1231`).
//!
//! The refusal comes before the cooldown, the ammo check and the cost, so
//! nothing is charged. It sends that `onErrorCode` plus a `CHAN_FEEDBACK`
//! line (no client Lua renders `onErrorCode`, AT-E1, so the line is what
//! the player reads on the first press) and logs one `wrong_weapon_refused`
//! row naming the ability, the required monikers and the active item.
//!
//! This replaces the 592 → active-weapon redirect (#495): Pistol Shot now
//! needs a pistol like every other weapon ability needs its weapon, and a
//! weapon's own basic attack is the weapon-granted ability the bandolier
//! swap puts on the hotbar (`weapon_abilities.rs`).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::CellEntity;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `CONDITION_FEEDBACK_WrongWeaponType` (`entities/defs/enumerations.xml`,
/// python `Atrea.enums`): the code python's launch returned for this rule.
pub(crate) const WRONG_WEAPON_TYPE_ERROR_CODE: u16 = 63;

/// The feedback line a refused press shows.
pub(crate) const WRONG_WEAPON_TEXT: &str = "You need a different weapon to use that ability.";

/// The `reason` the refusal row carries.
pub(crate) const REASON_WRONG_WEAPON_TYPE: &str = "wrong_weapon_type";

/// A player's cast the rule refuses: the active item, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WrongWeapon {
    pub active_item_id: Option<i32>,
}

/// `Some` when `entity` is a player whose active bandolier item does not
/// meet `def`'s weapon requirement. `None` for an NPC, an unknown ability,
/// an ability with no requirement, and a weapon that carries a required
/// moniker.
pub(super) fn wrong_weapon(
    space_mgr: &SpaceManager,
    entity: &CellEntity,
    def: Option<&AbilityDef>,
) -> Option<WrongWeapon> {
    let def = def?;
    if !entity.is_player || !def.requires_weapon() {
        return None;
    }
    let active_item_id = entity
        .bandolier_items
        .get(&entity.active_bandolier_slot)
        .map(|b| b.item_id);
    let monikers = active_item_id.map(|id| {
        space_mgr
            .item_monikers
            .get(&id)
            .map_or(&[][..], Vec::as_slice)
    });
    if def.weapon_satisfies(monikers) {
        None
    } else {
        Some(WrongWeapon { active_item_id })
    }
}

/// Moniker ids as `NAME(id)` for the refusal row.
fn moniker_labels(ids: &[i64]) -> String {
    ids.iter()
        .map(|&id| match cimmeria_names::book().moniker(id) {
            Some(name) => format!("{name}({id})"),
            None => id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Refuse a press [`wrong_weapon`] flagged: one INFO `abilities` row, then
/// `onErrorCode(.., 63)` plus the feedback line. An auto-cycle loop armed on
/// this ability is stopped, or its tick would repeat the refusal every
/// cooldown (a weapon swap clears the loop too, `active_slot`). The caller
/// returns before the cooldown, so nothing is charged and no timer is sent.
pub(super) async fn refuse_wrong_weapon(
    entity_id: u32,
    def: &AbilityDef,
    wrong: WrongWeapon,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let active_item_monikers = wrong
        .active_item_id
        .and_then(|item| space_mgr.item_monikers.get(&item))
        .map(|m| moniker_labels(m))
        .unwrap_or_default();
    // INFO, not DEBUG: the refusal is the visible symptom of a weapon/data
    // mismatch (the CS-07 audit's data-correction rows), and each press
    // costs the player a click, so the volume stays human-scale.
    tracing::info!(
        target: "abilities",
        event = "wrong_weapon_refused",
        decision_outcome = "refused",
        reason = REASON_WRONG_WEAPON_TYPE,
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        ability_id = def.ability_id,
        ability_name = %def.name,
        required_monikers = %moniker_labels(&def.item_monikers),
        active_item_id = wrong.active_item_id,
        active_item_name =
            cimmeria_cell_world::cell::effects::content_names::item_name(wrong.active_item_id),
        active_item_monikers = %active_item_monikers,
        "useAbility: the active weapon carries none of the ability's required monikers; \
         refused with WrongWeaponType, no cooldown charged"
    );
    super::no_mechanics::send_ability_refusal(
        entity_id,
        id,
        def.ability_id,
        WRONG_WEAPON_TYPE_ERROR_CODE,
        WRONG_WEAPON_TEXT,
        tx,
    )
    .await;
    let loop_on_this_ability = space_mgr.get_entity(entity_id).is_some_and(|e| {
        e.abilities.auto_cycle && e.abilities.auto_cycle_ability_id == Some(def.ability_id)
    });
    if loop_on_this_ability {
        if let Some(new_state) = crate::cell::combat::clear_auto_cycle(space_mgr, entity_id) {
            super::super::auto_cycle_state::send_auto_cycle_state(
                entity_id, new_state, tx, space_mgr,
            )
            .await;
        }
    }
}
