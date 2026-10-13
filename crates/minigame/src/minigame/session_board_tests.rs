//! Registry lifetime and bounds for the lab's Livewire board view.

use super::super::session_snapshot::FINISHED_BOARD_TTL;
use super::tests::register_livewire;
use super::*;

fn board() -> BoardSnapshot {
    BoardSnapshot {
        entity_id: 42,
        game_name: "Livewire".into(),
        state: "ready",
        outcome: None,
        goal_total: 2,
        goal_cut: 0,
        wires: vec![],
        events: vec![],
        events_dropped: 0,
    }
}

#[tokio::test(start_paused = true)]
async fn board_snapshot_retains_final_outcome_then_expires() {
    let reg = SessionRegistry::new();
    let first = register_livewire(&reg, 42).await.unwrap();
    reg.publish_board(42, &first, board()).await;
    assert!(reg.board_snapshot(42).await.is_none());
    reg.authenticate_and_claim(42, &first, "Livewire")
        .await
        .unwrap();
    reg.publish_board(42, &first, board()).await;
    reg.record_event(42, &first, "processover", Some("g1"))
        .await;
    let mut final_board = board();
    final_board.state = "finished";
    final_board.outcome = Some("victory");
    reg.publish_board(42, &first, final_board).await;
    reg.record_event(42, &first, "result_victory", None).await;
    reg.remove_if_ticket(42, &first).await;
    let retained = reg.board_snapshot(42).await.unwrap();
    assert_eq!(retained.outcome, Some("victory"));
    assert_eq!(
        retained
            .events
            .iter()
            .map(|e| e.event.as_str())
            .collect::<Vec<_>>(),
        [
            "registered",
            "claimed",
            "board_started",
            "processover",
            "result_victory",
            "ended"
        ]
    );
    assert_eq!(retained.events[3].wire_name.as_deref(), Some("g1"));
    tokio::time::advance(FINISHED_BOARD_TTL).await;
    assert!(reg.board_snapshot(42).await.is_none());
}

#[tokio::test]
async fn replacement_session_cannot_read_or_update_old_board() {
    let reg = SessionRegistry::new();
    let first = register_livewire(&reg, 42).await.unwrap();
    reg.authenticate_and_claim(42, &first, "Livewire")
        .await
        .unwrap();
    reg.publish_board(42, &first, board()).await;
    reg.remove_if_ticket(42, &first).await;
    let second = register_livewire(&reg, 42).await.unwrap();
    assert!(reg.board_snapshot(42).await.is_none());
    reg.authenticate_and_claim(42, &second, "Livewire")
        .await
        .unwrap();
    reg.publish_board(42, &first, board()).await;
    reg.record_event(42, &first, "processmove", Some("g1"))
        .await;
    assert!(reg.board_snapshot(42).await.is_none());
}

#[tokio::test]
async fn traffic_events_are_capped_and_do_not_include_ticket() {
    let reg = SessionRegistry::new();
    let ticket = register_livewire(&reg, 42).await.unwrap();
    reg.authenticate_and_claim(42, &ticket, "Livewire")
        .await
        .unwrap();
    reg.publish_board(42, &ticket, board()).await;
    for _ in 0..140 {
        reg.record_event(42, &ticket, "processmove", Some("g1"))
            .await;
    }
    let snapshot = reg.board_snapshot(42).await.unwrap();
    assert_eq!(snapshot.events.len(), 128);
    assert_eq!(snapshot.events_dropped, 15);
    assert!(!serde_json::to_string(&snapshot).unwrap().contains(&ticket));
}
