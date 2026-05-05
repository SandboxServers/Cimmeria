//! `regen_tick` out-of-combat health + focus regen coverage. Issue #208.
//!
//! Three load-bearing guards here:
//!
//! 1. `regen_tick_blocked_while_threatened` — players with non-empty
//!    `threatened_mobs` must not regen. Inverse: combat pacing collapses.
//! 2. `regen_tick_runs_when_bsf_in_combat_is_stuck` — pins the actual
//!    user-visible bug (#208 in production): `BSF_IN_COMBAT` sticks after
//!    one-shot kills / reload / no-target self-casts because the bit's
//!    setters bypass `threatened_mobs`. The gate must rely on
//!    `threatened_mobs` (the source of aggro truth), not the broken bit,
//!    or every player ends up "in combat" forever and never regens.
//! 3. `regen_tick_bundles_health_and_focus_into_one_stat_update` — both
//!    pools must land in a single `onStatUpdate` payload. Splitting them
//!    into two messages would double the wire traffic for every regen
//!    tick across every connected player.

use super::make_test_space_mgr;
use crate::cell::combat::state::{BSF_DEAD, BSF_IN_COMBAT};
use crate::cell::messages::CellToBaseMsg;
use cimmeria_entity::stats::{FOCUS, FOCUS_REGEN, HEALTH, HEALTH_REGEN};
use tokio::sync::mpsc;

/// Build a connected player at `health=cur/max` with no in-combat flag.
/// `regen` seeds `HEALTH_REGEN.cur` (the per-second tick amount). Focus
/// stays at the StatList default (0/0/0) so HP-only tests don't pick up
/// spurious focus regen — focus tests use [`set_focus`] to opt in.
fn setup_player(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    entity_id: u32,
    cur: i32,
    max: i32,
    regen: i32,
) {
    mgr.create_entity(entity_id, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.is_player = true;
        e.player_id = Some(100);
        if let Some(h) = e.stats.get_mut(HEALTH) {
            h.update(0, cur, max);
        }
        if let Some(r) = e.stats.get_mut(HEALTH_REGEN) {
            r.update(0, regen, regen.max(0));
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(entity_id);
}

/// Seed a connected player's focus pool + regen. Mirrors [`setup_player`]
/// for focus — kept separate so the HP-only tests don't accidentally
/// exercise the focus path through the same fixture.
fn set_focus(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    entity_id: u32,
    cur: i32,
    max: i32,
    regen: i32,
) {
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        if let Some(f) = e.stats.get_mut(FOCUS) {
            f.update(0, cur, max);
        }
        if let Some(r) = e.stats.get_mut(FOCUS_REGEN) {
            r.update(0, regen, regen.max(0));
        }
        e.stats.clear_dirty();
    }
}

/// Regression guard: a player with any NPC on `threatened_mobs` must not
/// regen — that's the source of aggro truth. Without this gate, players
/// would heal mid-fight while NPCs are actively engaged, and combat
/// pacing collapses.
#[tokio::test]
async fn regen_tick_blocked_while_threatened() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 5);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.threatened_mobs.insert(2001); // arbitrary NPC id
        e.state_field |= BSF_IN_COMBAT; // realistic — set together
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        50,
        "health must not change while threatened_mobs is non-empty"
    );
    assert!(
        rx.try_recv().is_err(),
        "no onStatUpdate must be sent for a threatened player"
    );
}

/// Regression guard for the production #208 bug: `BSF_IN_COMBAT` is
/// stuck-set in three real code paths (one-shot kills, reload in
/// isolation, no-target self-casts) because the bit's setters bypass
/// `threatened_mobs`. If regen ever switches back to gating on the bit,
/// every player ends up "permanently in combat" and the user-visible
/// "no HP recovery between fights" bug returns. Pinning the inverse
/// here — bit set, threat set empty → regen MUST run — is what makes
/// the gate-source choice load-bearing.
#[tokio::test]
async fn regen_tick_runs_when_bsf_in_combat_is_stuck() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 5);
    if let Some(e) = mgr.get_entity_mut(1) {
        // Stuck flag, empty threat set — the exact production state after
        // a one-shot guard kill in Castle CellBlock.
        e.state_field |= BSF_IN_COMBAT;
        assert!(e.threatened_mobs.is_empty());
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        55,
        "regen MUST run despite stuck BSF_IN_COMBAT — gate uses threatened_mobs"
    );
    assert!(
        rx.try_recv().is_ok(),
        "onStatUpdate sent even when BSF_IN_COMBAT is stuck-set"
    );
}

/// Out-of-combat happy path: HP advances by `HEALTH_REGEN.cur` and an
/// onStatUpdate carrying the new HEALTH lands on the wire.
#[tokio::test]
async fn regen_tick_advances_health_out_of_combat() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 7);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        57,
        "health.cur must advance by HEALTH_REGEN (7) on a 1Hz tick"
    );

    let msg = rx.try_recv().expect("expected onStatUpdate");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, 20, "ON_STAT_UPDATE");
            // First i32 is count; locate the HEALTH entry by stat_id.
            let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
            let mut found = false;
            for i in 0..count {
                let off = 4 + i * 16;
                let stat_id =
                    i32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]);
                if stat_id == HEALTH {
                    let cur = i32::from_le_bytes([
                        args[off + 8],
                        args[off + 9],
                        args[off + 10],
                        args[off + 11],
                    ]);
                    assert_eq!(cur, 57, "wire payload carries the regenerated HEALTH.cur");
                    found = true;
                }
            }
            assert!(found, "onStatUpdate payload must include HEALTH");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// `HEALTH_REGEN.cur == 0` (default for unscaled archetypes) must still
