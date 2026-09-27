//! BaseApp-side handlers for the GM crafting grants.
//!
//! [`handle_grant_expertise`] / [`handle_grant_applied_science`] mirror
//! `progression::handle_grant_cash`: change the persistent state in one
//! transaction, then push the client update. They are the canonical
//! one-way sinks for `CellToBaseMsg::GrantExpertise` /
//! `CellToBaseMsg::GrantAppliedSciencePoints`.
//!
//! Both write under the `sgw_player` row lock, as `spend` does, so a grant
//! racing a spend can neither drop the spent point nor the learned
//! discipline.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use cimmeria_entity::crafting::CraftingState;

use crate::base::crafting::persistence::{load_crafting_state_locked, save_crafting_state_in};
use crate::base::crafting::sync::{push_asp, push_discipline, CraftClient};
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::ConnectedClientState;

/// Crafting expertise hard cap, matching Python's `gainExpertise` and
/// `CraftingState::set_expertise`. We clamp explicitly here too so the
/// `onUpdateDiscipline` payload always reflects the persisted (clamped) value.
const EXPERTISE_CAP: i32 = 100;

/// Handle `gmGiveExpertise` from CellService — load crafting state, add the
/// expertise delta (clamped to `[0, 100]`), register the discipline if it's
/// new, persist, and push `onUpdateDiscipline` to the client.
#[tracing::instrument(
    name = "crafting.grant_expertise",
    level = "info",
    skip_all,
    fields(entity_id, player_id, discipline_id, amount)
)]
pub async fn handle_grant_expertise(
    entity_id: u32,
    player_id: i32,
    discipline_id: i32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            // Mirror GrantCash's no-DB stance: without persistence there's no
            // authoritative expertise to send, so drop rather than emit a
            // client update the server can't back.
            tracing::warn!(
                entity_id,
                player_id,
                discipline_id,
                amount,
                "GrantExpertise: no DB pool, dropping grant"
            );
            return;
        }
    };

    let new_expertise = match grant_expertise_in_db(pool, player_id, discipline_id, amount).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                discipline_id,
                amount,
                "GrantExpertise: transaction failed: {e}"
            );
            send_gm_feedback_to_client(
                entity_id,
                &format!("gmGiveExpertise: failed (save error for discipline {discipline_id})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        discipline_id,
        new_expertise,
        "GrantExpertise: persisted expertise"
    );

    // Definitive success feedback: the write committed, so the GM gets the
    // real persisted (clamped) expertise value.
    send_gm_feedback_to_client(
        entity_id,
        &format!("gmGiveExpertise: discipline {discipline_id} now {new_expertise}"),
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    // Push onUpdateDiscipline (method 136) to the client so the crafting UI
    // reflects the new discipline/percentage without a relog.
    let client = CraftClient {
        transport,
        connected,
        entity_to_addr,
    };
    push_discipline(entity_id, discipline_id, new_expertise, client).await;
}

/// The expertise grant's transaction: lock the row, add `amount` (clamped to
/// `[0, 100]`), register the discipline if it is new, save, commit. Returns
/// the persisted expertise.
async fn grant_expertise_in_db(
    pool: &PgPool,
    player_id: i32,
    discipline_id: i32,
    amount: i32,
) -> Result<i32, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut state: CraftingState = load_crafting_state_locked(&mut tx, player_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    // `saturating_add`: `amount` is only gated `> 0` on the cell side (no upper
    // bound), so a huge grant must not overflow i32 before the clamp.
    let new_expertise = state
        .get_expertise(discipline_id)
        .unwrap_or(0)
        .saturating_add(amount)
        .clamp(0, EXPERTISE_CAP);
    state.set_expertise(discipline_id, new_expertise);
    // Register the discipline as known if this is the first grant — a stray
    // expertise row without the discipline in `discipline_ids` is the drift
    // the persistence layer explicitly tolerates but doesn't create.
    if !state.discipline_ids.contains(&discipline_id) {
        state.discipline_ids.push(discipline_id);
    }
    save_crafting_state_in(&mut tx, player_id, &state).await?;
    tx.commit().await?;
    Ok(new_expertise)
}

/// Handle `gmGiveAppliedSciencePoints` from CellService — add `amount` to
/// `applied_science_points` in one statement, then push the new **total**
/// to the client as `onEntityProperty(GENERICPROPERTY_AppliedSciencePoints,
/// total)`, the property the discipline trainer listens for
/// (`DisciplineTrainer.lua:49-54`). The count updates without a relog
/// (audit C-06).
///
/// The single `UPDATE … RETURNING` takes the row lock itself, so a grant
/// racing a spend adds to the committed value instead of overwriting it.
#[tracing::instrument(
    name = "crafting.grant_applied_science",
    level = "info",
    skip_all,
    fields(entity_id, player_id, amount)
)]
pub async fn handle_grant_applied_science(
    entity_id: u32,
    player_id: i32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                amount,
                "GrantAppliedSciencePoints: no DB pool, dropping grant"
            );
            return;
        }
    };

    // Saturate at i32::MAX in SQL, as the old in-memory `saturating_add`
    // did: `amount` has no upper bound on the cell side.
    let updated: Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
        "UPDATE sgw_player \
         SET applied_science_points = \
             LEAST(applied_science_points::bigint + $2, 2147483647)::integer \
         WHERE player_id = $1 \
         RETURNING applied_science_points",
    )
    .bind(player_id)
    .bind(i64::from(amount))
    .fetch_optional(pool.as_ref())
    .await;
    let new_total = match updated {
        Ok(Some(total)) => total,
        Ok(None) => {
            tracing::error!(
                entity_id,
                player_id,
                amount,
                rows_affected = 0,
                "GrantAppliedSciencePoints: no sgw_player row -- nothing granted"
            );
            send_gm_feedback_to_client(
                entity_id,
                "gmGiveAppliedSciencePoints: failed (no such character)",
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                amount,
                "GrantAppliedSciencePoints: UPDATE failed: {e}"
            );
            send_gm_feedback_to_client(
                entity_id,
                "gmGiveAppliedSciencePoints: failed (save error)",
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        amount,
        total = new_total,
        "GrantAppliedSciencePoints: persisted ASP"
    );

    // Definitive success feedback: the write committed. This is the GM-command
    // confirmation line on CHAN_FEEDBACK; the ASP display updates from the
    // property push below.
    send_gm_feedback_to_client(
        entity_id,
        &format!("gmGiveAppliedSciencePoints: +{amount} (total {new_total})"),
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    let client = CraftClient {
        transport,
        connected,
        entity_to_addr,
    };
    push_asp(entity_id, new_total, client).await;
}

#[cfg(test)]
mod tests {
    //! Live-DB regression guard for the expertise base handler. Self-skips
    //! when `DATABASE_URL` is unset via `require_db_or_skip!`. Drives the real
    //! `handle_grant_expertise` (load → clamp → register discipline → save)
    //! against a fresh player row; the client-push half no-ops harmlessly
    //! because `entity_to_addr` is empty (the handler warns and skips the send,
    //! but the DB writes still commit — which is what we assert).

    use super::*;
    use crate::base::crafting::persistence::load_crafting_state;
    use crate::test_support::{require_db_or_skip, TestTransport};

    /// Sentinel base in the crafting `0x7000_Cxxx` block, past the
    /// persistence tests' `0x7000_C000..0x7000_CB55`. Fits in i32.
    const TEST_BASE: i32 = 0x7000_CC00;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_minimal_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("craft-h-test-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("craft-h-test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    fn empty_handler_io() -> (
        Arc<dyn Transport>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let connected = Arc::new(Mutex::new(HashMap::new()));
        // Empty entity_to_addr → the onUpdateDiscipline push is skipped
        // (handler warns), but the load/save DB writes still commit.
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        (transport, connected, entity_to_addr)
    }

    /// `handle_grant_expertise` must persist the clamped expertise AND register
    /// the discipline in `discipline_ids` when it's the first grant. Reverting
    /// either the `set_expertise` save or the `discipline_ids.push` trips this.
    #[tokio::test]
    async fn grant_expertise_persists_and_registers_discipline() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let (transport, connected, entity_to_addr) = empty_handler_io();
        let db_pool = Some(Arc::new(pool.clone()));

        // First grant: discipline 7, +40 expertise.
        handle_grant_expertise(
            999, // entity_id (no addr → client push skipped)
            player_id,
            7,
            40,
            &db_pool,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let state = load_crafting_state(&pool, player_id).await.expect("reload");
        assert_eq!(
            state.get_expertise(7),
            Some(40),
            "first expertise grant must persist the additive value"
        );
        assert!(
            state.discipline_ids.contains(&7),
            "first expertise grant must register the discipline in discipline_ids"
        );

        // Second grant: +80 should clamp to the 100 cap, not become 120.
        handle_grant_expertise(
            999,
            player_id,
            7,
            80,
            &db_pool,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let state = load_crafting_state(&pool, player_id)
            .await
            .expect("reload 2");
        assert_eq!(
            state.get_expertise(7),
            Some(100),
            "cumulative expertise must clamp to the 100 cap (40 + 80 → 100)"
        );
        assert_eq!(
            state.discipline_ids.iter().filter(|&&d| d == 7).count(),
            1,
            "a second grant for the same discipline must NOT duplicate it in discipline_ids"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// `handle_grant_applied_science` must accumulate ASP across grants.
    #[tokio::test]
    async fn grant_applied_science_accumulates() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 10;
        let player_id = TEST_BASE + 11;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        let db_pool = Some(Arc::new(pool.clone()));
        // Empty entity_to_addr → the GM-feedback client push is skipped
        // (send_to_witness_reliable no-ops), but the load/save DB writes commit.
        let (transport, connected, entity_to_addr) = empty_handler_io();

        handle_grant_applied_science(
            999,
            player_id,
            5,
            &db_pool,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
        handle_grant_applied_science(
            999,
            player_id,
            7,
            &db_pool,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let state = load_crafting_state(&pool, player_id).await.expect("reload");
        assert_eq!(
            state.applied_science_points, 12,
            "applied-science grants must accumulate (5 + 7 = 12)"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// The GM ASP grant pushes the new **total** as the ASP property, after
    /// the GM's confirmation line, so the discipline trainer's count updates
    /// without a relog (audit C-06). Removing the push leaves one packet;
    /// pushing the change (+5) instead of the total (9) fails the bytes.
    #[tokio::test]
    async fn grant_applied_science_pushes_the_total_property() {
        use crate::base::crafting::test_players::OneSession;
        use crate::mercury::{build_player_entity_method_packet, method_idx};
        use cimmeria_mercury::encryption::EncryptionVersion;

        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 20;
        let player_id = TEST_BASE + 21;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;
        sqlx::query("UPDATE sgw_player SET applied_science_points = 4 WHERE player_id = $1")
            .bind(player_id)
            .execute(&pool)
            .await
            .expect("seed ASP");

        const ENTITY: u32 = 4280;
        let session = OneSession::new(ENTITY, 55740);
        let db_pool = Some(Arc::new(pool.clone()));
        handle_grant_applied_science(
            ENTITY,
            player_id,
            5,
            &db_pool,
            &session.transport,
            &session.connected,
            &session.entity_to_addr,
        )
        .await;
        let sent = session.typed.filter_to(session.addr);
        cleanup(&pool, account_id, player_id).await;

        let packet = |seq, method, args: &[u8]| {
            build_player_entity_method_packet(
                &[0u8; 32],
                seq,
                &[],
                ENTITY,
                method,
                args,
                EncryptionVersion::V1,
            )
        };
        assert_eq!(
            sent,
            vec![
                packet(
                    0,
                    method_idx::ON_PLAYER_COMMUNICATION,
                    &cimmeria_wire::cell::chat::serialize_on_player_communication(
                        "SYSTEM",
                        0,
                        cimmeria_wire::cell::chat::CHAN_FEEDBACK,
                        "gmGiveAppliedSciencePoints: +5 (total 9)",
                    ),
                ),
                packet(1, method_idx::ON_ENTITY_PROPERTY, &[2, 0, 0, 0, 9, 0, 0, 0]),
            ]
        );
    }
}
