//! [`RunLease`]: the lease one `lab_uat_run` drives under, the caller's or
//! its own.

use std::sync::Arc;

use super::{AcquireRequest, LeaseBook};
use crate::supervisor::now_ms;

/// Owner name of the lease `lab_uat_run` takes when the caller passes none.
pub const UAT_RUN_OWNER: &str = "lab_uat_run";

/// The lease a `lab_uat_run` drives under. The runner calls tools in-process
/// (around the `call_tool` gate), so it touches this lease before every
/// step instead: that renews it through a long run, and stops the run at the
/// next step if someone took the lease over.
#[derive(Debug)]
pub struct RunLease {
    book: Arc<LeaseBook>,
    id: String,
    /// Acquired by the run itself: released when the run ends.
    own: bool,
}

impl RunLease {
    /// The caller's lease (already checked by the gate).
    pub fn caller(book: Arc<LeaseBook>, id: String) -> Self {
        Self {
            book,
            id,
            own: false,
        }
    }

    /// A lease of the run's own, owner [`UAT_RUN_OWNER`]; refused like any
    /// acquire while someone else holds the lab.
    pub fn acquire_own(book: Arc<LeaseBook>, purpose: String) -> Result<Self, String> {
        let l = book.acquire(AcquireRequest {
            owner: UAT_RUN_OWNER.into(),
            purpose,
            ..Default::default()
        })?;
        Ok(Self {
            book,
            id: l.lease_id,
            own: true,
        })
    }

    /// Before each step: renew, or say why the run no longer holds the lab.
    pub fn touch(&self, tool: &str) -> Result<(), String> {
        self.book.check(Some(&self.id), tool)
    }

    /// Whether the run acquired this lease itself.
    pub fn is_own(&self) -> bool {
        self.own
    }

    /// The permit the run's own actions are checked under.
    pub fn permit(&self) -> super::permit::Permit {
        super::permit::Permit::Lease {
            book: self.book.clone(),
            id: self.id.clone(),
        }
    }

    /// Keep this lease alive for as long as the returned guard lives, and
    /// report its loss. A run is not only tool calls: a spec's `wait_ms`
    /// step sleeps (30 s in committed specs, the minimum ttl), so renewing
    /// on tool calls alone let a caller's short lease lapse mid-run.
    pub fn keep_alive(&self) -> KeepAlive {
        self.keep_alive_with(Arc::new(now_ms))
    }

    /// [`Self::keep_alive`] on an explicit clock (tests).
    pub(crate) fn keep_alive_with(&self, clock: Arc<dyn Fn() -> i64 + Send + Sync>) -> KeepAlive {
        let (tx, rx) = tokio::sync::watch::channel(None::<String>);
        let book = self.book.clone();
        let id = self.id.clone();
        let mut changes = book.subscribe();
        let handle = tokio::spawn(async move {
            loop {
                let ttl = book.ttl_of(&id).unwrap_or(super::MIN_TTL_S);
                tokio::select! {
                    _ = tokio::time::sleep(renew_interval(ttl)) => {}
                    // A takeover or release: look now, not at the next renewal.
                    r = changes.changed() => if r.is_err() { break },
                }
                if let Err(e) = book.check_at(Some(&id), "lab_uat_run keep-alive", clock()) {
                    tracing::warn!(target: "lab.lease", event = "run_lease_lost", reason = %e,
                        "UAT run lost its lease; stopping the run");
                    let _ = tx.send(Some(e));
                    break;
                }
            }
        });
        KeepAlive { handle, rx }
    }
}

/// How often a keep-alive renews a lease of `ttl_s`: three times per ttl,
/// so one late wake-up never lets it lapse.
pub fn renew_interval(ttl_s: u64) -> std::time::Duration {
    std::time::Duration::from_millis((ttl_s * 1000 / 3).max(1000))
}

/// A running keep-alive ([`RunLease::keep_alive`]); stops when dropped.
#[derive(Debug)]
pub struct KeepAlive {
    handle: tokio::task::JoinHandle<()>,
    rx: tokio::sync::watch::Receiver<Option<String>>,
}

impl KeepAlive {
    /// Becomes `Some(reason)` once the lease is lost.
    pub fn revoked(&self) -> tokio::sync::watch::Receiver<Option<String>> {
        self.rx.clone()
    }
}

impl Drop for KeepAlive {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl Drop for RunLease {
    fn drop(&mut self) {
        // Every way out of a run (done, error, panic) releases a lease the
        // run took for itself. A lease taken over meanwhile is not ours.
        if self.own {
            let _ = self.book.release(&self.id);
        }
    }
}
