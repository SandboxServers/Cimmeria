//! The browser flow the driver may run (C13): one at a time, and `cancel` ends
//! the wait. The handle of the flow in progress lives here, so a flow that
//! finishes after a cancel clears only its own handle.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::commands::SignInFailure;
use super::driver::{guard, Browser};
use super::oauth::{BrowserFlow, CodeGrant};
use super::SignInError;

/// The browser flow handle of one driver.
pub(super) struct FlowSlot {
    /// Cancels the browser flow in progress, with that flow's number. `Some` only
    /// while a flow waits.
    waiting: Mutex<Option<(u64, tokio::sync::oneshot::Sender<()>)>>,
    /// The number the next browser flow takes, so a finished flow clears only its own handle.
    generation: AtomicU64,
}

impl FlowSlot {
    /// A slot with no flow waiting.
    pub(super) fn new() -> Self {
        Self {
            waiting: Mutex::new(None),
            generation: AtomicU64::new(0),
        }
    }

    /// Opens the browser flow `start` makes and waits for its redirect. One flow
    /// runs at a time, so a second call is `Busy`. `cancel` ends the wait.
    pub(super) async fn run<F>(
        &self,
        browser: &dyn Browser,
        start: F,
    ) -> Result<CodeGrant, SignInFailure>
    where
        F: Future<Output = Result<BrowserFlow, SignInError>>,
    {
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
        let mine = {
            let mut waiting = guard(&self.waiting)?;
            if waiting.is_some() {
                return Err(SignInFailure::Busy);
            }
            let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
            *waiting = Some((mine, cancel_tx));
            mine
        };
        let result: Result<CodeGrant, SignInFailure> = async {
            let flow = start.await.map_err(SignInFailure::from)?;
            if cancel_rx.try_recv().is_ok() {
                return Err(SignInFailure::Cancelled);
            }
            if browser.open(&flow.authorize_url).is_err() {
                return Err(SignInFailure::BrowserFailed {
                    url: flow.authorize_url.clone(),
                });
            }
            tracing::info!(target: "launcher.sign_in", event = "started", outcome = "opened", reason = "browser_opened");
            tokio::select! {
                grant = flow.finish() => grant.map_err(SignInFailure::from),
                _ = &mut cancel_rx => Err(SignInFailure::Cancelled),
            }
        }
        .await;
        if let Err(failure) = &result {
            let reason = match failure {
                SignInFailure::BrowserFailed { .. } => "browser_failed",
                SignInFailure::Cancelled => "cancelled",
                SignInFailure::Denied => "denied",
                SignInFailure::TimedOut => "timed_out",
                SignInFailure::Offline => "offline",
                _ => "flow_failed",
            };
            tracing::info!(target: "launcher.sign_in", event = "failed", outcome = "failed", reason);
        }
        if let Ok(mut waiting) = self.waiting.lock() {
            // A cancelled flow may have been replaced by a newer one: clear only our own handle.
            if matches!(*waiting, Some((flow, _)) if flow == mine) {
                *waiting = None;
            }
        }
        result
    }

    /// Ends the browser flow in progress, if one waits.
    pub(super) fn cancel(&self) -> Result<(), SignInFailure> {
        let waiting = guard(&self.waiting)?.take();
        if let Some((_, cancel)) = waiting {
            let _ = cancel.send(());
        }
        Ok(())
    }

    /// Whether a browser flow is waiting for the player's redirect.
    pub(super) fn is_waiting(&self) -> bool {
        self.waiting.lock().is_ok_and(|waiting| waiting.is_some())
    }
}
