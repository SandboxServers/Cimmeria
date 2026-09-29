//! The open respec a session holds between `.respeccraft` and the prompt's
//! Yes.
//!
//! `respecCrafting` (100) has no arguments, so the only thing telling a
//! confirmation from a stray or replayed send is this per-session record.
//! Taking it always clears it: one prompt allows one respec.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::base::crafting::options::CraftingSessionOptions;
use crate::base::ConnectedClientState;

/// How long a respec stays open after the prompt.
pub const RESPEC_WINDOW: Duration = Duration::from_secs(60);

/// A respec the player opened and has not confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingRespec {
    /// The character it was opened for. A confirmation from another
    /// character on the same connection does not match.
    pub player_id: i32,
    /// After this instant the confirmation is refused as expired.
    pub expires_at: Instant,
}

impl PendingRespec {
    /// A respec for `player_id`, open for [`RESPEC_WINDOW`] from `now`.
    pub fn open(player_id: i32, now: Instant) -> Self {
        PendingRespec {
            player_id,
            expires_at: now + RESPEC_WINDOW,
        }
    }
}

/// What a confirmation found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// An open respec for this player, inside the window.
    Open,
    /// An open respec for this player whose window has closed.
    Expired,
    /// Nothing open for this player.
    Nothing,
}

/// Take the respec open in `slot` for `player_id` at `now`. The slot is
/// always left empty, so a second confirmation finds [`Taken::Nothing`].
pub fn take(slot: &mut Option<PendingRespec>, player_id: i32, now: Instant) -> Taken {
    match slot.take() {
        Some(p) if p.player_id != player_id => Taken::Nothing,
        Some(p) if now > p.expires_at => Taken::Expired,
        Some(_) => Taken::Open,
        None => Taken::Nothing,
    }
}

/// Run `f` on the crafting state of the session that owns `entity_id`;
/// `None` when the entity has no session.
pub fn with_session<R>(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    f: impl FnOnce(&mut CraftingSessionOptions) -> R,
) -> Option<R> {
    let addr = entity_to_addr.lock().ok()?.get(&entity_id).copied()?;
    let mut clients = connected.lock().ok()?;
    let client = clients.get_mut(&addr)?;
    Some(f(super::super::options::session_options_mut(client)))
}
