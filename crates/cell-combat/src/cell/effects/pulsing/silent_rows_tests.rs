//! AB-T2: the pulse registration and ledger removal paths that returned
//! nothing without a row.

use std::time::Instant;

use cimmeria_cell_world::cell::effects::stat_buff::StatBuffRemoval;
use tokio::sync::mpsc;
use tracing::Level;

use super::register_active_effect;
use super::tests::{make_dot_effect, make_mgr};
use crate::test_support::{Captured, LogCapture};

fn row(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no `{event}` row in {all:#?}"))
}

/// **Regression guard (AB-T2, pulse family).** A DoT whose target left
/// between its first pulse and the registration returned `false` silently,
/// and the hit pipeline discarded the bool. The refusal now logs
/// `active_effect_not_registered` with its reason and the cast's id.
#[tokio::test]
async fn a_dot_on_a_departed_target_logs_why_it_will_not_tick() {
    let mut mgr = make_mgr();
    let dot = make_dot_effect(5, 1.0, 3);
    let (tx, _rx) = mpsc::channel(16);
    let outer = mgr.enter_cast_scope(Some(31));
    let logs = LogCapture::install();

    let registered = register_active_effect(&mut mgr, 404, 1, &dot, Instant::now(), &tx).await;

    mgr.exit_cast_scope(outer);
    assert!(!registered);
    let r = row(&logs.all(), "active_effect_not_registered");
    assert_eq!(r.target, "abilities.pulse");
    assert_eq!(r.level, Level::DEBUG);
    for (k, v) in [
        ("reason", "target_gone"),
        ("cast_id", "31"),
        ("player_id", "100"),
        ("effect_id", "7777"),
    ] {
        assert!(r.has_field(k, v), "{k} = {v}: {r:?}");
    }
}

/// **Regression guard (AB-T2, ledger family).** A removal aimed at an entity
/// that is gone returned an empty list with no row.
#[test]
fn a_ledger_removal_on_a_missing_entity_logs_it() {
    let mut mgr = make_mgr();
    let logs = LogCapture::install();

    let removed = mgr.remove_timed_effects(404, StatBuffRemoval::Cleansed, |_| true);

    assert!(removed.is_empty());
    let r = row(&logs.all(), "stat_buff_remove_nothing");
    assert_eq!(r.target, "abilities.ledger");
    assert_eq!(r.level, Level::DEBUG, "a directed removal is DEBUG");
    assert!(r.has_field("reason", "target_gone"), "{r:?}");
}

/// The sweep removals ask every entity and usually match nothing: TRACE, so
/// the per-tick expiry sweep cannot flood DEBUG.
#[test]
fn a_sweep_that_matches_nothing_stays_at_trace() {
    let mut mgr = make_mgr();
    let logs = LogCapture::install();

    mgr.remove_timed_effects(1, StatBuffRemoval::Expired, |_| true);

    let r = row(&logs.all(), "stat_buff_remove_nothing");
    assert_eq!(r.level, Level::TRACE, "{r:?}");
    assert!(r.has_field("reason", "no_match"), "{r:?}");
}
