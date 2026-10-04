//! A beneficial cast's telemetry joins on `cast_id` (colo smoke test,
//! 2026-10-04, build 7e7ba5779, Heal Focus 597, cast 1).
//!
//! That cast's server rows had three holes: both `beneficial_cast` rows
//! (launch and fire) and the `effect_routed` row carried no `cast_id`, and
//! the `onStatUpdate` fan-out's no-witnesses row carried no `event`. The
//! launch row ran before the launch minted the id; the fire and routing rows
//! simply never read the cast scope. These guards replay the cast (a 2 s
//! warmup Self heal with nothing selected, as on colo) and the instant form.
//!
//! The base half (`client_sent`) cannot carry the id; it carries the payload
//! fields that join it to the cell's `wire_sent` row instead
//! (`base-world-entry`'s `cell_dispatch::method_join`).

use super::beneficial::{heal_mgr, HEAL_FOCUS};
use super::duel_gate::A;
use super::warmup::after_warmup;
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::{Captured, LogCapture, NoContentEvents};

/// The targets the `cast_id` forensics query reads (AB-T7's list).
fn is_ability_target(target: &str) -> bool {
    target == "abilities"
        || target.starts_with("abilities.")
        || target == "vitals"
        || target == "base.entity_method"
}

/// Burn ids so the cast is not the counter's first and a row that logged
/// some other number cannot match by accident.
fn burn_effect_ids(mgr: &mut SpaceManager, skip: usize) {
    let e = mgr.get_entity_mut(A).unwrap();
    for _ in 0..skip {
        e.abilities.next_effect_id();
    }
}

fn launch_cast_id(all: &[Captured]) -> String {
    all.iter()
        .find(|c| c.has_field("event", "ability_launched"))
        .and_then(|c| c.fields.get("cast_id").cloned())
        .unwrap_or_else(|| panic!("no ability_launched cast_id in {all:#?}"))
}

/// Every row named `event` carries `cast_id`, and there is at least one.
fn assert_all_carry(all: &[Captured], event: &str, cast_id: &str) {
    let rows: Vec<_> = all.iter().filter(|c| c.has_field("event", event)).collect();
    assert!(!rows.is_empty(), "no `{event}` row in {all:#?}");
    for r in rows {
        assert_eq!(
            r.fields.get("cast_id").map(String::as_str),
            Some(cast_id),
            "`{event}` must carry the cast's id: {r:?}"
        );
    }
}

/// No row under an ability target lacks `event`: the forensics query and
/// the SigNoz views group by it, and the colo cast had one such row.
fn assert_every_ability_row_has_an_event(all: &[Captured]) {
    let bare: Vec<_> = all
        .iter()
        .filter(|c| is_ability_target(&c.target) && !c.fields.contains_key("event"))
        .collect();
    assert!(bare.is_empty(), "ability rows with no `event`: {bare:#?}");
}

/// **Regression guard.** The colo cast: Heal Focus with a warmup and
/// nothing selected. The launch and fire `beneficial_cast` rows and the
/// `effect_routed` row carry the launch's `cast_id`, and no ability row of
/// the cast lacks `event`. Fails on revert of any of the three: the launch
/// row logged before the mint, the other two never read the cast scope.
#[tokio::test]
async fn a_warmed_up_self_heal_names_its_cast_on_every_row() {
    let mut mgr = heal_mgr(1.5);
    burn_effect_ids(&mut mgr, 3);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, HEAL_FOCUS, 0, &tx, &mut mgr).await);
    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );

    let all = logs.all();
    let cast_id = launch_cast_id(&all);
    assert_eq!(cast_id, "4", "the burned counter's next id");
    assert_all_carry(&all, "beneficial_cast", &cast_id);
    assert_all_carry(&all, "effect_routed", &cast_id);
    assert_all_carry(&all, "beneficial_cast_applied", &cast_id);
    let stages: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("event", "beneficial_cast"))
        .filter_map(|c| c.fields.get("stage").cloned())
        .collect();
    assert_eq!(stages, ["launch", "fire"]);
    assert_every_ability_row_has_an_event(&all);
}

/// The instant form: launch and fire in one pass, the same ids.
#[tokio::test]
async fn an_instant_self_heal_names_its_cast_on_every_row() {
    let mut mgr = heal_mgr(0.0);
    burn_effect_ids(&mut mgr, 6);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, HEAL_FOCUS, A as i32, &tx, &mut mgr).await);

    let all = logs.all();
    let cast_id = launch_cast_id(&all);
    assert_eq!(cast_id, "7");
    assert_all_carry(&all, "beneficial_cast", &cast_id);
    assert_all_carry(&all, "effect_routed", &cast_id);
    assert_every_ability_row_has_an_event(&all);
}
