//! `.pet`: GM pet UAT tools (pets campaign PT-07).
//!
//! - `.pet summon <templateId|abilityId>`: spawn a pet for the caller at
//!   once, with no warmup. An id with a `pet_summons` row is a summon ability
//!   and spawns its template; any other id must be a cached
//!   `entity_templates` row, which is spawned as a pet whatever its class
//!   (handy for trying a look). One pet per owner (D-PT04): as in PT-03's
//!   summon, the new pet is spawned first and the caller's existing pets
//!   are then dismissed, so a failed spawn keeps the old pet. A dead caller
//!   is refused.
//! - `.pet dismiss`: despawn the caller's pets.
//! - `.pet stance <0-2>`: set the caller's pet's stance (`EPetStance`) and
//!   send `onPetStanceUpdate` to the caller.
//! - `.pet info`: the selected pet (any owner, read only), else the caller's.
//! - `.pet list`: every pet in the caller's space, with its owner.
//!
//! The mutating verbs act only on the caller's own pets: each candidate from
//! `PetRegistry::pets_of(caller)` must also pass `SpaceManager::owned_pet`,
//! which checks the pet is live and that the caller is the player who
//! summoned it (entity ids are reused, so the id alone is not ownership). A
//! pet that fails is left for the teardown sweep. No client-supplied pet id
//! is ever trusted here. Only `.pet info` reads the selected target, and
//! only to display it.
//!
//! This is a GM tool, not the player path: the summon ability (PT-03) and
//! the pet-bar commands (PT-04, with the `owned_pet` ownership guard) are the
//! player-facing routes.
//!
//! # Telemetry (target `pets.command`)
//!
//! [`dispatch`] opens the info span `pets.command` (Rule 1). Every accept
//! and every refusal is one event on the `pets.command` target carrying the
//! caller's `account_id` / `player_id` (Rule 5, resolved late):
//!
//! - INFO `decision_outcome = gm_summoned | gm_dismissed | gm_stance_set`;
//! - DEBUG `decision_outcome = gm_refused` with `reason` (a GM can trigger
//!   every refusal at will, so none is a WARN);
//! - DEBUG `decision_outcome = gm_inspected` for `info` / `list`.
//!
//! A line about another owner's pet adds `subject_player_id`. The despawns
//! and the spawn themselves are logged once more on `pets.lifecycle`.

use cimmeria_entity::cell_entity::{PetStance, PetState};
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::client_methods::pet::{build_pet_stance_update, ON_PET_STANCE_UPDATE};
use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::pets::{despawn_pet, PetDespawnReason};
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

/// Usage line for a missing or unknown sub-command.
pub(super) const USAGE: &str =
    ".pet summon <templateId|abilityId> | dismiss | stance <0-2> | info | list";

/// Most pets `.pet list` prints; the rest are counted.
const LIST_LIMIT: usize = 20;

/// Route `.pet <sub> [arg]`.
#[tracing::instrument(
    name = "pets.command",
    level = "info",
    skip_all,
    fields(
        entity_id = caller_id,
        verb = args.first().copied().unwrap_or(""),
        account_id = space_mgr.player_identity(caller_id).account_id,
        player_id = space_mgr.player_identity(caller_id).player_id,
    )
)]
pub(super) async fn dispatch(
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match args.first().copied() {
        Some("summon") => summon(caller_id, args, tx, space_mgr).await,
        Some("dismiss") => dismiss(caller_id, tx, space_mgr).await,
        Some("stance") => stance(caller_id, args, tx, space_mgr).await,
        Some("info") => info(caller_id, target_id, tx, space_mgr).await,
        Some("list") => list(caller_id, tx, space_mgr).await,
        _ => {
            refuse(
                caller_id,
                "unknown_verb",
                None,
                &format!("Usage: {USAGE}"),
                tx,
                space_mgr,
            )
            .await
        }
    }
}

/// Log a refusal on `pets.command` and answer the GM with `text`.
async fn refuse(
    caller_id: u32,
    reason: &'static str,
    pet_id: Option<u32>,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    tracing::debug!(
        target: "pets.command",
        decision_outcome = "gm_refused",
        reason,
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        pet_id,
        pet_name = pet_id.and_then(|p| space_mgr.entity_label(p)),
        "GM .pet refused",
    );
    send_gm_feedback(caller_id, text, tx).await;
}

