//! `pets.command` telemetry (owner rule "Telemetry is a first-class
//! requirement"): every refusal `reason` at its level, every accept `event`
//! with its before/after fields, the owner's identity on each row, and the
//! per-command info span.
//!
//! One scenario per `reason`. Each asserts exactly one `pets.command` row
//! with that reason, at the documented level, carrying `owner_id`,
//! `account_id` and `player_id` of the owner (Rule 5). The table in
//! `docs/analysis/pets/worknotes/pt-04.md` lists the same rows.

use tracing::Level;

use super::*;
use crate::test_support::{Captured, LogCapture};

/// Run `method(args)` as `OWNER` with the base channel closed, so every
/// send fails.
async fn call_with_closed_channel(mgr: &mut SpaceManager, method: u16, args: &[u8]) {
    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    assert!(super::super::dispatch(OWNER, method, args, &tx, mgr, &engine).await);
}

/// Build the world for `reason` and send the one command that triggers it.
async fn trigger(reason: &str) {
    let World {
        mut mgr,
        pet,
        other_pet,
    } = world();
    let (method, args) = match reason {
        "not_owner" => (PET_INVOKE_ABILITY, invoke_args(other_pet, PET_ABILITY, MOB)),
        "not_a_pet" => (PET_INVOKE_ABILITY, invoke_args(MOB, PET_ABILITY, MOB)),
        "pet_gone" => {
            let summoner = mgr.player_identity(OWNER);
            mgr.pets.register(OWNER, NOBODY, summoner);
            (PET_CHANGE_STANCE, stance_args(NOBODY, 2))
        }
        "owner_dead" => {
            mgr.get_entity_mut(OWNER).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
            (PET_CHANGE_STANCE, stance_args(pet, 2))
        }
        "pet_other_space" => {
            const STRAY: u32 = 200_020;
            mgr.spawn_npc(STRAY, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
                .unwrap();
            mgr.get_entity_mut(STRAY).unwrap().pet =
                Some(Box::new(cimmeria_entity::cell_entity::PetState::new(
                    OWNER,
                    PET_FIXTURE_ABILITIES.to_vec(),
                    0b111,
                    0,
                )));
            let summoner = mgr.player_identity(OWNER);
            mgr.pets.register(OWNER, STRAY, summoner);
            (PET_CHANGE_STANCE, stance_args(STRAY, 2))
        }
        "malformed_args" => (PET_CHANGE_STANCE, vec![1, 2]),
        "ability_not_in_list/known" => {
            (PET_INVOKE_ABILITY, invoke_args(pet, NOT_A_PET_ABILITY, MOB))
        }
        "ability_not_in_list/unknown" => (PET_INVOKE_ABILITY, invoke_args(pet, 999_999, MOB)),
        "ability_toggled_off" => {
            let state = mgr.get_entity_mut(pet).unwrap().pet.as_deref_mut().unwrap();
            state.toggled_off.push(PET_ABILITY);
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "ability_not_implemented" => {
            mgr.ability_defs.get_mut(&PET_ABILITY).unwrap().event_set_id = None;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "pet_dead" => {
            mgr.get_entity_mut(pet).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "pet_casting" => {
            mgr.ability_defs.get_mut(&PET_ABILITY).unwrap().warmup = 1.0;
            let _ = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
            let other = PET_FIXTURE_ABILITIES[1];
            (PET_INVOKE_ABILITY, invoke_args(pet, other, MOB))
        }
        "ability_on_cooldown" => {
            let _ = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "target_gone" => (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, NOBODY)),
        "target_other_space" => {
            const ELSEWHERE: u32 = 200_010;
            let p = mgr.get_entity(pet).unwrap().position;
            mgr.spawn_npc(ELSEWHERE, "Castle", [p.x + 1.0, p.y, p.z], [0.0; 3])
                .unwrap();
            mgr.get_entity_mut(ELSEWHERE).unwrap().faction = HOSTILE_FACTION;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, ELSEWHERE))
        }
        "target_not_hostile" => (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, FRIENDLY)),
        "target_not_combatant" => {
            mgr.get_entity_mut(MOB).unwrap().class_id = 0x01;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "target_resetting" => {
            cimmeria_cell_combat::cell::service::npc_ai::force_ai_state(
                mgr.get_entity_mut(MOB).unwrap(),
                cimmeria_entity::cell_entity::AiState::Leashing,
            );
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "target_dead" => {
            mgr.get_entity_mut(MOB).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "out_of_range" => {
            let x = mgr.get_entity(pet).unwrap().position.x;
            mgr.get_entity_mut(MOB).unwrap().position.x = x + 100.0;
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "no_line_of_sight" => {
            let pet_x = mgr.get_entity(pet).unwrap().position.x;
            let mob_x = mgr.get_entity(MOB).unwrap().position.x;
            let wall_x = (pet_x + mob_x) / 2.0;
            let wall = crate::test_support::occluder_fixtures::synthetic(&[(
                [wall_x - 0.15, 0.0, 0.0],
                [wall_x + 0.15, 4.0, 40.0],
            )]);
            let sid = mgr.get_entity_space_id(pet).unwrap();
            mgr.spaces.get_mut(&sid).unwrap().occluder = Some(wall);
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "cast_refused" => {
            // On the bar but no longer known to the entity: every PT-04
            // pre-check passes and `handle_use_ability` refuses.
            mgr.get_entity_mut(pet)
                .unwrap()
                .abilities
                .remove_ability(PET_ABILITY);
            (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB))
        }
        "bad_toggle_value" => (PET_ABILITY_TOGGLE, toggle_args(pet, PET_ABILITY, 2)),
        "stance_not_allowed" => (PET_CHANGE_STANCE, stance_args(pet, 5)),
        "feedback_send_failed" => {
            call_with_closed_channel(
                &mut mgr,
                PET_INVOKE_ABILITY,
                &invoke_args(pet, NOT_A_PET_ABILITY, MOB),
            )
            .await;
            return;
        }
        "owner_send_failed" => {
            call_with_closed_channel(&mut mgr, PET_CHANGE_STANCE, &stance_args(pet, 2)).await;
            return;
        }
        other => panic!("no scenario for {other}"),
    };
    let _ = call(&mut mgr, OWNER, method, &args).await;
}

/// `(scenario, reason, level)` for every refusal and send-failure row the
/// command handlers log. The ownership refusals are logged by
/// `SpaceManager::owned_pet` and pinned by
/// [`ownership_refusals_log_once_at_debug_with_the_caller_identity`].
const REFUSALS: &[(&str, &str, Level)] = &[
    ("owner_dead", "owner_dead", Level::DEBUG),
    ("pet_other_space", "pet_other_space", Level::DEBUG),
    ("malformed_args", "malformed_args", Level::WARN),
    (
        "ability_not_in_list/known",
        "ability_not_in_list",
        Level::WARN,
    ),
    (
        "ability_not_in_list/unknown",
        "ability_not_in_list",
        Level::DEBUG,
    ),
    ("ability_toggled_off", "ability_toggled_off", Level::DEBUG),
    (
        "ability_not_implemented",
        "ability_not_implemented",
        Level::DEBUG,
    ),
    ("pet_dead", "pet_dead", Level::DEBUG),
    ("pet_casting", "pet_casting", Level::DEBUG),
    ("ability_on_cooldown", "ability_on_cooldown", Level::DEBUG),
    ("target_gone", "target_gone", Level::DEBUG),
    ("target_other_space", "target_other_space", Level::DEBUG),
    ("target_not_hostile", "target_not_hostile", Level::DEBUG),
    ("target_not_combatant", "target_not_combatant", Level::DEBUG),
    ("target_resetting", "target_resetting", Level::DEBUG),
    ("target_dead", "target_dead", Level::DEBUG),
    ("out_of_range", "out_of_range", Level::DEBUG),
    ("no_line_of_sight", "no_line_of_sight", Level::DEBUG),
    ("cast_refused", "cast_refused", Level::WARN),
    ("bad_toggle_value", "bad_toggle_value", Level::DEBUG),
    ("stance_not_allowed", "stance_not_allowed", Level::DEBUG),
    ("feedback_send_failed", "feedback_send_failed", Level::WARN),
    ("owner_send_failed", "owner_send_failed", Level::WARN),
];

fn assert_owner_identity(row: &Captured, what: &str) {
    assert!(
        row.has_field("owner_id", &OWNER.to_string()),
        "{what}: {row:?}"
    );
    assert!(
        row.has_field("account_id", &OWNER_ACCOUNT_ID.to_string()),
        "{what}: {row:?}"
    );
    assert!(
        row.has_field("player_id", &OWNER_PLAYER_ID.to_string()),
        "{what}: {row:?}"
    );
}

/// Every refusal logs once on `pets.command`, with its `reason`, at its
/// level, and names the owner by entity id, account and character.
#[tokio::test]
async fn every_refusal_logs_its_reason_at_its_level_with_the_owner_identity() {
    for &(scenario, reason, level) in REFUSALS {
        let capture = LogCapture::install();
        trigger(scenario).await;
        let rows: Vec<Captured> = capture
            .all()
            .into_iter()
            .filter(|c| c.target == "pets.command" && c.has_field("reason", reason))
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{scenario}: exactly one reason={reason} row; captured: {:#?}",
            capture.all()
        );
        assert_eq!(rows[0].level, level, "{scenario}: level of reason={reason}");
        assert_owner_identity(&rows[0], scenario);
    }
}

/// The guard's rows come from `SpaceManager::owned_pet`, which names the
/// caller as `caller_id` (its `owner_id` field is the pet's real owner), so
/// they are pinned here rather than in [`REFUSALS`]. One DEBUG row each,
/// carrying the caller's account and character.
#[tokio::test]
async fn ownership_refusals_log_once_at_debug_with_the_caller_identity() {
    for reason in ["not_owner", "not_a_pet", "pet_gone"] {
        let capture = LogCapture::install();
        trigger(reason).await;
        let rows: Vec<Captured> = capture
            .all()
            .into_iter()
            .filter(|c| c.target == "pets.command" && c.has_field("reason", reason))
            .collect();
        assert_eq!(rows.len(), 1, "{reason}: {:#?}", capture.all());
        let row = &rows[0];
        assert_eq!(row.level, Level::DEBUG, "{reason}");
        assert!(row.has_field("event", "ownership_rejected"), "{row:?}");
        assert!(row.has_field("caller_id", &OWNER.to_string()), "{row:?}");
        assert!(
            row.has_field("account_id", &OWNER_ACCOUNT_ID.to_string()),
            "{row:?}"
        );
        assert!(
            row.has_field("player_id", &OWNER_PLAYER_ID.to_string()),
            "{row:?}"
        );
    }
}

/// The accepted-command row of one command: `event = <event>`.
fn accept_row(capture: &crate::test_support::LogCaptureGuard, event: &str) -> Captured {
    let rows: Vec<Captured> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.command" && c.has_field("event", event))
        .collect();
    assert_eq!(rows.len(), 1, "one event={event}: {:#?}", capture.all());
    assert_eq!(rows[0].level, Level::DEBUG);
    assert_owner_identity(&rows[0], event);
    rows[0].clone()
}

/// The info span every command opens, with the caller and the claimed pet.
fn assert_command_span(capture: &crate::test_support::LogCaptureGuard, pet: u32, command: &str) {
    let span = capture
        .all()
        .into_iter()
        .find(|c| c.target == "span:pets.command")
        .unwrap_or_else(|| panic!("no pets.command span: {:#?}", capture.all()));
    assert_eq!(span.level, Level::INFO);
    assert!(span.has_field("entity_id", &OWNER.to_string()), "{span:?}");
    assert!(span.has_field("pet_id", &pet.to_string()), "{span:?}");
    assert!(span.has_field("command", command), "{span:?}");
}

#[tokio::test]
async fn an_accepted_invoke_logs_invoked_with_ability_and_target() {
    let World { mut mgr, pet, .. } = world();
    let capture = LogCapture::install();
    let _ = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    let row = accept_row(&capture, "invoked");
    assert!(row.has_field("pet_id", &pet.to_string()));
    assert!(row.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()));
    assert!(row.has_field("ability_id", &PET_ABILITY.to_string()));
    assert!(row.has_field("target_id", &MOB.to_string()));
    assert!(row.has_field("engaged", "true"));
    assert_command_span(&capture, pet, "invoke_ability");
}

#[tokio::test]
async fn an_accepted_toggle_logs_toggled_with_ability_and_on() {
    let World { mut mgr, pet, .. } = world();
    let capture = LogCapture::install();
    let _ = toggle(&mut mgr, OWNER, pet, PET_ABILITY, 0).await;
    let row = accept_row(&capture, "toggled");
    assert!(row.has_field("ability_id", &PET_ABILITY.to_string()));
    assert!(row.has_field("on", "false"));
    assert!(row.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()));
    assert_command_span(&capture, pet, "ability_toggle");
}

#[tokio::test]
async fn an_accepted_stance_change_logs_stance_before_and_after() {
    let World { mut mgr, pet, .. } = world();
    let capture = LogCapture::install();
    let _ = stance(&mut mgr, OWNER, pet, 3).await;
    let row = accept_row(&capture, "stance_set");
    assert!(row.has_field("stance_before", "defensive"));
    assert!(row.has_field("stance_after", "aggressive"));
    assert!(row.has_field("source", "slot_index"));
    assert!(row.has_field("requested", "3"));
    assert_command_span(&capture, pet, "change_stance");
}
