//! GM chat mutes (SS-C3, D-SS26): [`MuteTable`], keyed by `player_id`, each
//! entry with an expiry on the caller's clock.
//!
//! A muted player's spatial chat and tells are refused at the base, in
//! `sendPlayerCommunication`, before anything reaches the cell or the
//! recipient. The table is keyed by the character (`sgw_player.player_id`),
//! not the session or the entity, so a mute holds across relog, character
//! swap back and gate travel. It lives in memory only: D-SS26 says mutes do
//! not survive a server restart, and persisted mutes are a later decision.
//!
//! Expiry is lazy. Nothing wakes up when a mute ends; the next check at or
//! after the expiry removes the entry and lets the line through, and every
//! insert sweeps the expired entries, so the table is bounded by the number
//! of live mutes.
//!
//! [`gm`] holds the `.mute` / `.unmute` handlers the cell's console reaches
//! through `ChatCellToBase::Mute` / `Unmute`.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub mod gm;

/// Longest mute a GM can set, in minutes (7 days); defined beside the
/// message that carries it.
pub use cimmeria_wire::cell::messages::MAX_MUTE_MINUTES;

/// One live mute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MuteEntry {
    /// When the mute ends. A check at or after this instant lets chat
    /// through.
    pub until: Instant,
    /// The GM who set it, for the refusal log.
    pub by_account_id: Option<u32>,
}

/// Every live mute on this server, keyed by `player_id`.
#[derive(Debug, Default)]
pub struct MuteTable {
    entries: Mutex<HashMap<i32, MuteEntry>>,
}

impl MuteTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mute `player_id` until `entry.until`, replacing any earlier mute.
    /// Returns the time the replaced mute still had to run, if one was live
    /// at `now` (the "before" value for the log).
    pub fn mute(&self, player_id: i32, entry: MuteEntry, now: Instant) -> Option<Duration> {
        let mut entries = self.lock();
        entries.retain(|_, e| e.until > now);
        entries
            .insert(player_id, entry)
            .map(|previous| previous.until.saturating_duration_since(now))
    }

    /// Lift `player_id`'s mute. Returns the time it still had to run, or
    /// `None` if the player was not muted at `now`.
    pub fn unmute(&self, player_id: i32, now: Instant) -> Option<Duration> {
        let removed = self.lock().remove(&player_id)?;
        (removed.until > now).then(|| removed.until - now)
    }

    /// The live mute on `player_id` at `now`, if any. An entry that has
    /// expired is removed here and reported as no mute.
    pub fn active(&self, player_id: i32, now: Instant) -> Option<MuteEntry> {
        let mut entries = self.lock();
        let entry = *entries.get(&player_id)?;
        if entry.until > now {
            Some(entry)
        } else {
            entries.remove(&player_id);
            None
        }
    }

    /// Entries held right now, expired or not (tests and diagnostics).
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<i32, MuteEntry>> {
        // A poisoned lock only means a panic elsewhere while holding it; the
        // map itself is always consistent (every write is one insert or
        // remove), so keep enforcing mutes rather than failing open.
        self.entries.lock().unwrap_or_else(|p| p.into_inner())
    }
}

static MUTES: LazyLock<MuteTable> = LazyLock::new(MuteTable::new);

/// The server's mute table: one per base process, shared by the chat gate
/// and the GM handlers.
pub fn mute_table() -> &'static MuteTable {
    &MUTES
}

/// Whole minutes left on a mute, rounded up, so "1 minute" is shown until
/// the last second.
pub fn minutes_left(remaining: Duration) -> u64 {
    remaining.as_secs().div_ceil(60).max(1)
}

/// The line a muted player reads for every refused chat line or tell.
pub fn muted_text(remaining: Duration) -> String {
    let minutes = minutes_left(remaining);
    let unit = if minutes == 1 { "minute" } else { "minutes" };
    format!("You are muted and cannot chat for another {minutes} {unit}.")
}

#[cfg(test)]
mod tests;
