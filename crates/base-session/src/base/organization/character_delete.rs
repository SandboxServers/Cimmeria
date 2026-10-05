//! Deleting a character and logging what it did to its organizations.
//!
//! A character's member rows cascade from `sgw_player`, and the
//! member-delete trigger then promotes a new leader or disbands (D-ORG12).
//! The lock order is kept in the database, for every delete path: the
//! `sgw_player_before_delete_lock_orgs` trigger locks the character's
//! organizations (in `org_id` order) after the `sgw_player` row and before
//! the cascade reaches the member rows (`api` § "Lock order").
//!
//! [`delete_character`] adds the two things SQL cannot do: it refuses a
//! character the account does not own before touching anything, and after
//! its commit it logs the trigger's audit rows (`audit::export_committed`).

use sqlx::PgPool;

use super::audit::{current_tx_id, export_committed, ExportSource, OrgEventRow};

/// The result of [`delete_character`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDeletion {
    /// `true` if a character was deleted.
    pub deleted: bool,
    /// What the member-delete trigger did to the character's organizations
    /// (promotions, disbands, memberless results), already logged at INFO
    /// on the `org` target and stamped exported.
    ///
    /// **Log only.** Never drive fanout from it: an export that failed
    /// leaves it empty (the startup sweep logs those rows later), and the
    /// export is at least once. Fanout reads the organization's state.
    pub org_events: Vec<OrgEventRow>,
}

/// Delete `player_id` if `account_id` owns it, then export the trigger's
/// audit rows for this transaction to the log.
///
/// The character row is locked first, filtered by `account_id`, so another
/// account's request locks nothing and returns `deleted = false`. The
/// DELETE then fires the `sgw_player` BEFORE DELETE trigger, which locks
/// the character's organizations before the member rows cascade.
///
/// A failed export is logged at WARN and does not fail the delete: the rows
/// stay unstamped and the next startup sweep logs them.
pub async fn delete_character(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
) -> Result<CharacterDeletion, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // The names are read with the lock and before the delete, so the lines
    // below can still name a character that no longer exists.
    let owned: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT p.player_name, a.account_name FROM sgw_player p          LEFT JOIN account a ON a.account_id = p.account_id          WHERE p.player_id = $1 AND p.account_id = $2 FOR UPDATE OF p",
    )
    .bind(player_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((player_name, account_name)) = owned else {
        tx.rollback().await?;
        tracing::debug!(
            target: "org",
            event = "delete_character",
            player_id, // nt:id-only no owned character matched, so no name was read
            account_id, // nt:id-only no owned character matched, so no name was read
            rows_affected = 0u64,
            "Character delete matched no owned character"
        );
        return Ok(CharacterDeletion {
            deleted: false,
            org_events: Vec::new(),
        });
    };
    let deleted = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2")
        .bind(player_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    let tx_id = current_tx_id(&mut tx).await?;
    tx.commit().await?;

    let org_events = match export_committed(pool, tx_id, ExportSource::CharacterDelete).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org_events_export",
                reason = "db_error",
                player_id,
                player_name = player_name.as_str(),
                account_id,
                account_name = account_name.as_deref(),
                error = %e,
                "Character deleted, but its organization events were not exported; the startup sweep will log them"
            );
            Vec::new()
        }
    };
    tracing::debug!(
        target: "org",
        event = "delete_character",
        player_id,
        player_name = player_name.as_str(),
        account_id,
        account_name = account_name.as_deref(),
        rows_affected = deleted.rows_affected(),
        org_events = org_events.len(),
        "Character deleted"
    );
    Ok(CharacterDeletion {
        deleted: deleted.rows_affected() > 0,
        org_events,
    })
}
