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

/// Add member names to a contact list. Ignores duplicates via ON CONFLICT DO
/// NOTHING. Returns the names that were actually inserted (not already present).
///
/// Uses a single batched INSERT with an UNNEST array binding to avoid
/// per-name round-trips for large requests.
pub(crate) async fn add_members(
    pool: &PgPool,
    player_id: i32,
    list_id: i32,
    names: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    // Verify ownership in one query before touching member rows.
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

    // Single batched INSERT; RETURNING gives us only the rows actually written.
    let added: Vec<String> = sqlx::query_scalar(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) \
         SELECT $1, name FROM UNNEST($2::text[]) AS t(name) \
         ON CONFLICT DO NOTHING \
         RETURNING player_name",
    )
    .bind(list_id)
    .bind(names)
    .fetch_all(pool)
    .await?;

    Ok(added)
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
