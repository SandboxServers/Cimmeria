//! The browser flow the driver may run (C13): one at a time, and `cancel` ends
//! the wait. The handle of the flow in progress lives here, so a flow that
//! finishes after a cancel clears only its own handle.
//!
//! A flow whose browser would not open is parked (CT-89): its listener and
//! verifier wait for `resume`, for at most [`SIGN_IN_TIMEOUT`], and `cancel`
//! or a sign-out drops it.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

use url::Url;

use super::commands::SignInFailure;
use super::driver::{guard, Browser};
use super::oauth::{BrowserFlow, CodeGrant, SIGN_IN_TIMEOUT};
use super::session::SignIn;
use super::SignInError;

/// What a parked flow completes once its grant arrives.
pub(super) enum Continue {
    /// A plain sign-in with `provider` on the server at `base`.
    Begin { base: Url, provider: String },
    /// Adding `provider` as a method to the signed-in `session`.
    AddMethod {
        session: Arc<SignIn>,
        provider: String,
    },
}

/// A browser flow whose browser would not open, kept for `resume` (CT-89).
struct Parked {
    /// The number of the flow's `waiting` handle.
    mine: u64,
    flow: BrowserFlow,
    cancel_rx: tokio::sync::oneshot::Receiver<()>,
    /// The operation epoch the original command captured (CT-87).
    epoch: u64,
    cont: Continue,
    /// When the flow was parked; its five minutes run from here.
    since: Instant,
}

/// The browser flow handle of one driver.
pub(super) struct FlowSlot {
    /// Cancels the browser flow in progress, with that flow's number. `Some` while
    /// a flow waits, parked or not.
    waiting: Mutex<Option<(u64, tokio::sync::oneshot::Sender<()>)>>,
    /// The flow parked by a browser that would not open, until resumed or dropped.
    parked: Mutex<Option<Parked>>,
    /// The number the next browser flow takes, so a finished flow clears only its own handle.
    generation: AtomicU64,
}

impl FlowSlot {
    /// A slot with no flow waiting.
    pub(super) fn new() -> Self {
        Self {
            waiting: Mutex::new(None),
            parked: Mutex::new(None),
            generation: AtomicU64::new(0),
        }
    }

    /// Opens the browser flow `start` makes and waits for its redirect. One flow
    /// runs at a time, so a second call is `Busy`. `cancel` ends the wait. Returns
    /// the grant with `cont` and `epoch`, which the caller completes. If the
    /// browser does not open, the flow parks and the call fails `BrowserFailed`.
    pub(super) async fn run<F>(
        &self,
        browser: &dyn Browser,
        start: F,
        cont: Continue,
        epoch: u64,
    ) -> Result<(CodeGrant, Continue, u64), SignInFailure>
    where
        F: Future<Output = Result<BrowserFlow, SignInError>>,
    {
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
        self.expire_parked();
        let mine = {
            let mut waiting = guard(&self.waiting)?;
            if waiting.is_some() {
                return Err(SignInFailure::Busy);
            }
            let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
            *waiting = Some((mine, cancel_tx));
            mine
        };
        let result: Result<(CodeGrant, Continue, u64), SignInFailure> = match start.await {
            Err(e) => Err(SignInFailure::from(e)),
            Ok(flow) => {
                if cancel_rx.try_recv().is_ok() {
                    Err(SignInFailure::Cancelled)
                } else if browser.open(&flow.authorize_url).is_err() {
                    // The flow keeps its listener and its `waiting` handle: `resume` finishes it.
                    let url = flow.authorize_url.clone();
                    let failure = SignInFailure::BrowserFailed { url: url.clone() };
                    log_failure(&failure);
                    self.park(Parked {
                        mine,
                        flow,
                        cancel_rx,
                        epoch,
                        cont,
                        since: Instant::now(),
                    });
                    return Err(failure);
                } else {
                    tracing::info!(target: "launcher.sign_in", event = "started", outcome = "opened", reason = "browser_opened");
                    tokio::select! {
                        grant = flow.finish() => grant
                            .map(|grant| (grant, cont, epoch))
                            .map_err(SignInFailure::from),
                        _ = &mut cancel_rx => Err(SignInFailure::Cancelled),
                    }
                }
            }
        };
        if let Err(failure) = &result {
            log_failure(failure);
        }
        self.clear_waiting(mine);
        result
    }

