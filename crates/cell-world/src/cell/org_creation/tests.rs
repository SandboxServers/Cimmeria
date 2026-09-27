//! The pending-creation state machine, with exact instants.

use std::time::{Duration, Instant};

use cimmeria_entity::organization::OrgType;

use super::*;

const P: i32 = 7;
const NPC: u32 = 900;
const SPACE: u32 = 3;

fn opened(now: Instant) -> PendingCreations {
    let mut s = PendingCreations::new();
    assert_eq!(
        s.open(P, OrgType::Team, NPC, SPACE, now),
        Ok(Opened::Created)
    );
    s
}

#[test]
fn a_name_needs_an_offer_and_takes_its_type_from_it() {
    let now = Instant::now();
    let mut s = PendingCreations::new();
    assert_eq!(
        s.begin_attempt(P, SPACE, now).map_err(|e| e.0),
        Err(TakeMiss::NoPending)
    );
    s.open(P, OrgType::Command, NPC, SPACE, now).unwrap();
    assert_eq!(s.begin_attempt(P, SPACE, now), Ok(OrgType::Command));
}

#[test]
fn one_name_in_flight_at_a_time() {
    let now = Instant::now();
    let mut s = opened(now);
    s.begin_attempt(P, SPACE, now).unwrap();
    assert_eq!(
        s.begin_attempt(P, SPACE, now).map_err(|e| e.0),
        Err(TakeMiss::InFlight)
    );
    assert_eq!(s.charge_attempt(P), Some(PENDING_CREATION_ATTEMPTS - 1));
    assert!(
        s.begin_attempt(P, SPACE, now).is_ok(),
        "answered, so free again"
    );
}

/// Three refusals spend the budget; a re-open inside the window neither
/// refills it nor hides it, so the registrar cannot be clicked around the
/// cap. After the window a new offer has a full budget.
#[test]
fn three_refusals_exhaust_the_window_and_reopening_does_not_refill_it() {
    let now = Instant::now();
    let mut s = opened(now);
    for left in (0..PENDING_CREATION_ATTEMPTS).rev() {
        s.begin_attempt(P, SPACE, now).unwrap();
        assert_eq!(s.charge_attempt(P), Some(left));
    }
    assert_eq!(
        s.begin_attempt(P, SPACE, now).map_err(|e| e.0),
        Err(TakeMiss::Exhausted)
    );
    assert_eq!(
        s.open(P, OrgType::Team, NPC, SPACE, now),
        Err(OpenReject::Exhausted)
    );

    let later = now + PENDING_CREATION_TTL;
    assert_eq!(
        s.open(P, OrgType::Team, NPC, SPACE, later),
        Ok(Opened::Created)
    );
    assert_eq!(s.get(P).unwrap().attempts_left, PENDING_CREATION_ATTEMPTS);
}

#[test]
fn reopening_keeps_budget_and_expiry_but_takes_the_new_type() {
    let now = Instant::now();
    let mut s = opened(now);
    s.begin_attempt(P, SPACE, now).unwrap();
    s.charge_attempt(P);
    let then = now + Duration::from_secs(60);
    assert_eq!(
        s.open(P, OrgType::Command, NPC + 1, SPACE, then),
        Ok(Opened::Refreshed)
    );
    let p = s.get(P).unwrap();
    assert_eq!(p.org_type, OrgType::Command);
    assert_eq!(p.npc_entity_id, NPC + 1);
    assert_eq!(p.opened_at, now);
    assert_eq!(p.attempts_left, PENDING_CREATION_ATTEMPTS - 1);
}

#[test]
fn an_expired_offer_is_removed_when_the_name_arrives() {
    let now = Instant::now();
    let mut s = opened(now);
    let just_before = now + PENDING_CREATION_TTL - Duration::from_millis(1);
    assert!(s.begin_attempt(P, SPACE, just_before).is_ok());
    s.charge_attempt(P);
    let (miss, gone) = s
        .begin_attempt(P, SPACE, now + PENDING_CREATION_TTL)
        .unwrap_err();
    assert_eq!(miss, TakeMiss::Expired);
    assert_eq!(gone.map(|p| p.npc_entity_id), Some(NPC));
    assert!(s.is_empty());
}

/// Gate travel re-creates the player in another space; the offer does not
/// follow them.
#[test]
fn a_change_of_space_ends_the_offer() {
    let now = Instant::now();
    let mut s = opened(now);
    let (miss, _) = s.begin_attempt(P, SPACE + 1, now).unwrap_err();
    assert_eq!(miss, TakeMiss::SpaceChanged);
    assert_eq!(miss.reason(), "pending_expired");
    assert!(s.get(P).is_none());
}

#[test]
fn consume_and_clear_close_the_offer() {
    let now = Instant::now();
    let mut s = opened(now);
    assert!(s.consume(P).is_some());
    assert_eq!(s.charge_attempt(P), None, "a late refusal finds nothing");
    let mut s = opened(now);
    assert!(s.clear(P).is_some());
    assert!(s.is_empty());
}

#[test]
fn miss_reasons_are_the_catalogued_labels() {
    assert_eq!(TakeMiss::NoPending.reason(), "no_pending_creation");
    assert_eq!(TakeMiss::Expired.reason(), "pending_expired");
    assert_eq!(TakeMiss::Exhausted.reason(), "rate_limited");
    assert_eq!(TakeMiss::InFlight.reason(), "creation_in_flight");
}
