use super::*;
use crate::test_support::LogCapture;
use std::sync::Mutex;
use tracing::Level;

async fn claimed() -> (SessionRegistry, MinigameSession) {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4242,
            7,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![100, 200],
            vec![700],
            Some("Hacker".into()),
            None,
            9,
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4242, &ticket, "Livewire")
        .await
        .unwrap();
    (registry, session)
}

#[tokio::test]
async fn result_dispatch_delivers_ticket_only_and_emits_once() {
    let (registry, session) = claimed().await;
    let (tx, mut rx) = mpsc::channel(1);
    let events = Mutex::new(Vec::new());
    let sink = |event| events.lock().unwrap().push(event);
    assert_eq!(
        send_minigame_result_with_sink(
            &registry,
            &tx,
            &session,
            RESULT_VICTORY,
            "victory_message",
            true,
            &sink
        )
        .await,
        DispatchOutcome::Delivered
    );
    assert!(
        matches!(rx.try_recv(), Ok(CellToBaseMsg::MinigameResult { ticket }) if ticket == session.ticket)
    );
    let receipt = registry.redeem_result(&session.ticket).await.unwrap();
    assert_eq!(
        (
            receipt.entity_id,
            receipt.player_id,
            receipt.session_instance_id
        ),
        (4242, 7, 9)
    );
    assert_eq!(
        (receipt.result_code, receipt.on_victory_chains),
        (1, vec![100, 200])
    );
    assert!(
        receipt.on_failure_chains.is_empty(),
        "a victory never carries the failure chains"
    );
    assert!(registry.redeem_result(&session.ticket).await.is_none());
    assert_eq!(
        send_minigame_result_with_sink(
            &registry,
            &tx,
            &session,
            RESULT_VICTORY,
            "victory_tick",
            true,
            &sink
        )
        .await,
        DispatchOutcome::Refused {
            player_canceled: false
        }
    );
    assert_eq!(events.lock().unwrap().len(), 1);
    assert!(rx.try_recv().is_err());
}

/// D-MG5: a defeat's receipt carries the registered failure chains (and no
/// victory chains), so the Base can hand them to the cell after redemption.
#[tokio::test]
async fn result_dispatch_defeat_receipt_carries_failure_chains() {
    let (registry, session) = claimed().await;
    let (tx, mut rx) = mpsc::channel(1);
    let sink = |_event| {};
    assert_eq!(
        send_minigame_result_with_sink(
            &registry,
            &tx,
            &session,
            RESULT_DEFEAT,
            "failure_tick",
            true,
            &sink
        )
        .await,
        DispatchOutcome::Delivered
    );
    assert!(
        matches!(rx.try_recv(), Ok(CellToBaseMsg::MinigameResult { ticket }) if ticket == session.ticket)
    );
    let receipt = registry.redeem_result(&session.ticket).await.unwrap();
    assert_eq!(receipt.result_code, RESULT_DEFEAT);
    assert_eq!(receipt.on_failure_chains, vec![700]);
    assert!(receipt.on_victory_chains.is_empty());
}

#[tokio::test]
async fn result_dispatch_closed_channel_revokes_receipt_and_logs_named_error() {
    let capture = LogCapture::install();
    let (registry, session) = claimed().await;
    let (tx, rx) = mpsc::channel(1);
    drop(rx);
    let events = Mutex::new(Vec::new());
    let sink = |event| events.lock().unwrap().push(event);
    assert_eq!(
        send_minigame_result_with_sink(
            &registry,
            &tx,
            &session,
            RESULT_VICTORY,
            "victory_message",
            true,
            &sink
        )
        .await,
        DispatchOutcome::DeliveryFailed
    );
    assert!(registry.redeem_result(&session.ticket).await.is_none());
    assert_eq!(
        send_minigame_result_with_sink(
            &registry,
            &tx,
            &session,
            RESULT_DEFEAT,
            "failure_tick",
            true,
            &sink
        )
        .await,
        DispatchOutcome::Refused {
            player_canceled: false
        }
    );
    assert!(events.lock().unwrap().is_empty());
    let event = capture
        .find_message(Level::ERROR, "Minigame: result delivery failed")
        .unwrap();
    assert!(event.has_field("phase", "victory_message"), "{event:#?}");
    assert!(event.has_field("result", "victory"), "{event:#?}");
    assert!(event.has_field("chain_count", "2"), "{event:#?}");
    assert!(
        event.has_field("reason", "result_delivery_failed"),
        "{event:#?}"
    );
    let logged = format!("{:#?}", capture.all());
    assert!(
        !logged.contains(&session.ticket),
        "bearer ticket leaked to result log"
    );
    assert!(
        !logged.contains("http://unused/"),
        "minigame URL leaked to result log"
    );
}

#[tokio::test]
async fn result_dispatch_failed_send_revokes_only_its_ticket_after_replacement() {
    let (registry, old) = claimed().await;
    let (tx, rx) = mpsc::channel(1);
    tx.send(CellToBaseMsg::MinigameResult {
        ticket: "occupied".into(),
    })
    .await
    .unwrap();
    let reg = registry.clone();
    let task = tokio::spawn(async move {
        send_minigame_result(&reg, &tx, &old, RESULT_VICTORY, "victory_message", true).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if registry.receipt_count().await == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(registry.cancel(4242).await);
    let replacement = registry
        .register(
            4242,
            7,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![300],
            vec![],
            None,
            None,
            9,
        )
        .await
        .unwrap();
    let claimed = registry
        .authenticate_and_claim(4242, &replacement, "Livewire")
        .await
        .unwrap();
    assert!(registry.deposit_result(&claimed, RESULT_DEFEAT, true).await);
    drop(rx);
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap(),
        DispatchOutcome::DeliveryFailed
    );
    assert_eq!(
        registry
            .redeem_result(&replacement)
            .await
            .unwrap()
            .result_code,
        RESULT_DEFEAT
    );
}
