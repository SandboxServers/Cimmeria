//! Launch lifecycle events on a real failing path (launcher-telemetry LT-17a).
//! The helper is replaced, so the first verify fails before the game starts.
use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

fn named<'a>(events: &[&'a Captured], name: &str) -> Vec<&'a Captured> {
    events
        .iter()
        .copied()
        .filter(|e| e.field("event") == Some(name))
        .collect()
}

#[test]
fn a_replaced_helper_logs_launch_not_started_verify_failed() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
    // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(async {
            let (_root, state, plan) = super::tests::fixture();
            std::fs::write(plan.resources.helper.path(), b"replacement").unwrap();
            let state = Arc::new(std::sync::Mutex::new(state));
            let _worker = dispatch(state.clone(), plan.id).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if state
                        .lock()
                        .unwrap()
                        .operations()
                        .snapshot()
                        .operation
                        .as_ref()
                        .unwrap()
                        .state
                        .terminal()
                    {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
        });
    });
    let launch: Vec<&Captured> = events
        .iter()
        .filter(|e| e.target == "launcher.launch")
        .collect();
    let started = named(&launch, "launch_started");
    let not_started = named(&launch, "launch_not_started");
    let finished = named(&launch, "launch_finished");
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].field("telemetry_dll"), Some("false"));
    assert_eq!(not_started.len(), 1);
    assert_eq!(not_started[0].level, Level::WARN);
    assert_eq!(not_started[0].field("reason"), Some("verify_failed"));
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].field("outcome"), Some("not_started"));
    let operation = started[0].field("operation");
    assert!(operation.is_some());
    assert_eq!(not_started[0].field("operation"), operation);
    assert_eq!(finished[0].field("operation"), operation);
}

#[test]
fn an_unreadable_choice_logs_game_telemetry_skipped() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
    // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(async {
            let (root, state, mut plan) = super::tests::fixture();
            plan.resources.client_telemetry = Some(super::tests::artifact(
                &root.path().canonicalize().unwrap(),
                "telemetry.dll",
            ));
            // A directory where the choice file belongs cannot be read as one.
            let choice = state.state_root().join("game-telemetry.json");
            std::fs::create_dir(&choice).unwrap();
            let session =
                super::telemetry::session(&Arc::new(std::sync::Mutex::new(state)), &plan, &[])
                    .await;
            assert!(session.is_none());
        });
    });
    let skipped: Vec<&Captured> = events
        .iter()
        .filter(|e| e.target == "launcher.launch")
        .filter(|e| e.field("event") == Some("game_telemetry_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].level, Level::WARN);
    assert_eq!(skipped[0].field("reason"), Some("choice_unreadable"));
    assert!(skipped[0].field("operation").is_some());
}
