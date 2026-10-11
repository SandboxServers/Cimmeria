//! Forget tutorials so a fixture row sees them again: the one-time
//! tutorial shows only on the first insert into `sgw_player_tutorials`
//! (`shown_tutorials::record_tutorial_shown`), so removing the row is what
//! makes the next `show_tutorial` a first time.

use sqlx::PgPool;

/// Delete `player_id`'s rows for `ids`; returns the rows removed.
pub(crate) async fn forget_tutorials(
    pool: &PgPool,
    player_id: i32,
    ids: &[i32],
) -> Result<u64, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    sqlx::query("DELETE FROM sgw_player_tutorials WHERE player_id = $1 AND tutorial_id = ANY($2)")
        .bind(player_id)
        .bind(ids)
        .execute(pool)
        .await
        .map(|done| done.rows_affected())
        .map_err(|e| format!("forgetting tutorials failed: {e}"))
}
