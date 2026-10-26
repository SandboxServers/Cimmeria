//! The operation epoch of the sign-in driver (CT-87). A sign-out or a Cancel bumps
//! the epoch under the state lock; a completion installs, or keeps a pending choice,
//! only when its epoch is still current and the target has not moved.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use url::Url;

use super::commands::SignInFailure;
use super::discovery;
use super::driver::{guard, SignInDriver};
use super::pending::PendingLink;
use super::session::SignIn;
use crate::storage::DesktopState;

impl SignInDriver {
    /// Installs a completion of `session` that started at `epoch`, under the state
    /// lock. Returns the sign-in the reply shows: `session` once installed, else the
    /// state's own sign-in.
    pub(super) fn install(
        &self,
        state: &Mutex<DesktopState>,
        session: &Arc<SignIn>,
        epoch: u64,
    ) -> Result<Option<Arc<SignIn>>, SignInFailure> {
        let mut st = guard(state)?;
        if !self.installable(&st, session, epoch)? {
            return Ok(st.sign_in());
        }
        st.set_sign_in(Some(Arc::clone(session)));
        self.pending.clear()?;
        Ok(Some(Arc::clone(session)))
    }

    /// Keeps the `needs_link` `choice` for `session` under the state lock, unless the
    /// completion is no longer installable. Returns the sign-in the reply shows: none
    /// once the choice is kept, else the state's own sign-in.
    pub(super) fn keep_choice(
        &self,
        state: &Mutex<DesktopState>,
        session: &SignIn,
        epoch: u64,
        choice: PendingLink,
    ) -> Result<Option<Arc<SignIn>>, SignInFailure> {
        let st = guard(state)?;
        if !self.installable(&st, session, epoch)? {
            return Ok(st.sign_in());
        }
        self.pending.keep(choice)?;
        Ok(None)
    }

    /// Whether a completion of `session` that started at `epoch` may be installed
    /// now, read under the state lock. `Err(Cancelled)` once a sign-out or Cancel
    /// ran; `Ok(false)` when the target moved to another auth base since.
    fn installable(
        &self,
        st: &DesktopState,
        session: &SignIn,
        epoch: u64,
    ) -> Result<bool, SignInFailure> {
        if self.epoch.load(Ordering::SeqCst) != epoch {
            tracing::debug!(target: "launcher.sign_in", reason = "superseded", "sign-in completion not installed");
            return Err(SignInFailure::Cancelled);
        }
        let same = self.same_target(st, session.base());
        if !same {
            tracing::debug!(target: "launcher.sign_in", reason = "target_moved", "sign-in completion not installed");
        }
        Ok(same)
    }

    /// Whether `base` is the auth base of the target on screen (the current login server).
    fn same_target(&self, st: &DesktopState, base: &Url) -> bool {
        st.sign_in_login_server()
            .ok()
            .flatten()
            .and_then(|server| discovery::auth_base(&server.url))
            .as_ref()
            == Some(base)
    }

    /// Voids every sign-in step in flight (CT-87). The epoch moves while the state is
    /// locked, so a step that reads it under the same lock sees the change; then the
    /// browser flow that waits, if any, is cancelled.
    pub(super) fn supersede(&self, state: &Mutex<DesktopState>) -> Result<(), SignInFailure> {
        {
            let _st = guard(state)?;
            self.epoch.fetch_add(1, Ordering::SeqCst);
        }
        self.flow.cancel()
    }
}
