//! `LogCapture` guards (TESTING.md type 12) for the `pets.buff` rows: each
//! transition carries `event` and the owner's identity, and each refusal
//! carries `reason` at the level the negative-logging convention sets.
//!
//! The fixture owner has `account_id = 7` and `player_id = 1007`
//! (`add_pet_owner`). Current-thread tests: `LogCapture` installs a
//! thread-local subscriber.

use std::time::Duration;

use cimmeria_entity::cell_entity::PlayerIdentity;
use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::abilities::use_ability::handle_use_ability;
use crate::cell::abilities::use_ability::owner_pet::owner_pet_tick_at;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard, NoContentEvents};

fn event(logs: &LogCaptureGuard, name: &str) -> Option<Captured> {
    logs.all()
        .into_iter()
        .find(|c| c.target == "pets.buff" && c.has_field("event", name))
}

fn assert_owner_identity(c: &Captured) {
    assert!(
        c.has_field("account_id", "7") && c.has_field("player_id", "1007"),
        "owner identity missing: {c:#?}"
    );
}

/// `buff_applied` and `buff_removed reason=toggled_off` carry the pet, the
/// owner's identity, the ability and effect, and the stats before and after.
#[tokio::test]
async fn toggle_rows_carry_identity_and_before_after() {
    let logs = LogCapture::install();
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    ready_again(&mut mgr);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);

    let applied = event(&logs, "buff_applied").expect("buff_applied");
    assert_eq!(applied.level, Level::DEBUG);
    assert_owner_identity(&applied);
    assert!(applied.has_field("entity_id", &pet.to_string()));
    assert!(applied.has_field("pet_id", &pet.to_string()));
    assert!(applied.has_field("owner_id", &OWNER.to_string()));
    assert!(applied.has_field("ability_id", &HOLY_WARRIOR.to_string()));
    assert!(applied.has_field("effect_id", &E_HOLY_WARRIOR.to_string()));
    assert!(applied.has_field("toggle", "true"));
    assert!(applied.has_field("stats_before", "[(11, 0), (12, 0)]"));
    assert!(applied.has_field("stats_after", "[(11, 100), (12, -100)]"));

    let removed = event(&logs, "buff_removed").expect("buff_removed");
    assert_owner_identity(&removed);
    assert!(removed.has_field("reason", "toggled_off"));
    assert!(removed.has_field("stats_after", "[(11, 0), (12, 0)]"));

    let cast = event(&logs, "owner_ability_applied").expect("owner_ability_applied");
    assert_owner_identity(&cast);
    assert!(cast.has_field("entity_id", &OWNER.to_string()));
    assert!(cast.has_field("pet_id", &pet.to_string()));
}

/// No pet is ordinary play (a client can press at will): DEBUG with
/// `reason = no_pet`, `stage = launch`. Fails when the refusal is silent.
#[tokio::test]
async fn a_refusal_is_debug_with_reason() {
    let logs = LogCapture::install();
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(256);
    mgr.get_entity_mut(pet).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
    assert!(!handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);

    let c = event(&logs, "owner_ability_refused").expect("owner_ability_refused");
    assert_eq!(c.level, Level::DEBUG);
    assert!(c.has_field("reason", "pet_dead"));
    assert!(c.has_field("stage", "launch"));
    assert!(c.has_field("error_code", "14"));
    assert_owner_identity(&c);
}

/// A reused owner id is a server-side window no client controls: WARN with
/// `reason = owner_identity_mismatch`.
#[tokio::test]
async fn an_identity_mismatch_is_a_warn() {
    let logs = LogCapture::install();
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(256);
    mgr.pets
        .register(OWNER, pet, PlayerIdentity::new(Some(9_999), Some(9_999)));
    assert!(!handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);

    let c = event(&logs, "owner_ability_refused").expect("owner_ability_refused");
    assert_eq!(c.level, Level::WARN);
    assert!(c.has_field("reason", "owner_identity_mismatch"));
}

/// To The Death: `doom_armed` at the fire, then `buff_removed reason=expired`
/// and an INFO `doom_fired` (`xp_granted = 0`, `killed = true`) when the
/// timer runs out.
#[tokio::test]
async fn doom_rows_trace_the_pets_death() {
    let logs = LogCapture::install();
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(512);
    let cast_at = std::time::Instant::now();
    assert!(handle_use_ability(OWNER, TO_THE_DEATH, 0, &tx, &mut mgr).await);
    let _ = resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let armed = event(&logs, "doom_armed").expect("doom_armed");
    assert_owner_identity(&armed);
    assert!(armed.has_field("pet_id", &pet.to_string()));

    let _ = owner_pet_tick_at(cast_at + Duration::from_secs(63), &tx, &mut mgr).await;
    let expired = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "buff_removed") && c.has_field("reason", "expired"))
        .expect("buff_removed reason=expired");
    assert!(expired.has_field("effect_id", &E_DEATH_ACCURACY.to_string()));
    let fired = event(&logs, "doom_fired").expect("doom_fired");
    assert_eq!(fired.level, Level::INFO);
    assert_owner_identity(&fired);
    assert!(fired.has_field("killed", "true"));
    assert!(fired.has_field("xp_granted", "0"));
    assert!(fired.has_field("decision_outcome", "pet_killed"));
}