    /// Finishes the parked flow: its redirect is awaited for what is left of its
    /// five minutes, and the grant comes back with the continuation. Nothing parked
    /// is `NoPendingFlow`; a flow past its limit is `TimedOut`.
    pub(super) async fn resume(&self) -> Result<(CodeGrant, Continue, u64), SignInFailure> {
        let parked = guard(&self.parked)?
            .take()
            .ok_or(SignInFailure::NoPendingFlow)?;
        let Parked {
            mine,
            flow,
            mut cancel_rx,
            epoch,
            cont,
            since,
        } = parked;
        let result = if since.elapsed() >= SIGN_IN_TIMEOUT {
            Err(SignInFailure::TimedOut)
        } else {
            tokio::select! {
                grant = flow.finish_within(SIGN_IN_TIMEOUT - since.elapsed()) => grant
                    .map(|grant| (grant, cont, epoch))
                    .map_err(SignInFailure::from),
                _ = &mut cancel_rx => Err(SignInFailure::Cancelled),
            }
        };
        if let Err(failure) = &result {
            log_failure(failure);
        }
        self.clear_waiting(mine);
        result
    }

    /// Ends the browser flow in progress, if one waits, and drops a parked flow:
    /// its listener closes.
    pub(super) fn cancel(&self) -> Result<(), SignInFailure> {
        let parked = guard(&self.parked)?.take();
        drop(parked);
        let waiting = guard(&self.waiting)?.take();
        if let Some((_, cancel)) = waiting {
            let _ = cancel.send(());
        }
        Ok(())
    }

    /// Whether a browser flow is waiting for the player's redirect, parked or not.
    pub(super) fn is_waiting(&self) -> bool {
        self.expire_parked();
        self.waiting.lock().is_ok_and(|waiting| waiting.is_some())
    }

    /// Keeps `flow` parked until `resume`, `cancel` or its five-minute limit.
    fn park(&self, parked: Parked) {
        if let Ok(mut slot) = self.parked.lock() {
            *slot = Some(parked);
        }
    }

    /// Drops a parked flow older than [`SIGN_IN_TIMEOUT`], and its `waiting` handle.
    fn expire_parked(&self) {
        let expired = match self.parked.lock() {
            Ok(mut slot)
                if slot
                    .as_ref()
                    .is_some_and(|p| p.since.elapsed() >= SIGN_IN_TIMEOUT) =>
            {
                slot.take()
            }
            _ => None,
        };
        if let Some(parked) = expired {
            self.clear_waiting(parked.mine);
        }
    }

    /// Clears the `waiting` handle of flow `mine`. A cancelled flow may have been
    /// replaced by a newer one, so only our own handle goes.
    fn clear_waiting(&self, mine: u64) {
        if let Ok(mut waiting) = self.waiting.lock() {
            if matches!(*waiting, Some((flow, _)) if flow == mine) {
                *waiting = None;
            }
        }
    }

    /// Moves the parked flow's `since` back by `by`, so a test can see it expire.
    #[cfg(test)]
    pub(super) fn backdate_parked(&self, by: Duration) {
        if let Ok(mut slot) = self.parked.lock() {
            if let Some(parked) = slot.as_mut() {
                parked.since = parked.since.checked_sub(by).unwrap_or(parked.since);
            }
        }
    }
}

/// Logs a failed browser flow with the reason it failed for.
fn log_failure(failure: &SignInFailure) {
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
