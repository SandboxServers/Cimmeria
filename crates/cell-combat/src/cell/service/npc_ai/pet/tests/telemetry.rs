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

/// The engage seam's refusal: a target the threat entry refuses is a WARN
/// with `reason = threat_refused` (an invariant violation: the stance never
/// picks such a target). Driven directly with a Passive pet, which
/// `generate_threat` refuses.
#[test]
fn engage_refusal_warns_with_a_reason() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Passive);

    let logs = LogCapture::install();
    let fighting = super::super::defend::engage(
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
            "threat entry refused",
            "threat_refused",
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
