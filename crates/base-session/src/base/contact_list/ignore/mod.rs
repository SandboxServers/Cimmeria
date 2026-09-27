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
//! - [`session_ignores`]: an online recipient's cached answer, lock held by
//!   the caller. Tells use it.
//! - [`player_ignores`]: the database answer for any recipient, online or
//!   not. Mail send (SS-M1) and duel challenges (SS-D1) use it; the cache
//!   may be absent for an offline recipient.
//!
//! Names compare case-insensitively, the D-SS13 case fold: an entry the
//! contact-list window stored as "bob" still ignores "Bob". `sgw_player`
//! names are `UNIQUE` but case-sensitive, so an entry can cover two
//! characters that differ only in case; the owner accepted that (it is the
//! safe direction for an Ignore). `chatIgnore` still stores the canonical
//! name it resolved, never the typed spelling.
//!
//! - [`IgnoreCache::ignores_player`]: the cached answer by `player_id`,
//!   for callers that hold the other player's id rather than a name (the
//!   SS-D1 duel seam `dispatch::duel::ignores`).

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

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
        }
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

/// The `player_id`s of the characters whose names match (case-insensitively)
/// an entry on `player_id`'s Ignore list.
pub async fn load_ignored_player_ids(
    pool: &PgPool,
    player_id: i32,
) -> Result<HashSet<i32>, sqlx::Error> {
    let rows: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT p.player_id FROM sgw_contact_list_member m          JOIN sgw_contact_list cl USING (list_id)          JOIN sgw_player p ON lower(p.player_name) = lower(m.player_name)          WHERE cl.player_id = $1 AND cl.flags = $2",
    )
    .bind(player_id)
    .bind(IGNORE_LIST_FLAGS)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
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

/// What [`resync_ignore_cache`] needs from the base.
#[derive(Clone, Copy)]
pub struct IgnoreSyncCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
}

/// Reload `player_id`'s Ignore list from the database, store it on the
/// session at `addr`, and push it to the cell entity `entity_id`. `path`
/// names the caller in the log (`world_entry`, `chat_ignore`,
/// `contact_list`). Returns the new set, or `None` when nothing could be
/// loaded (no pool, DB error), in which case the old copies are kept.
pub async fn resync_ignore_cache(
    ctx: IgnoreSyncCtx<'_>,
    addr: SocketAddr,
    player_id: i32,
    entity_id: u32,
    path: &'static str,
) -> Option<HashSet<String>> {
    let Some(pool) = ctx.db_pool else {
        tracing::warn!(
            target: "chat",
            event = "chat.ignore_sync_failed",
            %addr,
            player_id,
            entity_id,
            path,
            reason = "no_db_pool",
            "Ignore list not loaded: no DB pool; tells and spatial chat ignore nobody",
        );
        return None;
    };
    let loaded = match load_ignore_names(pool, player_id).await {
        Ok(n) => load_ignored_player_ids(pool, player_id)
            .await
            .map(|ids| (n, ids)),
        Err(e) => Err(e),
    };
    let (names, ignored_ids) = match loaded {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                %addr,
                player_id,
                entity_id,
                path,
                reason = "db_error",
                error = %e,
                "Ignore list reload failed; the previous copy is kept",
            );
            return None;
        }
    };

    let (account_id, before) = {
        let mut clients = ctx.connected.lock().unwrap();
        match clients.get_mut(&addr) {
            // Only the session still playing this character takes the set: a
            // logOff to character select between the load and here must not
            // hand char A's list to char B.
            Some(c) if c.active_player_id == Some(player_id) => {
                let before = c.ignore.len();
                c.ignore = IgnoreCache::with_player_ids(names.clone(), ignored_ids);
                (Some(c.account_id), Some(before))
            }
            Some(c) => (Some(c.account_id), None),
            None => (None, None),
        }
    };
    let Some(before) = before else {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_sync_failed",
            %addr,
            player_id,
            account_id,
            entity_id,
            path,
            reason = "session_changed",
            "Ignore list reloaded for a session that no longer plays this character; dropped",
        );
        return None;
    };

    tracing::debug!(
        target: "chat",
        event = "chat.ignore_synced",
        %addr,
        player_id,
        account_id,
        entity_id,
        path,
        before,
        after = names.len(),
        "Ignore list cached on the base session and pushed to the cell",
    );

    if let Some(tx) = ctx.cell_tx {
        if let Err(e) = tx
            .send(BaseToCellMsg::UpdateIgnoreList {
                entity_id,
                player_id,
                ignore_names: names.clone(),
            })
            .await
        {
            tracing::warn!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                %addr,
                player_id,
                account_id,
                entity_id,
                path,
                reason = "cell_send_failed",
                error = %e,
                "UpdateIgnoreList base->cell send failed; spatial chat keeps the old set",
            );
        }
    }
    Some(names)
}

#[cfg(test)]
mod tests;
