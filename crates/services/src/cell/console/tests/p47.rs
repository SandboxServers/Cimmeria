//! Packet P47 regression suite: `.speed <value>` — the checked integer
//! current-stat setter that writes `movementSpeedMod` and `rotationSpeedMod`
//! together and publishes the dirty set as an `onStatUpdate`.
//!
//! Filter prefix: `legacy_p47_`.
//!
//! The *tick-level* half of this packet's acceptance criteria (proving the
//! stat actually changes how far an NPC moves, not just what the stat reads)
//! lives in `crate::cell::service::ticks::npc_movement`'s own test module,
//! under the same `legacy_p47_` prefix — `npc_movement_tick` is
//! `pub(in crate::cell::service)` behind two private module declarations, so
//! reaching it from here would mean widening three `mod` declarations in
//! files this packet does not own. `cargo test --lib legacy_p47_` runs both
//! halves.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{MOVEMENT_SPEED_MOD, ROTATION_SPEED_MOD};
use tokio::sync::mpsc;

use super::super::stats;
use super::{decode_feedback, setup};
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `(min, cur, max)` of both speed stats on `eid`, in `SPEED_STATS` order.
/// Asserting the full triple (not just `cur`) is what pins legacy `setSpeed`'s
/// "current only, never max" contract.
fn speed_triples(mgr: &SpaceManager, eid: u32) -> Vec<(i32, i32, i32)> {
    let e = mgr.get_entity(eid).expect("entity must exist");
    [MOVEMENT_SPEED_MOD, ROTATION_SPEED_MOD]
        .iter()
        .map(|id| {
            let s = e.stats.get(*id).expect("speed stat must exist");
            (s.min, s.cur, s.max)
        })
        .collect()
}

/// Decode an `onStatUpdate` payload into `(stat_id, min, cur, max)` entries.
/// Wire shape: `[count: u32 LE][{id, min, cur, max}: 4 × i32 LE]…` — the same
/// `ARRAY<StatUpdate>` layout `mercury/aoi/create.rs` builds.
fn decode_stat_update(args: &[u8]) -> Vec<(i32, i32, i32, i32)> {
    let count = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    (0..count)
        .map(|i| {
            let o = 4 + i * 16;
            let f = |k: usize| i32::from_le_bytes(args[o + k..o + k + 4].try_into().unwrap());
            (f(0), f(4), f(8), f(12))
        })
        .collect()
}

/// Collect every `onStatUpdate` payload addressed to `entity_id`, from either
/// the direct-to-player (`EntityMethodCall`) or the witness-fanout
/// (`WitnessEntityMethod`) branch of `send_entity_method`.
fn drain_stat_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity_id: u32) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let (eid, idx, args) = match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => (entity_id, method_index, args),
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            } => (entity_id, method_index, args),
            _ => continue,
        };
        if eid == entity_id && idx == crate::mercury::method_idx::ON_STAT_UPDATE {
            out.push(args);
        }
    }
    out
}

/// Drain the channel exactly once, retaining both GM feedback lines and
/// `onStatUpdate` payloads addressed to `entity_id`. `drain_feedback` and
/// `drain_stat_updates` each fully drain the receiver on their own — calling
/// both against the same `rx` in one test silently checks an empty receiver
/// the second time (CodeRabbit review of PR #642 caught this: the two
/// rejection tests below asserted `drain_stat_updates(...).is_empty()`
/// *after* `drain_feedback` had already consumed everything, so the
/// assertion could never fail). Use this whenever a test needs to check both.
fn drain_all(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    entity_id: u32,
) -> (Vec<String>, Vec<Vec<u8>>) {
    let mut lines = Vec::new();
    let mut stat_updates = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
            continue;
        }
        let (eid, idx, args) = match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => (entity_id, method_index, args),
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            } => (entity_id, method_index, args),
            _ => continue,
        };
        if eid == entity_id && idx == crate::mercury::method_idx::ON_STAT_UPDATE {
            stat_updates.push(args);
        }
    }
    (lines, stat_updates)
}

/// Drain only the GM-facing feedback lines. Only safe to use in a test that
/// does not also need to inspect `onStatUpdate` traffic on the same `rx` —
/// see [`drain_all`] for that case.
fn drain_feedback(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
    }
    lines
}

