//! [`RunLease`]: the lease one `lab_uat_run` drives under, the caller's or
//! its own.

use std::sync::Arc;

use super::{AcquireRequest, LeaseBook};

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
