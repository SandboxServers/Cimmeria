//! `Action::GrantCash` (UB-50b1) on the cell: one firing sends one
//! `CellToBaseMsg::GrantCash` for the chain's player, and each refusal sends
//! nothing and says why (TESTING.md type 1, plus type 12 for the log rows).

use super::*;
use crate::test_support::LogCapture;

fn cash_action(amount: i32) -> Action {
    Action::GrantCash { amount }
}

fn resolved(amount: i32) -> ResolvedActions {
    ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(23480, cash_action(amount))],
    }
}

fn row(
    capture: &crate::test_support::LogCaptureGuard,
    level: tracing::Level,
    reason: &str,
) -> bool {
    capture.all().iter().any(|e| {
        e.target == "content"
            && e.level == level
            && e.has_field("event", "content.grant_cash")
            && e.has_field("reason", reason)
            && e.has_field("chain_id", "23480")
    })
}

/// A player's firing sends one grant of the authored amount to the base, and
/// logs one `requested` row. Revert: delete the `Action::GrantCash` dispatch arm,
/// and the action falls to the no-op `other =>` arm.
#[tokio::test]
async fn grant_cash_sends_one_grant_to_the_base() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(22, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(22).unwrap();
    p.is_player = true;
    p.player_id = Some(42);
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(resolved(60), 22, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    let sent = rx.try_recv().expect("one grant must reach the base");
    assert!(
        matches!(
            sent,
            CellToBaseMsg::GrantCash {
                entity_id: 22,
                player_id: 42,
                amount: 60,
                gm_feedback_to: None,
            }
        ),
        "the grant carries the player and the authored amount"
    );
    assert!(rx.try_recv().is_err(), "exactly one grant per firing");
    assert!(
        capture.all().iter().any(|e| {
            e.target == "content"
                && e.level == tracing::Level::INFO
                && e.has_field("event", "content.grant_cash")
                && e.has_field("outcome", "requested")
                && e.has_field("chain_id", "23480")
        }),
        "{:#?}",
        capture.all()
    );
}

/// A chain fired for an entity that is not the named player (an NPC, player 0)
/// grants nothing and says why. Revert: delete the identity check.
#[tokio::test]
async fn grant_cash_refuses_a_non_player_entity() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(21, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(resolved(60), 21, 0, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(rx.try_recv().is_err(), "nothing may reach the base");
    assert!(
        row(&capture, tracing::Level::WARN, "no_player"),
        "{:#?}",
        capture.all()
    );
}

/// A zero amount reaching the executor (a JSON-built chain bypasses the
/// loader) grants nothing and says why. Revert: delete the amount check.
#[tokio::test]
async fn grant_cash_refuses_a_non_positive_amount() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(22, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(22).unwrap();
    p.is_player = true;
    p.player_id = Some(42);
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(resolved(0), 22, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(rx.try_recv().is_err(), "nothing may reach the base");
    assert!(
        row(&capture, tracing::Level::WARN, "bad_amount"),
        "{:#?}",
        capture.all()
    );
}

/// A closed cell-to-base channel is logged with the player's ids, not
/// swallowed. Revert: replace the `if let Err` with `let _ =`.
#[tokio::test]
async fn grant_cash_warns_when_cell_to_base_channel_closed() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(22, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(22).unwrap();
    p.is_player = true;
    p.player_id = Some(42);
    let (tx, rx) = mpsc::channel(8);
    drop(rx);

    execute_actions(resolved(60), 22, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(
        row(&capture, tracing::Level::WARN, "base_channel_closed"),
        "{:#?}",
        capture.all()
    );
}
