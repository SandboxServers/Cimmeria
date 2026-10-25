//! Per-launch ticket store (contract C1).
//!
//! A signed-in launcher trades its session for a launch ticket: 32
//! alphanumeric characters from a CSPRNG. The client sends `SHA1HEX(ticket)`
//! as its password, so the store keeps only `SHA-256(SHA1HEX)`, never the
//! ticket or the SHA-1. One ticket lives per account. It is reusable inside
//! its life (Change Server re-sends it) and expires after 30 minutes idle or
//! 12 hours after minting, whichever comes first.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rand::distr::{Alphanumeric, SampleString};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Longest a ticket lives after it was minted.
pub const TICKET_ABSOLUTE: Duration = Duration::from_secs(12 * 3600);

/// Longest a ticket lives without a use or a heartbeat.
pub const TICKET_IDLE: Duration = Duration::from_secs(30 * 60);

/// Length of a minted ticket in characters.
pub const TICKET_LEN: usize = 32;

/// A freshly minted ticket. `ticket` is the plain text and leaves the store
/// only here; it is never logged.
pub struct MintedTicket {
    /// Account name the ticket was minted for.
    pub account_name: String,
    /// The plain ticket, handed to the launcher once.
    pub ticket: String,
    /// Seconds until the ticket expires without a use or heartbeat.
    pub expires_in_secs: u64,
}

/// Result of [`LaunchTicketStore::verify`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TicketCheck {
    /// No live ticket matches the name and hash.
    NoMatch,
    /// A live ticket matches; the account it belongs to.
    Valid { account_id: u32 },
}

/// One stored ticket. Holds the hash of the client's SHA-1, never the ticket.
struct Entry {
    /// ASCII-lowercased account name, for case-insensitive matching.
    name_lower: String,
    /// `SHA-256` of the uppercase SHA-1 hex of the ticket.
    hash: [u8; 32],
    /// The launch the ticket was minted for.
    launch_id: String,
    /// When the ticket was minted (absolute life starts here).
    minted: Instant,
    /// Last use or heartbeat (idle life starts here).
    last_used: Instant,
}

impl Entry {
    fn expired(&self, now: Instant) -> bool {
        now.duration_since(self.minted) >= TICKET_ABSOLUTE
            || now.duration_since(self.last_used) >= TICKET_IDLE
    }
}

/// In-memory store of live launch tickets, keyed by account id.
#[derive(Default)]
pub struct LaunchTicketStore {
    entries: Mutex<HashMap<u32, Entry>>,
}

impl fmt::Debug for LaunchTicketStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaunchTicketStore")
            .field("live_count", &self.live_count())
            .finish()
    }
}

impl LaunchTicketStore {
    fn lock(&self) -> MutexGuard<'_, HashMap<u32, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Mint a ticket for `account_id`, replacing any previous one.
    pub fn mint(
        &self,
        account_id: u32,
        account_name: &str,
        launch_id: &str,
        now: Instant,
    ) -> MintedTicket {
        let ticket = Alphanumeric.sample_string(&mut rand::rng(), TICKET_LEN);
        let entry = Entry {
            name_lower: account_name.to_ascii_lowercase(),
            hash: sha256(sha1_upper_hex(&ticket).as_bytes()),
            launch_id: launch_id.to_string(),
            minted: now,
            last_used: now,
        };
        self.lock().insert(account_id, entry);
        MintedTicket {
            account_name: account_name.to_string(),
            ticket,
            expires_in_secs: TICKET_IDLE.as_secs(),
        }
    }

    /// Check the client's `sha1_hex` password for `account_name`. The hex is
    /// uppercased before hashing; the name compare is ASCII case-insensitive.
    /// A `Valid` result refreshes the idle clock. An expired entry is removed.
    pub fn verify(&self, account_name: &str, sha1_hex: &str, now: Instant) -> TicketCheck {
        let name_lower = account_name.to_ascii_lowercase();
        let digest = sha256(sha1_hex.to_ascii_uppercase().as_bytes());
        let mut entries = self.lock();
        let Some((&account_id, entry)) = entries
            .iter_mut()
            .find(|(_, entry)| entry.name_lower == name_lower)
        else {
            return TicketCheck::NoMatch;
        };
        if entry.expired(now) {
            entries.remove(&account_id);
            return TicketCheck::NoMatch;
        }
        if !bool::from(entry.hash.as_slice().ct_eq(digest.as_slice())) {
            return TicketCheck::NoMatch;
        }
        entry.last_used = now;
        TicketCheck::Valid { account_id }
    }

