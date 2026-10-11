//! `CellToBaseMsg::PersistPosition` — write the player's last known world
//! and position to `sgw_player` when the session ends.
//!
//! Before this arm existed, `sgw_player.world_location` / `pos_*` were
//! written by exactly two paths: gate travel (destination world + arrival
//! point) and the GM teleport (`TeleportPlayer`). Logging out wrote
//! nothing, so a returning character spawned at the last gate arrival, the
//! last GM teleport, or the creation point — never where they logged out.
//! For a Cellblock character that meant a fresh instance at the start cell
//! with every door closed while the mission state was already past them.
//!
//! The cell owns the live position (movement validation writes it on every
//! accepted client move) and sends this once, from the `DisconnectEntity`
//! arm, before the entity is torn down. The base owns the DB write. Same
//! "cell mutates in memory, base persists" split as `SystemOptionsUpdate`.
//!
//! Columns written: `world_location`, `world_id` (resolved from
//! `resources.worlds` by name) and `pos_x` / `pos_y` / `pos_z` — the same
//! statement shape gate travel uses, so a relog and a gate arrival read
//! back identically. `world_location` carries a foreign key onto
//! `resources.worlds(world)`, so a name the table does not know (a dev
//! sandbox space) fails the whole write and the row keeps its previous
//! world *and* position: a world that cannot be resolved at login must not
//! become the resume point either.

use std::sync::Arc;

use sqlx::PgPool;

