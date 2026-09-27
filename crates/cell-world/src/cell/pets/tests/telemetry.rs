//! `LogCapture` guards (TESTING.md type 12) for the `pets.lifecycle` and
//! `pets.command` rows: each transition carries `event` and the owner's
//! identity, and each refusal seam carries `reason`.
//!
//! The fixture owner has `account_id = OWNER` and `player_id = OWNER + 1000`
//! (`add_pet_owner`). Tests are `#[tokio::test]` (current-thread) because
//! `LogCapture` installs a thread-local subscriber.

use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::cell::pets::{despawn_pet, pet_owner_sweep, PetDespawnReason};
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const OWNER_ACCOUNT: &str = "7";
const OWNER_PLAYER: &str = "1007";

/// The first event on `target` whose `event` field is `name`.
fn event(logs: &LogCaptureGuard, target: &str, name: &str) -> Option<Captured> {
    logs.all()
        .into_iter()
        .find(|c| c.target == target && c.has_field("event", name))
}

fn assert_owner_identity(c: &Captured) {
    assert!(
        c.has_field("account_id", OWNER_ACCOUNT) && c.has_field("player_id", OWNER_PLAYER),
        "owner identity missing: {c:#?}"
    );
}

#[tokio::test]
async fn summon_is_an_info_event_with_owner_identity_and_template() {
    let logs = LogCapture::install();
    let (_mgr, pet) = world_with_pet();
    let c = event(&logs, "pets.lifecycle", "summoned").expect("summoned event");
    assert_eq!(c.level, Level::INFO);
    assert_owner_identity(&c);
    assert!(c.has_field("pet_id", &pet.to_string()));
    // Rule 5 correlator: a lifecycle row's entity_id is the pet.
    assert!(c.has_field("entity_id", &pet.to_string()));
    assert!(c.has_field("owner_id", &OWNER.to_string()));
    assert!(c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()));
    assert!(c.has_field("ability_id", "1643"));
}

#[tokio::test]
async fn unknown_template_warns_with_reason() {
    let logs = LogCapture::install();
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [0.0; 3], 5);
    let _ = mgr.spawn_pet_from_template(OWNER, 9999, 0);
    let c = logs
        .find_event(Level::WARN, "pet summon failed", "unknown_template")
        .expect("summon_failed WARN with reason = unknown_template");
    assert!(c.has_field("event", "summon_failed"));
    assert!(c.has_field("template_id", "9999"));
    assert_owner_identity(&c);
}

#[tokio::test]
async fn ownership_rejections_log_their_reason_at_debug() {
    let logs = LogCapture::install();
    let (mut mgr, pet) = world_with_pet();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [1.0, 0.0, 1.0], 5);
    let owner_identity = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, 999_999, owner_identity);

    let _ = mgr.owned_pet(OTHER, pet);
    let _ = mgr.owned_pet(OWNER, OTHER);
    let _ = mgr.owned_pet(OWNER, 999_999);
    for reason in ["not_owner", "not_a_pet", "pet_gone"] {
        let c = logs
            .find_event(Level::DEBUG, "does not own", reason)
            .unwrap_or_else(|| panic!("ownership_rejected reason={reason}"));
        assert_eq!(c.target, "pets.command");
        assert!(c.has_field("event", "ownership_rejected"));
    }
    let not_owner = logs
        .find_event(Level::DEBUG, "does not own", "not_owner")
        .unwrap();
    assert!(not_owner.has_field("owner_id", &OWNER.to_string()));
    assert!(not_owner.has_field("account_id", &OTHER.to_string()));
    // A command row's entity_id is the caller.
    assert!(not_owner.has_field("entity_id", &OTHER.to_string()));
    // Success is not logged here.
    let before = logs.all().len();
    assert_eq!(mgr.owned_pet(OWNER, pet), Ok(pet));
    assert_eq!(logs.all().len(), before);
}

/// The key identity guard: the owner is destroyed before the sweep runs, so
/// only the identity captured at summon can name the player.
#[tokio::test]
async fn swept_despawn_of_a_gone_owner_still_names_the_owner() {
    let (mut mgr, pet) = world_with_pet();
    let (tx, _rx) = mpsc::channel(64);
    mgr.destroy_entity(OWNER);
    let logs = LogCapture::install();
    pet_owner_sweep(&tx, &mut mgr).await;
    let c = event(&logs, "pets.lifecycle", "despawned").expect("despawned event");
    assert_eq!(c.level, Level::INFO);
    assert!(c.has_field("reason", "owner_gone"));
    assert!(c.has_field("path", "sweep"));
    assert_owner_identity(&c);
    assert!(c.has_field("pet_id", &pet.to_string()));
    assert!(c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()));
}

