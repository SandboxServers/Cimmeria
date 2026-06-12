//! `CellToBaseMsg::StateFieldUpdate` — persist the user-preference bits
//! of the player's `state_field` bitmask.
//!
//! The cell side owns the in-memory `state_field` on every `CellEntity`;
//! the base side owns DB writes. When the player presses the auto-cycle
//! button (`setAutoCycle`, player method 83) and `BSF_AutoCycling`
//! transitions, the cell handler broadcasts `onStateFieldUpdate` to the
//! client, then sends this message so base persists the row. On next
//! login, the hydrate path in
//! `crates/services/src/base/world_entry_appearance.rs` reads the column
//! back into `InitPlayerState` and the preference survives the relog. (#412)
//!
//! Schema column:
//!
//! - `sgw_player.state_field INTEGER NOT NULL DEFAULT 0`
//!
//! Only bits in [`crate::cell::combat::PERSISTED_STATE_FIELD_MASK`] are
//! ever stored. The cell send site already masks; this handler masks
//! again defensively so a future send site that forgets can't leak a
//! transient combat bit (BSF_Dead, BSF_InCombat, BSF_MovementLock) into
//! durable state — a relog must always be a clean combat slate.
//!
//! No appearance refresh needed (unlike `ActiveSlotUpdate`) — the bit
//! affects the player's own UI highlight + the server loop driver, not
//! the visual model.

use std::sync::Arc;

use sqlx::PgPool;

use crate::cell::combat::PERSISTED_STATE_FIELD_MASK;

/// Persist a player's preference `state_field` bits to `sgw_player`.
/// Returns silently after a `warn` if the row is missing or the write
/// fails — the in-memory cell-side value still applies for the rest of
/// the session, so the toggle isn't completely lost even when
/// persistence breaks. Loud enough that ops will notice.
pub(super) async fn persist_state_field(
    player_id: i32,
    state_field: u32,
    db_pool: &Option<Arc<PgPool>>,
) {
    let Some(pool) = db_pool else {
        // No pool means the server is running without persistence (test
        // / repl / smoke mode). Mirror the system-options handler:
        // silent no-op.
        return;
    };

    let masked = state_field & PERSISTED_STATE_FIELD_MASK;

    match sqlx::query("UPDATE sgw_player SET state_field = $1 WHERE player_id = $2")
        .bind(masked as i32)
        .bind(player_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(res) if res.rows_affected() == 0 => {
            // No row updated — typically a test fixture or a deleted
            // character. Loud because losing this silently means the
            // next relog hydrates to 0 and the user thinks the toggle
            // didn't save.
            tracing::warn!(
                player_id,
                state_field = masked,
                "StateFieldUpdate: no rows updated (player row missing?)"
            );
        }
        Ok(_) => {
            tracing::info!(
                player_id,
                state_field = masked,
                "StateFieldUpdate: persisted"
            );
        }
        Err(e) => {
            tracing::warn!(
                player_id,
                state_field = masked,
                error = %e,
                "StateFieldUpdate: DB write failed (in-memory value still applies for this session)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB regression guards for the state_field persistence path
    //! (#412). Sentinel id range `0x7000_1700` — sibling slot reserved
    //! past `system_options::tests` at `0x7000_1600`.

    use super::*;
    use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK};
    use crate::test_support::require_db_or_skip;

    const TEST_BASE: i32 = 0x7000_1700;

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
        .bind(format!("statefield-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
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
        .bind(format!("stfd-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    async fn read_state_field(pool: &PgPool, player_id: i32) -> i32 {
        sqlx::query_scalar("SELECT state_field FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .expect("read state_field")
    }

    /// **#412 round-trip.** A fresh row defaults to 0; persisting the
    /// auto-cycle bit stores it; persisting 0 (explicit disable) clears
    /// it. Reverting the persist implementation to a no-op fails the
    /// middle assertion.
    #[tokio::test]
    async fn auto_cycle_bit_round_trips() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;
        let pool_opt = Some(Arc::new(pool.clone()));

        assert_eq!(
            read_state_field(&pool, player_id).await,
            0,
            "fresh player row must default state_field to 0"
        );

        persist_state_field(player_id, BSF_AUTO_CYCLING, &pool_opt).await;
        assert_eq!(
            read_state_field(&pool, player_id).await,
            BSF_AUTO_CYCLING as i32,
            "enable toggle must store the auto-cycle bit"
        );

        persist_state_field(player_id, 0, &pool_opt).await;
        assert_eq!(
            read_state_field(&pool, player_id).await,
            0,
            "disable toggle must clear the stored bit"
        );

        cleanup(&pool, account_id).await;
    }

    /// **#412 mask guard.** Transient combat bits must never reach the
    /// DB even when a caller passes an unmasked `state_field` (e.g. the
    /// raw broadcast value while the player is mid-fight: dead +
    /// in-combat + movement-locked + auto-cycling). Only the
    /// auto-cycle bit may survive the write. Reverting the
    /// `& PERSISTED_STATE_FIELD_MASK` in `persist_state_field` fails
    /// this.
    #[tokio::test]
    async fn transient_combat_bits_are_masked_out() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 0x10;
        let player_id = TEST_BASE + 0x11;
        cleanup(&pool, account_id).await;
        insert_player(&pool, account_id, player_id).await;
        let pool_opt = Some(Arc::new(pool.clone()));

        let mid_fight = BSF_DEAD | BSF_AUTO_CYCLING | BSF_IN_COMBAT | BSF_MOVEMENT_LOCK;
        persist_state_field(player_id, mid_fight, &pool_opt).await;
        assert_eq!(
            read_state_field(&pool, player_id).await,
            BSF_AUTO_CYCLING as i32,
            "only PERSISTED_STATE_FIELD_MASK bits may be stored — a dead, \
             movement-locked relog would otherwise spawn the player frozen"
        );

        // Pure-transient value (no preference bit at all) must store 0.
        persist_state_field(player_id, BSF_DEAD | BSF_IN_COMBAT, &pool_opt).await;
        assert_eq!(
            read_state_field(&pool, player_id).await,
            0,
            "an all-transient state_field must persist as 0"
        );

        cleanup(&pool, account_id).await;
    }

    /// Missing-row contract: persisting against a non-existent player_id
    /// must warn (not error / not silently succeed), mirroring the
    /// system-options handler per the negative-logging convention.
    #[tokio::test]
    async fn persist_no_row_is_silent_warn() {
        use crate::test_support::LogCapture;
        let pool = require_db_or_skip!();
        let pool_opt = Some(Arc::new(pool.clone()));
        let phantom_id = TEST_BASE + 0x20;
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(phantom_id)
            .execute(&pool)
            .await;

        let capture = LogCapture::install();
        persist_state_field(phantom_id, BSF_AUTO_CYCLING, &pool_opt).await;

        let event = capture
            .find_message(tracing::Level::WARN, "StateFieldUpdate: no rows updated")
            .expect(
                "missing-row persist must emit the documented warn — \
                 reverting the rows_affected==0 arm fails this guard",
            );
        assert!(
            event.has_field("player_id", &phantom_id.to_string()),
            "warn must carry player_id field; got {:?}",
            event
        );
    }
}
