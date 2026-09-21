//! `dialogButtonChoice` server-authority gate tests (CAT-J-01 / #479,
//! widened to an offered-dialog SET by DU-08).
//!
//! Split out of the parent's `mod tests` because that module is already
//! over the repo's file cap. Two helpers here are `pub(super)` and shared
//! with the `last_interaction_target` tests that stayed behind:
//! [`counter`] and [`dialog_choice_args`].

use super::*;
use crate::test_support::make_space_manager;

/// Register an `OnDialogChoice { dialog_id }` chain that bumps a counter
/// when it fires, so a test can observe whether the choice handler
/// actually ran the chain. Counter delta = proof of chain execution.
fn engine_with_dialog_choice_chain(dialog_id: i32, counter: &str) -> ChainEngine {
    let mut engine = ChainEngine::new();
    add_dialog_choice_chain(&mut engine, 70479, dialog_id, counter);
    engine
}

/// Same, onto an existing engine — the DU-08 eviction tests need two
/// dialogs keyed to two distinct counters so "which chain fired?" is
/// observable.
fn add_dialog_choice_chain(engine: &mut ChainEngine, chain_id: i64, dialog_id: i32, counter: &str) {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: chain_id,
        name: "test OnDialogChoice → increment counter".into(),
        enabled: true,
        trigger: Trigger::OnDialogChoice { dialog_id },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: counter.into(),
            amount: 1,
        }],
        priority: 0,
    });
}

pub(super) fn dialog_choice_args(dialog_id: i32, button_id: i32) -> Vec<u8> {
    let mut a = Vec::with_capacity(8);
    a.extend_from_slice(&dialog_id.to_le_bytes());
    a.extend_from_slice(&button_id.to_le_bytes());
    a
}

pub(super) fn counter(mgr: &SpaceManager, entity_id: u32, name: &str) -> i32 {
    mgr.get_entity(entity_id)
        .and_then(|e| e.counters.get(name).copied())
        .unwrap_or(0)
}

/// **#479 negative case.** A `DialogButtonChoice` for a `dialog_id` the
/// server never displayed (forged packet) must be rejected: the bound
/// `OnDialogChoice` chain does NOT fire, and a `warn!` is logged. Pre-fix
/// the handler fired the chain unconditionally, so an attacker could
/// drive GrantItem/AcceptMission/Teleport for any discovered dialog_id.
#[tokio::test]
async fn dialog_choice_for_unopened_dialog_is_rejected() {
    use crate::test_support::LogCapture;
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
        // Nothing offered — offered_dialog_ids stays empty (the default).
    }
    let engine = engine_with_dialog_choice_chain(5354, "j01");
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    let handled = dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(5354, 0),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert!(handled, "handler still consumes the method");

    assert_eq!(
        counter(&mgr, 1, "j01"),
        0,
        "a forged choice for an un-opened dialog must NOT fire the chain"
    );
    assert!(
        capture
            .find_message(tracing::Level::WARN, "dialogButtonChoice rejected")
            .is_some(),
        "rejection must emit the documented warn for ops/audit"
    );
}

/// **#479 positive case.** When the dialog WAS offered (recorded by
/// `send_dialog_display`), the matching choice fires the chain and the
/// id is removed one-shot (mirrors python `del displayedDialogs[id]`).
#[tokio::test]
async fn dialog_choice_for_offered_dialog_fires_chain_and_consumes_it() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
        p.offer_dialog(5354); // dialog was displayed
    }
    let engine = engine_with_dialog_choice_chain(5354, "j01");
    let (tx, _rx) = mpsc::channel(16);

    let handled = dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(5354, 0),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert!(handled);

    assert_eq!(
        counter(&mgr, 1, "j01"),
        1,
        "an offered dialog's choice must fire the bound OnDialogChoice chain"
    );
    assert_eq!(
        mgr.get_entity(1).map(|e| e.offered_dialogs()),
        Some(Vec::new()),
        "a valid choice must consume the id (one-shot) so a replay is rejected"
    );
}

/// **#479 mismatch case.** A choice for dialog B while only dialog A
/// was offered must be rejected — the attacker can't ride an unrelated
/// offer to fire a different dialog_id's chain.
#[tokio::test]
async fn dialog_choice_for_an_unoffered_dialog_id_is_rejected() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
        p.offer_dialog(1111); // a DIFFERENT dialog was offered
    }
    let engine = engine_with_dialog_choice_chain(5354, "j01");
    let (tx, _rx) = mpsc::channel(16);

    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(5354, 0),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        counter(&mgr, 1, "j01"),
        0,
        "choice for dialog 5354 must not fire when only dialog 1111 was offered"
    );
    assert_eq!(
        mgr.get_entity(1).map(|e| e.offered_dialogs()),
        Some(vec![1111]),
        "a rejected mismatched choice must leave the real offer intact"
    );
}

