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
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let ((), events) = capture(|| {
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
