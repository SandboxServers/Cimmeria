use super::terminal_outcome::report_terminal;
use super::*;
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::test_support::LogCapture;
use std::collections::HashMap;
use std::future::{poll_fn, Future};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn queue_behind_lock<F>(future: F) -> (oneshot::Receiver<()>, tokio::task::JoinHandle<F::Output>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (queued_tx, queued_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut future = Box::pin(future);
        let mut queued_tx = Some(queued_tx);
        poll_fn(move |cx| {
            let result = future.as_mut().poll(cx);
            if result.is_pending() {
                if let Some(tx) = queued_tx.take() {
                    let _ = tx.send(());
                }
            }
            result
        })
        .await
    });
    (queued_rx, task)
}

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

struct DoubleTerminal {
    tick: bool,
    second_victory: bool,
}

impl MinigameInstance for DoubleTerminal {
    fn started(&mut self) -> Vec<GameOutput> {
        vec![]
    }
    fn message(&mut self, _: &str, _: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        if self.tick {
            vec![]
        } else {
            self.outputs()
        }
    }
    fn tick(&mut self) -> Vec<GameOutput> {
        if self.tick {
            self.outputs()
        } else {
            vec![]
        }
    }
    fn aborted(&mut self) -> Vec<GameOutput> {
        vec![]
    }
    fn needs_tick(&self) -> bool {
        self.tick
    }
}

impl DoubleTerminal {
    fn outputs(&self) -> Vec<GameOutput> {
        vec![
            GameOutput::Victory,
            if self.second_victory {
                GameOutput::Victory
            } else {
                GameOutput::Failure
            },
        ]
    }
}

#[tokio::test]
async fn terminal_outcome_first_report_wins_on_message_and_tick() {
    for tick in [false, true] {
        for second_victory in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut client = TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap();
            let (server, _) = listener.accept().await.unwrap();
            let registry = SessionRegistry::new();
            let ticket = registry
                .register(
                    4200,
                    7,
                    "Hack".into(),
                    1,
                    1,
                    0,
                    0,
                    0,
                    1,
                    vec![1017],
                    vec![],
                    None,
                    None,
                    9,
                )
                .await
                .unwrap();
            let session = registry
                .authenticate_and_claim(4200, &ticket, "Hack")
                .await
                .unwrap();
            let (tx, mut rx) = mpsc::channel(4);
            let reg = registry.clone();
            let task = tokio::spawn(async move {
                session_loop::run_session(
                    server,
                    &reg,
                    &tx,
                    session,
                    Box::new(DoubleTerminal {
                        tick,
                        second_victory,
                    }),
                    ReadState::empty(),
                    Duration::from_secs(5),
                )
                .await
            });
            super::tests::read_until_game_begin(&mut client).await;
            if !tick {
                let frame = "<msg t='xt'><body action='xtReq'><![CDATA[<dataObj><var n='cmd' t='s'>go</var></dataObj>]]></body></msg>";
                send_null_terminated(&mut client, frame).await.unwrap();
            }
            let end = tokio::time::timeout(Duration::from_secs(5), task)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(end.reason, EndReason::Victory);
            assert!(
                matches!(rx.try_recv(), Ok(CellToBaseMsg::MinigameResult { ticket: got }) if got == ticket)
            );
            assert!(
                rx.try_recv().is_err(),
                "later terminal output must be ignored"
            );
            assert_eq!(
                registry.redeem_result(&ticket).await.unwrap().result_code,
                1
            );
        }
    }
}

#[tokio::test]
async fn terminal_outcome_refusal_and_delivery_failure_never_report_victory() {
    for refuse in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        let registry = SessionRegistry::new();
        let ticket = registry
            .register(
                4201,
                7,
                "Hack".into(),
                1,
                1,
                0,
                0,
                0,
                1,
                vec![1017],
                vec![],
                None,
                None,
                9,
            )
            .await
            .unwrap();
        let session = registry
            .authenticate_and_claim(4201, &ticket, "Hack")
            .await
            .unwrap();
        if refuse {
            assert!(registry.deposit_result(&session, 1, true).await);
            assert!(registry.redeem_result(&ticket).await.is_some());
        }
        let (tx, mut rx) = mpsc::channel(4);
        if !refuse {
            rx.close();
        }
        let reg = registry.clone();
        let task = tokio::spawn(async move {
            session_loop::run_session(
                server,
                &reg,
                &tx,
                session,
                Box::new(DoubleTerminal {
                    tick: false,
                    second_victory: false,
                }),
                ReadState::empty(),
                Duration::from_secs(5),
            )
            .await
        });
        super::tests::read_until_game_begin(&mut client).await;
        registry
            .publish_board(4201, &ticket, published_board(4201))
            .await;
        let frame = "<msg t='xt'><body action='xtReq'><![CDATA[<dataObj><var n='cmd' t='s'>go</var></dataObj>]]></body></msg>";
        send_null_terminated(&mut client, frame).await.unwrap();
        let end = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        let expected = if refuse {
            EndReason::ResultRefused
        } else {
            EndReason::ResultDeliveryFailed
        };
        assert_eq!(end.reason, expected);
        assert_eq!(end.reason.outcome(), "canceled");
        assert_eq!(
            end.reason.as_str(),
            if refuse {
                "result_refused"
            } else {
                "result_delivery_failed"
            }
        );
        assert!(rx.try_recv().is_err());
        assert!(registry.redeem_result(&ticket).await.is_none());
        let board = registry.board_snapshot(4201).await.unwrap();
        assert!(board
            .events
            .iter()
            .any(|event| event.event == "board_started"));
        assert!(!board
            .events
            .iter()
            .any(|event| { matches!(event.event.as_str(), "result_victory" | "result_defeat") }));
    }
}

