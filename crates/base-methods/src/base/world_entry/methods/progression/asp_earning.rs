//! The XP grant's persistence, including the Applied Science Points a
//! level-up earns.
//!
//! One statement writes the new XP, level and training points and adds
//! [`APPLIED_SCIENCE_POINTS_PER_LEVEL`] per level gained to
//! `applied_science_points`, so a level can never be persisted without its
//! points (or the points without the level). The levels gained are counted
//! against the level the row held under the row lock, not the session's
//! cached level: a grant that writes a level the row already holds earns
//! nothing, whatever the cache said.

use cimmeria_entity::known_names;
use cimmeria_game::player::APPLIED_SCIENCE_POINTS_PER_LEVEL;
use sqlx::PgPool;

/// What the XP grant's write changed, read in the same statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PersistedGrant {
    pub(super) level_before: i32,
    pub(super) asp_before: i32,
    pub(super) asp_after: i32,
}

impl PersistedGrant {
    /// Whether the write raised the level (and so earned points).
    pub(super) fn earned_asp(&self, level_after: i32) -> bool {
        level_after > self.level_before
    }
}

/// Write the grant's `exp`, `level` and `training_points`, and add the
/// points for every level gained, in one statement. `Ok(None)` when no
/// `sgw_player` row matched.
///
/// The ASP total saturates at `i32::MAX` (the column is `integer`), the same
/// clamp the GM grant uses.
pub(super) async fn persist_grant(
    pool: &PgPool,
    player_id: i32,
    exp: i32,
    level: i32,
    training_points: i32,
) -> Result<Option<PersistedGrant>, sqlx::Error> {
    let row: Option<(i32, i32, i32)> = sqlx::query_as(
        "WITH old AS ( \
             SELECT level AS level_before, applied_science_points AS asp_before \
             FROM sgw_player WHERE player_id = $4 FOR UPDATE) \
         UPDATE sgw_player p \
            SET exp = $1, level = $2, training_points = $3, \
                applied_science_points = LEAST( \
                    old.asp_before::bigint \
                        + GREATEST($2 - old.level_before, 0)::bigint * $5, \
                    2147483647)::integer \
           FROM old WHERE p.player_id = $4 \
         RETURNING old.level_before, old.asp_before, p.applied_science_points",
    )
    .bind(exp)
    .bind(level)
    .bind(training_points)
    .bind(player_id)
    .bind(i64::from(APPLIED_SCIENCE_POINTS_PER_LEVEL))
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(level_before, asp_before, asp_after)| PersistedGrant {
            level_before,
            asp_before,
            asp_after,
        }),
    )
}

/// Log the `asp_earned` transition under the `crafting` target. Called only
/// when the write raised the level.
pub(super) fn log_asp_earned(
    account_id: u32,
    player_id: i32,
    entity_id: u32,
    level_after: i32,
    grant: &PersistedGrant,
) {
    let player_label = known_names::player_name(player_id);
    tracing::info!(
        target: "crafting",
        event = "asp_earned",
        account_id,
        account_name = known_names::account_name(account_id),
        player_id,
        player_name = player_label,
        entity_id,
        entity_name = player_label,
        level_before = grant.level_before,
        level_after,
        asp_before = grant.asp_before,
        asp_after = grant.asp_after,
        "GrantXP: level-up earned Applied Science Points"
    );
}
