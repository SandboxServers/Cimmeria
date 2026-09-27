//! CM 88 `petInvokeAbility(INT32 petId, INT32 abilityId, INT32 targetId)`.
//!
//! Every pet-bar click lands here (A-06). After the ownership guard the
//! ability must be on the pet's bar and not toggled off, the pet must be
//! alive and ready, and an explicit target must be something the pet may
//! fight (PT-05's `npc_ai::pet::fight_refusal`: a combatant SGWMob its owner
//! could attack) in the pet's space, engageable, within reach and in sight.
//! Then the pet casts through `handle_use_ability_with_kill_credit` and
//! engages the target through the pet AI's `engage_pet_target`
//! (`PetEngagement::OwnerOrder`, both sides of the fight): at once for an
//! instant cast, or, for a cast with a warmup, when the warmup tick fires it
//! (`warmup::pet_order`). An interrupted warmup engages nothing.
//!
//! # Why the target gate lives here
//!
//! `handle_use_ability` gates targets for **player** attackers only (#444):
//! NPC AI calls it to attack players, which is legitimate. A pet is an NPC,
//! so without this gate an owner could aim their pet at a vendor, a quest
//! NPC, another player or another player's pet. The owner may not direct the
//! pet at anything the owner could not attack themselves.
//!
//! The pre-checks also keep refusals visible: `handle_use_ability` refuses a
//! cooldown or a busy caster silently, and answers out-of-range with an
//! `onErrorCode` addressed to the caster, which for a pet reaches no client.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::{
    owned_pet_or_refuse, pet_template_id, refuse, Caller, Refusal, FEEDBACK_INVALID_ENTITY,
    FEEDBACK_NOT_LIVING, FEEDBACK_NOT_READY, FEEDBACK_NO_LINE_OF_SIGHT,
    FEEDBACK_NO_SUCH_PET_ABILITY, FEEDBACK_OUTSIDE_WEAPON_RANGE, FEEDBACK_RELATIONSHIP_FRIEND,
};
use crate::cell::combat::is_dead_state;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_cell_combat::cell::service::npc_ai::pet::{
    engage_pet_target, fight_refusal, target_state_refusal, PetEngagement,
};
use cimmeria_cell_world::cell::pets::order_refusal_code;

/// The range `handle_use_ability` uses when an ability's `max_range` is the
/// `0` "server default" sentinel. Kept equal to it so this pre-check never
/// passes a cast the launch then refuses.
const DEFAULT_ABILITY_RANGE: f32 = 30.0;

/// Parsed CM 88 args (12 bytes, three LE INT32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InvokeArgs {
    pub(super) pet_id: i32,
    pub(super) ability_id: i32,
    pub(super) target_id: i32,
}

impl InvokeArgs {
    pub(super) fn parse(args: &[u8]) -> Option<Self> {
        let word = |i: usize| -> Option<i32> {
            Some(i32::from_le_bytes(args.get(i..i + 4)?.try_into().ok()?))
        };
        Some(Self {
            pet_id: word(0)?,
            ability_id: word(4)?,
            target_id: word(8)?,
        })
    }
}

