//! Deleting a character without inverting the ORG-LOCK order.
//!
//! A character's member rows cascade from `sgw_player`, and the
//! member-delete trigger then locks each organization to promote a new
//! leader or disband (D-ORG12). Done as a bare `DELETE FROM sgw_player`,
//! that locks the member row first and the organization second, the
//! reverse of D-ORG04; a kick of that member, or a trigger promoting them,
//! in a transaction that already holds the organization then deadlocks
//! against the delete. [`delete_character`] takes the organization locks
//! first, in `org_id` order, so the trigger only re-takes locks the
//! transaction already holds. After the commit it exports the audit rows
//! the trigger wrote (`audit::export_committed`), so each promotion or
//! disband reaches the `org` log target once.

use sqlx::PgPool;

use super::api::lock_org_quiet;
use super::audit::{current_tx_id, export_committed, OrgEventRow};

/// The result of [`delete_character`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDeletion {
    /// `true` if a character was deleted.
    pub deleted: bool,
    /// What the member-delete trigger did to the character's organizations
    /// (promotions, disbands, memberless results), already logged at INFO
    /// on the `org` target and stamped exported.
    pub org_events: Vec<OrgEventRow>,
}

/// Delete `player_id` if `account_id` owns it, locking the character's
/// organizations first, then export the trigger's audit rows for this
/// transaction to the log.
///
/// The membership read happens before the locks, so an organization the
/// character joins in between is not pre-locked; its trigger still runs,
/// with the old lock order, which is no worse than a bare delete.
///
/// A failed export is logged at WARN and does not fail the delete: the rows
/// stay unstamped and the next startup sweep logs them.
pub async fn delete_character(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
) -> Result<CharacterDeletion, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let org_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT org_id FROM sgw_organization_members WHERE player_id = $1 ORDER BY org_id",
    )
    .bind(player_id)
    .fetch_all(&mut *tx)
    .await?;
    for &org_id in &org_ids {
        lock_org_quiet(&mut tx, org_id).await?;
    }
    let deleted = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2")
        .bind(player_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    let tx_id = current_tx_id(&mut tx).await?;
    tx.commit().await?;

    let org_events = match export_committed(pool, tx_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org_events_export",
                reason = "db_error",
                player_id,
                account_id,
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
        account_id,
        org_count = org_ids.len(),
        rows_affected = deleted.rows_affected(),
        org_events = org_events.len(),
        "Character delete with its organizations locked first"
    );
    Ok(CharacterDeletion {
        deleted: deleted.rows_affected() > 0,
        org_events,
    })
}