/// regen at 1 HP/sec — the floor exists so newly-rolled players don't
/// hard-stall after their first fight.
#[tokio::test]
async fn regen_tick_floor_when_health_regen_is_zero() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 0);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        51,
        "floor of 1 HP/sec applies when HEALTH_REGEN is 0"
    );
    assert!(
        rx.try_recv().is_ok(),
        "onStatUpdate sent for the floor case"
    );
}

/// Full-HP players must produce no wire traffic — the gate exists to
/// suppress per-second `onStatUpdate` spam to the entire roster.
#[tokio::test]
async fn regen_tick_skips_full_hp_players() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 100, 100, 5);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        100,
        "health stays at max"
    );
    assert!(
        rx.try_recv().is_err(),
        "no onStatUpdate for a full-HP player"
    );
}

/// Dead players must not regen — `BSF_DEAD` mirrors the in-combat gate.
/// Without it, a corpse would tick back to full HP between death and
/// respawn and the death state would be visually contradictory.
#[tokio::test]
async fn regen_tick_blocked_while_dead() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 0, 100, 5);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.state_field |= BSF_DEAD;
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        0,
        "dead player must not regen"
    );
    assert!(rx.try_recv().is_err(), "no onStatUpdate for a dead player");
}

/// Regen must clamp at `health.max` — overshoot would leak through the
/// `serialize_dirty` payload as `cur > max`, violating the wire invariant
/// the client UI assumes.
#[tokio::test]
async fn regen_tick_clamps_to_max() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 98, 100, 50);

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(h.cur, 100, "regen clamps to max even with a large delta");
    assert!(h.cur <= h.max, "wire invariant cur <= max preserved");
}

/// Focus pool advances on its own when HP is full but focus is below
/// max. Mirrors the human-archetype shape: `focus = 1570` from
/// `archetypes.sql` plus a `FocusDamage` effect dropped it below the
/// cap. The tick must pull it back without requiring HP to also be
/// dirty.
#[tokio::test]
async fn regen_tick_advances_focus_when_health_is_full() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 100, 100, 5);
    set_focus(&mut mgr, 1, 800, 1570, 12);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.stats.get(HEALTH).unwrap().cur,
        100,
        "HP at max must not change"
    );
    assert_eq!(
        entity.stats.get(FOCUS).unwrap().cur,
        812,
        "focus advances by FOCUS_REGEN (12) even with HP full"
    );
    assert!(
        rx.try_recv().is_ok(),
        "onStatUpdate fires when only focus changed"
    );
}

/// Both pools below max must land in **one** `onStatUpdate` payload —
/// not two. Splitting would double regen wire traffic across every
/// connected player. Pins the bundling guarantee.
#[tokio::test]
async fn regen_tick_bundles_health_and_focus_into_one_stat_update() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 7);
    set_focus(&mut mgr, 1, 500, 1570, 11);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(entity.stats.get(HEALTH).unwrap().cur, 57);
    assert_eq!(entity.stats.get(FOCUS).unwrap().cur, 511);

    // Pull exactly one message; assert no second message lands on the
    // channel. The pool entries must both be inside this single payload.
    let msg = rx.try_recv().expect("expected onStatUpdate");
    assert!(
        rx.try_recv().is_err(),
        "regen must emit exactly one onStatUpdate carrying both pools, not one per pool"
    );
    match msg {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(method_index, 20, "ON_STAT_UPDATE");
            let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
            let mut found_health = false;
            let mut found_focus = false;
            for i in 0..count {
                let off = 4 + i * 16;
                let stat_id =
                    i32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]);
                let cur = i32::from_le_bytes([
                    args[off + 8],
                    args[off + 9],
                    args[off + 10],
                    args[off + 11],
                ]);
                if stat_id == HEALTH {
                    assert_eq!(cur, 57);
                    found_health = true;
                } else if stat_id == FOCUS {
                    assert_eq!(cur, 511);
                    found_focus = true;
                }
            }
            assert!(
                found_health && found_focus,
                "single payload must contain BOTH HEALTH and FOCUS entries"
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// Focus pool with `max = 0` (the StatList default for non-human or
/// pre-archetype entities) must be skipped entirely — the floor-of-1
/// only applies to pools the entity actually owns. Without the
/// `pool.max > 0` gate, every NPC and freshly-instantiated player would
/// tick focus from 0 → 1 every second and emit useless wire traffic.
#[tokio::test]
async fn regen_tick_skips_focus_when_max_is_zero() {
    let mut mgr = make_test_space_mgr();
    setup_player(&mut mgr, 1, 50, 100, 5);
    // setup_player leaves focus at the StatList default (0/0/0).
    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.stats.get(FOCUS).unwrap().cur,
        0,
        "focus with max=0 must not be touched by the floor"
    );
    assert_eq!(
        entity.stats.get(FOCUS).unwrap().max,
        0,
        "focus.max must remain 0"
    );
    // HP regen still ran — that's covered by other tests; this one only
    // pins the focus-skip invariant.
}
