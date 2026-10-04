//! The answer to a press of an ability that cannot do anything
//! (ability-mechanics AB-12, D-AB10, D-AB11).
//!
//! Most seeded abilities have no mechanic the server can resolve yet: no
//! damage NVP and no effect script (audit B-02). Before this gate a player's
//! press of one charged the cooldown, sent the timer, maybe played an
//! animation, and did nothing else: a silent press (B-60). Now the launch
//! refuses it before the cooldown with `onErrorCode` and a `CHAN_FEEDBACK`
//! line, and logs one `abilities` row. Out-of-scope families (stealth,
//! self-revive, Asgard energy, turrets; D-AB11) get the same answer, because
//! they have no mechanic either.
//!
//! [`ability_has_mechanics`] reads the data, so an ability lights up on its
//! own when a generator packet (AB-03 damage, AB-04 stats) gives one of its
//! effects a number or a script. Only player casts are gated: an NPC's
//! animation-only attack and a pet's cast keep their own paths.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{ability_effects_have_mechanics, AbilityDef};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::cell_methods::player::world::reload::ABILITY_RELOAD_WEAPON;
use crate::cell::cover::COVER_STANCE_ABILITY;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The feedback line a refused press shows.
pub(crate) const NO_EFFECT_TEXT: &str = "That ability has no effect yet.";

/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility` (167,
/// `entities/defs/enumerations.xml`). The client enum has no "no effect"
/// value; 167 is the code the pet-order gate already sends for the same
/// refusal (`ability_not_implemented`, pets PT-11). No client Lua renders
/// `onErrorCode` (AT-E1), so the chat line is what the player reads.
pub(crate) const NO_MECHANICS_ERROR_CODE: u16 = 167;

/// `ERRORCODE_SYSTEM_Ability`, the only `EErrorCodeSystem` value.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// The `reason` the refusal row carries.
pub(crate) const REASON_NO_MECHANICS: &str = "no_mechanics";

/// Whether a cast of `def` does something the server resolves:
///
/// - an effect deals damage from its NVPs or runs a script the installed
///   registry answers ([`ability_effects_have_mechanics`]; a blank or
///   unregistered `script_name` does not count; heal and stat NVPs count
///   through the script that reads them, and the cover stance 1451 runs one);
/// - it is Reload (596): the reload pipeline runs it, not an effect script
///   (its effect 658 names the unregistered `Reload`);
/// - it summons a pet (`pet_summons`) or acts on the owner's pet (an
///   owner-pet script, PT-08);
/// - it places a deployable (`deployables`);
/// - it is an ammo toggle (`ammo_modifiers.toggle_ability_id`): never
///   launched as a modifier (D-AM07), and its press stays exactly as before;
/// - it is a weapon shot (`required_ammo > 0`): it spends a round, and the
///   loaded ammo's modifier and on-hit effect ride on it (D-AM07).
///
/// An event set alone does not count: an animation is not a mechanic.
pub(crate) fn ability_has_mechanics(space_mgr: &SpaceManager, def: &AbilityDef) -> bool {
    let id = def.ability_id;
    let scripts = space_mgr.effect_scripts();
    ability_effects_have_mechanics(def, &space_mgr.effect_defs, |s| scripts.contains(s))
        || def.required_ammo > 0
        || id == COVER_STANCE_ABILITY
        || id == ABILITY_RELOAD_WEAPON
        || space_mgr.pet_summons.pet_summon_for(id).is_some()
        || super::owner_pet::is_owner_pet_ability(space_mgr, id)
        || space_mgr.deployable_specs.deployable_for(id).is_some()
        || space_mgr.ammo_catalog.is_toggle_ability(id)
}

/// Refuse a player's press of an ability with no mechanics: `onErrorCode`
/// plus the feedback line, one DEBUG `abilities` row, and `true`. The
/// caller returns before the cooldown, so nothing is charged and no timer is
/// sent. `false` (nothing sent) for an NPC, an ability the server has no
/// def for (the caller's own unknown-id path handles that), an ability the
/// active weapon grants (the basic attack never goes quiet), and every
/// ability with a mechanic.
pub(super) async fn refuse_without_mechanics(
    entity_id: u32,
    ability_id: i32,
    def: Option<&AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(def) = def else {
        return false;
    };
    if !space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player)
        || ability_has_mechanics(space_mgr, def)
        || super::super::resolve::is_ability_granted_by_active_weapon(
            space_mgr, entity_id, ability_id,
        )
    {
        return false;
    }
    let id = space_mgr.player_identity(entity_id);
    // DEBUG: any client can press any bar button at will, and the
    // `abilities=debug` OTEL_FILTER row exports it.
    tracing::debug!(
        target: "abilities",
        event = "no_mechanics_refused",
        decision_outcome = "refused",
        reason = REASON_NO_MECHANICS,
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        ability_name = %def.name,
        effect_count = def.effect_ids.len(),
        animates = def.event_set_id.is_some(),
        "useAbility: the ability has no mechanic yet; refused with feedback, no cooldown charged"
    );
    send_no_effect_feedback(entity_id, ability_id, tx).await;
    true
}

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, 167)` and the
/// `CHAN_FEEDBACK` line, in that order.
async fn send_no_effect_feedback(
    entity_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut err = Vec::with_capacity(7);
    err.push(ERRORCODE_SYSTEM_ABILITY); // SystemID
    err.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    err.extend_from_slice(&NO_MECHANICS_ERROR_CODE.to_le_bytes());
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_EFFECT_TEXT);
    for (method_index, args) in [
        (crate::mercury::method_idx::ON_ERROR_CODE, err),
        (crate::mercury::method_idx::ON_PLAYER_COMMUNICATION, chat),
    ] {
        if tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await
            .is_err()
        {
            tracing::warn!(
                target: "abilities",
                event = "no_mechanics_feedback_send_failed",
                entity_id,
                ability_id,
                method_index,
                "useAbility: the no-effect feedback could not be queued (base channel closed)"
            );
        }
    }
}