/// Persist a player's last known world + position to `sgw_player`.
///
/// Returns silently after a `warn` if the row is missing or the write
/// fails: the session is already over, so nothing in memory can be
/// corrected, but ops must see it because the next login will land the
/// player somewhere stale. A non-finite coordinate is refused outright —
/// storing NaN would make the next world entry undefined.
///
/// Returns `true` only when the row was written; the shutdown flush
/// (TG-DB-02) counts a player saved on that.
pub(super) async fn persist_position(
    player_id: i32,
    player_name: Option<&str>,
    world_name: &str,
    position: [f32; 3],
    db_pool: &Option<Arc<PgPool>>,
) -> bool {
    let Some(pool) = db_pool else {
        // No pool means the server is running without persistence (test
        // / repl / smoke mode). A DEBUG row only: it is expected there.
        tracing::debug!(
            player_id,
            player_name,
            world = world_name,
            reason = "no_db_pool",
            "PersistPosition: no database pool, position not stored"
        );
        return false;
    };

    // The session may already be gone when this runs; the name cache keeps
    // the character name for the rows below.
    let player_name = player_name.or_else(|| cimmeria_entity::known_names::player_name(player_id));

    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            player_id,
            player_name,
            world = world_name,
            x = position[0],
            y = position[1],
            z = position[2],
            reason = "non_finite_position",
            "PersistPosition: refusing to store a non-finite coordinate — the \
             row keeps its previous position"
        );
        return false;
    }

    match sqlx::query_scalar::<_, i32>(
        "UPDATE sgw_player \
           SET world_location = $1, \
               world_id = COALESCE((SELECT world_id FROM resources.worlds WHERE world = $1), world_id), \
               pos_x = $2, pos_y = $3, pos_z = $4 \
         WHERE player_id = $5 \
         RETURNING account_id",
    )
    .bind(world_name)
    .bind(position[0])
    .bind(position[1])
    .bind(position[2])
    .bind(player_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(account_id)) => {
            tracing::info!(
                account_id,
                account_name = cimmeria_entity::known_names::account_name(account_id),
                player_id,
                player_name,
                world = world_name,
                x = position[0],
                y = position[1],
                z = position[2],
                "PersistPosition: persisted"
            );
            true
        }
        Ok(None) => {
            tracing::warn!(
                rows_affected = 0,
                expected = 1,
                reason = "player_row_missing",
                player_id,
                player_name,
                world = world_name,
                x = position[0],
                y = position[1],
                z = position[2],
                "PersistPosition: no rows updated (player row missing?)"
            );
            false
        }
        Err(e) => {
            tracing::warn!(
                reason = "db_write_failed",
                player_id,
                player_name,
                world = world_name,
                x = position[0],
                y = position[1],
                z = position[2],
                error = %e,
                "PersistPosition: DB write failed (next login will use the previous position)"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB regression guards for the logout position persistence path.
    //! Sentinel id range `0x7000_1C00` — the slot past the auth credentials
    //! guards at `0x7000_1B00`.

    use super::*;
    use crate::test_support::require_db_or_skip;

    const TEST_BASE: i32 = 0x7000_1C00;

    async fn cleanup(pool: &PgPool, account_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("persistpos-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'Castle_CellBlock', \
                       'BS_HumanMale.BS_HumanMale', -334.231, 73.472, -228.026, 0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("ppos-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    #[derive(sqlx::FromRow, Debug, PartialEq)]
    struct StoredPosition {
        world_location: String,
        world_id: Option<i32>,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
    }

    async fn read_position(pool: &PgPool, player_id: i32) -> StoredPosition {
        sqlx::query_as(
            "SELECT world_location, world_id, pos_x, pos_y, pos_z \
             FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read position")
    }

    /// **Round-trip.** A logout in Castle at an arbitrary point must come
    /// back from `sgw_player` as that world and that point, with `world_id`
    /// resolved through `resources.worlds` the same way gate travel resolves
    /// it. Reverting the persist implementation to a no-op leaves the row at
    /// the creation point and fails the first assertion.
    #[tokio::test]
    async fn live_db_logout_position_round_trips_with_world_id() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;
        let pool_opt = Some(Arc::new(pool.clone()));
        // The session is gone at logout, so the name comes from the cache.
        let player_name = format!("ppos-{player_id}");
        cimmeria_entity::known_names::remember_player(player_id, &player_name);

        // Resolve the expected world_id from the seed rather than hard-coding
        // the Castle row id — the table is recovered data and may be renumbered.
        let expected_world_id: Option<i32> =
            sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = 'Castle'")
                .fetch_optional(&pool)
                .await
                .expect("query resources.worlds");
        assert!(
            expected_world_id.is_some(),
            "fixture sanity: the seed must carry a resources.worlds row named Castle"
        );

        let capture = LogCapture::install();
        persist_position(
            player_id,
            None,
            "Castle",
            [411.349, 70.111, 987.685],
            &pool_opt,
        )
        .await;

        let event = capture
            .find_message(tracing::Level::INFO, "PersistPosition: persisted")
            .expect("a persisted position must log an INFO row");
        assert!(
            event.has_field("account_id", &account_id.to_string()),
            "persisted row must carry the RETURNING account_id; got {event:?}"
        );
        assert!(
            event.has_field("player_name", &player_name),
            "persisted row must carry the cached player_name; got {event:?}"
        );

        let stored = read_position(&pool, player_id).await;
        assert_eq!(
            stored,
            StoredPosition {
                world_location: "Castle".to_string(),
                world_id: expected_world_id,
                pos_x: 411.349,
                pos_y: 70.111,
                pos_z: 987.685,
            },
            "logout must store the live world + position so the next login resumes \
             there instead of at the last gate arrival / creation point"
        );

        cleanup(&pool, account_id).await;
    }

    /// **Unknown world is refused as a whole.** `world_location` carries a
    /// foreign key onto `resources.worlds(world)`, so a name the table does
    /// not know (a dev sandbox space) must fail the write and leave the row
    /// — world *and* coordinates — exactly as it was, with the failure
    /// logged at WARN. Splitting the statement so the coordinates land
    /// without the world would resume the player at foreign coordinates in
    /// their previous world; this guard fails that "fix".
    #[tokio::test]
    async fn live_db_unknown_world_name_is_refused_and_the_row_is_unchanged() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 0x10;
        let player_id = TEST_BASE + 0x11;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;
        let pool_opt = Some(Arc::new(pool.clone()));

        let before = read_position(&pool, player_id).await;
        let capture = LogCapture::install();

        persist_position(
            player_id,
            None,
            "NoSuchWorld_PersistPositionGuard",
            [1.0, 2.0, 3.0],
            &pool_opt,
        )
        .await;

        let after = read_position(&pool, player_id).await;
        assert_eq!(
            after, before,
            "a world name absent from resources.worlds must leave the row untouched — \
             neither the name nor the coordinates may land"
        );
        let event = capture
            .find_message(tracing::Level::WARN, "PersistPosition: DB write failed")
            .expect(
                "the refused write must be logged at WARN so ops can see the dropped resume point",
            );
        assert!(
            event.has_field("player_id", &player_id.to_string()),
            "warn must carry player_id; got {event:?}"
        );

        cleanup(&pool, account_id).await;
    }

    /// **Non-finite guard.** A NaN coordinate must not reach the row: the
    /// previous position stays and the documented warn fires. Reverting the
    /// `is_finite` check stores NaN and fails the position assertion.
    #[tokio::test]
    async fn live_db_non_finite_position_is_refused_and_warned() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 0x20;
        let player_id = TEST_BASE + 0x21;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;
        let pool_opt = Some(Arc::new(pool.clone()));

        let before = read_position(&pool, player_id).await;
        let capture = LogCapture::install();

        persist_position(
            player_id,
            None,
            "Castle",
            [f32::NAN, 70.0, 987.0],
            &pool_opt,
        )
        .await;

        let after = read_position(&pool, player_id).await;
        assert_eq!(
            after, before,
            "a non-finite coordinate must leave the row untouched"
        );
        let event = capture
            .find_message(
                tracing::Level::WARN,
                "PersistPosition: refusing to store a non-finite coordinate",
            )
            .expect("the refusal must be logged at WARN so ops can see the dropped write");
        assert!(
            event.has_field("reason", "non_finite_position"),
            "warn must carry the documented reason; got {event:?}"
        );

        cleanup(&pool, account_id).await;
    }

    /// Missing-row contract: persisting against a non-existent player_id
    /// must warn (not error / not silently succeed), mirroring the sibling
    /// persistence handlers per the negative-logging convention.
    #[tokio::test]
    async fn live_db_persist_no_row_is_silent_warn() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let pool_opt = Some(Arc::new(pool.clone()));
        let phantom_id = TEST_BASE + 0x30;
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(phantom_id)
            .execute(&pool)
            .await;

        let capture = LogCapture::install();
        persist_position(phantom_id, None, "Castle", [1.0, 2.0, 3.0], &pool_opt).await;

        let event = capture
            .find_message(tracing::Level::WARN, "PersistPosition: no rows updated")
            .expect(
                "missing-row persist must emit the documented warn — \
                 reverting the rows_affected==0 arm fails this guard",
            );
        assert!(
            event.has_field("player_id", &phantom_id.to_string()),
            "warn must carry player_id field; got {event:?}"
        );
        assert!(
            event.has_field("rows_affected", "0") && event.has_field("expected", "1"),
            "warn must carry the Pattern B counts rows_affected = 0, expected = 1; got {event:?}"
        );
    }
}
