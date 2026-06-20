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
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for contact-list persistence tests.
    /// Distinct from crafting (0x7000_2000 / 0x7000_3000) to avoid collisions.
    const TEST_BASE: i32 = 0x7000_4000;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        // Members + lists cascade from player DELETE.
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_minimal_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("cl-persist-test-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("cl-persist-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// `load_contact_lists` returns empty vec for a player with no lists.
    #[tokio::test]
    async fn load_empty_returns_empty() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let lists = load_contact_lists(&pool, player_id)
            .await
            .expect("load should not error");
        assert!(lists.is_empty());

        cleanup(&pool, account_id, player_id).await;
    }

    /// `ensure_system_lists` creates Friends + Ignore on first call and is idempotent.
    #[tokio::test]
    async fn ensure_system_lists_creates_and_is_idempotent() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 10;
        let player_id = TEST_BASE + 11;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let (f1, i1) = ensure_system_lists(&pool, player_id)
            .await
            .expect("first call");
        let (f2, i2) = ensure_system_lists(&pool, player_id)
            .await
            .expect("second call — must be idempotent");

        // Same ids on both calls.
        assert_eq!(f1, f2, "Friends list_id must be stable across ensure calls");
        assert_eq!(i1, i2, "Ignore list_id must be stable across ensure calls");
        assert_ne!(f1, i1, "Friends and Ignore must have different list_ids");

        let lists = load_contact_lists(&pool, player_id)
            .await
            .expect("load after ensure");
        assert_eq!(lists.len(), 2);
        let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"Friends"));
        assert!(names.contains(&"Ignore"));

        cleanup(&pool, account_id, player_id).await;
    }

    /// `create_list` inserts a new list and returns its id.
    #[tokio::test]
    async fn create_list_inserts_and_returns_id() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 20;
        let player_id = TEST_BASE + 21;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let list_id = create_list(&pool, player_id, "Allies", 0)
            .await
            .expect("create");
        assert!(list_id > 0);

        let lists = load_contact_lists(&pool, player_id).await.expect("load");
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].list_id, list_id);
        assert_eq!(lists[0].name, "Allies");

        cleanup(&pool, account_id, player_id).await;
    }

    /// `delete_list` returns false when the list does not belong to the player
    /// (ownership rejection guard).
    #[tokio::test]
    async fn delete_list_rejects_wrong_owner() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 30;
        let player_id = TEST_BASE + 31;
        let other_account = TEST_BASE + 32;
        let other_player = TEST_BASE + 33;
        cleanup(&pool, account_id, player_id).await;
        cleanup(&pool, other_account, other_player).await;
        insert_minimal_player(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, other_account, other_player).await;

        let list_id = create_list(&pool, player_id, "MyList", 0)
            .await
            .expect("create");

        // other_player trying to delete player_id's list must return false.
        let deleted = delete_list(&pool, other_player, list_id)
            .await
            .expect("delete call should not error");
        assert!(
            !deleted,
            "delete_list must return false when the list belongs to a different player"
        );

        // The list must still exist.
        let lists = load_contact_lists(&pool, player_id).await.expect("load");
        assert_eq!(
            lists.len(),
            1,
            "list must survive wrong-owner delete attempt"
        );

        cleanup(&pool, account_id, player_id).await;
        cleanup(&pool, other_account, other_player).await;
    }

    /// `add_members` inserts names and ignores duplicates; `remove_members`
    /// deletes them and returns only the actually-removed names.
    #[tokio::test]
    async fn add_and_remove_members_round_trip() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 40;
        let player_id = TEST_BASE + 41;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let list_id = create_list(&pool, player_id, "Contacts", 0)
            .await
            .expect("create");

        let names = vec!["Alice".to_string(), "Bob".to_string()];
        let added = add_members(&pool, player_id, list_id, &names)
            .await
            .expect("add");
        assert_eq!(added.len(), 2);

        // Duplicate add must be a no-op (ON CONFLICT DO NOTHING).
        let added_again = add_members(&pool, player_id, list_id, &["Alice".to_string()])
            .await
            .expect("add again");
        assert!(
            added_again.is_empty(),
            "duplicate add must not insert a new row"
        );

        let lists = load_contact_lists(&pool, player_id).await.expect("load");
        assert_eq!(lists[0].members.len(), 2);

        // Remove one.
        let removed = remove_members(&pool, player_id, list_id, &["Alice".to_string()])
            .await
            .expect("remove");
        assert_eq!(removed, vec!["Alice".to_string()]);

        let lists2 = load_contact_lists(&pool, player_id)
            .await
            .expect("load after remove");
        assert_eq!(lists2[0].members, vec!["Bob".to_string()]);

        cleanup(&pool, account_id, player_id).await;
    }

    /// `find_watchers` returns the player_ids of everyone who has a given name
    /// in any list, without duplicates.
    #[tokio::test]
    async fn find_watchers_returns_correct_player_ids() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 50;
        let player_id = TEST_BASE + 51;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        // Two lists, both containing "WatchedPlayer".
        let list1 = create_list(&pool, player_id, "L1", 0).await.unwrap();
        let list2 = create_list(&pool, player_id, "L2", 0).await.unwrap();
        add_members(&pool, player_id, list1, &["WatchedPlayer".to_string()])
            .await
            .unwrap();
        add_members(&pool, player_id, list2, &["WatchedPlayer".to_string()])
            .await
            .unwrap();

        let watchers = find_watchers(&pool, "WatchedPlayer")
            .await
            .expect("find_watchers");

        // player_id should appear exactly once despite appearing in two lists.
        assert_eq!(
            watchers.iter().filter(|&&id| id == player_id).count(),
            1,
            "find_watchers must return distinct player_ids (DISTINCT clause regression guard)"
        );

        // Not-watched player should not appear.
        assert!(
            !watchers.contains(&(TEST_BASE + 99)),
            "unrelated player must not appear in watchers"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Character delete cascades: after deleting the owning `sgw_player` row,
    /// no `sgw_contact_list` or `sgw_contact_list_member` rows may survive.
    /// This is invariant #4 (no orphaned social data after character deletion).
    #[tokio::test]
    async fn character_delete_cascades_to_lists_and_members() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 60;
        let player_id = TEST_BASE + 61;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let list_id = create_list(&pool, player_id, "ToBeGone", 0).await.unwrap();
        add_members(&pool, player_id, list_id, &["Ghost".to_string()])
            .await
            .unwrap();

        // Delete the character.
        sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(&pool)
            .await
            .expect("delete player");

        // Assert no orphaned rows remain.
        let orphan_lists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list WHERE player_id = $1")
                .bind(player_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            orphan_lists, 0,
            "sgw_contact_list rows must cascade-delete with the player (invariant #4)"
        );

        let orphan_members: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
                .bind(list_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            orphan_members, 0,
            "sgw_contact_list_member rows must cascade-delete with the list (invariant #4)"
        );

        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .ok();
    }
}
