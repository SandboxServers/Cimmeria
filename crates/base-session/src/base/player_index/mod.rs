//! Name -> online session lookup for tells, mail notification and duel
//! challenges (D-SS13).
//!
//! # A view, not a copy
//!
//! [`OnlinePlayerIndex`] is a view over the connected-client map, not a
//! second map kept beside it. A session is listed while it is in the map
//! **and** has [`ConnectedClientState::listed_online`] set, a character
//! name and an active `player_id`. So every teardown that removes the
//! session (disconnect, inactivity timeout, send error, duplicate login,
//! the gate-travel abandon) drops the listing with it, and there is no
//! second structure that a new teardown path could forget to update.
//!
//! The flag is what the two in-session transitions change:
//!
//! - world entry (`play_character`) sets it with the character name;
//! - `logOff` clears it on both variants. A full exit keeps the session (and
//!   `player_name`) until the client's disconnect reaps it, but the
//!   character has already left the world, so it must not be reachable.
//!
//! Reanchor and gate travel keep the session, its name and its `player_id`,
//! so the listing stays. The entity id changes across gate travel, which is
//! why [`OnlinePlayer`] carries only the address and `player_id`: callers
//! read `player_entity_id` from the session at send time.
//!
//! # Name resolution (D-SS13)
//!
//! Exact match first; if there is none, a case-insensitive match that is
//! unique. Two sessions matching (exactly or after case folding) are
//! [`NameLookup::Ambiguous`], refused rather than guessed. `sgw_player`
//! names are `UNIQUE` but case-sensitive, so "Bob" and "bob" can both
//! exist. Mail resolves against `sgw_player` instead (offline recipients
//! are valid); this index is for online-only targets.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;

use super::ConnectedClientState;

/// One online character, as a name lookup returns it.
///
/// Deliberately no entity id: it changes across gate travel. Read
/// `player_entity_id` from the session at `addr` when sending. (This is not
/// the admin-API snapshot `base::OnlinePlayer`; that one is re-exported at
/// `base::` and this one is not.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnlinePlayer {
    pub addr: SocketAddr,
    pub player_id: i32,
}

/// The result of resolving a typed name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameLookup {
    Found(OnlinePlayer),
    /// More than one online character matches. The caller asks the player
    /// to type the exact name.
    Ambiguous,
    NotFound,
}

/// Read-only name index over a locked connected-client map.
///
/// Borrow it for the length of one lookup; do not hold the map's lock
/// across an `.await`. [`lookup_online`] does the locking for callers that
/// do not already hold it.
#[derive(Clone, Copy)]
pub struct OnlinePlayerIndex<'a> {
    clients: &'a HashMap<SocketAddr, ConnectedClientState>,
}

impl<'a> OnlinePlayerIndex<'a> {
    pub fn new(clients: &'a HashMap<SocketAddr, ConnectedClientState>) -> Self {
        Self { clients }
    }

    /// Every listed character: `(name, player)`, in no particular order.
    pub fn entries(&self) -> impl Iterator<Item = (&'a str, OnlinePlayer)> + 'a {
        self.clients
            .iter()
            .filter_map(|(addr, c)| listing(*addr, c))
    }

    /// Resolve `name` per D-SS13. An empty name is never found.
    pub fn lookup(&self, name: &str) -> NameLookup {
        if name.is_empty() {
            return NameLookup::NotFound;
        }
        match self.unique(|n| n == name) {
            NameLookup::NotFound => {
                let folded = name.to_lowercase();
                self.unique(|n| n.to_lowercase() == folded)
            }
            found_or_ambiguous => found_or_ambiguous,
        }
    }

    fn unique(&self, matches: impl Fn(&str) -> bool) -> NameLookup {
        let mut hits = self.entries().filter(|(n, _)| matches(n));
        match (hits.next(), hits.next()) {
            (None, _) => NameLookup::NotFound,
            (Some((_, player)), None) => NameLookup::Found(player),
            (Some(_), Some(_)) => NameLookup::Ambiguous,
        }
    }
}

fn listing(addr: SocketAddr, c: &ConnectedClientState) -> Option<(&str, OnlinePlayer)> {
    if !c.listed_online {
        return None;
    }
    let name = c.player_name.as_deref()?;
    let player_id = c.active_player_id?;
    Some((name, OnlinePlayer { addr, player_id }))
}

/// Lock `connected` and resolve `name`. A poisoned lock resolves to
/// `NotFound`: a lookup must never guess.
pub fn lookup_online(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    name: &str,
) -> NameLookup {
    match connected.lock() {
        Ok(clients) => OnlinePlayerIndex::new(&clients).lookup(name),
        Err(_) => NameLookup::NotFound,
    }
}

#[cfg(test)]
mod tests;