/// `.pet summon <id>`.
async fn summon(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(id) = args.get(1).and_then(|s| s.parse::<i32>().ok()) else {
        let text = ".pet summon: give a template id or a summon ability id";
        refuse(caller_id, "bad_args", None, text, tx, space_mgr).await;
        return;
    };
    // A summon ability first: ability and template ids live in different
    // ranges today, and the ability is what a player would cast.
    let (template_id, summon_ability_id) = match space_mgr.pet_summons.pet_summon_for(id) {
        Some(row) => (row.template_id, id),
        None if space_mgr.spawn_templates.contains_key(&id) => (id, 0),
        None => {
            let text =
                format!(".pet summon: {id} is neither a summon ability nor a cached template");
            refuse(caller_id, "unknown_id", None, &text, tx, space_mgr).await;
            return;
        }
    };
    if !space_mgr.get_entity(caller_id).is_some_and(|e| e.is_player) {
        let text = ".pet summon: only a player can own a pet";
        refuse(caller_id, "not_a_player", None, text, tx, space_mgr).await;
        return;
    }
    let class_note = match space_mgr.spawn_templates.get(&template_id) {
        Some(t) if t.class != "pet" => format!(" (template class '{}', spawned as a pet)", t.class),
        _ => String::new(),
    };

    // PT-03's summon refuses a dead caster too: a corpse's pet would be torn
    // down by the owner-death hook on the next death anyway.
    if space_mgr
        .get_entity(caller_id)
        .is_some_and(|e| combat::is_dead_state(e.state_field))
    {
        let text = ".pet summon: you are dead";
        refuse(caller_id, "owner_dead", None, text, tx, space_mgr).await;
        return;
    }

    // Same order as PT-03's `fire_summon`: list the caller's pets, spawn the
    // new one (which registers it with the caller's identity), and only then
    // retire the old ones through the shared teardown. A spawn that fails
    // leaves the GM with the pet it had.
    let current = owned_pets(caller_id, space_mgr);
    let pet_id = match space_mgr.spawn_pet_from_template(caller_id, template_id, summon_ability_id)
    {
        Ok(pet_id) => pet_id,
        // `spawn_pet_from_template` already logged the WARN on
        // `pets.lifecycle` with its own reason.
        Err(e) => {
            let text = format!(".pet summon: failed, {e}");
            refuse(caller_id, "spawn_failed", None, &text, tx, space_mgr).await;
            return;
        }
    };
    let replaced = despawn_each(&current, tx, space_mgr).await;
    let id = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "pets.command",
        decision_outcome = "gm_summoned",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        pet_id,
        pet_name = space_mgr.entity_label(pet_id),
        owner_id = caller_id,
        owner_name = id.player_name,
        template_id,
        template_name = cimmeria_names::book().template(template_id),
        ability_id = summon_ability_id,
        ability_name = cimmeria_names::book().ability(summon_ability_id),
        replaced = ?replaced,
        "GM .pet summon",
    );
    let via = if summon_ability_id != 0 {
        format!(" via ability {summon_ability_id}")
    } else {
        String::new()
    };
    let replaced_note = if replaced.is_empty() {
        String::new()
    } else {
        format!(", replacing {replaced:?}")
    };
    send_gm_feedback(
        caller_id,
        &format!(
            ".pet summon: pet {pet_id} from template {template_id}{via}{class_note}{replaced_note}"
        ),
        tx,
    )
    .await;
}

/// The pets `caller` really owns: registered to its entity id AND summoned
/// by the player holding that id now. `owned_pet` logs each refusal once on
/// `pets.command` (`event = ownership_rejected`); a refused pet belongs to
/// an earlier holder of the id and the teardown sweep removes it.
fn owned_pets(caller: u32, space_mgr: &SpaceManager) -> Vec<u32> {
    space_mgr
        .pets
        .pets_of(caller)
        .into_iter()
        .filter(|&pet| space_mgr.owned_pet(caller, pet).is_ok())
        .collect()
}

/// Despawn every pet `owner` owns, through the shared teardown
/// (`despawn_pet`, reason `dismissed`). Returns the ids that were despawned;
/// `despawn_pet` scrubs the registry either way and WARNs on a miss.
async fn dismiss_all(
    owner: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let pets = owned_pets(owner, space_mgr);
    despawn_each(&pets, tx, space_mgr).await
}

/// Despawn `pets` with reason `dismissed`. Returns the ids that were
/// despawned.
async fn despawn_each(
    pets: &[u32],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let mut despawned = Vec::new();
    for &pet_id in pets {
        let outcome = despawn_pet(space_mgr, pet_id, PetDespawnReason::Dismissed, tx).await;
        if matches!(outcome, DespawnOutcome::Despawned { .. }) {
            despawned.push(pet_id);
        }
    }
    despawned
}

