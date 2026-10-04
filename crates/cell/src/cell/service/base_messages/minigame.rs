//! `BaseToCellMsg::MinigameResult` handler — on a victory result, fires each
//! `on_victory_chains` chain through the content engine. Extracted from
//! `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Handle `BaseToCellMsg::MinigameResult`.
pub(super) async fn handle_minigame_result(
    entity_id: u32,
    result_code: u8,
    on_victory_chains: Vec<i64>,
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
        "Minigame result"
    );
    if result_code == 1 {
        // Victory — fire on_victory_chains through the content engine
        let player_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.player_id)
            .unwrap_or(0);
        for chain_id in &on_victory_chains {
            content::fire_chain_by_id(*chain_id, entity_id, player_id, engine, tx, space_mgr).await;
        }
    }
}
