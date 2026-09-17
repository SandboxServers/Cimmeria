//! Packet P05 regression suite: `.givecash`, `.givexp` (both new
//! registrations).
//!
//! These are the first *mutating* commands in the legacy-command-parity
//! campaign — everything through P04 was read-only. The cell-side handler
//! (this suite) only validates input and forwards a typed
//! `CellToBaseMsg::GrantCash`/`GrantXP` with `gm_feedback_to: Some(caller)`;
//! it must NOT send any optimistic "requested" feedback of its own — the
//! base sends the real, post-commit outcome. The live-DB/wire-fanout
//! coverage for the base-side `gm_feedback_to` split lives in
//! `crates/services/src/base/world_entry/methods/progression/tests.rs`
//! (same `legacy_p05_` filter).
//!
//! Filter prefix: `legacy_p05_`.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Create a fresh connected player entity in `setup()`'s "Agnos" space.
/// `player_id: None` reproduces a player entity with no DB-backed character
/// (e.g. mid play-character flow) so the "target has no player id" rejection
/// path is reachable.
fn player_target(mgr: &mut SpaceManager, entity_id: u32, player_id: Option<i32>) {
    mgr.create_entity(entity_id, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.is_player = true;
        e.player_id = player_id;
    }
}

/// Drain every queued message, returning the (at most one) `GrantCash`
/// payload and every decoded feedback line.
fn drain_grant_cash(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> (Option<(u32, i32, i32, Option<u32>)>, Vec<String>) {
    let mut grant = None;
    let mut feedback = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match &msg {
            CellToBaseMsg::GrantCash {
                entity_id,
                player_id,
                amount,
                gm_feedback_to,
            } => grant = Some((*entity_id, *player_id, *amount, *gm_feedback_to)),
            _ => {
                if let Some(text) = decode_feedback(&msg) {
                    feedback.push(text);
                }
            }
        }
    }
    (grant, feedback)
}

/// Drain every queued message, returning the (at most one) `GrantXP`
/// payload and every decoded feedback line.
fn drain_grant_xp(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> (Option<(u32, u64, Option<u32>)>, Vec<String>) {
    let mut grant = None;
    let mut feedback = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match &msg {
            CellToBaseMsg::GrantXP {
                entity_id,
                xp_amount,
                gm_feedback_to,
            } => grant = Some((*entity_id, *xp_amount, *gm_feedback_to)),
            _ => {
                if let Some(text) = decode_feedback(&msg) {
                    feedback.push(text);
                }
            }
        }
    }
    (grant, feedback)
}

// ── .givecash ────────────────────────────────────────────────────────────

/// Happy path: `.givecash <amount>` on a distinct target sends exactly one
/// `GrantCash` carrying the TARGET's entity/player id, the requested amount,
/// and `gm_feedback_to: Some(caller)` — never the caller's own id as the
/// recipient, never `None` (which would drop GM feedback entirely). No
/// optimistic feedback line is sent from the cell side; the real outcome
/// comes from the base after the DB write commits.
#[tokio::test]
async fn legacy_p05_givecash_emits_grant_with_caller_feedback_split() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, Some(500));
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "givecash",
        gm,
        &["100"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let (grant, feedback) = drain_grant_cash(&mut rx);
    assert_eq!(
        grant,
        Some((target, 500, 100, Some(gm))),
        "GrantCash must target the selected player, carry the amount, and \
         route GM feedback to the caller"
    );
    assert!(
        feedback.is_empty(),
        "the cell side must not send an optimistic feedback line; got {feedback:?}"
    );
}

/// A non-positive amount (0 or negative) must not grant — D02 keeps the
/// current Rust bound (native `gmGiveCash` also rejects `<= 0`) rather than
/// reproducing legacy's unbounded `giveCash`.
#[tokio::test]
async fn legacy_p05_givecash_rejects_non_positive_amount() {
    for amount in ["0", "-5"] {
        let (mut mgr, gm, _npc) = setup();
        let target = 2u32;
        player_target(&mut mgr, target, Some(500));
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        exec(
            "givecash",
            gm,
            &[amount],
            Some(target),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let (grant, feedback) = drain_grant_cash(&mut rx);
        assert_eq!(grant, None, "amount {amount} must not grant");
        assert!(
            feedback.iter().any(|t| t.contains("positive")),
            "amount {amount} must feed back a positive-amount rejection; got {feedback:?}"
        );
    }
}

/// A selected target with no `player_id` (e.g. mid play-character flow)
/// must be rejected with a clear reason, not silently forward a grant with a
/// bogus `player_id`.
#[tokio::test]
async fn legacy_p05_givecash_rejects_target_without_player_id() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, None);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "givecash",
        gm,
        &["100"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let (grant, feedback) = drain_grant_cash(&mut rx);
    assert_eq!(grant, None, "a target with no player_id must not grant");
    assert!(
        feedback.iter().any(|t| t.contains("player id")),
        "must explain the target has no player id; got {feedback:?}"
    );
}

