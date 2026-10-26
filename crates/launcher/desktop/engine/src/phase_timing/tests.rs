//! Paired timing records survive failure, cancellation and unwind without identifiers.
use super::*;
use crate::test_capture::capture;
use std::time::Duration;

#[test]
fn phase_pair_keeps_correlation_workload_and_monotonic_duration() {
    let id = Uuid::new_v4();
    let (answer, events) = capture(|| {
        let mut timer = Context::for_operation(id).start(Phase::AdoptionInitialScan);
        timer.started = Instant::now() - Duration::from_millis(25);
        timer.workload(3, 4096);
        timer.result(Ok::<_, ()>(42))
    });
    assert_eq!(answer, Ok(42));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].field("event"), Some("phase_started"));
    assert_eq!(events[0].field("duration_ms"), None);
    assert_eq!(events[1].field("event"), Some("phase_finished"));
    assert_eq!(events[1].field("outcome"), Some("succeeded"));
    assert_eq!(events[1].field("phase"), Some("adoption_initial_scan"));
    assert_eq!(events[1].field("files"), Some("3"));
    assert_eq!(events[1].field("bytes"), Some("4096"));
    assert!(
        events[1]
            .field("duration_ms")
            .unwrap()
            .parse::<u64>()
            .unwrap()
            >= 25
    );
    for key in ["operation", "phase_sequence"] {
        assert_eq!(events[0].field(key), events[1].field(key));
        assert!(events[0].field(key).unwrap().parse::<u64>().unwrap() > 0);
    }
    assert!(events.iter().all(|e| e.target == "adoption"));
    assert!(!format!("{events:?}").contains(&id.to_string()));
}

#[test]
fn failure_and_unwind_are_not_success_and_do_not_change_the_result() {
    let (result, events) =
        capture(|| Context::preview().measure(Phase::PlayAdmission, || Err::<(), _>("refused")));
    assert_eq!(result, Err("refused"));
    assert_eq!(events[1].field("outcome"), Some("failed"));
    assert_eq!(events[1].target, "launcher.launch");
    assert!(
        !format!("{events:?}").contains("refused"),
        "no arbitrary error text"
    );
    let (_, events) = capture(|| {
        let _ = std::panic::catch_unwind(|| {
            let _timer = Context::preview().start(Phase::AdoptionReplay);
            panic!("fixture panic");
        });
    });
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].field("outcome"), Some("panicked"));
}

#[test]
fn dropping_a_pending_future_finishes_only_its_started_phase() {
    use std::future::Future;
    let (_, events) = capture(|| {
        let mut future = Box::pin(async {
            let _timer = Context::preview().start(Phase::PlayPreviousSession);
            std::future::pending::<()>().await;
        });
        let waker = futures_util::task::noop_waker();
        let mut context = std::task::Context::from_waker(&waker);
        assert!(future.as_mut().poll(&mut context).is_pending());
        drop(future);
    });
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].field("outcome"), Some("interrupted"));
}

#[test]
fn repeated_phases_have_distinct_pairs_under_the_same_operation() {
    let (_, events) = capture(|| {
        let context = Context::preview();
        context
            .start(Phase::PlayResources)
            .finish(Outcome::Succeeded);
        context.start(Phase::PlayResources).finish(Outcome::Failed);
    });
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].field("operation"), events[2].field("operation"));
    assert_ne!(
        events[0].field("phase_sequence"),
        events[2].field("phase_sequence")
    );
}

#[test]
fn the_cooked_presync_phase_has_its_code_and_launch_target() {
    let (_, events) =
        capture(|| Context::preview().measure(Phase::PlayCookedPresync, || Ok::<_, ()>(())));
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].field("phase"), Some("play_cooked_presync"));
    assert_eq!(events[1].field("outcome"), Some("succeeded"));
    assert!(events.iter().all(|e| e.target == "launcher.launch"));
}
