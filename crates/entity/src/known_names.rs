//! Process-wide `player_id → player_name`, `account_id → account_name` and
//! `org_id → org_name` maps, for log lines written where no session is in
//! reach (Rule 6).
//!
//! # Why this exists
//!
//! The base resolves identity names from `ConnectedClientState`
//! (`session_identity`). That needs the `connected` map, and most of the
//! inventory, vendor and crafting helpers that log a `player_id` take only
//! the ID and a pool: they sit under a transaction, far from the session.
//! Threading a `PlayerIdentity` through every one of them would change a few
//! dozen signatures to carry two log fields. Instead the base remembers each
//! name once, where the session learns it (login for the account,
//! `playCharacter` for the character, an organization's row read for the
//! organization), and a helper looks it up by ID.
//!
//! # Why a stale entry can't misname anyone
//!
//! `player_id`, `account_id` and `org_id` are database serials: an ID is
//! never handed to a second row, so an entry outlives its session without
//! ever naming the wrong one. A rename overwrites the entry the next time the
//! row is read.
//!
//! # Bounds
//!
//! Names are interned ([`crate::name_intern`]), so a map entry costs one
//! pointer. Each map holds at most [`MAX_KNOWN`] IDs; past that a new ID is
//! not remembered, its lines leave the name out, and one `known_names.full`
//! WARN names the map. A name the interner refuses (blank, over 64 bytes, or
//! the interner itself full) removes the ID's entry, so a rename never keeps
//! the old name. A poisoned lock resolves to `None`: an observability helper
//! never panics the path it describes.
//!
//! # Coverage
//!
//! Only what the base has read since it started: players who have played a
//! character, accounts that have logged in, and organizations whose row was
//! read. An offline player named in a vault, mail or member line resolves to
//! `None` and the line leaves the name off.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 6.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};

use crate::name_intern::intern;

/// Most IDs each map remembers.
pub const MAX_KNOWN: usize = 65_536;

/// One ID -> name map, with the one-time warning for when it fills.
struct Registry {
    kind: &'static str,
    map: LazyLock<RwLock<HashMap<i64, &'static str>>>,
    full_warned: AtomicBool,
}

impl Registry {
    const fn new(kind: &'static str) -> Self {
        Self {
            kind,
            map: LazyLock::new(|| RwLock::new(HashMap::new())),
            full_warned: AtomicBool::new(false),
        }
    }

    /// Store `name` for `id`. A name that can't be interned (blank, too
    /// long, or the interner is full) removes the entry instead, so a
    /// rename never leaves the old name behind: an unnamed line is right,
    /// a wrongly named one is not.
    fn remember(&self, id: i64, name: &str) {
        let Some(name) = intern(name) else {
            if let Ok(mut m) = self.map.write() {
                m.remove(&id);
            }
            return;
        };
        if self.lookup(id) == Some(name) {
            return;
        }
        let full = {
            let Ok(mut m) = self.map.write() else {
                return;
            };
            if m.len() < MAX_KNOWN || m.contains_key(&id) {
                m.insert(id, name);
                false
            } else {
                true
            }
        };
        // Warned with the lock released, as the interner does.
        if full && !self.full_warned.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                target: "names",
                event = "known_names.full",
                reason = "cap_reached",
                kind = self.kind,
                cap = MAX_KNOWN,
                "known_names map is full: new IDs are left unnamed on log lines"
            );
        }
    }

    fn lookup(&self, id: i64) -> Option<&'static str> {
        self.map.read().ok()?.get(&id).copied()
    }
}

static PLAYERS: Registry = Registry::new("player");
static ACCOUNTS: Registry = Registry::new("account");
static ORGS: Registry = Registry::new("org");

/// An ID as the log sites hold it: the database's `i32`, the session's
/// `u32`, or either in an `Option` (an identity half not known yet).
pub trait LoggedId {
    /// The ID widened to the map key, `None` when absent.
    fn key(self) -> Option<i64>;
}

macro_rules! logged_id {
    ($($t:ty),*) => {$(
        impl LoggedId for $t {
            fn key(self) -> Option<i64> {
                Some(i64::from(self))
            }
        }
        impl LoggedId for Option<$t> {
            fn key(self) -> Option<i64> {
                self.map(i64::from)
            }
        }
    )*};
}
logged_id!(i32, u32, i64);

/// Remember the character name of `player_id`. A blank or uninternable
/// name forgets the old one.
pub fn remember_player(player_id: impl LoggedId, name: &str) {
    if let Some(id) = player_id.key() {
        PLAYERS.remember(id, name);
    }
}

/// Remember the login name of `account_id`. A blank or uninternable name
/// forgets the old one.
pub fn remember_account(account_id: impl LoggedId, name: &str) {
    if let Some(id) = account_id.key() {
        ACCOUNTS.remember(id, name);
    }
}

/// Remember the display name of a Team or Command. A blank or uninternable
/// name forgets the old one.
pub fn remember_org(org_id: impl LoggedId, name: &str) {
    if let Some(id) = org_id.key() {
        ORGS.remember(id, name);
    }
}

/// The character name of `player_id`, when a session has played it since
/// the server started.
pub fn player_name(player_id: impl LoggedId) -> Option<&'static str> {
    PLAYERS.lookup(player_id.key()?)
}

/// The login name of `account_id`, when it has logged in since the server
/// started.
pub fn account_name(account_id: impl LoggedId) -> Option<&'static str> {
    ACCOUNTS.lookup(account_id.key()?)
}

/// The display name of `org_id`, when the base has read its row since the
/// server started (a member's login, or any locked organization change).
pub fn org_name(org_id: impl LoggedId) -> Option<&'static str> {
    ORGS.lookup(org_id.key()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // IDs far above any seeded or test sentinel, so parallel tests that log
    // real players never collide with these.
    const PID: i32 = 2_000_000_101;
    const AID: u32 = 2_000_000_102;

    #[test]
    fn remembered_names_resolve_by_id() {
        remember_player(PID, "Teal'c");
        remember_account(AID, "sgc_login");
        remember_org(PID, "SG-1");
        assert_eq!(player_name(PID), Some("Teal'c"));
        assert_eq!(org_name(PID), Some("SG-1"));
        assert_eq!(account_name(AID), Some("sgc_login"));
    }

    #[test]
    fn a_rename_overwrites_the_old_name() {
        let pid = PID + 1;
        remember_player(pid, "Old Name");
        remember_player(pid, "New Name");
        assert_eq!(player_name(pid), Some("New Name"));
    }

    /// A new name that can't be interned (here, too long) drops the old
    /// one: a renamed row must not keep logging under its old name.
    #[test]
    fn an_uninternable_rename_removes_the_old_name() {
        let pid = PID + 4;
        remember_player(pid, "Old Name");
        remember_player(pid, &"x".repeat(crate::name_intern::MAX_NAME_BYTES + 1));
        assert_eq!(player_name(pid), None);
        remember_player(pid, "Back Again");
        remember_player(pid, "  ");
        assert_eq!(player_name(pid), None, "a blank name unnames it too");
    }

    #[test]
    fn an_unknown_or_absent_id_is_unresolved() {
        assert_eq!(player_name(PID + 2), None);
        assert_eq!(account_name(AID + 2), None);
        assert_eq!(player_name(None::<i32>), None);
    }

    #[test]
    fn every_id_shape_finds_the_same_entry() {
        let pid = PID + 3;
        remember_player(pid, "Daniel");
        assert_eq!(player_name(Some(pid)), Some("Daniel"));
        assert_eq!(player_name(i64::from(pid)), Some("Daniel"));
    }
}
