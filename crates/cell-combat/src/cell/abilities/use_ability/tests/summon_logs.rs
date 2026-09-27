//! Pets PT-03 telemetry (TESTING.md type 12): every summon transition logs
//! a `pets.lifecycle` event with the owner's `account_id` / `player_id`, and
//! every refusal logs its `reason` at the level the negative-logging
//! convention asks for (DEBUG when a client can trigger it at will, WARN
//! when it cannot).

use tracing::Level;

use crate::test_support::{Captured, LogCapture, NoContentEvents};

use super::summon::{
    after_summon_warmup, cast_and_complete, ready_again, summon_def, summon_mgr, OWNER, SUMMON,
    TEMPLATE,
};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::spawner::PetSummon;

/// The owner's `player_id` in these tests (`add_pet_owner` sets
/// `account_id = entity id`, so the account is `OWNER`).
const PLAYER_ID: i32 = 4242;

fn mgr_with_identity() -> SpaceManager {
    let mut mgr = summon_mgr();
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(PLAYER_ID);
    mgr
}

fn row(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.target == "pets.lifecycle" && c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no pets.lifecycle event={event}: {all:#?}"))
}

/// The Rule 5 correlators every summon row carries.
fn assert_owner_correlators(c: &Captured) {
    assert!(c.has_field("owner_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("player_id", &PLAYER_ID.to_string()), "{c:?}");
    assert!(c.has_field("ability_id", &SUMMON.to_string()), "{c:?}");
    assert!(c.has_field("template_id", &TEMPLATE.to_string()), "{c:?}");
}

/// Launch, warmup start, fire, replace and spawn each log a DEBUG event
/// with the owner's identity; the replace names `replaced_pet_id` and the
/// spawn names `pet_id`.
#[tokio::test]
async fn summon_transitions_log_debug_events_with_the_owners_identity() {
    let mut mgr = mgr_with_identity();
    let (tx, mut rx) = mpsc::channel(256);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let first = mgr.pets.pets_of(OWNER)[0];
    ready_again(&mut mgr);

    let logs = LogCapture::install();
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let second = mgr.pets.pets_of(OWNER)[0];
    let all = logs.all();

    for event in [
        "summon_launched",
        "summon_warmup_started",
        "summon_fired",
        "summon_replaced_pet",
        "summon_spawned",
    ] {
        let c = row(&all, event);
        assert_eq!(c.level, Level::DEBUG, "{event}: {c:?}");
        assert_owner_correlators(&c);
    }
    assert!(row(&all, "summon_replaced_pet").has_field("replaced_pet_id", &first.to_string()));
    assert!(row(&all, "summon_spawned").has_field("pet_id", &second.to_string()));
    assert!(row(&all, "summon_warmup_started")
        .fields
        .contains_key("warmup_secs"));
}

/// An interrupted summon warmup logs `summon_interrupted` with the
/// interrupt's `reason`.
#[tokio::test]
async fn interrupted_summon_logs_its_reason() {
    let mut mgr = mgr_with_identity();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.get_entity_mut(OWNER).unwrap().position.x += 2.0;

    let logs = LogCapture::install();
    resolve_warmups(after_summon_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let c = row(&logs.all(), "summon_interrupted");
    assert_eq!(c.level, Level::DEBUG);
    assert!(c.has_field("reason", "caster_moved"), "{c:?}");
    assert_owner_correlators(&c);
}

/// Seam: a summon the player has not trained (a client can send any
/// ability id, so DEBUG).
#[tokio::test]
async fn untrained_summon_refusal_logs_debug_not_trained() {
    let mut mgr = mgr_with_identity();
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .remove_ability(SUMMON);
    let summon = mgr.pet_summons.pet_summon_for(SUMMON).unwrap();
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert!(super::super::summon::refuse_summon_launch(OWNER, SUMMON, summon, &tx, &mgr).await);
    let c = logs
        .find_event(Level::DEBUG, "summon refused at launch", "not_trained")
        .expect("DEBUG summon_refused reason=not_trained");
    assert!(c.has_field("stage", "launch"));
    assert_owner_correlators(&c);
}

/// Seam: a summon row whose template is not cached (a seed defect no client
/// can cause, so WARN).
#[tokio::test]
async fn missing_template_refusal_logs_warn_unknown_template() {
    let mut mgr = mgr_with_identity();
    mgr.spawn_templates.remove(&TEMPLATE);
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert!(!handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    let c = logs
        .find_event(Level::WARN, "summon refused at launch", "unknown_template")
        .expect("WARN summon_refused reason=unknown_template");
    assert!(c.has_field("event", "summon_refused"));
    assert_owner_correlators(&c);
}

/// Seam: the template vanishes during the warmup; the fire-time re-check
/// refuses with WARN `stage=fire`.
#[tokio::test]
async fn fire_time_missing_template_logs_warn() {
    let mut mgr = mgr_with_identity();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.spawn_templates.remove(&TEMPLATE);

    let logs = LogCapture::install();
    resolve_warmups(after_summon_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let c = logs
        .find_event(
            Level::WARN,
            "summon refused after its warmup",
            "unknown_template",
        )
        .expect("WARN summon_refused reason=unknown_template at fire");
    assert!(c.has_field("stage", "fire"));
    assert_owner_correlators(&c);
}

/// Seam: the fire-time re-check finds the owner dead (the backstop behind
/// the death interrupt), WARN `reason=owner_dead`.
#[tokio::test]
async fn fire_time_dead_owner_logs_warn() {
    let mut mgr = mgr_with_identity();
    mgr.get_entity_mut(OWNER).unwrap().state_field |= cimmeria_wire::state_field::BSF_DEAD;
    let summon = PetSummon {
        ability_id: SUMMON,
        template_id: TEMPLATE,
        max_active: 1,
    };
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    super::super::summon::fire_summon(
        OWNER,
        SUMMON,
        1,
        summon,
        &Some(summon_def(SUMMON)),
        &tx,
        &mut mgr,
    )
    .await;
    let c = logs
        .find_event(Level::WARN, "summon refused after its warmup", "owner_dead")
        .expect("WARN summon_refused reason=owner_dead");
    assert_owner_correlators(&c);
    assert!(mgr.pets.is_empty());
}
