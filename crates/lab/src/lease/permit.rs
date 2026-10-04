//! The lease a running tool acts under, re-checked at every action.
//!
//! The `call_tool` gate admits a guarded call once, but a flow (login,
//! delete a character, walk somewhere, a UAT run) then acts for seconds or
//! minutes. If its lease is taken over, released or expires meanwhile, the
//! flow must stop before its next action, or two holders drive one client.
//! So the gate runs the tool inside [`scope`], and every point where the
//! supervisor acts on the client calls [`ensure`] first:
//!
//! - every bridge call (`Supervisor::bridge_call`, the hook re-apply);
//! - every posted window message except releases ([`ensure_input`]);
//! - the process spawn in `launch_client`, after its preparation.
//!
//! A failed check is an error starting "lease revoked", which the flow
//! returns like any other failed step. A pass renews the lease, so a long
//! flow keeps it alive.
//!
//! Outside any scope (read-only tools, which never act) `ensure` passes.
//! The watchdog's recovery runs under [`Permit::AnyHolder`]: it may relaunch
//! and log back in only while some lease is held.

use std::sync::Arc;

use super::LeaseBook;

/// What a scope is allowed to act under.
#[derive(Debug, Clone)]
pub enum Permit {
    /// A tool admitted with this lease: it acts while the lease is current.
    Lease { book: Arc<LeaseBook>, id: String },
    /// The watchdog's crash recovery: it acts while anyone holds a lease.
    AnyHolder { book: Arc<LeaseBook> },
}

tokio::task_local! {
    static PERMIT: Permit;
}

/// Run `fut` under `permit`.
pub async fn scope<F: std::future::Future>(permit: Permit, fut: F) -> F::Output {
    PERMIT.scope(permit, fut).await
}

/// Before an action named `action`: pass, or say why the lease is gone.
pub fn ensure(action: &str) -> Result<(), String> {
    let Ok(permit) = PERMIT.try_with(Permit::clone) else {
        return Ok(());
    };
    match permit {
        Permit::Lease { book, id } => book
            .check(Some(&id), action)
            .map_err(|e| format!("lease revoked, {action} not done: {e}")),
        Permit::AnyHolder { book } => {
            if book.is_held() {
                Ok(())
            } else {
                Err(format!(
                    "lease revoked, {action} not done: no lab lease is held, so crash recovery stops"
                ))
            }
        }
    }
}

/// Window messages that only let go of something.
const RELEASES: [u32; 5] = [
    0x0101, // WM_KEYUP
    0x0105, // WM_SYSKEYUP
    0x0202, // WM_LBUTTONUP
    0x0205, // WM_RBUTTONUP
    0x0208, // WM_MBUTTONUP
];

/// [`ensure`] for a posted window message. A release always goes through:
/// refusing it after a revocation would leave a key or button held down for
/// the next holder.
pub fn ensure_input(msg: u32) -> Result<(), String> {
    if RELEASES.contains(&msg) {
        return Ok(());
    }
    ensure(&format!("window message 0x{msg:04x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::AcquireRequest;

    fn req(owner: &str, force: bool) -> AcquireRequest {
        AcquireRequest {
            owner: owner.into(),
            purpose: "permit test".into(),
            force,
            reason: force.then(|| "test takeover".into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn outside_a_scope_everything_passes() {
        assert!(ensure("bridge lua_eval").is_ok());
        assert!(ensure_input(0x0100).is_ok());
    }

    /// Regression guard: an admitted tool's next action fails once its
    /// lease is taken over, but releases still go through.
    #[tokio::test]
    async fn a_takeover_revokes_the_next_action_but_not_releases() {
        let book = Arc::new(LeaseBook::default());
        let a = book.acquire(req("a", false)).unwrap();
        let permit = Permit::Lease {
            book: book.clone(),
            id: a.lease_id,
        };
        scope(permit, async {
            ensure("bridge lua_eval").unwrap();
            book.acquire(req("b", true)).unwrap();
            let e = ensure("bridge lua_eval").unwrap_err();
            assert!(e.starts_with("lease revoked"), "{e}");
            assert!(e.contains("taken over"), "{e}");
            assert!(ensure_input(0x0100).is_err(), "WM_KEYDOWN refused");
            assert!(ensure_input(0x0101).is_ok(), "WM_KEYUP still allowed");
            assert!(ensure_input(0x0202).is_ok(), "WM_LBUTTONUP still allowed");
        })
        .await;
    }

    #[tokio::test]
    async fn recovery_acts_only_while_someone_holds_a_lease() {
        let book = Arc::new(LeaseBook::default());
        let l = book.acquire(req("a", false)).unwrap();
        scope(Permit::AnyHolder { book: book.clone() }, async {
            ensure("launch").unwrap();
            book.release(&l.lease_id).unwrap();
            assert!(ensure("launch").unwrap_err().starts_with("lease revoked"));
        })
        .await;
    }
}
