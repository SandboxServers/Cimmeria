//! The one transaction behind the `grant_ability` content action (Class
//! Start v6, CS-01a). `sgw_player` is locked first, then
//! `sgw_player_ability_grants` is written, like every other provenance
//! writer (see `grant_provenance`).

use cimmeria_entity::cell_entity::AbilityGrantKind;
use sqlx::PgPool;

/// What [`persist_content_grant`] did, id by id, in request order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ContentGrantWrite {
    /// Appended to `abilities`.
    pub learned: Vec<i32>,
    /// Already known and not trained: the row was added, promoted from
    /// `gm`, or already there.
    pub already_known: Vec<i32>,
    /// Bought from a trainer, now converted into a grant (OD-CS06): out of
    /// `trained_abilities`, cost refunded, content row written.
    pub converted: Vec<i32>,
    /// The archetype's character-creation starters among the request.
    /// Appended if missing, never given a row and never credited
    /// (review F4: a starter is never spend, D-AT03).
    pub starters: Vec<i32>,
    /// Ids that now earn branch credit, in request order: every requested
    /// id with a row (`learned` minus starters, `already_known`,
    /// `converted`).
    pub credited: Vec<i32>,
    /// Ids whose provenance row this call inserted or promoted from `gm`
    /// (sorted).
    pub rows_written: Vec<i32>,
    /// The `skill_point_cost` refunded for `converted`.
    pub refunded: i32,
    /// After the commit.
    pub training_points: i32,
    pub tree_points_spent: i32,
}

/// Why [`persist_content_grant`] wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContentGrantRefusal {
    /// No `sgw_player` row.
    PlayerRowMissing,
    /// `source_kind` was `gm`: content never writes one (the loader refuses
    /// it first; this is the base's own check).
    GmKind,
    /// The character's archetype is not in the grant's `archetypes`
    /// (review F2). Carries `sgw_player.archetype`.
    ArchetypeMismatch { archetype: i32 },
}

/// Grant `ability_ids` to `player_id` with provenance `kind`, atomically.
///
/// One transaction, `sgw_player` locked first. A non-empty `archetypes`
/// that does not list the character's archetype writes nothing. Then, per
/// id:
///
/// - a character-creation starter of the archetype is appended if missing,
///   and gets no row and no credit;
/// - an id in `trained_abilities` is converted: removed from it, its tree
///   node's `skill_point_cost` refunded to `training_points` and taken off
///   `tree_points_spent` (floored at 0), and the content row written, so a
///   respec keeps it;
/// - an unknown id is appended and gets a row;
/// - a known id gets a row if it has none. A `gm` row is promoted to `kind`
///   (a content grant must survive the reset); any other row is kept, so the
///   first content source wins.
///
/// A replay finds every id known with its row and writes nothing.
pub(super) async fn persist_content_grant(
    pool: &PgPool,
    player_id: i32,
    ability_ids: &[i32],
    kind: AbilityGrantKind,
    source_id: Option<i32>,
    archetypes: &[i32],
) -> sqlx::Result<Result<ContentGrantWrite, ContentGrantRefusal>> {
    if !kind.is_credited() {
        return Ok(Err(ContentGrantRefusal::GmKind));
    }
    let mut txn = pool.begin().await?;
    let row: Option<(Vec<i32>, Vec<i32>, i32, i32, i32)> = sqlx::query_as(
        "SELECT abilities, trained_abilities, archetype, training_points, tree_points_spent \
           FROM sgw_player WHERE player_id = $1 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut *txn)
    .await?;
    let Some((known, trained, archetype, training_points, tree_points_spent)) = row else {
        return Ok(Err(ContentGrantRefusal::PlayerRowMissing));
    };
    if !archetypes.is_empty() && !archetypes.contains(&archetype) {
        return Ok(Err(ContentGrantRefusal::ArchetypeMismatch { archetype }));
    }
    let starters = super::gm_ability_bulk::starter_abilities(&mut *txn, archetype).await?;

    let mut write = ContentGrantWrite {
        training_points,
        tree_points_spent,
        ..Default::default()
    };
    let mut seen: Vec<i32> = Vec::with_capacity(ability_ids.len());
    for &id in ability_ids {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        if !known.contains(&id) {
            write.learned.push(id);
        }
        if starters.contains(&id) {
            write.starters.push(id);
            continue;
        }
        if trained.contains(&id) {
            write.converted.push(id);
        } else if known.contains(&id) {
            write.already_known.push(id);
        }
        write.credited.push(id);
    }

    if !write.learned.is_empty() {
        sqlx::query(
            "UPDATE sgw_player SET abilities = abilities || $2::integer[] WHERE player_id = $1",
        )
        .bind(player_id)
        .bind(&write.learned)
        .execute(&mut *txn)
        .await?;
    }
    if !write.converted.is_empty() {
        // The cost the trainer debited: the node's `skill_point_cost` in the
        // character's archetype tree (the cell's catalog reads the same
        // table). A trained id with no node refunds nothing.
        write.refunded = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT SUM(GREATEST(t.skill_point_cost, 0)) \
               FROM resources.archetype_ability_tree t \
              WHERE t.archetype = (enum_range(NULL::resources.\"EArchetype\"))[$1 + 1] \
                AND t.ability_id = ANY($2)",
        )
        .bind(archetype)
        .bind(&write.converted)
        .fetch_one(&mut *txn)
        .await?
        .unwrap_or(0)
        .try_into()
        .unwrap_or(i32::MAX);
        let (points, spent): (i32, i32) = sqlx::query_as(
            "UPDATE sgw_player \
                SET trained_abilities = ARRAY(\
                        SELECT t FROM unnest(trained_abilities) WITH ORDINALITY AS u(t, n) \
                         WHERE NOT (t = ANY($2)) ORDER BY n), \
                    training_points = training_points + $3, \
                    tree_points_spent = GREATEST(tree_points_spent - $3, 0) \
              WHERE player_id = $1 \
             RETURNING training_points, tree_points_spent",
        )
        .bind(player_id)
        .bind(&write.converted)
        .bind(write.refunded)
        .fetch_one(&mut *txn)
        .await?;
        write.training_points = points;
        write.tree_points_spent = spent;
    }
    if !write.credited.is_empty() {
        write.rows_written = sqlx::query_scalar(
            "INSERT INTO sgw_player_ability_grants \
                 (player_id, ability_id, source_kind, source_id) \
             SELECT $1, u.id, $3, $4 FROM unnest($2::integer[]) AS u(id) \
             ON CONFLICT (player_id, ability_id) DO UPDATE \
                SET source_kind = EXCLUDED.source_kind, \
                    source_id = EXCLUDED.source_id, \
                    granted_at = now() \
              WHERE sgw_player_ability_grants.source_kind = 'gm' \
             RETURNING ability_id",
        )
        .bind(player_id)
        .bind(&write.credited)
        .bind(kind.as_str())
        .bind(source_id)
        .fetch_all(&mut *txn)
        .await?;
        write.rows_written.sort_unstable();
    }
    txn.commit().await?;
    Ok(Ok(write))
}
