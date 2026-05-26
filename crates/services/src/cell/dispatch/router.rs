//! [`dispatch_cell_method`] — the single entry point that routes a flattened
//! cell method index to its per-interface dispatch function.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::cell_methods;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// Dispatch a client->server cell method call to the appropriate handler.
///
/// Each interface's dispatch function returns `true` if it handled the method,
/// `false` if the index is outside its range. We try each interface in
/// inheritance order and stop at the first match.
///
/// The `#[instrument]` here is the canonical "what method did the
/// player just invoke?" span. Every interface-level dispatch inside
/// becomes a child of this span, and any DB query / chain action /
/// witness fan-out further down the call tree inherits the span
/// context automatically. SigNoz groups them by `method_index` so the
/// service map shows method-call hot spots at a glance.
///
/// `level = "debug"` because the rate is bounded by player count × tick;
/// flip `RUST_LOG=cimmeria_services::cell::dispatch=debug` to turn on.
#[tracing::instrument(
    name = "cell.dispatch",
    level = "debug",
    skip_all,
    fields(entity_id, method_index, args_len = args.len(), space_id = tracing::field::Empty),
)]
pub async fn dispatch_cell_method(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Backfill space_id so SigNoz can pivot dispatches by world/instance.
    if let Some(e) = space_mgr.get_entity(entity_id) {
        tracing::Span::current().record("space_id", e.space_id.0);
    }
    // SGWBeing interface (0–1)
    if cell_methods::being::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWAbilityManager interface (2–4)
    if cell_methods::ability_manager::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWCombatant interface (5–7)
    if cell_methods::combatant::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // OrganizationMember interface (8–19)
    if cell_methods::organization::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // MinigamePlayer interface (20–34)
    if cell_methods::minigame::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // GateTravel interface (35)
    if cell_methods::gate_travel::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWInventoryManager interface (36–42) — needs engine for useItem content chains
    if cell_methods::inventory::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
    {
        return;
    }
    // SGWMailManager interface (43–51)
    if cell_methods::mail::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // Missionary interface (52–54)
    if cell_methods::missionary::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // ContactListManager interface (55–60)
    if cell_methods::contact_list::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWBlackMarketManager interface (61–66)
    if cell_methods::black_market::dispatch(entity_id, method_index, args, tx, space_mgr).await {
        return;
    }
    // SGWPlayer own methods (67–108) — needs engine for content chains
    if cell_methods::player::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await {
        return;
    }

    tracing::info!(
        entity_id,
        method_index,
        args_len = args.len(),
        "Unhandled cell method call"
    );
}
