//! Tests for the per-actor inbound Mercury seq. Each test uses its own actor
//! id (9221 to 9226) because the map is process-global.

use super::inbound::{note_inbound_at, take_inbound, INBOUND_MAX_AGE};
use super::InteractionKind;
use cimmeria_wire::cell::cell_methods::{inventory, player};
use std::time::{Duration, Instant};

/// Pins that a noted `interact` seq is taken once, and a second take finds nothing.
#[test]
fn interact_seq_is_taken_once() {
    let t0 = Instant::now();
    note_inbound_at(9221, player::INTERACT, Some(555), t0);
    let later = t0 + Duration::from_millis(10);
    assert_eq!(
        take_inbound(9221, InteractionKind::Interact, later),
        Some(555)
    );
    assert_eq!(take_inbound(9221, InteractionKind::Interact, later), None);
}

/// Pins that an `interact` note never answers a `useItem` take, and the reverse.
#[test]
fn kinds_do_not_cross() {
    let t0 = Instant::now();
    note_inbound_at(9222, inventory::USE_ITEM, Some(7), t0);
    assert_eq!(take_inbound(9222, InteractionKind::Interact, t0), None);
    assert_eq!(take_inbound(9222, InteractionKind::UseItem, t0), Some(7));
    note_inbound_at(9222, player::INTERACT, Some(8), t0);
    assert_eq!(take_inbound(9222, InteractionKind::UseItem, t0), None);
    assert_eq!(take_inbound(9222, InteractionKind::Interact, t0), Some(8));
}

/// Pins that a seq older than the max age is not taken.
#[test]
fn old_seq_expires() {
    let t0 = Instant::now();
    note_inbound_at(9223, player::INTERACT, Some(9), t0);
    let late = t0 + INBOUND_MAX_AGE + Duration::from_millis(1);
    assert_eq!(take_inbound(9223, InteractionKind::Interact, late), None);
}

/// Pins that methods other than the two interaction calls, and a `None` seq, leave nothing to take.
#[test]
fn other_methods_and_none_are_ignored() {
    let t0 = Instant::now();
    note_inbound_at(9224, 0, Some(1), t0);
    note_inbound_at(9224, player::INTERACT, None, t0);
    assert_eq!(take_inbound(9224, InteractionKind::Interact, t0), None);
    assert_eq!(take_inbound(9224, InteractionKind::UseItem, t0), None);
}

/// Pins that a newer call of the same kind overwrites an older one.
#[test]
fn newer_call_wins() {
    let t0 = Instant::now();
    note_inbound_at(9225, player::INTERACT, Some(1), t0);
    note_inbound_at(
        9225,
        player::INTERACT,
        Some(2),
        t0 + Duration::from_millis(1),
    );
    assert_eq!(
        take_inbound(
            9225,
            InteractionKind::Interact,
            t0 + Duration::from_millis(2)
        ),
        Some(2)
    );
}
