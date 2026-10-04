//! The answer to a shield press with nowhere to put the shield
//! (ability-mechanics AB-10).
//!
//! An absorb shield fills its `absorb*` stats, which cap at their max. When
//! every pool a shield would fill is already full (another caster's shield,
//! or the same shield refreshed past what it can hold), the ledger refuses
//! the entry: an icon that absorbs nothing would lie to the player. So a
//! player's launch of an ability whose only mechanic is such a shield is
//! refused here, before the cooldown, with `onErrorCode` and a feedback line,
//! like a press with no mechanic (`no_mechanics.rs`).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{effect_is_implemented, shield_pools, AbilityDef, TCM_SINGLE};

use super::beneficial::{resolve_cast_target, CastTarget};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The feedback line a refused press shows.
pub(crate) const SHIELD_FULL_TEXT: &str = "Your shields are already at full strength.";

/// The `reason` the refusal row carries (the script's own skip reason).
pub(crate) const REASON_ABSORB_FULL: &str = "absorb_full";

const ABSORB_SHIELD: &str = "AbsorbShield";

/// Whether a player's cast of `def` would only put up shields that have no
/// room on the entity the cast resolves to. `false` for an NPC, for an
/// ability with any mechanic besides `AbsorbShield`, for a cast that does
/// not resolve to the caster or an ally, and for an area shield
/// (`TCM_AERadius`, `TCM_Group`, ...: Personal Shield's 4306): effect routing
/// fans those out to allies, so a full caster does not mean a wasted cast.
/// Each landing that has no room is skipped by the script itself
/// (`shield_skipped`, `absorb_full`).
pub(super) fn shield_has_no_room(
    entity_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(def) = def else {
        return false;
    };
    if !space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player) {
        return false;
    }
    let doing: Vec<_> = def
        .effect_ids
        .iter()
        .filter_map(|id| space_mgr.effect_defs.get(id))
        .filter(|e| effect_is_implemented(Some(e)))
        .collect();
    if doing.is_empty()
        || doing.iter().any(|e| {
            e.script_name.as_deref() != Some(ABSORB_SHIELD)
                || e.target_collection_method != TCM_SINGLE
        })
    {
        return false;
    }
    let target = match resolve_cast_target(space_mgr, entity_id, Some(def), wire_target) {
        CastTarget::Caster => entity_id,
        CastTarget::Ally(id) => id,
        _ => return false,
    };
    let Some(holder) = space_mgr.get_entity(target) else {
        return false;
    };
    doing.iter().all(|e| {
        shield_pools(e)
            .is_none_or(|pools| holder.absorb_room(&pools, (e.effect_id, entity_id)) == 0)
    })
}

/// Refuse a press [`shield_has_no_room`] flagged: one DEBUG `abilities`
/// row, then `onErrorCode` plus the feedback line. The caller returns before
/// the cooldown.
pub(super) async fn refuse_shield_full(
    entity_id: u32,
    def: &AbilityDef,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    // DEBUG: a player can press a shield at will.
    tracing::debug!(
        target: "abilities",
        event = "shield_full_refused",
        decision_outcome = "refused",
        reason = REASON_ABSORB_FULL,
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        ability_id = def.ability_id,
        ability_name = cimmeria_names::book().ability(def.ability_id),
        "useAbility: every pool the shield would fill is full; refused with feedback, no cooldown charged"
    );
    super::no_mechanics::send_ability_feedback(entity_id, id, def.ability_id, SHIELD_FULL_TEXT, tx)
        .await;
}
