//! `BaseToCellMsg::MinigameResult` handler — on a victory result, fires each
//! `on_victory_chains` chain through the content engine. Extracted from
//! `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::known_names;

use crate::cell::content;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Server-channel result codes that fire `on_failure_chains` (D-MG5).
const RESULT_FAILURE: u8 = 2;
const RESULT_DEFEATED: u8 = 4;

/// Handle `BaseToCellMsg::MinigameResult`.
pub(super) async fn handle_minigame_result(
    entity_id: u32,
    player_id: i32,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    on_failure_chains: Vec<i64>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        entity_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        receipt_player_id = player_id,
        receipt_player_name = known_names::player_name(player_id),
        result_code,
        result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(result_code),
        chains = ?on_victory_chains,
        failure_chains = ?on_failure_chains,
        "Minigame result"
    );
    // Any code ends the minigame (D-MG15): victory, failure, cancel, a failed
    // launch or an expired ticket. Release before the chains fire so a chain
    // that moves or locks the player starts from a free state.
    cimmeria_cell_world::cell::minigame_hold::release_minigame_hold(entity_id, tx, space_mgr).await;
    if result_code == 1 {
        // Victory: the receipt's player (minted by Base at launch) must be the
        // player the cell holds in this slot, or a slot reused by someone else
        // would collect the previous player's reward. Checked once, before any chain.
        let Some(entity) = space_mgr.get_entity(entity_id) else {
            tracing::warn!(
                entity_id, // nt:id-only the cell holds no entity under this id, so there is no name to give
                expected_player_id = player_id,
                expected_player_name = known_names::player_name(player_id),
                chain_ids = ?on_victory_chains,
                reason = "entity_missing",
                "Minigame victory for an entity the cell does not hold; no chain fires"
            );
            return;
        };
        if !entity.is_player || player_id == 0 || entity.player_id != Some(player_id) {
            tracing::warn!(
                entity_id,
                entity_name = id.player_name,
                expected_player_id = player_id,
                expected_player_name = known_names::player_name(player_id),
                actual_player_id = entity.player_id.unwrap_or(0),
                actual_player_name = known_names::player_name(entity.player_id),
                chain_ids = ?on_victory_chains,
                reason = "player_mismatch",
                "Minigame victory receipt does not match the player the cell holds; no chain fires"
            );
            return;
        }
        let held_player_id = player_id;
        for chain_id in &on_victory_chains {
            // Conditioned chains (Castle 1372/1375) must have their conditions
            // evaluated; unconditioned legacy chains keep the old unguarded path.
            if engine.chain_has_conditions(*chain_id) {
                content::fire_guarded_minigame_chain_by_id(
                    *chain_id, entity_id, engine, tx, space_mgr,
                )
                .await;
            } else {
                content::fire_chain_by_id(
                    *chain_id,
                    entity_id,
                    held_player_id,
                    engine,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    } else if result_code == RESULT_FAILURE || result_code == RESULT_DEFEATED {
        // Same receipt-owner check as the victory path: a slot reused by
        // another player must not run the previous player's failure chains.
        let held = space_mgr
            .get_entity(entity_id)
            .filter(|e| e.is_player && player_id != 0 && e.player_id == Some(player_id))
            .is_some();
        if !held {
            if !on_failure_chains.is_empty() {
                tracing::warn!(
                    entity_id,
                    entity_name = id.player_name,
                    expected_player_id = player_id,
                    expected_player_name = known_names::player_name(player_id),
                    chain_ids = ?on_failure_chains,
                    reason = "player_mismatch",
                    "Minigame failure receipt does not match the player the cell holds; no chain fires"
                );
            }
            return;
        }
        for chain_id in &on_failure_chains {
            content::fire_chain_by_id(*chain_id, entity_id, player_id, engine, tx, space_mgr).await;
        }
    }
}