/// `.givecash` on a non-player target (e.g. an NPC) must be rejected by the
/// shared `Target::Player` guard before the handler ever runs.
#[tokio::test]
async fn legacy_p05_givecash_rejects_non_player_target() {
    let (mut mgr, gm, _npc) = setup(); // setup's npc has is_player = false
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::console::handle_console_command(gm, ".givecash 100", &tx, &mut mgr, &engine).await;

    let (grant, feedback) = drain_grant_cash(&mut rx);
    assert_eq!(
        grant, None,
        "a non-player target must never reach give_cash"
    );
    assert!(
        feedback
            .iter()
            .any(|t| t.contains("expected a player as a target")),
        "a non-player target must be rejected before give_cash runs; got {feedback:?}"
    );
}

// ── .givexp ──────────────────────────────────────────────────────────────

/// Happy path: `.givexp <amount>` on a distinct target sends exactly one
/// `GrantXP` carrying the TARGET's entity id, the amount safely cast to
/// `u64`, and `gm_feedback_to: Some(caller)`.
#[tokio::test]
async fn legacy_p05_givexp_emits_grant_with_caller_feedback_split() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, Some(500));
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec("givexp", gm, &["250"], Some(target), &tx, &mut mgr, &engine).await;

    let (grant, feedback) = drain_grant_xp(&mut rx);
    assert_eq!(
        grant,
        Some((target, 250u64, Some(gm))),
        "GrantXP must target the selected player, carry the amount as u64, \
         and route GM feedback to the caller"
    );
    assert!(
        feedback.is_empty(),
        "the cell side must not send an optimistic feedback line; got {feedback:?}"
    );
}

/// A non-positive amount must not grant — confirming `amount > 0` before the
/// `i32 -> u64` cast, matching the native `gmGiveXp` ordering (guards a
/// negative `i32` wrapping into an absurd unsigned grant).
#[tokio::test]
async fn legacy_p05_givexp_rejects_non_positive_amount() {
    for amount in ["0", "-5"] {
        let (mut mgr, gm, _npc) = setup();
        let target = 2u32;
        player_target(&mut mgr, target, Some(500));
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        exec(
            "givexp",
            gm,
            &[amount],
            Some(target),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let (grant, feedback) = drain_grant_xp(&mut rx);
        assert_eq!(grant, None, "amount {amount} must not grant");
        assert!(
            feedback.iter().any(|t| t.contains("positive")),
            "amount {amount} must feed back a positive-amount rejection; got {feedback:?}"
        );
    }
}

/// A selected target with no `player_id` must be rejected with a clear
/// reason before any `GrantXP` is sent.
#[tokio::test]
async fn legacy_p05_givexp_rejects_target_without_player_id() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, None);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec("givexp", gm, &["250"], Some(target), &tx, &mut mgr, &engine).await;

    let (grant, feedback) = drain_grant_xp(&mut rx);
    assert_eq!(grant, None, "a target with no player_id must not grant");
    assert!(
        feedback.iter().any(|t| t.contains("player id")),
        "must explain the target has no player id; got {feedback:?}"
    );
}

/// `.givexp` on a non-player target must be rejected by the shared
/// `Target::Player` guard before the handler ever runs.
#[tokio::test]
async fn legacy_p05_givexp_rejects_non_player_target() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::console::handle_console_command(gm, ".givexp 250", &tx, &mut mgr, &engine).await;

    let (grant, feedback) = drain_grant_xp(&mut rx);
    assert_eq!(grant, None, "a non-player target must never reach give_xp");
    assert!(
        feedback
            .iter()
            .any(|t| t.contains("expected a player as a target")),
        "a non-player target must be rejected before give_xp runs; got {feedback:?}"
    );
}
