//! Timeouts for the public minigame port.
//!
//! The SFS port is published on every host interface, so anything on the
//! internet can open a socket to it. The original C++ host had none of
//! these limits (no read timeout); they exist because the port is
//! reachable, not because the SWFs need them.
//!
//! Who may connect is not decided here: the listener admits one connection
//! per valid ticket through `SessionRegistry::try_admit` (#1119), so there
//! is no global or per-address connection cap.
//!
//! The numbers come from how the client behaves. The SWF sends `verChk` and
//! `login` itself as soon as it loads, before the player touches anything,
//! so the handshake takes well under a second.

use std::time::Duration;

/// Limits applied by the minigame listener. `Default` is what the server
/// runs with; tests shrink them.
#[derive(Debug, Clone)]
pub struct ListenerLimits {
    /// Time from accept until `login` succeeds. A peer that has not logged
    /// in by then is closed.
    pub handshake_timeout: Duration,
    /// Time an authenticated session may go without sending a frame. On
    /// expiry the session ends as a cancel (result 0), the same as the
    /// player closing the window.
    pub idle_timeout: Duration,
}

impl Default for ListenerLimits {
    fn default() -> Self {
        Self {
            handshake_timeout: Duration::from_secs(30),
            // Livewire's board can sit unstarted (timer stopped) while the
            // player reads it, so this is long: it reclaims abandoned
            // sockets, it is not a play-time limit.
            idle_timeout: Duration::from_secs(30 * 60),
        }
    }
}
