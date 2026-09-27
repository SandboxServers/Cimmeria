//! DB persistence for contact lists.
//!
//! Two tables:
//! - `sgw_contact_list (list_id, player_id, name, flags)` — list headers.
//! - `sgw_contact_list_member (list_id, player_name)` — members by name string.
//!
//! All functions take a `&PgPool` and return `Result<_, sqlx::Error>`.
//! Callers decide whether a DB error is fatal.

use sqlx::PgPool;

/// A loaded contact list, including its members.
#[derive(Debug, Clone)]
pub(crate) struct ContactList {
    pub list_id: i32,
    pub name: String,
    pub flags: i32,
    pub members: Vec<String>,
}

/// Load all contact lists (headers + members) for a player.
///
/// Returns an empty `Vec` if the player has no lists yet (first login before
/// `ensure_system_lists` runs). A DB error propagates as `Err`.
pub(crate) async fn load_contact_lists(
    pool: &PgPool,
    player_id: i32,
) -> Result<Vec<ContactList>, sqlx::Error> {
    // Load headers. (i32, String, i32) maps to (list_id, name, flags).
    let rows: Vec<(i32, String, i32)> = sqlx::query_as::<_, (i32, String, i32)>(
        "SELECT list_id, name, flags FROM sgw_contact_list \
         WHERE player_id = $1 ORDER BY list_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Load all members for this player's lists in one query.
    let list_ids: Vec<i32> = rows.iter().map(|r| r.0).collect();
    // (i32, String) maps to (list_id, player_name).
    let members: Vec<(i32, String)> = sqlx::query_as::<_, (i32, String)>(
        "SELECT list_id, player_name FROM sgw_contact_list_member \
         WHERE list_id = ANY($1) ORDER BY list_id, player_name",
    )
    .bind(&list_ids)
    .fetch_all(pool)
    .await?;

    // Merge: for each list header, collect its members.
    let mut result = Vec::with_capacity(rows.len());
    let mut member_iter = members.into_iter().peekable();

    for (list_id, name, flags) in rows {
        let mut list_members = Vec::new();
        while member_iter.peek().is_some_and(|m| m.0 == list_id) {
            list_members.push(member_iter.next().unwrap().1);
        }
        result.push(ContactList {
            list_id,
            name,
            flags,
            members: list_members,
        });
    }
    Ok(result)
}

/// Ensure the two system lists (Friends / Ignore) exist for a player.
///
/// Idempotent — safe to call on every login. On conflict (returning player)
/// uses a UNION ALL fallback select so **no WAL write** is emitted for
/// existing rows. Returns the (friends_list_id, ignore_list_id) pair.
///
/// Flags 300 / 301 are the EMoniker text monikers from the spec that identify
/// the system lists to the client.
pub(crate) async fn ensure_system_lists(
    pool: &PgPool,
    player_id: i32,
) -> Result<(i32, i32), sqlx::Error> {
    // Friends — insert-or-select without touching the row on conflict.
    let friends_id: i32 = sqlx::query_scalar(
        "WITH ins AS ( \
             INSERT INTO sgw_contact_list (player_id, name, flags) \
             VALUES ($1, 'Friends', 300) \
             ON CONFLICT (player_id, name) DO NOTHING \
             RETURNING list_id \
         ) \
         SELECT list_id FROM ins \
         UNION ALL \
         SELECT list_id FROM sgw_contact_list \
         WHERE player_id = $1 AND name = 'Friends' \
         LIMIT 1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await?;

    // Ignore — same pattern.
    let ignore_id: i32 = sqlx::query_scalar(
        "WITH ins AS ( \
             INSERT INTO sgw_contact_list (player_id, name, flags) \
             VALUES ($1, 'Ignore', 301) \
             ON CONFLICT (player_id, name) DO NOTHING \
             RETURNING list_id \
         ) \
         SELECT list_id FROM ins \
         UNION ALL \
         SELECT list_id FROM sgw_contact_list \
         WHERE player_id = $1 AND name = 'Ignore' \
         LIMIT 1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await?;

    Ok((friends_id, ignore_id))
}

/// Insert a new contact list for `player_id`, returning the server-assigned
/// `list_id`. Returns `Err` if the (player_id, name) pair already exists or
/// on any DB error.
pub(crate) async fn create_list(
    pool: &PgPool,
    player_id: i32,
    name: &str,
    flags: u32,
) -> Result<i32, sqlx::Error> {
    let list_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_contact_list (player_id, name, flags) \
         VALUES ($1, $2, $3) RETURNING list_id",
    )
    .bind(player_id)
    .bind(name)
    .bind(flags as i32)
    .fetch_one(pool)
    .await?;
    Ok(list_id)
}

/// Delete a contact list. Returns `Ok(true)` if a row was deleted (ownership
/// confirmed), `Ok(false)` if no row matched (not owned or doesn't exist).
pub(crate) async fn delete_list(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sgw_contact_list WHERE list_id = $1 AND player_id = $2")
        .bind(list_id)
        .bind(player_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Rename a contact list. Returns `Ok(true)` on success (row owned and updated),
/// `Ok(false)` if no row matched.
pub(crate) async fn rename_list(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    name: &str,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("UPDATE sgw_contact_list SET name = $1 WHERE list_id = $2 AND player_id = $3")
            .bind(name)
            .bind(list_id)
            .bind(player_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// Update the flags on a contact list. Returns `Ok(true)` on success,
/// `Ok(false)` if no row matched.
pub(crate) async fn update_flags(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    flags: u32,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("UPDATE sgw_contact_list SET flags = $1 WHERE list_id = $2 AND player_id = $3")
            .bind(flags as i32)
            .bind(list_id)
            .bind(player_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// Load a single contact list header (no members). Used to re-read after update.
/// Returns `None` if the list doesn't exist or isn't owned by `player_id`.
pub(crate) async fn load_list_header(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
) -> Result<Option<(String, i32)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, i32)>(
        "SELECT name, flags FROM sgw_contact_list WHERE list_id = $1 AND player_id = $2",
    )
    .bind(list_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Add member names to a contact list; the inserted names. See
/// [`add_members_bounded`], which this wraps. Test fixtures use it.
#[cfg(test)]
pub(crate) async fn add_members(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    names: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    add_members_bounded(pool, player_id, list_id, names)
        .await
        .map(|r| r.added)
}

/// What [`add_members_bounded`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MembersAdded {
    /// Names actually inserted, in request order.
    pub added: Vec<String>,
    /// Names refused because the list was at its cap (Ignore list only).
    pub over_cap: Vec<String>,
}

/// Add member names to a contact list. Duplicates are skipped via ON
/// CONFLICT DO NOTHING (the primary key, and the case-insensitive
/// `sgw_contact_list_member_list_lower_name_key`).
///
/// Runs in one transaction that first locks the owner's list row
/// (`FOR UPDATE`, which is also the ownership check), so it serialises with
/// [`add_member_capped`]. On the Ignore list (flags 301) the names are
/// inserted one by one against the list's count, and every name past
/// `MAX_IGNORE_LIST_MEMBERS` lands in `over_cap` instead: the contact-list
/// UI cannot grow the list past the cap `chatIgnore` enforces, one batch or
/// many (PR #893 review). Other lists keep the single batched insert.
pub(crate) async fn add_members_bounded(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    names: &[String],
) -> Result<MembersAdded, sqlx::Error> {
    use crate::base::contact_list::ignore::{IGNORE_LIST_FLAGS, MAX_IGNORE_LIST_MEMBERS};

    let mut tx = pool.begin().await?;
    let flags = lock_owned_list(&mut tx, player_id, list_id).await?;

    if names.is_empty() {
        return Ok(MembersAdded::default());
    }

    if flags != IGNORE_LIST_FLAGS {
        // Single batched INSERT; RETURNING gives us only the rows actually written.
        let added: Vec<String> = sqlx::query_scalar(
            "INSERT INTO sgw_contact_list_member (list_id, player_name) \
             SELECT $1, name FROM UNNEST($2::text[]) AS t(name) \
             ON CONFLICT DO NOTHING \
             RETURNING player_name",
        )
        .bind(list_id)
        .bind(names)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(MembersAdded {
            added,
            over_cap: Vec::new(),
        });
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(&mut *tx)
            .await?;
    let mut count = usize::try_from(count).unwrap_or(usize::MAX);
    let mut result = MembersAdded::default();
    for name in names {
        if count >= MAX_IGNORE_LIST_MEMBERS {
            result.over_cap.push(name.clone());
            continue;
        }
        let inserted: Option<String> = sqlx::query_scalar(
            "INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING RETURNING player_name",
        )
        .bind(list_id)
        .bind(name)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(stored) = inserted {
            result.added.push(stored);
            count += 1;
        }
    }
    tx.commit().await?;
    Ok(result)
}

/// Lock `list_id` if `player_id` owns it and return its flags;
/// `RowNotFound` otherwise.
async fn lock_owned_list(
    tx: &mut sqlx::PgConnection,
    player_id: i32,
    list_id: i32,
) -> Result<i32, sqlx::Error> {
    let owned: Option<i32> = sqlx::query_scalar(
        "SELECT flags FROM sgw_contact_list WHERE list_id = $1 AND player_id = $2 FOR UPDATE",
    )
    .bind(list_id)
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await?;
    owned.ok_or(sqlx::Error::RowNotFound)
}

/// How [`add_member_capped`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CappedAdd {
    /// Inserted; `before` is how many entries the list held first.
    Added { before: usize },
    /// The list already holds this name, compared case-insensitively; the
    /// stored spelling.
    Duplicate(String),
    /// The list already holds `cap` entries.
    Full,
}

/// Add one name to a list only if it is not there yet (case-insensitively)
/// and the list holds fewer than `cap` entries, atomically: the owner's
/// list row is locked `FOR UPDATE` for the read-check-insert, so two
/// overlapping adds (two `chatIgnore`s, or one and a contact-list UI add)
/// serialise and the second sees the first's row. The unique index on
/// `(list_id, lower(player_name))` backs the duplicate check.
/// `RowNotFound` when `player_id` does not own `list_id`.
pub(crate) async fn add_member_capped(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    name: &str,
    cap: usize,
) -> Result<CappedAdd, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_owned_list(&mut tx, player_id, list_id).await?;

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT player_name FROM sgw_contact_list_member \
         WHERE list_id = $1 AND lower(player_name) = lower($2) LIMIT 1",
    )
    .bind(list_id)
    .bind(name)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(stored) = existing {
        return Ok(CappedAdd::Duplicate(stored));
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(&mut *tx)
            .await?;
    let before = usize::try_from(count).unwrap_or(usize::MAX);
    if before >= cap {
        return Ok(CappedAdd::Full);
    }

    let inserted: Option<String> = sqlx::query_scalar(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING RETURNING player_name",
    )
    .bind(list_id)
    .bind(name)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Ok(CappedAdd::Duplicate(name.to_string()));
    }
    tx.commit().await?;
    Ok(CappedAdd::Added { before })
}

/// Remove member names from a contact list. Returns the names that were
/// actually present (and thus deleted). Verifies ownership before touching rows.
///
/// Uses a single batched DELETE with ANY to avoid per-name round-trips.
pub(crate) async fn remove_members(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    names: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    // Verify ownership before touching member rows.
    let owned: Option<i32> = sqlx::query_scalar(
        "SELECT list_id FROM sgw_contact_list WHERE list_id = $1 AND player_id = $2",
    )
    .bind(list_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await?;
    if owned.is_none() {
        return Err(sqlx::Error::RowNotFound);
    }

    if names.is_empty() {
        return Ok(Vec::new());
    }

    // Single batched DELETE; RETURNING gives us only rows that existed.
    let removed: Vec<String> = sqlx::query_scalar(
        "DELETE FROM sgw_contact_list_member \
         WHERE list_id = $1 AND player_name = ANY($2::text[]) \
         RETURNING player_name",
    )
    .bind(list_id)
    .bind(names)
    .fetch_all(pool)
    .await?;

    Ok(removed)
}

/// Find player_ids of all players who have `player_name` in any of their
/// contact lists. Used for the login/logout presence fanout (Phase 4).
///
/// Returns distinct `player_id` values — a player may have the same name in
/// multiple lists but should only receive one event.
pub(crate) async fn find_watchers(
    pool: &PgPool,
    player_name: &str,
) -> Result<Vec<i32>, sqlx::Error> {
    let rows: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT cl.player_id \
         FROM sgw_contact_list_member m \
         JOIN sgw_contact_list cl USING (list_id) \
         WHERE m.player_name = $1",
    )
    .bind(player_name)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests;