#[tracing::instrument(
    name = "pets.command",
    target = "pets.command",
    level = "info",
    skip_all,
    fields(
        command = caller.command.label(),
        entity_id = caller.owner_id,
        pet_id = args.pet_id,
        ability_id = args.ability_id,
        target_id = args.target_id,
    )
)]
pub(super) async fn handle(
    caller: Caller,
    args: InvokeArgs,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let InvokeArgs {
        pet_id,
        ability_id,
        target_id,
    } = args;
    let Some(pet) = owned_pet_or_refuse(caller, pet_id, ability_id, tx, space_mgr).await else {
        return;
    };
    if let Err(refusal) = check_invoke(space_mgr, caller.owner_id, pet, ability_id, target_id) {
        refuse(caller, pet_id, refusal, tx).await;
        return;
    }
    let template_id = pet_template_id(space_mgr, pet);

    // PT-06 routes this wrapper's kill credit to the owner through
    // `credit_recipient`; the call is the same either way.
    let committed = crate::cell::abilities::handle_use_ability_with_kill_credit(
        pet,
        ability_id,
        target_id,
        &crate::cell::content::EngineEvents(engine),
        tx,
        space_mgr,
    )
    .await;
    if !committed {
        // Every refusal `handle_use_ability` makes for an NPC caster is
        // pre-checked above, so this is a race or a new gate: say so.
        refuse(
            caller,
            pet_id,
            Refusal::warn("cast_refused", FEEDBACK_NOT_READY, ability_id),
            tx,
        )
        .await;
        return;
    }

    // A cast with a warmup has only started: engaging now would leave the
    // pet fighting a target the warmup tick then refuses (it turned
    // friendly, went behind a wall). Record the order instead; the tick
    // engages it when the cast fires (`warmup::pet_order`) and an
    // interrupted cast engages nothing. An instant cast has fired already.
    let warming = space_mgr
        .get_entity(pet)
        .is_some_and(|e| e.pending_cast.is_some());
    let deferred = warming && target_id > 0;
    if let Some(state) = space_mgr
        .get_entity_mut(pet)
        .and_then(|e| e.pet.as_deref_mut())
    {
        // A new order replaces an older one still waiting on its warmup.
        state.deferred_order = deferred.then_some(target_id as u32);
    }
    // The pet AI's engagement (PT-05): both sides of the fight, the pet's
    // rule and the target's state, as an owner order (a surrendered NPC is
    // still a target, as for a player's own attack).
    let mut engaged = false;
    if !warming && target_id > 0 {
        match engage_pet_target(space_mgr, pet, target_id as u32, PetEngagement::OwnerOrder) {
            Ok(()) => engaged = true,
            // The cast just killed it: nothing to engage, nothing to report.
            Err("target_dead") => {}
            Err(reason) => {
                let refusal = Refusal::debug(reason, order_refusal_code(reason), ability_id);
                refuse(caller, pet_id, refusal, tx).await;
            }
        }
    }
    tracing::debug!(
        target: "pets.command",
        event = "invoked",
        decision_outcome = "invoked",
        command = caller.command.label(),
        owner_id = caller.owner_id,
        account_id = caller.account_id,
        player_id = caller.player_id,
        pet_id = pet,
        template_id,
        ability_id,
        target_id,
        engaged,
        engage_deferred = deferred,
        "pet command: the pet cast the owner's ability"
    );
}