/// Happy path: both stats' `cur` becomes the requested value, `min`/`max` are
/// untouched (legacy `setSpeed` calls `setCurrent`, never `setMax`), and the
/// caller gets the exact legacy-prose feedback line.
#[tokio::test]
async fn legacy_p47_speed_sets_both_current_values_and_leaves_bounds_alone() {
    let (mut mgr, gm, npc) = setup();
    assert_eq!(
        speed_triples(&mgr, npc),
        vec![(0, 100, 500), (0, 100, 500)],
        "fixture precondition: both speed stats start at the StatList default"
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("speed", gm, &["250"], Some(npc), &tx, &mut mgr, &engine).await;

    assert_eq!(
        speed_triples(&mgr, npc),
        vec![(0, 250, 500), (0, 250, 500)],
        "both speed stats' cur must be 250 with min/max preserved — \
         legacy setSpeed sets current only"
    );
    assert_eq!(
        drain_feedback(&mut rx),
        vec![format!("Set speed of entity {npc} to 250")],
        "exactly one feedback line, matching legacy's 'Set speed of entity %d to %f' prose"
    );
}

/// The published `onStatUpdate` carries BOTH speed stat ids at the new value —
/// not just `movementSpeedMod`, and not an empty/count-only payload. This is
/// the "sends dirty stats so the client actually sees the change" criterion.
#[tokio::test]
async fn legacy_p47_speed_publishes_on_stat_update_with_both_stat_ids() {
    let (mut mgr, gm, npc) = setup();
    // Make the target a player so `send_entity_method` takes its direct-to-
    // client branch; the NPC witness-fanout branch is covered separately by
    // `legacy_p47_speed_on_npc_target_fans_out_to_witness`.
    mgr.get_entity_mut(npc).unwrap().is_player = true;

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("speed", gm, &["300"], Some(npc), &tx, &mut mgr, &engine).await;

    let payloads = drain_stat_updates(&mut rx, npc);
    assert_eq!(payloads.len(), 1, "exactly one onStatUpdate must be sent");
    let mut entries = decode_stat_update(&payloads[0]);
    entries.sort_by_key(|e| e.0);
    let mut expected = vec![
        (MOVEMENT_SPEED_MOD, 0, 300, 500),
        (ROTATION_SPEED_MOD, 0, 300, 500),
    ];
    expected.sort_by_key(|e| e.0);
    assert_eq!(
        entries, expected,
        "onStatUpdate must carry exactly both speed stats at the new current value"
    );
}

/// NPC target: the update reaches an AoI witness via `WitnessEntityMethod`.
/// Both ids are in `PUBLIC_STATS`, so the NPC branch's
/// `serialize_dirty_public` must not filter them out.
#[tokio::test]
async fn legacy_p47_speed_on_npc_target_fans_out_to_witness() {
    let (mut mgr, gm, npc) = setup();
    mgr.connect_entity(gm);
    let _ = mgr.compute_aoi_changes(); // tick 1: NPC enters the caller's AoI

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("speed", gm, &["200"], Some(npc), &tx, &mut mgr, &engine).await;

    let payloads = drain_stat_updates(&mut rx, npc);
    assert_eq!(
        payloads.len(),
        1,
        "the NPC's speed change must reach its one AoI witness"
    );
    let ids: Vec<i32> = decode_stat_update(&payloads[0])
        .iter()
        .map(|e| e.0)
        .collect();
    assert!(
        ids.contains(&MOVEMENT_SPEED_MOD) && ids.contains(&ROTATION_SPEED_MOD),
        "public serialization must keep both speed stats: {ids:?}"
    );
}

/// D03: the feedback line is addressed to the calling GM, never to the entity
/// whose speed changed (`Target::Being` routinely makes those different).
#[tokio::test]
async fn legacy_p47_speed_feedback_goes_to_caller_not_target() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("speed", gm, &["150"], Some(npc), &tx, &mut mgr, &engine).await;

    let mut saw = false;
    while let Ok(msg) = rx.try_recv() {
        let CellToBaseMsg::EntityMethodCall { entity_id, .. } = &msg else {
            continue;
        };
        let eid = *entity_id;
        if decode_feedback(&msg).is_some() {
            saw = true;
            assert_eq!(
                eid, gm,
                ".speed feedback must route to the caller ({gm}), never the target ({npc})"
            );
        }
    }
    assert!(saw, "expected a feedback line from .speed");
}

