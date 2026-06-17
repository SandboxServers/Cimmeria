//! BaseApp-side handlers for the GM crafting grants.
//!
//! [`handle_grant_expertise`] / [`handle_grant_applied_science`] mirror
//! `progression::handle_grant_cash`: load the persistent state, mutate it,
//! save, and (for expertise) push the client update. They are the canonical
//! one-way sinks for `CellToBaseMsg::GrantExpertise` /
//! `CellToBaseMsg::GrantAppliedSciencePoints`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::crafting::persistence::{load_crafting_state, save_crafting_state};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

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

    let mut state = match load_crafting_state(pool, player_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                discipline_id,
                "GrantExpertise: load_crafting_state failed: {e}"
            );
            return;
        }
    };

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

    if let Err(e) = save_crafting_state(pool, player_id, &state).await {
        tracing::error!(
            entity_id,
            player_id,
            discipline_id,
            new_expertise,
            "GrantExpertise: save_crafting_state failed: {e}"
        );
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        discipline_id,
        new_expertise,
        "GrantExpertise: persisted expertise"
    );

    // Push onUpdateDiscipline (method 136) to the client so the crafting UI
    // reflects the new discipline/percentage without a relog.
    let payload =
        cimmeria_entity::crafting::serialize_on_update_discipline(discipline_id, new_expertise);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_UPDATE_DISCIPLINE,
                &payload,
            )
        },
    )
    .await;
}

/// Handle `gmGiveAppliedSciencePoints` from CellService — load crafting state,
/// add `amount` to `applied_science_points`, and persist.
///
/// There is no outbound applied-science-points client method in the SGWPlayer
/// method table, so this handler persists only. The client refreshes its ASP
/// display the next time the crafting window is opened (which re-reads state)
/// or on relog (via the world-entry crafting-state load). We deliberately do
/// NOT invent a method index to push an update.
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

    let mut state = match load_crafting_state(pool, player_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "GrantAppliedSciencePoints: load_crafting_state failed: {e}"
            );
            return;
        }
    };

    state.applied_science_points = state.applied_science_points.saturating_add(amount);

    if let Err(e) = save_crafting_state(pool, player_id, &state).await {
        tracing::error!(
            entity_id,
            player_id,
            amount,
            "GrantAppliedSciencePoints: save_crafting_state failed: {e}"
        );
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        amount,
        total = state.applied_science_points,
        "GrantAppliedSciencePoints: persisted ASP (no client push — client \
         refreshes on next crafting open / relog)"
    );
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

    /// Sentinel base stepped past the persistence-module range (0x7000_2000)
    /// so concurrent live-DB runs don't collide. Fits in i32.
    const TEST_BASE: i32 = 0x7000_3000;

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

        handle_grant_applied_science(999, player_id, 5, &db_pool).await;
        handle_grant_applied_science(999, player_id, 7, &db_pool).await;

        let state = load_crafting_state(&pool, player_id).await.expect("reload");
        assert_eq!(
            state.applied_science_points, 12,
            "applied-science grants must accumulate (5 + 7 = 12)"
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
