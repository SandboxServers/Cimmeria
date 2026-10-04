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

use cimmeria_entity::abilities::{ability_effects_have_mechanics, AbilityDef, EF_ALWAYS_PERSIST};
use cimmeria_entity::cell_entity::PlayerIdentity;
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

/// The feedback line a refused press of a passive ability shows.
pub(crate) const PASSIVE_TEXT: &str = "That ability is passive: it works while you know it.";

/// The `reason` of a refused passive cast.
pub(crate) const REASON_PASSIVE: &str = "passive_ability";

/// Whether `def` is a passive (ability mechanics AB-08): `passive_yn`, or
/// every effect `EF_AlwaysPersist`. Its effects run through
/// `apply_passives` while it is known, never through a cast: since AB-08
/// they are `TimedStat` mechanics, and a forged cast of 1574 (effect 4782,
/// no beneficial bit, cooldown 0) at a hostile would run the attack path
/// (a QR roll, threat, in-combat) for free.
pub(crate) fn is_passive_ability(space_mgr: &SpaceManager, def: &AbilityDef) -> bool {
    def.passive
        || (!def.effect_ids.is_empty()
            && def.effect_ids.iter().all(|id| {
                space_mgr
                    .effect_defs
                    .get(id)
                    .is_some_and(|e| e.flags & EF_ALWAYS_PERSIST != 0)
            }))
}

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

/// Whether a press of `ability_id` must get the no-effect refusal: a player
/// caster, an ability the server has a def for, and either a passive
/// ([`is_passive_ability`]) or no mechanic and not granted by the active
/// weapon (the basic attack never goes quiet). `false` for an
/// NPC and for an ability with no def (the caller's own unknown-id path
/// handles that). Checked right after the known-ability and cooldown checks,
/// so a dead, missing or friendly target never swallows the answer.
pub(super) fn lacks_mechanics(
    entity_id: u32,
    ability_id: i32,
    def: Option<&AbilityDef>,
    space_mgr: &SpaceManager,
) -> bool {
    def.is_some_and(|def| {
        space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player)
            && (is_passive_ability(space_mgr, def)
                || (!ability_has_mechanics(space_mgr, def)
                    && !super::super::resolve::is_ability_granted_by_active_weapon(
                        space_mgr, entity_id, ability_id,
                    )))
    })
}

/// Refuse a press [`lacks_mechanics`] flagged: one DEBUG `abilities` row,
/// then `onErrorCode` plus the feedback line. The caller returns before the
/// cooldown, so nothing is charged and no timer is sent.
pub(super) async fn refuse_without_mechanics(
    entity_id: u32,
    def: &AbilityDef,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let ability_id = def.ability_id;
    let id = space_mgr.player_identity(entity_id);
    let passive = is_passive_ability(space_mgr, def);
    let (reason, text) = if passive {
        (REASON_PASSIVE, PASSIVE_TEXT)
    } else {
        (REASON_NO_MECHANICS, NO_EFFECT_TEXT)
    };
    // DEBUG: any client can press any bar button at will, and the
    // `abilities=debug` OTEL_FILTER row exports it.
    tracing::debug!(
        target: "abilities",
        event = "no_mechanics_refused",
        decision_outcome = "refused",
        reason,
        passive,
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        ability_name = %def.name,
        effect_count = def.effect_ids.len(),
        animates = def.event_set_id.is_some(),
        "useAbility: the ability has no mechanic yet, or is a passive; refused with feedback, no cooldown charged"
    );
    send_ability_feedback(entity_id, id, ability_id, text, tx).await;
}

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, 167)` and the
/// `CHAN_FEEDBACK` line `text`, in that order: the answer to a refused press
/// (no mechanic here, a full shield in `shield_full.rs`).
pub(super) async fn send_ability_feedback(
    entity_id: u32,
    who: PlayerIdentity,
    ability_id: i32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    send_ability_refusal(
        entity_id,
        who,
        ability_id,
        NO_MECHANICS_ERROR_CODE,
        text,
        tx,
    )
    .await;
}

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, code)` and the
/// `CHAN_FEEDBACK` line `text`, in that order, for a refusal with its own
/// code (a stunned caster in `incapacitated.rs`).
pub(super) async fn send_ability_refusal(
    entity_id: u32,
    who: PlayerIdentity,
    ability_id: i32,
    code: u16,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut err = Vec::with_capacity(7);
    err.push(ERRORCODE_SYSTEM_ABILITY); // SystemID
    err.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    err.extend_from_slice(&code.to_le_bytes());
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
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
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id,
                ability_id,
                method_index,
                "useAbility: the no-effect feedback could not be queued (base channel closed)"
            );
        }
    }
}