/// Everything that must hold before the pet casts `ability_id` at
/// `target_id`. `pet` has already passed the ownership guard.
pub(crate) fn check_invoke(
    space_mgr: &SpaceManager,
    owner_id: u32,
    pet: u32,
    ability_id: i32,
    target_id: i32,
) -> Result<(), Refusal> {
    let entity = space_mgr.get_entity(pet).ok_or(Refusal::warn(
        "pet_gone",
        FEEDBACK_INVALID_ENTITY,
        ability_id,
    ))?;
    let state = entity.pet.as_deref().ok_or(Refusal::warn(
        "pet_gone",
        FEEDBACK_INVALID_ENTITY,
        ability_id,
    ))?;

    if !state.ability_list.contains(&ability_id) {
        return Err(not_in_list(space_mgr, ability_id));
    }
    if state.toggled_off.contains(&ability_id) {
        return Err(Refusal::debug(
            "ability_toggled_off",
            FEEDBACK_NO_SUCH_PET_ABILITY,
            ability_id,
        ));
    }
    if is_dead(entity) {
        return Err(Refusal::debug("pet_dead", FEEDBACK_NOT_LIVING, ability_id));
    }
    if entity.pending_cast.is_some() {
        return Err(Refusal::debug(
            "pet_casting",
            FEEDBACK_NOT_READY,
            ability_id,
        ));
    }
    if entity.abilities.is_on_cooldown(ability_id) {
        return Err(Refusal::debug(
            "ability_on_cooldown",
            FEEDBACK_NOT_READY,
            ability_id,
        ));
    }

    // `targetId <= 0` is an untargeted cast: `fire_cast` resolves no damage
    // for it, so there is nothing to gate.
    if target_id <= 0 {
        return Ok(());
    }
    let target_u = target_id as u32;
    // `get_entity` searches every space; a target elsewhere is not "here".
    let same_space = space_mgr.get_entity_space_id(target_u).is_some()
        && space_mgr.get_entity_space_id(target_u) == space_mgr.get_entity_space_id(pet);
    let target = match space_mgr.get_entity(target_u) {
        Some(t) if same_space => t,
        // DEBUG: the id is real but elsewhere (another instance); only a
        // forged packet names one, and a WARN would let it flood the log.
        Some(_) => {
            return Err(Refusal::debug(
                "target_other_space",
                FEEDBACK_INVALID_ENTITY,
                ability_id,
            ))
        }
        None => {
            return Err(Refusal::debug(
                "target_gone",
                FEEDBACK_INVALID_ENTITY,
                ability_id,
            ))
        }
    };
    // The pet rule (PT-05's `fight_refusal`): a combatant SGWMob its owner
    // could attack itself, so never a player, a pet (whatever its faction)
    // or a being (`target_not_combatant`), and never what the #444 rule
    // spares (`target_not_hostile`). DEBUG, not WARN: the pet bar sends the
    // owner's current target (`Unit.Target`), so either is an ordinary
    // misclick. The guard already proved the owner is live.
    let fight = space_mgr
        .get_entity(owner_id)
        .map_or(Some("target_not_hostile"), |owner| {
            fight_refusal(owner, target)
        });
    if let Some(reason) = fight {
        return Err(Refusal::debug(
            reason,
            FEEDBACK_RELATIONSHIP_FRIEND,
            ability_id,
        ));
    }
    // The engagement's state rule, checked before the cast so a refused
    // order casts nothing: dead, or walking home / leaving the world.
    if let Some(reason) = target_state_refusal(target, PetEngagement::OwnerOrder) {
        let code = if reason == "target_dead" {
            FEEDBACK_NOT_LIVING
        } else {
            FEEDBACK_INVALID_ENTITY
        };
        return Err(Refusal::debug(reason, code, ability_id));
    }
    let max_range = space_mgr
        .ability_defs
        .get(&ability_id)
        .filter(|d| d.max_range > 0)
        .map_or(DEFAULT_ABILITY_RANGE, |d| d.max_range as f32);
    if entity.position.distance_to(&target.position) > max_range {
        return Err(Refusal::debug(
            "out_of_range",
            FEEDBACK_OUTSIDE_WEAPON_RANGE,
            ability_id,
        ));
    }
    // Fire-time line of sight: `fire_los` skips every NPC shooter (the NPC
    // fight tick checks sight before it attacks), and this order does not
    // come from the fight tick. Without it the owner could have the pet hit
    // through walls. Same test the fight tick uses, mobile shooter.
    if !space_mgr.attack_line_of_sight(pet, target_u, false) {
        return Err(Refusal::debug(
            "no_line_of_sight",
            FEEDBACK_NO_LINE_OF_SIGHT,
            ability_id,
        ));
    }
    Ok(())
}

/// `ability_not_in_list`: WARN for an ability the server knows (a stale bar
/// or a bug worth seeing), DEBUG for an id with no ability definition, which
/// only a forged packet sends. Same split as `useAbility`'s not-known path,
/// so a client cannot fill the log index with bogus ids.
pub(crate) fn not_in_list(space_mgr: &SpaceManager, ability_id: i32) -> Refusal {
    if space_mgr.ability_defs.contains_key(&ability_id) {
        Refusal::warn(
            "ability_not_in_list",
            FEEDBACK_NO_SUCH_PET_ABILITY,
            ability_id,
        )
    } else {
        Refusal::debug(
            "ability_not_in_list",
            FEEDBACK_NO_SUCH_PET_ABILITY,
            ability_id,
        )
    }
}

/// `BSF_DEAD` or no health left: the same two tests the fight tick uses.
fn is_dead(entity: &cimmeria_entity::cell_entity::CellEntity) -> bool {
    is_dead_state(entity.state_field) || entity.stats.get(HEALTH).is_none_or(|s| s.cur <= 0)
}