/// The reused-id intro refusal is a WARN naming the summoner (Rule 5) and
/// the id's new holder; the ordinary non-owner witness logs nothing.
#[tokio::test]
async fn reused_owner_id_intro_warns_with_both_identities() {
    let (mut mgr, pet) = world_with_pet();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    super::reuse_owner_id_by_another_player(&mut mgr);
    let logs = LogCapture::install();
    let _ = mgr.compute_aoi_changes();
    let refused: Vec<Captured> = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "pet_list_replay_refused"))
        .collect();
    assert_eq!(
        refused.len(),
        1,
        "one refusal, none for OTHER: {refused:#?}"
    );
    let c = &refused[0];
    assert_eq!(c.level, Level::WARN);
    assert_eq!(c.target, "pets.lifecycle");
    assert!(c.has_field("reason", "owner_identity_mismatch"));
    assert_owner_identity(c);
    assert!(c.has_field("witness_account_id", "4242"));
    assert!(c.has_field("witness_player_id", "4243"));
    assert!(c.has_field("entity_id", &pet.to_string()));
    assert!(c.has_field("witness_id", &OWNER.to_string()));
}

/// Kill credit from a pet whose owner id was reused is withheld with a WARN.
#[tokio::test]
async fn reused_owner_id_credit_refusal_warns() {
    let (mut mgr, pet) = world_with_pet();
    super::reuse_owner_id_by_another_player(&mut mgr);
    let logs = LogCapture::install();
    assert_eq!(mgr.credit_recipient(pet), None);
    let c = event(&logs, "pets.credit", "credit_refused").expect("credit_refused WARN");
    assert_eq!(c.level, Level::WARN);
    assert!(c.has_field("reason", "owner_identity_mismatch"));
    assert_owner_identity(&c);
    assert!(c.has_field("holder_player_id", "4243"));
    assert!(c.has_field("pet_id", &pet.to_string()));
}

#[tokio::test]
async fn disconnect_logs_despawn_and_owner_forgotten() {
    let (mut mgr, pet) = world_with_pet();
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();
    mgr.disconnect_entity(OWNER, &tx).await;
    let d = event(&logs, "pets.lifecycle", "despawned").expect("despawned event");
    assert!(d.has_field("reason", "owner_disconnected"));
    assert!(d.has_field("path", "disconnect"));
    assert!(d.has_field("pet_id", &pet.to_string()));
    assert_owner_identity(&d);
    let f = event(&logs, "pets.lifecycle", "owner_forgotten").expect("owner_forgotten");
    assert_eq!(f.level, Level::DEBUG);
    assert!(f.has_field("pet_count", "1"));
    assert!(f.has_field("despawned", "1"));
    assert_owner_identity(&f);
}

#[tokio::test]
async fn orphan_registry_entry_is_scrubbed_with_a_warn() {
    let (mut mgr, _pet) = world_with_pet();
    let (tx, _rx) = mpsc::channel(64);
    let owner_identity = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, 999_999, owner_identity);
    let logs = LogCapture::install();
    pet_owner_sweep(&tx, &mut mgr).await;
    let c = logs
        .find_event(Level::WARN, "without an entity", "pet_entity_gone")
        .expect("registry_scrubbed WARN");
    assert!(c.has_field("event", "registry_scrubbed"));
    assert!(c.has_field("path", "sweep"));
    assert!(c.has_field("pet_id", "999999"));
    assert_owner_identity(&c);
}

#[tokio::test]
async fn despawning_a_missing_pet_warns_with_reason() {
    let (mut mgr, _pet) = world_with_pet();
    let (tx, _rx) = mpsc::channel(64);
    let owner_identity = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, 999_999, owner_identity);
    let logs = LogCapture::install();
    let _ = despawn_pet(&mut mgr, 999_999, PetDespawnReason::Dismissed, &tx).await;
    let c = logs
        .find_event(Level::WARN, "did not remove", "dismissed")
        .expect("despawn_failed WARN");
    assert!(c.has_field("event", "despawn_failed"));
    assert!(c.has_field("outcome", "NotFound"));
    assert!(c.has_field("path", "direct"));
    assert_owner_identity(&c);
}
