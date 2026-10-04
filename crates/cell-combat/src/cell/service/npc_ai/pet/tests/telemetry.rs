//! The `pets.ai` rows as a contract (owner rule "telemetry is a first-class
//! requirement"): the refusal and ignore seams each log with a `reason`, and
//! every row names the event, the pet, and the owner's identity.

use super::*;
use crate::cell::combat::{generate_threat, AggroCause, BSF_DEAD};
use crate::test_support::LogCapture;

/// Owner dead (or gone, or in another space): the pet holds and says why,
/// until the owner sweep despawns it.
#[tokio::test]
async fn owner_missing_row_names_the_reason() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    mgr.get_entity_mut(OWNER).unwrap().state_field |= BSF_DEAD;
    let before = pos(&mgr, pet);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    let row = pets_ai_row(&logs, "pet_owner_missing").expect("owner_missing row");
    assert!(row.has_field("event", "owner_missing"), "{row:?}");
    assert!(row.has_field("reason", "owner_dead"), "{row:?}");
    assert_owner_identity(&row);
    assert_eq!(pos(&mgr, pet), before, "the pet holds");
    assert_eq!(
        state(&mgr, pet),
        AiState::Idle,
        "and is not armed to follow"
    );
}

/// The engage seam's refusal: a target the engagement refuses is a WARN
/// with its `reason` (an invariant violation: the stance never picks such a
/// target). Driven directly with a mob its owner could not attack.
#[test]
fn engage_refusal_warns_with_a_reason() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    // Not the hostile faction: the engagement refuses what the owner could
    // not attack.
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);

    let logs = LogCapture::install();
    let fighting = super::super::engage::engage_stance_pick(
        &mut mgr,
        pet,
        OWNER,
        MOB,
        super::super::stance::EngageWhy::DefendOwner,
    );
    assert!(!fighting);
    let row = logs
        .find_event(
            tracing::Level::WARN,
            "engagement was refused",
            "target_not_hostile",
        )
        .expect("engage_refused warn");
    assert!(row.has_field("event", "engage_refused"), "{row:?}");
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
    assert_owner_identity(&row);
}

/// A hit is the other Follow -> Fighting edge: it logs `fight_entered` with
/// the cause, the attacker and the state it left.
#[tokio::test]
async fn a_hit_logs_fight_entered() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    tick(&mut mgr).await;

    let logs = LogCapture::install();
    let _ = generate_threat(&mut mgr, MOB, pet, 20.0, AggroCause::Damage);
    let row = pets_ai_row(&logs, "pet_fight_entered").expect("fight_entered row");
    assert!(row.has_field("event", "fight_entered"), "{row:?}");
    assert!(row.has_field("cause", "damage"), "{row:?}");
    assert!(row.has_field("from", "follow"), "{row:?}");
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
    assert_owner_identity(&row);
}

/// Every `pets.ai` row of a busy turn carries `event`, `pet_id`, `owner_id`
/// and the owner's `account_id` / `player_id`.
#[tokio::test]
async fn every_pets_ai_row_carries_event_and_owner_identity() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [25.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, OWNER);
    add_mob(&mut mgr, MOB_2, [14.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB_2, pet);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    move_to(&mut mgr, OWNER, [120.0, 0.0, 10.0]);
    tick(&mut mgr).await;
    tick(&mut mgr).await;

    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.ai")
        .collect();
    assert!(rows.len() >= 3, "a busy turn logs: {rows:#?}");
    for row in &rows {
        assert!(row.fields.contains_key("event"), "{row:?}");
        assert!(row.has_field("pet_id", &pet.to_string()), "{row:?}");
        assert_owner_identity(row);
    }
}

/// Rule 5 correlator: every `pets.ai` row names the pet as `entity_id`,
/// as the `pets.lifecycle` rows do. Driven through a stance fight that ends
/// (engage, owner mirror, target dropped, follow re-armed).
#[tokio::test]
async fn every_pets_ai_row_carries_the_pet_as_entity_id() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [10.0, 0.0, 16.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Aggressive);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    mgr.get_entity_mut(MOB).unwrap().faction = 0;
    tick(&mut mgr).await;
    tick(&mut mgr).await;

    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.ai")
        .collect();
    assert!(rows.len() >= 3, "a fight's worth of rows: {rows:#?}");
    for row in rows {
        assert!(row.has_field("entity_id", &pet.to_string()), "{row:?}");
    }
}

/// Rule 6 (NT-25): a `pets.ai` row names the pet (`pet_name` and its
/// template, D-NT5), the owner and the target next to their IDs. Driven
/// through the engage refusal, with a NameBook naming the fixture (the book
/// is process-global, so an empty one goes back after).
#[test]
fn a_pets_ai_row_names_the_pet_owner_and_target() {
    let mut book = cimmeria_names::NameBook::empty();
    book.insert(cimmeria_names::Table::Texts, 8087, "Test Pet");
    book.insert(cimmeria_names::Table::Texts, 9001, "Jaffa Guard");
    book.insert(
        cimmeria_names::Table::Templates,
        PET_FIXTURE_TEMPLATE_ID.into(),
        "NT25_Pet_Template",
    );
    cimmeria_names::global().store(book);

    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let owner = mgr.get_entity_mut(OWNER).unwrap();
    owner.player_id = Some(OWNER_PLAYER_ID);
    owner.stamp_log_names(Some("Tealc"), Some("tealc_login"));
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("pet spawns");
    // Not the hostile faction, so the engagement refuses it.
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);
    mgr.get_entity_mut(MOB).unwrap().name_id = Some(9001);

    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    let _ = super::super::engage::engage_stance_pick(
        &mut mgr,
        pet,
        OWNER,
        MOB,
        super::super::stance::EngageWhy::DefendOwner,
    );
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());

    let row = pets_ai_row(&logs, "pet_engage_refused").expect("engage_refused row");
    for (key, value) in [
        ("entity_name", "Test Pet"),
        ("pet_name", "Test Pet"),
        ("template_name", "NT25_Pet_Template"),
        ("owner_name", "Tealc"),
        ("player_name", "Tealc"),
        ("account_name", "tealc_login"),
        ("target_name", "Jaffa Guard"),
    ] {
        assert!(row.has_field(key, value), "{key}={value} missing: {row:?}");
    }
}
