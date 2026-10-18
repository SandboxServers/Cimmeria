//! Session-entry fence for the world-entry continuation.
//!
//! A continuation finalizes one session entry: the key, generation and
//! character it started under. When the session at the same address is
//! replaced while one of its stages is in flight, a send the old continuation
//! makes must not reach the new session's key. [`scoped`] runs a future with an
//! [`EntryGuard`] in scope, and the three send chokepoints in the parent module
//! consult it under the `connected` lock they already hold.
//!
//! The guard applies only to sends whose target is the guard's address. Sends
//! to other players (organization notices, contact fan-out) are never dropped.
//! Work moved onto a `tokio::spawn`ed task does not inherit the scope.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use super::ConnectedClientState;

/// One session entry: the address it was started on, and the check that says
/// whether a session state still belongs to that entry.
pub struct EntryGuard {
    addr: SocketAddr,
    check: Arc<dyn Fn(&ConnectedClientState) -> bool + Send + Sync>,
}

impl EntryGuard {
    /// Guard for sends to `addr`. `check` returns true while the session state
    /// still belongs to the entry.
    pub fn new(
        addr: SocketAddr,
        check: impl Fn(&ConnectedClientState) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            addr,
            check: Arc::new(check),
        }
    }
}

tokio::task_local! {
    static ENTRY_GUARD: EntryGuard;
}

/// Runs `fut` with `guard` in scope for the sends it makes on its own task.
/// The scope ends when the future does.
pub async fn scoped<F: Future>(guard: EntryGuard, fut: F) -> F::Output {
    ENTRY_GUARD.scope(guard, fut).await
}

/// True when a send to `addr` must be dropped: the scope's guard names `addr`
/// and the session state the caller already holds no longer belongs to its
/// entry. Takes the locked state, so it never re-locks `connected`.
pub(crate) fn superseded(addr: SocketAddr, state: &ConnectedClientState) -> bool {
    ENTRY_GUARD
        .try_with(|guard| guard.addr == addr && !(guard.check)(state))
        .unwrap_or(false)
}
