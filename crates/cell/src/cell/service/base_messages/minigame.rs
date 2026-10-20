//! `BaseToCellMsg::MinigameResult` handler — on a victory result, fires each
//! `on_victory_chains` chain through the content engine. Extracted from
//! `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Server-channel result codes that fire `on_failure_chains` (D-MG5).
const RESULT_FAILURE: u8 = 2;
const RESULT_DEFEATED: u8 = 4;

/// Handle `BaseToCellMsg::MinigameResult`.
pub(super) async fn handle_minigame_result(
    entity_id: u32,
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
        // Victory — fire on_victory_chains through the content engine
        let entity = space_mgr.get_entity(entity_id);
        if entity.is_none() {
            // The chains still fire below with player 0 (behaviour unchanged);
            // this row says that the player they name is not really there.
            tracing::warn!(
                entity_id, // nt:id-only the cell holds no entity under this id, so there is no name to give
                chain_count = on_victory_chains.len(),
                result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(
                    result_code
                ),
                reason = "entity_missing",
                "Minigame victory for an entity the cell does not hold; chains fire with player 0"
            );
        }
        let player_id = entity.and_then(|e| e.player_id).unwrap_or(0);
        for chain_id in &on_victory_chains {
            content::fire_chain_by_id(*chain_id, entity_id, player_id, engine, tx, space_mgr).await;
        }
    } else if result_code == RESULT_FAILURE || result_code == RESULT_DEFEATED {
        // Same player-id resolution as the victory path: a missing entity
        // fires with player 0 rather than dropping the failure chains.
        let player_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.player_id)
            .unwrap_or(0);
        for chain_id in &on_failure_chains {
            content::fire_chain_by_id(*chain_id, entity_id, player_id, engine, tx, space_mgr).await;
        }
    }
}
