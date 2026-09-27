//! The Ignore list as the chat, mail and duel paths read it (D-SS15).
//!
//! The contact list's system Ignore list (flags 301) is the only source. It is
//! one-directional: if A has B on it, B's tells, say/emote/yell, duel
//! challenges and mail to A are refused or withheld. Nobody is hidden from
//! anyone's AoI.
//!
//! Three copies of the list exist, and every change keeps them together:
//!
//! 1. the database rows (`sgw_contact_list_member` under the flags-301 list),
//!    the authority;
//! 2. [`IgnoreCache`] on the owner's base session
//!    ([`ConnectedClientState::ignore`]), which the tell path reads;
//! 3. `CellEntity::ignore_names` on the owner's cell entity, pushed by
//!    `BaseToCellMsg::UpdateIgnoreList`, which spatial chat reads.
//!
//! [`resync_ignore_cache`] reloads (1) and rewrites (2) and (3). It runs at
//! every world entry (after `InitPlayerState`, so a gate-travel entity is
//! re-seeded) and after every Ignore-list change, whether it came from
//! `chatIgnore` (0xC5) or the contact-list UI.
//!
//! # Queries for other packets
//!
//! - [`session_ignores`]: an online recipient's cached answer by name, lock
//!   held by the caller. Tells use it.
//! - [`IgnoreCache::ignores_player`]: the cached answer by `player_id`.
//!   Duel challenges use it (`dispatch::duel::ignores`, SS-D1).
//! - [`recipients_ignoring`]: the batched database answer for many
//!   recipients, online or not, on any executor. Mail send uses it inside its
//!   send transaction, under the recipients' row locks (SS-M1).
//! - [`player_ignores`]: the single-recipient database answer, for any other
//!   caller that needs an offline recipient.
//!
//! Names compare case-insensitively, the D-SS13 case fold: an entry the
//! contact-list window stored as "bob" still ignores "Bob". `sgw_player`
//! names are `UNIQUE` but case-sensitive, so an entry can cover two
//! characters that differ only in case; the owner accepted that (it is the
//! safe direction for an Ignore). `chatIgnore` still stores the canonical
//! name it resolved, never the typed spelling.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use sqlx::PgPool;

use crate::base::ConnectedClientState;

/// `sgw_contact_list.flags` of the system Ignore list (the EMoniker text id
/// `ensure_system_lists` creates it with; Friends is 300).
pub const IGNORE_LIST_FLAGS: i32 = 301;

/// Most names `chatIgnore` puts on one player's Ignore list. Project policy
/// from CAT-L-04's remediation ("cap member_count at e.g. 100 per list"),
/// not recovered data: nothing in the client or the legacy server states a
/// limit.
pub const MAX_IGNORE_LIST_MEMBERS: usize = 100;

/// The comparison key for an Ignore entry or a speaker name (D-SS13 fold).
pub fn fold_name(name: &str) -> String {
    name.to_lowercase()
}

/// The owner's Ignore list as the base session caches it: the folded names,
/// and the `player_id`s of the characters those names resolve to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreCache {
    folded: HashSet<String>,
    player_ids: HashSet<i32>,
    /// The last resync version handed out, and the newest one applied
    /// ([`resync_ignore_cache`]). A resync takes its version before it
    /// reads the database, so the highest version always read after the last
    /// commit; an older one that finishes later is dropped.
    issued: u64,
    applied: u64,
}

impl IgnoreCache {
    /// A cache from the stored names alone (no `player_id`s).
    pub fn new(names: HashSet<String>) -> Self {
        Self::with_player_ids(names, HashSet::new())
    }

    pub fn with_player_ids(names: HashSet<String>, player_ids: HashSet<i32>) -> Self {
        Self {
            folded: names.iter().map(|n| fold_name(n)).collect(),
            player_ids,
            issued: 0,
            applied: 0,
        }
    }

    /// Hand out the next resync version. Take it before reading the
    /// database.
    pub fn begin_sync(&mut self) -> u64 {
        self.issued += 1;
        self.issued
    }

