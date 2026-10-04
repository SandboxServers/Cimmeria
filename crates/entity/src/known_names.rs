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
//! not remembered and its lines leave the name out. A poisoned lock resolves
//! to `None`: an observability helper never panics the path it describes.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 6.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use crate::name_intern::intern;

/// Most IDs each map remembers.
pub const MAX_KNOWN: usize = 65_536;

static PLAYERS: LazyLock<RwLock<HashMap<i64, &'static str>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ACCOUNTS: LazyLock<RwLock<HashMap<i64, &'static str>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ORGS: LazyLock<RwLock<HashMap<i64, &'static str>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn remember(map: &RwLock<HashMap<i64, &'static str>>, id: i64, name: &str) {
    let Some(name) = intern(name) else {
        return;
    };
    if map.read().ok().and_then(|m| m.get(&id).copied()) == Some(name) {
        return;
    }
    let Ok(mut m) = map.write() else {
        return;
    };
    if m.len() < MAX_KNOWN || m.contains_key(&id) {
        m.insert(id, name);
    }
}

fn lookup(map: &RwLock<HashMap<i64, &'static str>>, id: i64) -> Option<&'static str> {
    map.read().ok()?.get(&id).copied()
}

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

/// Remember the character name of `player_id`. A blank name is ignored.
pub fn remember_player(player_id: impl LoggedId, name: &str) {
    if let Some(id) = player_id.key() {
        remember(&PLAYERS, id, name);
    }
}

/// Remember the login name of `account_id`. A blank name is ignored.
pub fn remember_account(account_id: impl LoggedId, name: &str) {
    if let Some(id) = account_id.key() {
        remember(&ACCOUNTS, id, name);
    }
}

/// Remember the display name of a Team or Command. A blank name is ignored.
pub fn remember_org(org_id: impl LoggedId, name: &str) {
    if let Some(id) = org_id.key() {
        remember(&ORGS, id, name);
    }
}

/// The character name of `player_id`, when a session has played it since
/// the server started.
pub fn player_name(player_id: impl LoggedId) -> Option<&'static str> {
    lookup(&PLAYERS, player_id.key()?)
}

/// The login name of `account_id`, when it has logged in since the server
/// started.
pub fn account_name(account_id: impl LoggedId) -> Option<&'static str> {
    lookup(&ACCOUNTS, account_id.key()?)
}

/// The display name of `org_id`, when the base has read its row since the
/// server started (a member's login, or any locked organization change).
pub fn org_name(org_id: impl LoggedId) -> Option<&'static str> {
    lookup(&ORGS, org_id.key()?)
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
    fn a_rename_overwrites_and_blank_is_ignored() {
        let pid = PID + 1;
        remember_player(pid, "Old Name");
        remember_player(pid, "New Name");
        remember_player(pid, "  ");
        assert_eq!(player_name(pid), Some("New Name"));
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