#[tokio::test]
async fn terminal_outcome_cancel_first_refusal_uses_lock_time_cancel_state() {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4203,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![1017],
            vec![],
            None,
            None,
            9,
        )
        .await
        .unwrap();
    let session = registry
        .authenticate_and_claim(4203, &ticket, "Hack")
        .await
        .unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let guard = registry.hold_registry_lock_for_test().await;
    let reg = registry.clone();
    let (cancel_queued, cancel) = queue_behind_lock(async move { reg.cancel(4203).await });
    tokio::time::timeout(Duration::from_secs(1), cancel_queued)
        .await
        .unwrap()
        .unwrap();
    let reg = registry.clone();
    let (terminal_queued, terminal) = queue_behind_lock(async move {
        report_terminal(&reg, &tx, &session, 1, "victory_message").await
    });
    tokio::time::timeout(Duration::from_secs(1), terminal_queued)
        .await
        .unwrap()
        .unwrap();
    drop(guard);
    assert!(tokio::time::timeout(Duration::from_secs(1), cancel)
        .await
        .unwrap()
        .unwrap());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), terminal)
            .await
            .unwrap()
            .unwrap(),
        EndReason::PlayerCanceled
    );
    assert!(rx.try_recv().is_err());
    assert!(registry.redeem_result(&ticket).await.is_none());
}

#[tokio::test]
async fn terminal_outcome_cancel_after_victory_keeps_the_message_but_drops_the_receipt() {
    let capture = LogCapture::install();
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
            vec![],
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
    let (tx, mut rx) = mpsc::channel(1);
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(1),
            report_terminal(&registry, &tx, &session, 1, "victory_message")
        )
        .await
        .unwrap(),
        EndReason::Victory
    );
    assert!(registry
        .board_snapshot(4204)
        .await
        .unwrap()
        .events
        .iter()
        .any(|event| event.event == "result_victory"));
    assert!(registry.cancel(4204).await);
    let row = capture
        .find_message(
            tracing::Level::WARN,
            "Minigame victory receipt dropped before redemption; its chains will not fire",
        )
        .expect("a canceled victory receipt must log a WARN");
    assert!(
        row.has_field("reason", "victory_receipt_invalidated"),
        "{row:#?}"
    );
    assert!(row.has_field("game", "Hack"), "{row:#?}");
    assert!(
        matches!(rx.try_recv(), Ok(CellToBaseMsg::MinigameResult { ticket: queued }) if queued == ticket)
    );
    assert!(registry.redeem_result(&ticket).await.is_none());
}

#[tokio::test(start_paused = true)]
async fn terminal_outcome_production_close_logs_failed_results_as_canceled() {
    for refuse in [false, true] {
        let capture = LogCapture::install();
        let registry = SessionRegistry::new();
        let ticket = registry
            .register(
                4202,
                7,
                "Hack".into(),
                1,
                1,
                0,
                0,
                0,
                1,
                vec![1017],
                vec![],
                None,
                None,
                9,
            )
            .await
            .unwrap();
        let (mut client, server, peer) = super::listener_tests::loopback_pair().await;
        let (tx, rx) = mpsc::channel(4);
        if !refuse {
            drop(rx);
        }
        let permit = super::admission_tests::test_permit(peer).await;
        let reg = registry.clone();
        let task = tokio::spawn(handle_connection(
            server,
            peer,
            permit,
            reg,
            tx,
            9339,
            ListenerLimits::default(),
        ));
        super::listener_tests::send_handshake(&mut client, 4202, &ticket).await;
        super::tests::read_until_game_begin(&mut client).await;
        if refuse {
            registry.remove(4202).await;
        }
        // A placeholder's `victory` counts only after 5 s of play (D-MG13).
        tokio::time::advance(Duration::from_secs(5)).await;
        let victory = "<msg t='xt'><body action='xtReq'><![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]></body></msg>";
        send_null_terminated(&mut client, victory).await.unwrap();
        task.await.unwrap();
        let row = capture
            .find_message(tracing::Level::INFO, "Minigame session ended")
            .unwrap();
        assert!(
            row.has_field(
                "end_reason",
                if refuse {
                    "result_refused"
                } else {
                    "result_delivery_failed"
                }
            ),
            "{row:#?}"
        );
        assert!(row.has_field("outcome", "canceled"), "{row:#?}");
    }
}