/// `.pet dismiss`.
async fn dismiss(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let pets = dismiss_all(caller_id, tx, space_mgr).await;
    if pets.is_empty() {
        let text = ".pet dismiss: you have no pet out";
        refuse(caller_id, "no_pet", None, text, tx, space_mgr).await;
        return;
    }
    let id = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "pets.command",
        decision_outcome = "gm_dismissed",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        owner_id = caller_id,
        owner_name = id.player_name,
        pets = ?pets,
        "GM .pet dismiss",
    );
    send_gm_feedback(caller_id, &format!(".pet dismiss: dismissed {pets:?}"), tx).await;
}

/// `.pet stance <0-2>`.
async fn stance(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `EPetStance` is an INT8 on the wire. Parse wide, then refuse anything
    // outside the enum rather than truncating (300 must not become 44).
    let parsed = args
        .get(1)
        .and_then(|s| s.parse::<i32>().ok())
        .and_then(|v| i8::try_from(v).ok())
        .and_then(|v| PetStance::try_from(v).ok());
    let Some(stance) = parsed else {
        let text = ".pet stance: give 0 (passive), 1 (defensive) or 2 (aggressive)";
        refuse(caller_id, "bad_stance", None, text, tx, space_mgr).await;
        return;
    };
    // `owned_pets` has already run the ownership guard, so the pet is live
    // and was summoned by this caller.
    let Some(pet_id) = owned_pets(caller_id, space_mgr).first().copied() else {
        let text = ".pet stance: you have no pet out";
        refuse(caller_id, "no_pet", None, text, tx, space_mgr).await;
        return;
    };
    let Some(pet) = space_mgr
        .get_entity_mut(pet_id)
        .and_then(|e| e.extensions.get_mut::<PetState>())
    else {
        let text = ".pet stance: your pet is gone";
        refuse(caller_id, "pet_gone", Some(pet_id), text, tx, space_mgr).await;
        return;
    };
    if !pet.allows(stance) {
        let text = format!(
            ".pet stance: this pet cannot be {}; allowed: {}",
            stance.label(),
            stance_labels(pet)
        );
        refuse(
            caller_id,
            "stance_not_allowed",
            Some(pet_id),
            &text,
            tx,
            space_mgr,
        )
        .await;
        return;
    }
    // PT-05 has no stance setter: its AI tick reads `pet.stance` every pass
    // and drops a fight itself when the stance turns Passive (the
    // `passive_stance` disengage), so the write is the whole state change.
    let previous = std::mem::replace(&mut pet.stance, stance);
    let owner = pet.owner_id;
    let id = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "pets.command",
        decision_outcome = "gm_stance_set",
        event = "stance_changed",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        pet_id,
        pet_name = space_mgr.entity_label(pet_id),
        owner_id = owner,
        owner_name = space_mgr.entity_label(owner),
        from = previous.label(),
        stance = stance.label(),
        "GM .pet stance",
    );
    // Owner only: the stance highlights the owner's pet bar.
    if let Err(e) = tx
        .send(CellToBaseMsg::WitnessEntityMethod {
            witness_id: owner,
            entity_id: pet_id,
            method_index: ON_PET_STANCE_UPDATE,
            args: build_pet_stance_update(stance.wire()),
            entity_is_player: false,
        })
        .await
    {
        tracing::warn!(
            target: "pets.command",
            decision_outcome = "send_failed",
            reason = "cell_to_base_closed",
            entity_id = caller_id,
            entity_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            pet_id,
            pet_name = space_mgr.entity_label(pet_id),
            owner_id = owner,
            owner_name = space_mgr.entity_label(owner),
            error = %e,
            "GM .pet stance: onPetStanceUpdate not sent; the pet bar shows the old stance",
        );
    }
    send_gm_feedback(
        caller_id,
        &format!(".pet stance: pet {pet_id} is now {}", stance.label()),
        tx,
    )
    .await;
}

