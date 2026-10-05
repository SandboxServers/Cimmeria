//! The fire half of an owner-pet cast: run the ability's pet effects on the
//! owner's pet (pets PT-08).

use cimmeria_entity::cell_entity::PetState;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::effects::pet_scripts::acts_on_owner_pet;
use cimmeria_entity::abilities::{AbilityDef, EffectDef, AF_TOGGLED, TCM_SINGLE};

use super::super::super::super::messages::CellToBaseMsg;
use super::super::super::super::space_manager::SpaceManager;
use super::super::super::messaging::WireRoute;
use super::super::super::wire_ledger::{self, WireCtx};
use super::super::sequence::{play_ability_sequence, AbilityPhase, PhaseSequence};
use super::feedback::send_line;
use super::launch::{dooms_pet, refuse, resolve};

/// The ability's effects that run on the pet, in `effect_ids` order. An
/// effect with no pet script (Holy Warrior's 4087 "Stance Removal", To The
/// Death's 4122 "Pet Death") runs nothing here: see
/// `effects::pet_scripts` for what each stands for.
fn pet_effects(space_mgr: &SpaceManager, ability_def: &Option<AbilityDef>) -> Vec<EffectDef> {
    let Some(def) = ability_def else {
        return Vec::new();
    };
    def.effect_ids
        .iter()
        .filter_map(|eid| space_mgr.effect_defs.get(eid))
        .filter(|e| e.script_name.as_deref().is_some_and(acts_on_owner_pet))
        .cloned()
        .collect()
}

/// Fire an owner-pet cast whose warmup has completed (or that had none).
///
/// Runs in place of target resolution: nothing here enters the damage
/// pipeline, threat or kill credit. The cooldown was charged at launch and
/// stays charged when the pet is gone by now.
pub(in crate::cell::abilities::use_ability) async fn fire_owner_pet(
    owner: u32,
    ability_id: i32,
    effect_seq: i32,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let event_set_id = ability_def.as_ref().and_then(|d| d.event_set_id);
    let pets = match resolve(space_mgr, owner, ability_id) {
        Ok(pets) => pets,
        Err(refusal) => {
            // The pet died, left or was doomed during the warmup. Show the
            // cast cancelled, then say why.
            play_ability_sequence(
                PhaseSequence {
                    phase: AbilityPhase::Interrupt,
                    entity_id: owner,
                    ability_id,
                    target_id: 0,
                    instance_id: effect_seq,
                    event_set_id,
                },
                tx,
                space_mgr,
            )
            .await;
            refuse(owner, ability_id, refusal, "fire", tx, space_mgr).await;
            return;
        }
    };
    play_ability_sequence(
        PhaseSequence {
            phase: AbilityPhase::End,
            entity_id: owner,
            ability_id,
            target_id: pets[0] as i32,
            instance_id: effect_seq,
            event_set_id,
        },
        tx,
        space_mgr,
    )
    .await;

    let effects = pet_effects(space_mgr, ability_def);
    let now = Instant::now();
    for effect in &effects {
        // `TCM_Single` lands on one pet; `TCM_AERadius` / `TCM_Group` on
        // every pet the owner has out here (one today, D-PT04).
        let targets: &[u32] = if effect.target_collection_method == TCM_SINGLE {
            &pets[..1]
        } else {
            &pets
        };
        for &pet in targets {
            let Some(script) = effect.script_name.clone() else {
                continue;
            };
            let mut ctx = crate::cell::effects::EffectContext {
                source_id: owner,
                target_id: pet,
                effect,
                space_mgr,
            };
            crate::cell::effects::dispatch_by_name(&script, &mut ctx);
            if effect.is_pulsing() {
                // A heal over time: the first pulse just ran, the pulsing
                // tick runs the rest on the pet (decision 5).
                let _registered = crate::cell::effects::register_active_effect(
                    space_mgr, pet, owner, effect, now, tx,
                )
                .await;
            }
        }
    }
    for &pet in &pets {
        flush_pet_stats(pet, tx, space_mgr).await;
    }

    let id = space_mgr.player_identity(owner);
    let effect_ids: Vec<i32> = effects.iter().map(|e| e.effect_id).collect();
    tracing::debug!(
        target: "pets.buff",
        event = "owner_ability_applied",
        decision_outcome = "owner_ability_applied",
        entity_id = owner,
        entity_name = space_mgr.entity_label(owner),
        owner_id = owner,
        owner_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        pet_id = pets[0],
        pet_name = space_mgr.entity_label(pets[0]),
        pet_ids = ?pets,
        effect_ids = ?effect_ids,
        "owner ability applied to the pet"
    );
    if let Some(text) = state_line(space_mgr, ability_def, &effects, pets[0], ability_id) {
        send_line(owner, id, ability_id, &text, tx).await;
    }
}

/// The chat line for a cast that changed a state the client does not show:
/// a toggle's new state, or To The Death's countdown.
fn state_line(
    space_mgr: &SpaceManager,
    ability_def: &Option<AbilityDef>,
    effects: &[EffectDef],
    pet: u32,
    ability_id: i32,
) -> Option<String> {
    let def = ability_def.as_ref()?;
    if def.flags & AF_TOGGLED != 0 {
        let on = effects
            .iter()
            .any(|e| space_mgr.has_pet_buff(pet, e.effect_id));
        let state = if on { "on" } else { "off" };
        return Some(format!("{} is {state}.", def.name));
    }
    if dooms_pet(space_mgr, ability_id) {
        let secs = space_mgr
            .get_entity(pet)
            .and_then(|e| e.extensions.get::<PetState>())
            .and_then(|p| p.doomed_at)
            .map(|at| {
                at.saturating_duration_since(Instant::now())
                    .as_secs_f32()
                    .round()
            })?;
        return Some(format!(
            "Your pet fights to the death: it dies in {secs:.0} seconds."
        ));
    }
    None
}

/// Send `pet`'s dirty stats to its witnesses (its owner among them).
pub(super) async fn flush_pet_stats(
    pet: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity_mut(pet) else {
        return;
    };
    let dirty = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();
    if !dirty.is_empty() {
        wire_ledger::send(
            pet,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            dirty,
            WireRoute::Witnesses,
            WireCtx::new("owner_pet"),
            tx,
            space_mgr,
        )
        .await;
    }
}