    /// Refresh the idle clock of the account's ticket. Needs the launch id it
    /// was minted for; returns `false` on a mismatch, a miss or an expiry.
    pub fn heartbeat(&self, account_id: u32, launch_id: &str, now: Instant) -> bool {
        let mut entries = self.lock();
        let Some(entry) = entries.get_mut(&account_id) else {
            return false;
        };
        if entry.expired(now) {
            entries.remove(&account_id);
            return false;
        }
        if !same_launch(&entry.launch_id, launch_id) {
            return false;
        }
        entry.last_used = now;
        true
    }

    /// End the account's ticket. Needs the matching launch id; returns `false`
    /// when there is no such ticket.
    pub fn end(&self, account_id: u32, launch_id: &str) -> bool {
        let mut entries = self.lock();
        let matches = entries
            .get(&account_id)
            .is_some_and(|entry| same_launch(&entry.launch_id, launch_id));
        if matches {
            entries.remove(&account_id);
        }
        matches
    }

    /// Drop the account's ticket at once, whatever its launch.
    pub fn revoke_account(&self, account_id: u32) {
        self.lock().remove(&account_id);
    }

    /// Remove every expired ticket and return how many were removed.
    pub fn reap(&self, now: Instant) -> usize {
        let mut entries = self.lock();
        let before = entries.len();
        entries.retain(|_, entry| !entry.expired(now));
        before - entries.len()
    }

    /// Number of tickets held, expired or not until [`Self::reap`] runs.
    pub fn live_count(&self) -> usize {
        self.lock().len()
    }
}

/// Constant-time compare of a stored launch id with the presented one.
fn same_launch(stored: &str, presented: &str) -> bool {
    bool::from(stored.as_bytes().ct_eq(presented.as_bytes()))
}

