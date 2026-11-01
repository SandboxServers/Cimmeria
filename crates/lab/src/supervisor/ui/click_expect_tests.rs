use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{click_evidence, ClickExpect, ExpectBefore};
use crate::proof::{EvVerdict, Why};
use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};

/// A fake client whose every `lua_eval` (a visibility read or a poll)
/// answers `done` from `polls`, given the running 0-based count of calls.
/// Returns the count of `lua_eval` calls.
async fn fake(
    polls: impl Fn(usize) -> bool + Send + Sync + 'static,
) -> (crate::supervisor::Supervisor, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let responder: Responder = Arc::new(move |method, _: &Value| {
        if method != "lua_eval" {
            return Err(format!("unexpected call: {method}"));
        }
        let n = seen.fetch_add(1, Ordering::SeqCst);
        Ok(lua_ok(&[json!({ "done": polls(n) }).to_string()]))
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
    let (sup, _) = fake(|n| n >= 1).await;
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
    let (sup, _) = fake(|_| false).await;
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
    let (sup, calls) = fake(|_| true).await;
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
    let (sup, _) = fake(|_| false).await;
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
    let (sup, calls) = fake(|_| true).await;
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

#[tokio::test]
async fn expected_window_is_read_through_get_window() {
    // Only the `getWindow` branch of `visible_chunk` sees a name that is
    // not a Lua global, such as a slash path.
    let responder: Responder = Arc::new(|method, params: &Value| {
        if method != "lua_eval" {
            return Err(format!("unexpected call: {method}"));
        }
        let chunk = params["chunk"].as_str().unwrap_or_default();
        let done = chunk.contains("getWindow");
        Ok(lua_ok(&[json!({ "done": done }).to_string()]))
    });
    let sup = fake_bridge::supervisor(responder).await;
    let e = ClickExpect::from_args(Some("VendorWin/BuyButton".into()), None, None);
    let before = ExpectBefore {
        visible: Some(false),
        hidden: None,
    };
    let checks = sup.expect_after(&e, &before).await;
    assert_eq!(checks[0].ok, Some(true));
}
