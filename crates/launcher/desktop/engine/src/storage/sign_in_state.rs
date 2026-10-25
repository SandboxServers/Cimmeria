//! The sign-in that Play uses, and why the last Play did not start (C9).
//! Kept in memory only: the sign-in itself is stored by `sign_in::store`.

use crate::sign_in::session::SignIn;
use crate::storage::DesktopState;
use std::sync::Arc;

impl DesktopState {
    /// The signed-in session Play may ask for a launch ticket, if the player is signed in.
    pub fn sign_in(&self) -> Option<Arc<SignIn>> {
        self.sign_in.clone()
    }

    /// Set after a sign-in starts or ends. `None` means no signed-in session.
    pub fn set_sign_in(&mut self, sign_in: Option<Arc<SignIn>>) {
        self.sign_in = sign_in;
    }

    /// The reason the last Play did not start, for the panel (CT-40 and CT-42).
    pub fn play_refusal(&self) -> Option<&'static str> {
        self.play_refusal
    }

    /// Record why Play stopped, or clear it with `None` when a Play starts.
    pub fn set_play_refusal(&mut self, reason: Option<&'static str>) {
        self.play_refusal = reason;
    }
}
