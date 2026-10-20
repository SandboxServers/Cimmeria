use super::*;

fn published_board(entity_id: u32) -> crate::minigame::session::BoardSnapshot {
    crate::minigame::session::BoardSnapshot {
        entity_id,
        game_name: "Hack".into(),
        state: "ready",
        outcome: None,
        difficulty: 1,
        game_started: false,
        timer_state: 0,
        time_remaining: 20.0,
        goal_total: 1,
        goal_cut: 0,
        wires: vec![],
        events: vec![],
        events_dropped: 0,
    }
}

#[tokio::test]
async fn deposited_receipt_carries_the_launch_player_name() {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4204,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![1017],
            Some("Hacker".into()),
            None,
            9,
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4204, &ticket, "Hack")
        .await
        .unwrap();
    registry
        .publish_board(4204, &ticket, published_board(4204))
        .await;
    assert!(registry.deposit_result(&session, 1, true).await);
    let receipt = registry.redeem_result(&ticket).await.unwrap();
    assert_eq!(receipt.player_name.as_deref(), Some("Hacker"));
}

#[tokio::test]
async fn a_session_deposits_at_most_one_result() {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4204,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![1017],
            None,
            None,
            9,
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4204, &ticket, "Hack")
        .await
        .unwrap();
    registry
        .publish_board(4204, &ticket, published_board(4204))
        .await;
    assert!(
        registry.deposit_result(&session, 1, true).await,
        "first terminal result deposits"
    );
    assert!(
        !registry.deposit_result(&session, 2, true).await,
        "a second terminal result is refused"
    );
    let receipt = registry.redeem_result(&ticket).await.expect("one receipt");
    assert_eq!(receipt.result_code, 1);
    assert!(
        registry.redeem_result(&ticket).await.is_none(),
        "a receipt redeems once"
    );
}