    /// Replace the list with what the resync `version` read, unless a newer
    /// resync has already been applied. Returns whether it was applied.
    pub fn apply_sync(
        &mut self,
        version: u64,
        names: &HashSet<String>,
        player_ids: HashSet<i32>,
    ) -> bool {
        if version <= self.applied {
            return false;
        }
        self.folded = names.iter().map(|n| fold_name(n)).collect();
        self.player_ids = player_ids;
        self.applied = version;
        true
    }

    /// Whether `speaker` is on the list, case-insensitively.
    pub fn ignores(&self, speaker: &str) -> bool {
        self.folded.contains(&fold_name(speaker))
    }

    /// Whether the character `player_id` is on the list. Filled by
    /// [`resync_ignore_cache`] from `sgw_player`.
    pub fn ignores_player(&self, player_id: i32) -> bool {
        self.player_ids.contains(&player_id)
    }

    pub fn len(&self) -> usize {
        self.folded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folded.is_empty()
    }
}

/// Whether the session at `recipient` ignores `speaker` (case-insensitive).
/// A missing session ignores nobody (the caller has already resolved it as
/// online).
pub fn session_ignores(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    recipient: SocketAddr,
    speaker: &str,
) -> bool {
    clients
        .get(&recipient)
        .is_some_and(|c| c.ignore.ignores(speaker))
}

/// Whether `recipient_player_id` has `speaker` on their Ignore list, from the
/// database, case-insensitively. Works for an offline recipient.
pub async fn player_ignores(
    pool: &PgPool,
    recipient_player_id: i32,
    speaker: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS ( \
             SELECT 1 FROM sgw_contact_list_member m \
             JOIN sgw_contact_list cl USING (list_id) \
             WHERE cl.player_id = $1 AND cl.flags = $2 AND lower(m.player_name) = lower($3) \
         )",
    )
    .bind(recipient_player_id)
    .bind(IGNORE_LIST_FLAGS)
    .bind(speaker)
    .fetch_one(pool)
    .await
}

/// The `list_id` of `player_id`'s system Ignore list, created (with Friends)
/// if it does not exist yet. Idempotent.
pub async fn ensure_ignore_list(pool: &PgPool, player_id: i32) -> Result<i32, sqlx::Error> {
    let (_friends, ignore) =
        crate::base::contact_list::persistence::ensure_system_lists(pool, player_id).await?;
    Ok(ignore)
}

/// The recipients among `recipient_ids` whose Ignore list holds `sender`
/// (case-insensitively), in one query: the batched form of
/// [`player_ignores`], on any executor so a caller can run it inside its own
/// transaction (mail send checks it under the recipients' row locks).
pub async fn recipients_ignoring<'e, E>(
    executor: E,
    recipient_ids: &[i32],
    sender: &str,
) -> Result<HashSet<i32>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    if recipient_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT cl.player_id FROM sgw_contact_list_member m \
         JOIN sgw_contact_list cl USING (list_id) \
         WHERE cl.player_id = ANY($1) AND cl.flags = $2 \
         AND lower(m.player_name) = lower($3)",
    )
    .bind(recipient_ids)
    .bind(IGNORE_LIST_FLAGS)
    .bind(sender)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().collect())
}

/// How [`add_ignore_entry`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoreAdd {
    /// Inserted; the list held `before` entries first.
    Added { list_id: i32, before: usize },
    /// Already on the list, compared case-insensitively; the stored spelling.
    Duplicate(String),
    /// The list already holds [`MAX_IGNORE_LIST_MEMBERS`] entries.
    Full,
}

/// Add `name` to `player_id`'s Ignore list, atomically capped at
/// [`MAX_IGNORE_LIST_MEMBERS`] and unique case-insensitively: the list row
/// is locked for the read-check-insert, so overlapping adds (two
/// `chatIgnore`s, or one and a contact-list UI edit) cannot both pass the
/// cap or add the same name twice. The caller announces an `Added` with
/// `handlers::announce_added_members`.
pub async fn add_ignore_entry(
    pool: &PgPool,
    player_id: i32,
    name: &str,
) -> Result<IgnoreAdd, sqlx::Error> {
    use crate::base::contact_list::persistence::{add_member_capped, CappedAdd};
    let list_id = ensure_ignore_list(pool, player_id).await?;
    Ok(
        match add_member_capped(pool, player_id, list_id, name, MAX_IGNORE_LIST_MEMBERS).await? {
            CappedAdd::Added { before } => IgnoreAdd::Added { list_id, before },
            CappedAdd::Duplicate(stored) => IgnoreAdd::Duplicate(stored),
            CappedAdd::Full => IgnoreAdd::Full,
        },
    )
}

