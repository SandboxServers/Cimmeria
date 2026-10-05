//! `sgw_player_ability_grants`: where a free ability came from (Class Start
//! v6, CS-01a, lock L6). Every write here runs on a transaction that has
//! already locked the character's `sgw_player` row, so the known set and its
//! provenance never disagree (lock order: `sgw_player`, then this table).
//!
//! - `content_grant_write::persist_content_grant`: the `grant_ability`
//!   content action.
//! - [`record_gm_grants`]: `.giveability`, GM grant-all and the Debug Area
//!   granter, kind `gm`.
//! - [`credited_grants`] / [`delete_gm_grants`]: what the GM reset keeps and
//!   drops.
//!
//! A trained ability never has a row while it is trained: it lives in
//! `trained_abilities`, and a respec refunds it. A content grant of a
//! trained ability converts it first (OD-CS06).

/// Record `gm` provenance for ids a GM tool has just appended.
///
/// `ON CONFLICT DO NOTHING`: an id that already has a row keeps it, so a GM
/// grant never downgrades a signature or tutorial grant to `gm` (which the
/// next reset would remove).
pub(super) async fn record_gm_grants<'e, E>(
    executor: E,
    player_id: i32,
    ability_ids: &[i32],
) -> sqlx::Result<u64>
where
    E: sqlx::PgExecutor<'e>,
{
    if ability_ids.is_empty() {
        return Ok(0);
    }
    let r = sqlx::query(
        "INSERT INTO sgw_player_ability_grants (player_id, ability_id, source_kind) \
         SELECT $1, u.id, 'gm' FROM unnest($2::integer[]) AS u(id) \
         ON CONFLICT (player_id, ability_id) DO NOTHING",
    )
    .bind(player_id)
    .bind(ability_ids)
    .execute(executor)
    .await?;
    Ok(r.rows_affected())
}

/// The character's abilities with a non-`gm` provenance row, oldest grant
/// first: what the GM reset keeps and the spend gate credits.
pub(super) async fn credited_grants<'e, E>(executor: E, player_id: i32) -> sqlx::Result<Vec<i32>>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_scalar(
        "SELECT ability_id FROM sgw_player_ability_grants \
          WHERE player_id = $1 AND source_kind <> 'gm' \
          ORDER BY granted_at, ability_id",
    )
    .bind(player_id)
    .fetch_all(executor)
    .await
}

/// Delete the character's `gm` provenance rows (the GM reset).
pub(super) async fn delete_gm_grants<'e, E>(executor: E, player_id: i32) -> sqlx::Result<u64>
where
    E: sqlx::PgExecutor<'e>,
{
    let r = sqlx::query(
        "DELETE FROM sgw_player_ability_grants WHERE player_id = $1 AND source_kind = 'gm'",
    )
    .bind(player_id)
    .execute(executor)
    .await?;
    Ok(r.rows_affected())
}
