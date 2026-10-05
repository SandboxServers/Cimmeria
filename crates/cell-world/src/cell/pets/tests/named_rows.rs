//! Rule 6 (NT-28b): pet rows name the pet, its template and its owner, and
//! the owner is named from the identity captured at summon, never from
//! whoever holds the owner's entity id now (#889). Type 12 (`LogCapture`).
//!
//! The NameBook is process-global, so each test stores one naming the
//! fixture and puts an empty one back before asserting.

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::cell::pets::{drain_arrivals, pet_owner_sweep, PetArrival};
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

/// The summon VFX sequence the arrival rows carry.
const SEQUENCE: i32 = 2293;

fn store_book() {
    let mut book = cimmeria_names::NameBook::empty();
    book.insert(cimmeria_names::Table::Texts, 8087, "Test Pet");
    book.insert(
        cimmeria_names::Table::Templates,
        PET_FIXTURE_TEMPLATE_ID.into(),
        "NT28_Pet_Template",
    );
    book.insert(
        cimmeria_names::Table::Sequences,
        SEQUENCE.into(),
        "NT28_Summon_Sequence",
    );
    cimmeria_names::global().store(book);
    tracing::callsite::rebuild_interest_cache();
}

/// `OWNER` ("Tealc") summons a pet, then is destroyed and its entity id
/// handed to another player ("Impostor") before the sweep. Returns the
/// pet. The book stays stored; the caller clears it.
fn pet_whose_owner_id_was_reused() -> (SpaceManager, u32) {
    store_book();
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .stamp_log_names(Some("Tealc"), Some("tealc_login"));
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    reuse_owner_id_by_another_player(&mut mgr);
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .stamp_log_names(Some("Impostor"), Some("impostor_login"));
    (mgr, pet)
}

fn event(logs: &LogCaptureGuard, name: &str) -> Captured {
    logs.all()
        .into_iter()
        .find(|c| c.target == "pets.lifecycle" && c.has_field("event", name))
        .unwrap_or_else(|| panic!("{name} row"))
}

/// The pet, its template and the summoner, by name; never the impostor.
fn assert_names_pet_and_summoner(c: &Captured) {
    for (k, v) in [
        ("entity_name", "Test Pet"),
        ("pet_name", "Test Pet"),
        ("template_name", "NT28_Pet_Template"),
        ("owner_name", "Tealc"),
        ("player_name", "Tealc"),
        ("account_name", "tealc_login"),
    ] {
        assert!(c.has_field(k, v), "{k}={v} missing: {c:#?}");
    }
    for v in ["Impostor", "impostor_login"] {
        assert!(
            !c.fields.values().any(|f| f == v),
            "the id's new holder is named: {c:#?}"
        );
    }
}

/// Teardown: the sweep despawns the pet of the reused owner id. The
/// `despawned` row names the pet and the summoner captured at summon.
/// Fails with `owner_name` (or `pet_name`) removed from `despawn_pet_via`,
/// or if the owner were named from the live holder of the id.
#[tokio::test]
async fn swept_despawn_names_the_pet_and_its_summoner_not_the_ids_new_holder() {
    let (mut mgr, pet) = pet_whose_owner_id_was_reused();
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();
    pet_owner_sweep(&tx, &mut mgr).await;
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());

    let c = event(&logs, "despawned");
    assert!(c.has_field("pet_id", &pet.to_string()), "{c:#?}");
    assert!(c.has_field("reason", "owner_gone"), "{c:#?}");
    assert_names_pet_and_summoner(&c);
}

/// Arrival: the queued summon VFX for that pet is withheld from the id's
/// new holder. The WARN names the pet, its template, the summon sequence
/// and the summoner. Fails with `owner_name`, `pet_name` or
/// `sequence_name` removed from the drop row.
#[tokio::test]
async fn arrival_drop_names_the_pet_the_sequence_and_the_summoner() {
    let (mut mgr, pet) = pet_whose_owner_id_was_reused();
    mgr.pets.queue_arrival(
        pet,
        PetArrival::new(OWNER, PET_FIXTURE_TEMPLATE_ID, SEQUENCE, vec![1, 2, 3]),
    );
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();
    drain_arrivals(Instant::now(), &tx, &mut mgr).await;
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());

    let c = event(&logs, "arrival_vfx_dropped");
    assert_eq!(c.level, Level::WARN);
    assert!(c.has_field("reason", "owner_identity_mismatch"), "{c:#?}");
    assert!(
        c.has_field("sequence_name", "NT28_Summon_Sequence"),
        "{c:#?}"
    );
    assert_names_pet_and_summoner(&c);
}
