//! Per-launch ticket store (contract C1).
//!
//! A signed-in launcher trades its session for a launch ticket: 32
//! alphanumeric characters from a CSPRNG. The client sends `SHA1HEX(ticket)`
//! as its password, so the store keeps only `SHA-256(SHA1HEX)`, never the
//! ticket or the SHA-1. One ticket lives per account. It is reusable inside
//! its life (Change Server re-sends it). Until its first SOAP redemption a
//! ticket expires five minutes after minting; after that it expires after 30
//! minutes idle or 12 hours after minting, whichever comes first. The launcher
//! hands the ticket to the client within seconds, so an honest flow never
//! reaches the short limit.
//!
//! Every ticket carries a [`TicketGuard`]. Revoking an account flips the
//! guards it was issued, so a handoff already in flight dies with the
//! account's sessions (C16.4).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use rand::distr::{Alphanumeric, SampleString};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Longest a ticket lives after it was minted.
pub const TICKET_ABSOLUTE: Duration = Duration::from_secs(12 * 3600);

/// Longest a ticket lives without a use or a heartbeat.
pub const TICKET_IDLE: Duration = Duration::from_secs(30 * 60);

/// Longest a ticket lives after minting until its first SOAP redemption.
pub const TICKET_UNREDEEMED: Duration = Duration::from_secs(5 * 60);

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

/// Liveness flag of an account's tickets. The store flips it to `false` when
/// the account is revoked; every clone sees the flip.
#[derive(Clone, Debug)]
pub struct TicketGuard(Arc<AtomicBool>);

impl TicketGuard {
    /// `true` until the account is revoked.
    pub fn is_current(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Result of [`LaunchTicketStore::verify`].
#[derive(Clone, Debug)]
pub enum TicketCheck {
    /// No live ticket matches the name and hash, or the match was revoked.
    NoMatch,
    /// A live ticket matches: its account and the guard it was minted under.
    Valid { account_id: u32, guard: TicketGuard },
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
    /// The account's guard at mint time.
    guard: TicketGuard,
    /// `true` once a SOAP `verify` has accepted the ticket. An unredeemed
    /// ticket lives only [`TICKET_UNREDEEMED`] after minting.
    redeemed: bool,
}

impl Entry {
    fn expired(&self, now: Instant) -> bool {
        if !self.redeemed {
            return now.duration_since(self.minted) >= TICKET_UNREDEEMED;
        }
        now.duration_since(self.minted) >= TICKET_ABSOLUTE
            || now.duration_since(self.last_used) >= TICKET_IDLE
    }
}

/// Both maps behind one lock, so a guard can never be issued between a
/// revoke's flip and its removal.
#[derive(Default)]
struct Inner {
    /// Live tickets, keyed by account id.
    entries: HashMap<u32, Entry>,
    /// Weak handles to every guard issued per account, for `revoke_account`.
    guards: HashMap<u32, Vec<Weak<AtomicBool>>>,
}

/// In-memory store of live launch tickets, keyed by account id.
#[derive(Default)]
pub struct LaunchTicketStore {
    inner: Mutex<Inner>,
}

impl fmt::Debug for LaunchTicketStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaunchTicketStore")
            .field("live_count", &self.live_count())
            .finish()
    }
}

impl LaunchTicketStore {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A fresh guard registered under `account_id`. The ticket route takes it
    /// before its DB reads and passes it to [`Self::mint_guarded`].
    pub fn guard(&self, account_id: u32) -> TicketGuard {
        register(&mut self.lock(), account_id)
    }

    /// Mint a ticket for `account_id`, replacing any previous one. Takes its
    /// own guard.
    pub fn mint(
        &self,
        account_id: u32,
        account_name: &str,
        launch_id: &str,
        now: Instant,
    ) -> MintedTicket {
        let mut inner = self.lock();
        let guard = register(&mut inner, account_id);
        let ticket = store_ticket(&mut inner, account_id, account_name, launch_id, now, guard);
        MintedTicket {
            account_name: account_name.to_string(),
            ticket,
            expires_in_secs: TICKET_IDLE.as_secs(),
        }
    }

    /// Mint like [`Self::mint`], but under the caller's `guard`. Returns `None`
    /// when the guard was revoked, so nothing is minted for a revoked account.
    pub fn mint_guarded(
        &self,
        account_id: u32,
        account_name: &str,
        launch_id: &str,
        now: Instant,
        guard: &TicketGuard,
    ) -> Option<MintedTicket> {
        let mut inner = self.lock();
        if !guard.is_current() {
            return None;
        }
        let ticket = store_ticket(
            &mut inner,
            account_id,
            account_name,
            launch_id,
            now,
            guard.clone(),
        );
        Some(MintedTicket {
            account_name: account_name.to_string(),
            ticket,
            expires_in_secs: TICKET_IDLE.as_secs(),
        })
    }

