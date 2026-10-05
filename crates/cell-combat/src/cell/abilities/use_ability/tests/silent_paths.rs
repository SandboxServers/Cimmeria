//! AB-T2: the launch, warmup and fire paths that used to return without a
//! row. Each guard drives one quiet exit and asserts the row it now logs;
//! deleting the row (the pre-AB-T2 `return false`) fails the find.

use std::time::Instant;

use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::{Captured, LogCapture, NoContentEvents};

const ABILITY: i32 = 9101;

/// Player 1 (player_id 101, account 901) who knows [`ABILITY`].
fn caster_mgr() -> SpaceManager {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.ability_defs
        .insert(ABILITY, make_ability(ABILITY, 0, 10));
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .add_ability(ABILITY);
    mgr
}

fn row(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no `{event}` row in {all:#?}"))
}

/// **Regression guard (AB-T2, gate family).** A dead caster's press used to
/// return `false` with no row at all. It now logs `use_ability_caster_dead`
/// under `abilities` with the player's ids and the ability.
#[tokio::test]
async fn a_dead_casters_press_logs_why_it_was_dropped() {
    let mut mgr = caster_mgr();
    mgr.get_entity_mut(1).unwrap().state_field |= crate::cell::combat::state::BSF_DEAD;
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    assert!(!handle_use_ability(1, ABILITY, 0, &tx, &mut mgr).await);

    let r = row(&logs.all(), "use_ability_caster_dead");
    assert_eq!(r.target, "abilities");
    assert_eq!(r.level, tracing::Level::DEBUG);
    for (k, v) in [
        ("stage", "gate"),
        ("reason", "caster_dead"),
        ("account_id", "901"),
        ("player_id", "101"),
        ("caster_kind", "player"),
        ("ability_id", "9101"),
    ] {
        assert!(r.has_field(k, v), "{k} = {v}: {r:?}");
    }
}

/// **Regression guard (AB-T2, gate family).** The ammo refusal moved from
/// the module path to `abilities` with a stable event and numeric fields.
#[tokio::test]
async fn an_empty_magazine_logs_the_rounds_it_needed() {
    let mut mgr = caster_mgr();
    mgr.ability_defs
        .insert(ABILITY, make_ability(ABILITY, 3, 10));
    // Drawn, so the holstered-draw queue does not take the shot first.
    mgr.get_entity_mut(1).unwrap().weapon_holstered = false;
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    assert!(!handle_use_ability(1, ABILITY, 0, &tx, &mut mgr).await);

    let r = row(&logs.all(), "use_ability_no_ammo");
    assert_eq!(r.target, "abilities");
    assert!(r.has_field("ammo_required", "3"), "{r:?}");
    assert!(r.has_field("ammo_current", "0"), "{r:?}");
    assert!(r.has_field("player_id", "101"), "{r:?}");
}

/// **Regression guard (AB-T2, warmup family).** A caster left in
/// `pending_casts` with no warming cast was dropped by the tick without a
/// word. It now logs `warmup_candidate_stale`.
#[tokio::test]
async fn a_stale_warmup_candidate_is_dropped_with_a_row() {
    let mut mgr = caster_mgr();
    mgr.pending_casts.insert(1);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    let fired = resolve_warmups(Instant::now(), &tx, &mut mgr, &NoContentEvents).await;

    assert_eq!(fired, 0);
    assert!(
        mgr.pending_casts.is_empty(),
        "the stale candidate is dropped"
    );
    let r = row(&logs.all(), "warmup_candidate_stale");
    assert_eq!(r.target, "abilities");
    assert!(r.has_field("reason", "no_pending_cast"), "{r:?}");
    assert!(r.has_field("player_id", "101"), "{r:?}");
}

/// **Regression guard (AB-T2, fire family).** A committed cast with no
/// target skipped the whole target pipeline silently. The fire now logs
/// `fire_target_skipped` with the launch row's `cast_id`.
#[tokio::test]
async fn a_targetless_fire_logs_that_the_target_pipeline_was_skipped() {
    let mut mgr = caster_mgr();
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    assert!(handle_use_ability(1, ABILITY, 0, &tx, &mut mgr).await);

    let all = logs.all();
    let cast_id = row(&all, "ability_launched")
        .fields
        .get("cast_id")
        .cloned()
        .expect("the launch row carries cast_id");
    let r = row(&all, "fire_target_skipped");
    assert_eq!(r.target, "abilities");
    assert!(r.has_field("reason", "no_target"), "{r:?}");
    assert!(r.has_field("cast_id", &cast_id), "{r:?}");
}
