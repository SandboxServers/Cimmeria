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
//! transaction already holds.

use sqlx::PgPool;

use super::api::lock_org;

/// Delete `player_id` if `account_id` owns it, locking the character's
/// organizations first. Returns `true` if a character was deleted.
///
/// The membership read happens before the locks, so an organization the
/// character joins in between is not pre-locked; its trigger still runs,
/// with the old lock order, which is no worse than a bare delete.
pub async fn delete_character(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let org_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT org_id FROM sgw_organization_members WHERE player_id = $1 ORDER BY org_id",
    )
    .bind(player_id)
    .fetch_all(&mut *tx)
    .await?;
    for org_id in org_ids {
        lock_org(&mut tx, org_id).await?;
    }
    let deleted = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2")
        .bind(player_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(deleted.rows_affected() > 0)
}