/// `SHA-256` of `data` as a fixed-size array.
fn sha256(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Compute the uppercase-hex SHA-1 of `input`, the form the client sends.
fn sha1_upper_hex(input: &str) -> String {
    let digest = Sha1::digest(input.as_bytes());
    let mut out = String::with_capacity(40);
    for byte in digest {
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAUNCH: &str = "launch-0001";

    fn sha1_of(ticket: &str) -> String {
        sha1_upper_hex(ticket)
    }

    #[test]
    fn minted_ticket_shape() {
        let store = LaunchTicketStore::default();
        let now = Instant::now();
        let first = store.mint(1, "Player_1234", LAUNCH, now);
        let second = store.mint(1, "Player_1234", LAUNCH, now);
        assert_eq!(first.ticket.len(), TICKET_LEN);
        assert!(first.ticket.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(first.ticket, second.ticket);
    }

    #[test]
    fn verify_accepts_the_sha1_of_the_ticket_any_case() {
        let store = LaunchTicketStore::default();
        let now = Instant::now();
        let minted = store.mint(1, "PLAYER_1234", LAUNCH, now);
        let upper = sha1_of(&minted.ticket);
        assert_eq!(
            store.verify("Player_1234", &upper, now),
            TicketCheck::Valid { account_id: 1 }
        );
        let lower = upper.to_ascii_lowercase();
        assert_eq!(
            store.verify("Player_1234", &lower, now),
            TicketCheck::Valid { account_id: 1 }
        );
    }

    #[test]
    fn verify_refuses_wrong_name_wrong_hash_and_plain_ticket() {
        let store = LaunchTicketStore::default();
        let now = Instant::now();
        let minted = store.mint(1, "Player_1234", LAUNCH, now);
        assert_eq!(
            store.verify("Player_9999", &sha1_of(&minted.ticket), now),
            TicketCheck::NoMatch
        );
        assert_eq!(
            store.verify("Player_1234", &sha1_of("not-the-ticket"), now),
            TicketCheck::NoMatch
        );
        assert_eq!(
            store.verify("Player_1234", &minted.ticket, now),
            TicketCheck::NoMatch
        );
    }

    #[test]
    fn ticket_is_reusable_within_its_life() {
        let store = LaunchTicketStore::default();
        let t0 = Instant::now();
        let minted = store.mint(1, "Player_1234", LAUNCH, t0);
        let sha1 = sha1_of(&minted.ticket);
        for minutes in [0, 20, 40] {
            let now = t0 + Duration::from_secs(minutes * 60);
            assert_eq!(
                store.verify("Player_1234", &sha1, now),
                TicketCheck::Valid { account_id: 1 }
            );
        }
    }

    #[test]
    fn idle_and_absolute_expiry() {
        let t0 = Instant::now();

        let idle = LaunchTicketStore::default();
        let minted = idle.mint(1, "Player_1234", LAUNCH, t0);
        let sha1 = sha1_of(&minted.ticket);
        assert_eq!(
            idle.verify("Player_1234", &sha1, t0 + Duration::from_secs(31 * 60)),
            TicketCheck::NoMatch
        );

        let store = LaunchTicketStore::default();
        let minted = store.mint(1, "Player_1234", LAUNCH, t0);
        let sha1 = sha1_of(&minted.ticket);
        for step in 1..=35u64 {
            let now = t0 + Duration::from_secs(step * 20 * 60);
            assert!(store.heartbeat(1, LAUNCH, now), "heartbeat at step {step}");
        }
        let at_absolute = t0 + TICKET_ABSOLUTE;
        assert_eq!(
            store.verify("Player_1234", &sha1, at_absolute),
            TicketCheck::NoMatch
        );
    }

    #[test]
    fn mint_replaces_the_previous_ticket() {
        let store = LaunchTicketStore::default();
        let now = Instant::now();
        let first = store.mint(1, "Player_1234", LAUNCH, now);
        let second = store.mint(1, "Player_1234", LAUNCH, now);
        assert_eq!(
            store.verify("Player_1234", &sha1_of(&first.ticket), now),
            TicketCheck::NoMatch
        );
        assert_eq!(
            store.verify("Player_1234", &sha1_of(&second.ticket), now),
            TicketCheck::Valid { account_id: 1 }
        );
    }

    #[test]
    fn heartbeat_and_end_need_the_launch_id() {
        let store = LaunchTicketStore::default();
        let now = Instant::now();
        let minted = store.mint(1, "Player_1234", LAUNCH, now);
        assert!(!store.heartbeat(1, "launch-9999", now));
        assert!(!store.end(1, "launch-9999"));
        assert_eq!(store.live_count(), 1);
        assert_eq!(
            store.verify("Player_1234", &sha1_of(&minted.ticket), now),
            TicketCheck::Valid { account_id: 1 }
        );
        assert!(store.end(1, LAUNCH));
        assert_eq!(store.live_count(), 0);
    }

    #[test]
    fn store_never_holds_the_ticket() {
        let store = LaunchTicketStore::default();
        let minted = store.mint(1, "Player_1234", LAUNCH, Instant::now());
        let shown = format!("{store:?}");
        assert!(!shown.contains(&minted.ticket));
        assert!(!shown.contains(&sha1_of(&minted.ticket)));
        assert!(shown.contains("live_count"));
    }

    #[test]
    fn reap_counts_expired() {
        let store = LaunchTicketStore::default();
        let t0 = Instant::now();
        store.mint(1, "Player_0001", LAUNCH, t0);
        store.mint(2, "Player_0002", LAUNCH, t0);
        let later = t0 + TICKET_ABSOLUTE;
        store.mint(3, "Player_0003", LAUNCH, later);
        assert_eq!(store.reap(later), 2);
        assert_eq!(store.live_count(), 1);
    }
}
