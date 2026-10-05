//! Stopping a run when its lab lease is lost.
//!
//! `lab_uat_run` hands the runner the keep-alive's revocation signal
//! (`lease::run::KeepAlive::revoked`). Once it fires, the row being driven
//! is cut off at its next await (a tool call, a `wait_ms` sleep, a server
//! read) and BLOCKED, and every remaining row is BLOCKED without being
//! driven: a displaced run must not keep acting, nor record the new
//! holder's activity as its own evidence.

use tokio::sync::watch;

use super::Runner;
use crate::uat::invoke::ToolInvoker;

/// The revocation signal: `Some(reason)` once the lease is lost.
pub type Revocation = watch::Receiver<Option<String>>;

/// The reason prefix every row the revocation stopped carries.
pub const REVOKED: &str = "lease revoked";

impl<I: ToolInvoker> Runner<'_, I> {
    /// Stop the run when `rx` reports the lease lost.
    pub fn with_revocation(mut self, rx: Revocation) -> Self {
        self.revoked = Some(rx);
        self
    }

    /// Why the run lost its lease, if it has.
    pub(crate) fn revocation_reason(&self) -> Option<String> {
        self.revoked.as_ref().and_then(|rx| rx.borrow().clone())
    }
}

/// Resolves with the reason once the lease is lost; never, if it is not
/// (or the signal's sender is gone).
pub(crate) async fn revoked(rx: Option<Revocation>) -> String {
    let Some(mut rx) = rx else {
        return std::future::pending().await;
    };
    loop {
        if let Some(r) = rx.borrow_and_update().clone() {
            return r;
        }
        if rx.changed().await.is_err() {
            return std::future::pending().await;
        }
    }
}