fn stance_labels(pet: &PetState) -> String {
    pet.allowed_stances()
        .iter()
        .map(|s| s.label())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `name (id)` for an owner, or just the id when the entity has no name.
fn owner_label(space_mgr: &SpaceManager, owner: u32) -> String {
    match space_mgr
        .get_entity(owner)
        .and_then(|e| e.character_name.clone())
    {
        Some(name) => format!("{name} ({owner})"),
        None => format!("entity {owner}"),
    }
}

/// Distance from `pet_id` to `owner`, if both exist.
fn owner_distance(space_mgr: &SpaceManager, pet_id: u32, owner: u32) -> Option<f32> {
    let pet = space_mgr.get_entity(pet_id)?;
    let owner = space_mgr.get_entity(owner)?;
    Some(pet.position.distance_to(&owner.position))
}

/// `.pet info`.
async fn info(
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let subject = target_id
        .filter(|&t| space_mgr.pets.is_pet(t))
        .or_else(|| owned_pets(caller_id, space_mgr).first().copied());
    let Some(pet_id) = subject else {
        let text = ".pet info: select a pet, or summon one with .pet summon";
        refuse(caller_id, "no_pet", None, text, tx, space_mgr).await;
        return;
    };
    let Some((entity, pet)) = space_mgr
        .get_entity(pet_id)
        .and_then(|e| e.extensions.get::<PetState>().map(|p| (e, p)))
    else {
        let text = format!(".pet info: pet {pet_id} is gone");
        refuse(caller_id, "pet_gone", Some(pet_id), &text, tx, space_mgr).await;
        return;
    };
    let owner = pet.owner_id;
    let id = space_mgr.player_identity(caller_id);
    // Another owner's pet names its summoner as the subject (Rule 5): the
    // identity captured at summon, since the owner's entity id may since
    // have been reused.
    let subject = (owner != caller_id).then(|| space_mgr.pets.summoner_identity(pet_id));
    let subject_player_id = subject.and_then(|s| s.player_id);
    let subject_player_name = subject.and_then(|s| s.player_name);
    tracing::debug!(
        target: "pets.command",
        decision_outcome = "gm_inspected",
        verb = "info",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        subject_player_id,
        subject_player_name = subject_player_name,
        pet_id,
        pet_name = space_mgr.entity_label(pet_id),
        owner_id = owner,
        owner_name = space_mgr.entity_label(owner),
        "GM .pet info",
    );
    let distance = owner_distance(space_mgr, pet_id, owner)
        .map_or_else(|| "owner not found".to_string(), |d| format!("{d:.1} u"));
    let last_teleport = pet.last_teleport_at.map_or_else(
        || "never".to_string(),
        |t| format!("{:.1} s ago", t.elapsed().as_secs_f32()),
    );
    let lines = [
        format!(
            ".pet info: pet {pet_id}, template {}, owner {}, summoned by ability {}",
            entity.template_id.unwrap_or(0),
            owner_label(space_mgr, owner),
            pet.summon_ability_id
        ),
        format!(
            "  stance {} (allowed: {}), AI state {:?}",
            pet.stance.label(),
            stance_labels(pet),
            entity.ai_state()
        ),
        format!(
            "  abilities {:?}, toggled off {:?}",
            pet.ability_list, pet.toggled_off
        ),
        format!("  distance to owner {distance}, last teleport {last_teleport}"),
    ];
    for line in lines {
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

/// `.pet list`.
async fn list(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let space = space_mgr.get_entity_space_id(caller_id);
    let mut pairs: Vec<(u32, u32)> = space_mgr
        .pets
        .pairs()
        .into_iter()
        .filter(|&(pet, _)| space.is_some() && space_mgr.get_entity_space_id(pet) == space)
        .collect();
    pairs.sort_unstable();
    let id = space_mgr.player_identity(caller_id);
    tracing::debug!(
        target: "pets.command",
        decision_outcome = "gm_inspected",
        verb = "list",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        space_id = space,
        world = space.and_then(|s| space_mgr.world_name_for_space(s)),
        pets = pairs.len(),
        "GM .pet list",
    );
    send_gm_feedback(
        caller_id,
        &format!(".pet list: {} pet(s) in this space", pairs.len()),
        tx,
    )
    .await;
    for &(pet_id, owner) in pairs.iter().take(LIST_LIMIT) {
        let Some(entity) = space_mgr.get_entity(pet_id) else {
            continue;
        };
        let stance = entity
            .extensions
            .get::<PetState>()
            .map_or("?", |p| p.stance.label());
        let distance = owner_distance(space_mgr, pet_id, owner)
            .map_or_else(|| "-".to_string(), |d| format!("{d:.1} u"));
        let line = format!(
            "  pet {pet_id} template {} owner {} stance {stance} distance {distance}",
            entity.template_id.unwrap_or(0),
            owner_label(space_mgr, owner),
        );
        send_gm_feedback(caller_id, &line, tx).await;
    }
    if pairs.len() > LIST_LIMIT {
        send_gm_feedback(
            caller_id,
            &format!("  ... and {} more", pairs.len() - LIST_LIMIT),
            tx,
        )
        .await;
    }
}
