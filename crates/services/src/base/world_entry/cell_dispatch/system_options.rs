//! `CellToBaseMsg::SystemOptionsUpdate` — persist the player's
//! server-synced client options.
//!
//! The cell side owns the in-memory `SystemOptions` block on every
//! `CellEntity`; the base side owns DB writes. When the client pushes a
//! changed option via `updateSystemOptions` (player method 93), the cell
//! handler applies it in memory, then sends this message so base persists
//! the row. On next login, the hydrate path in
//! `crates/services/src/base/world_entry_appearance.rs` reads these
//! columns back into `InitPlayerState` and the option survives the relog.
//!
//! Schema columns:
//!
//! - `sgw_player.auto_reload BOOLEAN NOT NULL DEFAULT TRUE`
//! - `sgw_player.reload_on_activate BOOLEAN NOT NULL DEFAULT FALSE`
//!
//! Defaults mirror `SGWGame/Content/XML/SystemOptions.xml`.
//!
//! No appearance refresh needed (unlike `ActiveSlotUpdate`) — these
//! options affect server-side decision logic, not the visual model.

use std::sync::Arc;

use sqlx::PgPool;

/// Persist a player's autoReload + reloadOnActivate to `sgw_player`.
/// Returns silently after a `warn` if the row is missing or the write
/// fails — the in-memory cell-side value still applies for the rest of
/// the session, so the user's checkbox toggle isn't completely lost
/// even when persistence breaks. Loud enough that ops will notice.
pub(super) async fn persist_system_options(
    player_id: i32,
    auto_reload: bool,
    reload_on_activate: bool,
    db_pool: &Option<Arc<PgPool>>,
) {
    let Some(pool) = db_pool else {
        // No pool means the server is running without persistence (test
        // / repl / smoke mode). Mirror what the bandolier-slot handler
        // does: silent no-op.
        return;
    };

    match sqlx::query(
        "UPDATE sgw_player SET auto_reload = $1, reload_on_activate = $2 \
         WHERE player_id = $3",
    )
    .bind(auto_reload)
    .bind(reload_on_activate)
    .bind(player_id)
    .execute(pool.as_ref())
    .await
    {
        Ok(res) if res.rows_affected() == 0 => {
            // No row updated — typically a test fixture or a deleted
            // character. Loud because losing this silently means the
            // next relog hydrates to defaults and the user thinks the
            // checkbox didn't save.
            tracing::warn!(
                player_id,
                auto_reload,
                reload_on_activate,
                "SystemOptionsUpdate: no rows updated (player row missing?)"
            );
        }
        Ok(_) => {
            tracing::info!(
                player_id,
                auto_reload,
                reload_on_activate,
                "SystemOptionsUpdate: persisted"
            );
        }
        Err(e) => {
            tracing::warn!(
                player_id,
                auto_reload,
                reload_on_activate,
                error = %e,
                "SystemOptionsUpdate: DB write failed (in-memory value still applies for this session)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB regression guard for the autoReload / reloadOnActivate
    //! persistence path. Sentinel id range `0x7000_1600` — sibling slot
    //! reserved past the highest existing reservation
    //! (`paid_recharge::tests` at `0x7000_1400` and `player_load::core`'s
    //! second block at `0x7000_1500`).

    use super::*;
    use crate::test_support::require_db_or_skip;

    const TEST_BASE: i32 = 0x7000_1600;

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
        .bind(format!("sysopt-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
        // Defaults — auto_reload=TRUE, reload_on_activate=FALSE — are
        // honoured by the schema's DEFAULT clauses; we don't bind them
        // here. The test's first assertion below pins those defaults.
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'CombatSim', \
                       'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("syso-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// Default-row contract: a freshly-inserted player must have
    /// `auto_reload = TRUE` and `reload_on_activate = FALSE`, matching
    /// `SGWGame/Content/XML/SystemOptions.xml`. A schema regression that
    /// drops the DEFAULT clauses or flips the polarity would fail this.
    #[tokio::test]
    async fn fresh_player_row_has_xml_defaults() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;

        let (auto_reload, reload_on_activate): (bool, bool) = sqlx::query_as(
            "SELECT auto_reload, reload_on_activate FROM sgw_player \
             WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .expect("read defaults");
        assert!(auto_reload, "auto_reload must default to TRUE");
        assert!(
            !reload_on_activate,
            "reload_on_activate must default to FALSE"
        );

        cleanup(&pool, account_id).await;
    }

    /// Persist-on-change contract: calling `persist_system_options` with
    /// flipped values writes them through to the row, and a follow-up
    /// call with different values overwrites cleanly. Reverting the
    /// implementation to the `tracing::info!("UNIMPLEMENTED")` stub
    /// fails this — the columns never change.
    #[tokio::test]
    async fn persist_flips_columns_in_db() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 0x10;
        let player_id = TEST_BASE + 0x11;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;

        // First persist: flip both columns.
        let pool_opt = Some(Arc::new(pool.clone()));
        persist_system_options(player_id, false, true, &pool_opt).await;

        let (a1, r1): (bool, bool) = sqlx::query_as(
            "SELECT auto_reload, reload_on_activate FROM sgw_player \
             WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .expect("read after persist");
        assert!(!a1, "first persist must set auto_reload to FALSE");
        assert!(r1, "first persist must set reload_on_activate to TRUE");

        // Second persist: half-flip — auto_reload back to true, leave
        // reload_on_activate on. Writes the full pair atomically so a
        // partial-update bug would surface here.
        persist_system_options(player_id, true, true, &pool_opt).await;
        let (a2, r2): (bool, bool) = sqlx::query_as(
            "SELECT auto_reload, reload_on_activate FROM sgw_player \
             WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .expect("read after second persist");
        assert!(a2, "second persist must flip auto_reload back to TRUE");
        assert!(r2, "reload_on_activate must remain TRUE across persists");

        cleanup(&pool, account_id).await;
    }

    /// Missing-row contract: persisting against a non-existent player_id
    /// must not error out — it just no-ops and logs a `warn!`. Mirrors
    /// the bandolier-slot handler's behavior so a stale player_id
    /// (race with character deletion) doesn't crash the dispatcher.
    /// The `LogCapture` assertion is the actual regression guard —
    /// without it, the test would pass even if the function silently
    /// swallowed the no-rows path. The fix-revert shape is removing
    /// the explicit `rows_affected == 0` arm; that arm's `warn!`
    /// emission is what the capture pins.
    #[tokio::test]
    async fn persist_no_row_is_silent_warn() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let pool_opt = Some(Arc::new(pool.clone()));
        // ID guaranteed to not exist — same sentinel range, but a
        // shifted offset that the other tests never use. Explicitly
        // clean it first so a previous test failure that left rows
        // around can't change the result here.
        let phantom_id = TEST_BASE + 0x20;
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(phantom_id)
            .execute(&pool)
            .await;

        let capture = LogCapture::install();
        persist_system_options(phantom_id, true, true, &pool_opt).await;

        // The handler's "no rows updated" arm must emit exactly the
        // warn we expect — message substring pinned per the
        // negative-logging convention.
        let event = capture
            .find_message(tracing::Level::WARN, "SystemOptionsUpdate: no rows updated")
            .expect(
                "missing-row persist must emit the documented warn — \
                 reverting the rows_affected==0 arm fails this guard",
            );
        // The phantom id should be on the event so an operator can
        // identify the stale character without a DB query.
        assert!(
            event.has_field("player_id", &phantom_id.to_string()),
            "warn must carry player_id field; got {:?}",
            event
        );
    }
}
