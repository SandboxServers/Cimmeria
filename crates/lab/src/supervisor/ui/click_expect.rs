//! A click judged by the window it was meant to change (AX-40).
//!
//! A click proves only that the cursor was over its target. A caller that
//! names the window the click should show or hide gets a check for it: the
//! window is read before the click and waited for after it. Without one the
//! result is inconclusive with `posted_only`.

use std::time::Duration;

use serde_json::json;

use super::click_checked::visible_chunk;
use super::Supervisor;
use crate::proof::{Check, EvVerdict, Evidence, Source, Why};

/// Default wait for an expected window, ms.
const DEFAULT_TIMEOUT_MS: u64 = 2_000;
/// Shortest and longest wait a caller may ask for, ms.
const TIMEOUT_RANGE_MS: (u64, u64) = (200, 10_000);

/// What a click should change: the windows that must show or vanish.
#[derive(Debug, Clone, Default)]
pub struct ClickExpect {
    /// This window must be visible after the click.
    pub visible: Option<String>,
    /// This window must not be visible after the click.
    pub hidden: Option<String>,
    /// How long to wait for either window; set by [`ClickExpect::from_args`].
    pub timeout: Duration,
}

impl ClickExpect {
    /// The expectation the tool arguments name; the timeout is clamped to
    /// 200 ms..=10 s and defaults to 2 s.
    pub fn from_args(
        visible: Option<String>,
        hidden: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        let (lo, hi) = TIMEOUT_RANGE_MS;
        let ms = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).clamp(lo, hi);
        Self {
            visible,
            hidden,
            timeout: Duration::from_millis(ms),
        }
    }

    /// Does the click expect nothing (so nothing is read or waited for)?
    pub fn is_empty(&self) -> bool {
        self.visible.is_none() && self.hidden.is_none()
    }
}

/// What the expected windows looked like before the click: each field is
/// whether that window was visible, `None` when the read failed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExpectBefore {
    /// Was the `visible` window visible?
    pub visible: Option<bool>,
    /// Was the `hidden` window visible?
    pub hidden: Option<bool>,
}

impl Supervisor {
    /// Read each expected window's visibility before the click
    /// (`click_checked::visible_chunk`). An empty expectation reads nothing.
    pub async fn expect_before(&self, e: &ClickExpect) -> ExpectBefore {
        if e.is_empty() {
            return ExpectBefore::default();
        }
        ExpectBefore {
            visible: self.read_visible(e.visible.as_deref()).await,
            hidden: self.read_visible(e.hidden.as_deref()).await,
        }
    }

    /// The checks for the expected windows after the click.
    pub async fn expect_after(&self, e: &ClickExpect, before: &ExpectBefore) -> Vec<Check> {
        let mut checks = Vec::new();
        if let Some(w) = &e.visible {
            checks.push(self.expect_one(w, false, before.visible, e.timeout).await);
        }
        if let Some(w) = &e.hidden {
            checks.push(self.expect_one(w, true, before.hidden, e.timeout).await);
        }
        checks
    }

    /// Visibility of `window` now, or `None` when there is no window or the
    /// read failed.
    async fn read_visible(&self, window: Option<&str>) -> Option<bool> {
        let w = window?;
        let v = self.lua_json(&visible_chunk(w)).await.ok()?;
        v["done"].as_bool()
    }

    /// One expected window. `gone` expects it hidden: the check then waits
    /// for it to vanish. A window already in the expected state is not
    /// waited for; that proves nothing, so the check is `already_true`.
    async fn expect_one(
        &self,
        window: &str,
        gone: bool,
        before: Option<bool>,
        timeout: Duration,
    ) -> Check {
        let name = if gone {
            "window_hidden"
        } else {
            "window_visible"
        };
        if before == Some(!gone) {
            return Check::new(name, Source::Client, None, true)
                .with_before(json!(before))
                .with_why(Why::AlreadyTrue);
        }
        match self.wait_window(window, gone, timeout).await {
            Ok(ms) => Check::new(name, Source::Client, Some(true), true)
                .with_before(json!(before))
                .with_after(json!({ "visible": !gone, "ms": ms })),
            Err(e) => Check::new(name, Source::Client, Some(false), true)
                .with_before(json!(before))
                .with_after(json!({ "visible": gone, "error": e }))
                .with_why(Why::NoEffect),
        }
    }
}

/// Pure: the evidence of one click. `received_by` is the window under the
/// cursor, `row_selected` is `Some` only for a row click, `expect` holds the
/// checks from [`Supervisor::expect_after`].
pub fn click_evidence(
    received_by: Option<&str>,
    target: &str,
    row_selected: Option<bool>,
    expect: Vec<Check>,
) -> Evidence {
    let mut checks = vec![Check::new(
        "cursor_on_target",
        Source::Client,
        Some(received_by.is_some()),
        false,
    )
    .with_before(json!(target))
    .with_after(json!(received_by))];
    if let Some(sel) = row_selected {
        let check = Check::new("row_selected", Source::Client, Some(sel), true);
        checks.push(if sel {
            check
        } else {
            check.with_why(Why::NoEffect)
        });
    }
    checks.extend(expect);
    if checks.iter().any(|c| c.required) {
        Evidence::judge(checks)
    } else {
        Evidence::from_verdict(EvVerdict::Inconclusive, Why::PostedOnly, checks)
    }
}

#[cfg(test)]
#[path = "click_expect_tests.rs"]
mod tests;
