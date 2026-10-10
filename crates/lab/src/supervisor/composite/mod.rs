//! Composite tools: one call for a sequence an agent used to spend a turn
//! per step on. Each turn of a lab-driving agent re-sends its whole
//! context, so fewer calls is the largest saving there is.
//!
//! - [`ensure`] — `lab_ensure_in_world`: start, wait for the window and
//!   bridge, log in, pick or create the character, play, finish the intro
//!   dialog, virtual focus. Idempotent.
//! - [`batch`] — `client_batch`: ordered read and probe steps with
//!   references to earlier results.
//! - [`sequence`] — `client_ui_sequence`: clicks, keys, typing, drags and
//!   window waits for one scripted UI step.
//!
//! A composite is admitted under one lease, like every guarded tool, and
//! each bridge call and posted input inside it renews that lease
//! ([`crate::lease::permit`]); [`Supervisor::idle`] renews it through long
//! waits that make no call.

pub mod batch;
pub mod ensure;
pub mod sequence;

use std::time::{Duration, Instant};

use super::{process, Supervisor};
use crate::supervisor::flows::widgets;

/// How often a long idle wait re-checks (and so renews) the lease.
const IDLE_SLICE: Duration = Duration::from_secs(5);

impl Supervisor {
    /// Sleep for `d`, re-checking the lease every few seconds so a long
    /// wait keeps it and a revoked lease ends the wait.
    pub async fn idle(&self, d: Duration) -> Result<(), String> {
        let end = Instant::now() + d;
        loop {
            crate::lease::permit::ensure("wait")?;
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(());
            }
            tokio::time::sleep(left.min(IDLE_SLICE)).await;
        }
    }

    /// Wait until the client's main loop has ticked `n` more times (the
    /// bridge heartbeat counts ticks).
    pub async fn wait_frames(&self, n: u64) -> Result<(), String> {
        let start = self
            .bridge
            .heartbeat()
            .await
            .map_err(|e| format!("heartbeat: {e}"))?;
        let deadline = Instant::now() + Duration::from_millis((n * 200).clamp(2_000, 30_000));
        loop {
            crate::lease::permit::ensure("wait frames")?;
            let now = self
                .bridge
                .heartbeat()
                .await
                .map_err(|e| format!("heartbeat: {e}"))?;
            if now >= start + n {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!("only {} of {n} frames ticked", now - start));
            }
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }

    /// Wait for the client's main window to exist (a fresh client takes
    /// about 30 s to show it). Returns how long it took.
    pub async fn wait_main_window(&self, timeout: Duration) -> Result<u64, String> {
        let t0 = Instant::now();
        loop {
            let pid = self.state.lock().await.pid.ok_or("no client running")?;
            if !process::is_alive(pid) {
                return Err(format!("the client (pid {pid}) exited"));
            }
            let found = tokio::task::spawn_blocking(move || process::find_main_window(pid))
                .await
                .map_err(|e| format!("window lookup: {e}"))?;
            if found.is_some() {
                return Ok(t0.elapsed().as_millis() as u64);
            }
            if t0.elapsed() > timeout {
                return Err(format!(
                    "the client has no main window after {} ms",
                    t0.elapsed().as_millis()
                ));
            }
            self.idle(Duration::from_millis(500)).await?;
        }
    }

    /// Wait until `window` is visible (or, with `gone`, not visible).
    pub async fn wait_window(
        &self,
        window: &str,
        gone: bool,
        timeout: Duration,
    ) -> Result<u64, String> {
        let vis = widgets::visible(window);
        let cond = if gone { format!("not ({vis})") } else { vis };
        let w = self
            .poll_until(&cond, None, timeout, Duration::from_millis(250))
            .await?;
        if w.met {
            Ok(w.elapsed_ms)
        } else {
            Err(format!(
                "{window} {} after {} ms",
                if gone { "still visible" } else { "not visible" },
                w.elapsed_ms
            ))
        }
    }
}
