use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{click_evidence, ClickExpect, ExpectBefore};
use crate::proof::{EvVerdict, Why};
use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};

/// A fake client whose visibility reads answer `done` from `visible_now`
/// and whose waits (poll chunks) answer `met` from `polls`. Returns the
/// count of `lua_eval` calls.
async fn fake(
    visible_now: bool,
    polls: impl Fn(usize) -> bool + Send + Sync + 'static,
) -> (crate::supervisor::Supervisor, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let poll_count = Arc::new(AtomicUsize::new(0));
    let responder: Responder = Arc::new(move |method, params: &Value| {
        if method != "lua_eval" {
            return Err(format!("unexpected call: {method}"));
        }
        seen.fetch_add(1, Ordering::SeqCst);
        let chunk = params["chunk"].as_str().unwrap_or_default();
        if chunk.contains("done = ") {
            let done = json!({ "done": visible_now });
            return Ok(lua_ok(&[done.to_string()]));
        }
        let n = poll_count.fetch_add(1, Ordering::SeqCst);
        let met = polls(n);
        Ok(lua_ok(&[met.to_string(), String::new(), String::new()]))
    });
    (fake_bridge::supervisor(responder).await, calls)
}

fn expect_visible(timeout_ms: Option<u64>) -> ClickExpect {
    ClickExpect::from_args(Some("VendorWin".into()), None, timeout_ms)
}

#[test]
fn no_expectation_is_posted_only() {
    let e = click_evidence(Some("Btn"), "Btn", None, vec![]);
    assert_eq!(e.verdict, EvVerdict::Inconclusive);
    assert_eq!(e.why, Why::PostedOnly);
    assert_eq!(e.checks.len(), 1);
    assert!(!e.checks[0].required);
}

#[test]
fn unselected_row_denies() {
    let e = click_evidence(Some("List"), "List", Some(false), vec![]);
    assert_eq!(e.verdict, EvVerdict::Denied);
    assert_eq!(e.why, Why::NoEffect);
}

#[test]
fn timeout_is_clamped() {
    let t = |ms| ClickExpect::from_args(None, None, ms).timeout;
    assert_eq!(t(Some(60_000)), Duration::from_secs(10));
    assert_eq!(t(Some(10)), Duration::from_millis(200));
    assert_eq!(t(None), Duration::from_secs(2));
    assert!(ClickExpect::from_args(None, None, None).is_empty());
}

#[tokio::test]
async fn expected_window_appearing_confirms() {
    // The first wait poll sees it hidden, the second sees it shown.
    let (sup, _) = fake(false, |n| n >= 1).await;
    let e = expect_visible(None);
    let before = ExpectBefore {
        visible: Some(false),
        hidden: None,
    };
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].name, "window_visible");
    assert_eq!(checks[0].ok, Some(true));
    let ev = click_evidence(Some("Btn"), "Btn", None, checks);
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
}

#[tokio::test]
async fn expected_window_never_appearing_denies() {
    let (sup, _) = fake(false, |_| false).await;
    let e = expect_visible(Some(300));
    let before = ExpectBefore {
        visible: Some(false),
        hidden: None,
    };
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(checks[0].ok, Some(false));
    assert_eq!(checks[0].why, Some(Why::NoEffect));
    let ev = click_evidence(Some("Btn"), "Btn", None, checks);
    assert_eq!(ev.verdict, EvVerdict::Denied);
}

#[tokio::test]
async fn already_visible_is_already_true() {
    let (sup, calls) = fake(true, |_| true).await;
    let e = expect_visible(None);
    // The before-read is the one lua_eval; the expectation adds none.
    let before = sup.expect_before(&e).await;
    assert_eq!(before.visible, Some(true));
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(checks[0].ok, None);
    assert_eq!(checks[0].why, Some(Why::AlreadyTrue));
    let ev = click_evidence(Some("Btn"), "Btn", None, checks);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
}

#[tokio::test]
async fn hidden_mirror() {
    let e = ClickExpect::from_args(None, Some("DialogWin".into()), None);
    // Before visible, then gone: the wait sees it hidden at once.
    let (sup, _) = fake(true, |_| true).await;
    let before = ExpectBefore {
        visible: None,
        hidden: Some(true),
    };
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(checks[0].name, "window_hidden");
    assert_eq!(
        click_evidence(Some("Btn"), "Btn", None, checks).verdict,
        EvVerdict::Confirmed
    );
    // Already hidden before the click: already_true, no wait.
    let (sup, calls) = fake(false, |_| true).await;
    let before = ExpectBefore {
        visible: None,
        hidden: Some(false),
    };
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(checks[0].why, Some(Why::AlreadyTrue));
    assert_eq!(
        click_evidence(Some("Btn"), "Btn", None, checks).verdict,
        EvVerdict::Inconclusive
    );
}