/// A non-integer value is rejected by the shared parser with zero mutation.
#[tokio::test]
async fn legacy_p47_speed_rejects_non_integer_without_mutation() {
    let (mut mgr, gm, npc) = setup();
    let before = speed_triples(&mgr, npc);

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("speed", gm, &["fast"], Some(npc), &tx, &mut mgr, &engine).await;

    assert_eq!(
        speed_triples(&mgr, npc),
        before,
        "a malformed value must leave both speed stats untouched"
    );
    let (lines, stat_updates) = drain_all(&mut rx, npc);
    assert!(
        lines.iter().any(|l| l.contains("must be an integer")),
        "malformed .speed must explain the integer requirement: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("Set speed of entity")),
        "a rejected .speed must not claim success: {lines:?}"
    );
    assert!(
        stat_updates.is_empty(),
        "a rejected .speed must publish no onStatUpdate"
    );
}

/// Out-of-range values are REJECTED, not silently clamped the way legacy's
/// `Stat.setCurrent` would have (`.speed 9000` quietly becoming 500). Both the
/// above-max and below-min directions must leave BOTH stats untouched — the
/// atomicity half of the deviation: `movementSpeedMod` is validated first, so
/// a naive implementation that wrote as it validated would corrupt it before
/// discovering the problem.
#[tokio::test]
async fn legacy_p47_speed_rejects_out_of_range_without_mutation() {
    for arg in ["501", "-1", "9000"] {
        let (mut mgr, gm, npc) = setup();
        let before = speed_triples(&mgr, npc);

        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);
        exec("speed", gm, &[arg], Some(npc), &tx, &mut mgr, &engine).await;

        assert_eq!(
            speed_triples(&mgr, npc),
            before,
            ".speed {arg} is out of range and must mutate neither speed stat"
        );
        let (lines, stat_updates) = drain_all(&mut rx, npc);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("must be between 0 and 500 (100 = normal)")),
            ".speed {arg} must report the permitted range: {lines:?}"
        );
        assert!(
            stat_updates.is_empty(),
            ".speed {arg} must publish no onStatUpdate"
        );
    }
}

/// The inclusive bounds themselves are accepted — `0` (frozen) and `500` (the
/// stat ceiling) are legal values, not off-by-one rejections.
#[tokio::test]
async fn legacy_p47_speed_accepts_inclusive_bounds() {
    for (arg, expect) in [("0", 0), ("500", 500)] {
        let (mut mgr, gm, npc) = setup();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);
        exec("speed", gm, &[arg], Some(npc), &tx, &mut mgr, &engine).await;

        assert_eq!(
            speed_triples(&mgr, npc),
            vec![(0, expect, 500), (0, expect, 500)],
            ".speed {arg} sits on an inclusive bound and must be accepted"
        );
        assert!(
            drain_feedback(&mut rx)
                .iter()
                .any(|l| l == &format!("Set speed of entity {npc} to {expect}")),
            ".speed {arg} must report success"
        );
    }
}

/// A target that vanished between selection and dispatch reports "no entity."
/// rather than panicking — same defensive branch shape as `stats::show`.
#[tokio::test]
async fn legacy_p47_speed_on_vanished_target_reports_no_entity() {
    let (mut mgr, gm, npc) = setup();
    mgr.destroy_entity(npc);
    let (tx, mut rx) = mpsc::channel(16);
    // Call `set_speed` directly with the stale id — `resolve_target` would
    // normally reject a dead selection before `exec` is reached, so this
    // exercises the handler's own guard in isolation.
    stats::set_speed(gm, npc, &["200"], &tx, &mut mgr).await;

    assert_eq!(
        drain_feedback(&mut rx),
        vec!["speed: no entity.".to_string()],
        "a vanished target must report 'no entity.', not panic"
    );
}

/// The caller's own speed stats are untouched when a distinct target is
/// modified (D03's caller/subject split, asserted on state rather than
/// feedback).
#[tokio::test]
async fn legacy_p47_speed_leaves_the_caller_alone() {
    let (mut mgr, gm, npc) = setup();
    let caller_before = speed_triples(&mgr, gm);

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);
    exec("speed", gm, &["400"], Some(npc), &tx, &mut mgr, &engine).await;

    assert_eq!(
        speed_triples(&mgr, gm),
        caller_before,
        "the calling GM's own speed must not change when modifying a target"
    );
    assert_eq!(
        speed_triples(&mgr, npc),
        vec![(0, 400, 500), (0, 400, 500)],
        "the target's speed must be the one that changed"
    );
}