    /// Check the client's `sha1_hex` password for `account_name`. The hex is
    /// uppercased before hashing; the name compare is ASCII case-insensitive.
    /// Every ticket of that name is checked, so two accounts that share a name
    /// each verify with their own ticket. A `Valid` result redeems the ticket
    /// and refreshes the idle clock. An expired entry is removed; a matching
    /// entry whose guard was flipped is removed and answers `NoMatch`.
    pub fn verify(&self, account_name: &str, sha1_hex: &str, now: Instant) -> TicketCheck {
        let name_lower = account_name.to_ascii_lowercase();
        let digest = sha256(sha1_hex.to_ascii_uppercase().as_bytes());
        let mut inner = self.lock();
        let candidates: Vec<u32> = inner
            .entries
            .iter()
            .filter(|(_, entry)| entry.name_lower == name_lower)
            .map(|(&account_id, _)| account_id)
            .collect();
        let mut found = None;
        for account_id in candidates {
            let Some(entry) = inner.entries.get(&account_id) else {
                continue;
            };
            if entry.expired(now) {
                inner.entries.remove(&account_id);
                continue;
            }
            // Compare every candidate; no early exit on a match.
            let matched = bool::from(entry.hash.as_slice().ct_eq(digest.as_slice()));
            if matched && found.is_none() {
                found = Some(account_id);
            }
        }
        let Some(account_id) = found else {
            return TicketCheck::NoMatch;
        };
        let Some(entry) = inner.entries.get_mut(&account_id) else {
            return TicketCheck::NoMatch;
        };
        let live = entry.guard.is_current();
        if !live {
            inner.entries.remove(&account_id);
            return TicketCheck::NoMatch;
        }
        entry.last_used = now;
        entry.redeemed = true;
        TicketCheck::Valid {
            account_id,
            guard: entry.guard.clone(),
        }
    }

    /// Refresh the idle clock of the account's ticket. Needs the launch id it
    /// was minted for; returns `false` on a mismatch, a miss or an expiry. An
    /// unredeemed ticket is not extended: it keeps its five-minute mint clock.
    pub fn heartbeat(&self, account_id: u32, launch_id: &str, now: Instant) -> bool {
        let mut inner = self.lock();
        let Some(entry) = inner.entries.get_mut(&account_id) else {
            return false;
        };
        if entry.expired(now) {
            inner.entries.remove(&account_id);
            return false;
        }
        if !same_launch(&entry.launch_id, launch_id) {
            return false;
        }
        if entry.redeemed {
            entry.last_used = now;
        }
        true
    }

    /// End the account's ticket. Needs the matching launch id; returns `false`
    /// when there is no such ticket. Does not flip guards.
    pub fn end(&self, account_id: u32, launch_id: &str) -> bool {
        let mut inner = self.lock();
        let matches = inner
            .entries
            .get(&account_id)
            .is_some_and(|entry| same_launch(&entry.launch_id, launch_id));
        if matches {
            inner.entries.remove(&account_id);
        }
        matches
    }

    /// Drop the account's ticket at once, whatever its launch, and flip every
    /// guard issued to the account. This is the only path that flips guards.
    /// A handoff already issued must die with the account's sessions, while an
    /// `end` from the launcher must not kill a running Change Server.
    pub fn revoke_account(&self, account_id: u32) {
        let mut inner = self.lock();
        inner.entries.remove(&account_id);
        if let Some(list) = inner.guards.remove(&account_id) {
            for weak in list {
                if let Some(flag) = weak.upgrade() {
                    flag.store(false, Ordering::SeqCst);
                }
            }
        }
    }

    /// Remove every expired ticket and drop the guard handles nothing holds
    /// any more. Returns how many tickets were removed.
    pub fn reap(&self, now: Instant) -> usize {
        let mut inner = self.lock();
        let before = inner.entries.len();
        inner.entries.retain(|_, entry| !entry.expired(now));
        inner.guards.retain(|_, list| {
            list.retain(|weak| weak.strong_count() > 0);
            !list.is_empty()
        });
        before - inner.entries.len()
    }

    /// Number of tickets held, expired or not until [`Self::reap`] runs.
    pub fn live_count(&self) -> usize {
        self.lock().entries.len()
    }
}

/// Create a guard for `account_id` and record a weak handle to it.
fn register(inner: &mut Inner, account_id: u32) -> TicketGuard {
    let flag = Arc::new(AtomicBool::new(true));
    inner
        .guards
        .entry(account_id)
        .or_default()
        .push(Arc::downgrade(&flag));
    TicketGuard(flag)
}

/// Generate a ticket, store its entry under `account_id` (replacing any
/// previous one) and return the plain ticket.
fn store_ticket(
    inner: &mut Inner,
    account_id: u32,
    account_name: &str,
    launch_id: &str,
    now: Instant,
    guard: TicketGuard,
) -> String {
    let ticket = Alphanumeric.sample_string(&mut rand::rng(), TICKET_LEN);
    let entry = Entry {
        name_lower: account_name.to_ascii_lowercase(),
        hash: sha256(sha1_upper_hex(&ticket).as_bytes()),
        launch_id: launch_id.to_string(),
        minted: now,
        last_used: now,
        guard,
        redeemed: false,
    };
    inner.entries.insert(account_id, entry);
    ticket
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
#[path = "launch_ticket_tests.rs"]
mod tests;
