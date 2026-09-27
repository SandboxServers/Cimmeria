//! Pending organization creations (ORG-05): the registrar dialogs the
//! server has opened and is waiting for a name from.
//!
//! `onOrganizationCreation` (cell method 94) carries only the name; the
//! organization type is implied by which dialog the server opened (audit
//! A-09). So the cell remembers, per character, what it offered: the type,
//! the registrar, the space, when, and how many naming attempts are left.
//! Cell method 94 is honoured only against that entry, and the type always
//! comes from it, never from the client.
//!
//! This module is pure state: no I/O, no clock. Every time-dependent call
//! takes `now`, so the 5-minute expiry is tested with exact instants. The
//! handlers live in `cimmeria-cell-methods`
//! (`cell_methods::organization::creation`).
//!
//! Keyed by `player_id` (entity ids are recycled). An entry ends when:
//!
//! - the creation succeeds ([`PendingCreations::consume`]);
//! - it expires ([`PENDING_CREATION_TTL`]) or the character is in another
//!   space than the one the registrar was in (checked lazily when the name
//!   arrives, so gate travel needs no hook);
//! - the player disconnects ([`PendingCreations::clear`]).
//!
//! **Attempt budget.** Each refused name costs one of
//! [`PENDING_CREATION_ATTEMPTS`]. An exhausted entry is kept until it
//! expires, and re-opening the registrar meanwhile does not refill it, so
//! a player gets at most that many names per window. Re-opening an entry
//! that still has attempts keeps its budget and its expiry. Only one name
//! may be in flight to the base at a time ([`Pending::in_flight`]).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_entity::organization::OrgType;

/// How long a registrar's offer stays open.
pub const PENDING_CREATION_TTL: Duration = Duration::from_secs(5 * 60);

/// Names a player may try per offer window.
pub const PENDING_CREATION_ATTEMPTS: u8 = 3;

/// Count one Team or Command action on `org_actions_total`. Every label is
/// from a closed set; never an id. The base counts its own rows on the same
/// series (`base::organization::creation::count_org_action`).
pub fn count_org_action(action: &'static str, outcome: &'static str, reason: &'static str) {
    cimmeria_observability::counter!(
        "org_actions_total",
        "action" => action,
        "outcome" => outcome,
        "reason" => reason,
    );
}

/// One open offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending {
    pub org_type: OrgType,
    /// The registrar that made the offer.
    pub npc_entity_id: u32,
    /// The space the registrar was in.
    pub space_id: u32,
    pub opened_at: Instant,
    pub attempts_left: u8,
    /// A name was forwarded to the base and has not been answered yet.
    pub in_flight: bool,
}

/// Why the registrar may not open a new offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenReject {
    /// Every attempt of the current window is spent.
    Exhausted,
}

/// Why a name cannot be taken against the player's offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeMiss {
    /// No offer.
    NoPending,
    /// The offer is older than [`PENDING_CREATION_TTL`]; it is removed.
    Expired,
    /// The player is in another space than the registrar was; the offer is
    /// removed.
    SpaceChanged,
    /// Every attempt is spent.
    Exhausted,
    /// A name for this offer is already at the base.
    InFlight,
}

impl TakeMiss {
    /// The `reason` label of the refusal.
    pub fn reason(self) -> &'static str {
        match self {
            TakeMiss::NoPending => "no_pending_creation",
            TakeMiss::Expired | TakeMiss::SpaceChanged => "pending_expired",
            TakeMiss::Exhausted => "rate_limited",
            TakeMiss::InFlight => "creation_in_flight",
        }
    }
}

/// What [`PendingCreations::open`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// A new offer.
    Created,
    /// An unexpired offer with attempts left now names this registrar and
    /// type; its budget and expiry are unchanged.
    Refreshed,
}

/// Every open offer on this cell, keyed by `player_id`.
#[derive(Debug, Default)]
pub struct PendingCreations {
    by_player: HashMap<i32, Pending>,
}

impl PendingCreations {
    pub fn new() -> Self {
        Self::default()
    }

    /// The player's offer, if any (expired or not).
    pub fn get(&self, player_id: i32) -> Option<&Pending> {
        self.by_player.get(&player_id)
    }

    fn expired(p: &Pending, now: Instant) -> bool {
        now.saturating_duration_since(p.opened_at) >= PENDING_CREATION_TTL
    }

    /// Record an offer of `org_type` from `npc_entity_id` in `space_id`.
    pub fn open(
        &mut self,
        player_id: i32,
        org_type: OrgType,
        npc_entity_id: u32,
        space_id: u32,
        now: Instant,
    ) -> Result<Opened, OpenReject> {
        if let Some(p) = self.by_player.get_mut(&player_id) {
            if !Self::expired(p, now) {
                if p.attempts_left == 0 {
                    return Err(OpenReject::Exhausted);
                }
                p.org_type = org_type;
                p.npc_entity_id = npc_entity_id;
                p.space_id = space_id;
                return Ok(Opened::Refreshed);
            }
        }
        self.by_player.insert(
            player_id,
            Pending {
                org_type,
                npc_entity_id,
                space_id,
                opened_at: now,
                attempts_left: PENDING_CREATION_ATTEMPTS,
                in_flight: false,
            },
        );
        Ok(Opened::Created)
    }

    /// Check a name may be sent against the player's offer, for a character
    /// now in `space_id`. On success the offer is marked in flight and its
    /// type returned. An expired offer, or one from another space, is
    /// removed and returned in the miss so the caller can log it.
    pub fn begin_attempt(
        &mut self,
        player_id: i32,
        space_id: u32,
        now: Instant,
    ) -> Result<OrgType, (TakeMiss, Option<Pending>)> {
        let Some(p) = self.by_player.get_mut(&player_id) else {
            return Err((TakeMiss::NoPending, None));
        };
        if Self::expired(p, now) {
            let gone = self.by_player.remove(&player_id);
            return Err((TakeMiss::Expired, gone));
        }
        if p.space_id != space_id {
            let gone = self.by_player.remove(&player_id);
            return Err((TakeMiss::SpaceChanged, gone));
        }
        if p.in_flight {
            return Err((TakeMiss::InFlight, Some(*p)));
        }
        if p.attempts_left == 0 {
            return Err((TakeMiss::Exhausted, Some(*p)));
        }
        p.in_flight = true;
        Ok(p.org_type)
    }

    /// A name was refused (by the cell's text check or by the base): spend
    /// one attempt and clear the in-flight mark. Returns the attempts left,
    /// or `None` when the player has no offer (it expired or was cleared
    /// while the name was at the base).
    pub fn charge_attempt(&mut self, player_id: i32) -> Option<u8> {
        let p = self.by_player.get_mut(&player_id)?;
        p.attempts_left = p.attempts_left.saturating_sub(1);
        p.in_flight = false;
        Some(p.attempts_left)
    }

    /// The organization was created: the offer is closed.
    pub fn consume(&mut self, player_id: i32) -> Option<Pending> {
        self.by_player.remove(&player_id)
    }

    /// The player left: drop their offer.
    pub fn clear(&mut self, player_id: i32) -> Option<Pending> {
        self.by_player.remove(&player_id)
    }

    /// Open offers, expired ones included until they are next touched.
    pub fn len(&self) -> usize {
        self.by_player.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_player.is_empty()
    }
}

#[cfg(test)]
mod tests;