/// **#479 replay idempotency.** Two identical valid choices in a row:
/// the first fires and consumes the offer; the second finds the id
/// gone and is rejected. Closes the replay sub-finding for free.
#[tokio::test]
async fn replayed_dialog_choice_is_rejected_after_first() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
        p.offer_dialog(5354);
    }
    let engine = engine_with_dialog_choice_chain(5354, "j01");
    let (tx, _rx) = mpsc::channel(16);
    let args = dialog_choice_args(5354, 0);

    dispatch(1, DIALOG_BUTTON_CHOICE, &args, &tx, &mut mgr, &engine).await;
    dispatch(1, DIALOG_BUTTON_CHOICE, &args, &tx, &mut mgr, &engine).await;

    assert_eq!(
        counter(&mgr, 1, "j01"),
        1,
        "the chain must fire exactly once — the replayed second choice is \
         rejected because the first consumed the offer"
    );
}

/// **DU-08 regression guard (client contract F13).**
///
/// The client holds one non-tutorial dialog at a time. When the server
/// displays B while zero-button A is still open, the client evicts A
/// through its discard path and sends `dialogButtonChoice(A, -1)`
/// AFTER the server has already recorded B. With the old single pin,
/// that late close was rejected and A's `dialog_choice` chain never
/// fired — a silently lost progression step (2574, 2577, 2581, 5003,
/// 5004, 5008, 5009 are all zero-button chain keys in the Castle
/// seeds).
///
/// Both ids go through the real `send_dialog_display`, so this guard
/// covers the display side too. It FAILS on pre-DU-08 code: the
/// second display overwrote the pin, `(A, -1)` hit the mismatch arm
/// and `a01` stayed 0.
#[tokio::test]
async fn an_evicted_dialogs_late_close_is_accepted_and_fires_its_chain() {
    const A: i32 = 2574;
    const B: i32 = 2576;
    const CLOSE: i32 = -1; // F8: a zero-button close sends -1

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
    }
    let mut engine = engine_with_dialog_choice_chain(A, "a01");
    add_dialog_choice_chain(&mut engine, 70480, B, "b01");
    let (tx, _rx) = mpsc::channel(32);

    // Server displays A, then B. The client's slot now holds B and it
    // has discarded A.
    crate::cell::interactions::send_dialog_display(1, 100, A, &tx, &mut mgr).await;
    crate::cell::interactions::send_dialog_display(1, 100, B, &tx, &mut mgr).await;

    // A's eviction close arrives late.
    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(A, CLOSE),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        counter(&mgr, 1, "a01"),
        1,
        "the evicted dialog's late (-1) close must fire ITS chain — a \
         single open-dialog pin rejects this and loses the step"
    );
    assert!(
        !mgr.get_entity(1).unwrap().dialog_is_offered(A),
        "the accepted close must consume A (one-shot)"
    );

    // B is still answerable afterwards, and fires only its own chain.
    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(B, 71),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(
        counter(&mgr, 1, "b01"),
        1,
        "the surviving dialog must still be answerable after the eviction close"
    );
    assert_eq!(
        counter(&mgr, 1, "a01"),
        1,
        "answering B must not re-fire A's chain"
    );
    assert_eq!(
        mgr.get_entity(1).map(|e| e.offered_dialogs()),
        Some(Vec::new()),
        "both offers consumed"
    );
}

/// **DU-08 authority guard.** Widening the pin to a set must not
/// widen what a client can forge. An id that was never displayed is
/// still rejected even while other dialogs ARE offered, and a
/// legitimately-answered id cannot be replayed by a later eviction
/// close — the take removed it from the set.
#[tokio::test]
async fn a_forged_id_is_rejected_and_an_answered_id_cannot_be_reclosed() {
    const A: i32 = 2574;
    const FORGED: i32 = 5354;

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
    }
    let mut engine = engine_with_dialog_choice_chain(A, "a01");
    add_dialog_choice_chain(&mut engine, 70481, FORGED, "forged");
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::interactions::send_dialog_display(1, 100, A, &tx, &mut mgr).await;

    // Forged: never displayed, but another dialog IS offered.
    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(FORGED, 8),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(
        counter(&mgr, 1, "forged"),
        0,
        "an id that was never offered must still be rejected — the set \
         must not become a free pass for every dialog id"
    );
    assert!(
        mgr.get_entity(1).unwrap().dialog_is_offered(A),
        "a rejected forgery must not disturb the real offer"
    );

    // A is answered by a button click, which closes it client-side.
    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(A, 8),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    // A later (-1) naming A — replay, or a stale discard — is rejected.
    dispatch(
        1,
        DIALOG_BUTTON_CHOICE,
        &dialog_choice_args(A, -1),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(
        counter(&mgr, 1, "a01"),
        1,
        "a dialog already answered via a button click must not fire again \
         on a later close — this is what keeps eviction from double-advancing"
    );
}