/// Every name on `player_id`'s Ignore list(s).
pub async fn load_ignore_names(
    pool: &PgPool,
    player_id: i32,
) -> Result<HashSet<String>, sqlx::Error> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT m.player_name FROM sgw_contact_list_member m \
         JOIN sgw_contact_list cl USING (list_id) \
         WHERE cl.player_id = $1 AND cl.flags = $2",
    )
    .bind(player_id)
    .bind(IGNORE_LIST_FLAGS)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// `player_id`'s Ignore list as one snapshot: the stored names, and the
/// `player_id`s of the characters they match case-insensitively. One query,
/// so the two sets always describe the same list (a change committed between
/// two separate reads could otherwise leave a name without its id, or an id
/// without its name). A name no character has contributes no id.
pub async fn load_ignore_snapshot(
    pool: &PgPool,
    player_id: i32,
) -> Result<(HashSet<String>, HashSet<i32>), sqlx::Error> {
    let rows: Vec<(String, Option<i32>)> = sqlx::query_as(
        "SELECT m.player_name, p.player_id FROM sgw_contact_list_member m \
         JOIN sgw_contact_list cl USING (list_id) \
         LEFT JOIN sgw_player p ON lower(p.player_name) = lower(m.player_name) \
         WHERE cl.player_id = $1 AND cl.flags = $2",
    )
    .bind(player_id)
    .bind(IGNORE_LIST_FLAGS)
    .fetch_all(pool)
    .await?;
    let mut names = HashSet::new();
    let mut ids = HashSet::new();
    for (name, id) in rows {
        names.insert(name);
        ids.extend(id);
    }
    Ok((names, ids))
}

/// "X is not accepting your messages.": the one refusal line for a tell,
/// mail or duel challenge to a player who ignores the sender (D-SS15).
pub fn not_accepting_text(recipient: &str) -> String {
    format!("{recipient} is not accepting your messages.")
}

/// Result of matching a typed name against a set of real names (D-SS13:
/// exact first, then a unique case-insensitive match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameMatch {
    Found(String),
    Ambiguous,
    NotFound,
}

/// Match `typed` against `names` per D-SS13.
pub fn match_name<'a>(names: impl IntoIterator<Item = &'a str> + Clone, typed: &str) -> NameMatch {
    if typed.is_empty() {
        return NameMatch::NotFound;
    }
    if names.clone().into_iter().any(|n| n == typed) {
        return NameMatch::Found(typed.to_string());
    }
    let folded = typed.to_lowercase();
    let mut hits = names.into_iter().filter(|n| n.to_lowercase() == folded);
    match (hits.next(), hits.next()) {
        (None, _) => NameMatch::NotFound,
        (Some(n), None) => NameMatch::Found(n.to_string()),
        (Some(_), Some(_)) => NameMatch::Ambiguous,
    }
}

/// A typed name resolved against `sgw_player`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterLookup {
    Found { player_id: i32, name: String },
    Ambiguous,
    NotFound,
}

/// Resolve a typed character name against `sgw_player` (offline characters
/// included), per D-SS13.
pub async fn resolve_character(pool: &PgPool, typed: &str) -> Result<CharacterLookup, sqlx::Error> {
    if typed.is_empty() {
        return Ok(CharacterLookup::NotFound);
    }
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT player_id, player_name FROM sgw_player \
         WHERE lower(player_name) = lower($1)",
    )
    .bind(typed)
    .fetch_all(pool)
    .await?;
    Ok(
        match match_name(rows.iter().map(|(_, n)| n.as_str()), typed) {
            NameMatch::Found(name) => match rows.iter().find(|(_, n)| *n == name) {
                Some(&(player_id, _)) => CharacterLookup::Found { player_id, name },
                None => CharacterLookup::NotFound,
            },
            NameMatch::Ambiguous => CharacterLookup::Ambiguous,
            NameMatch::NotFound => CharacterLookup::NotFound,
        },
    )
}

mod resync;
pub use resync::{resync_ignore_cache, IgnoreSyncCtx};

#[cfg(test)]
mod tests;
