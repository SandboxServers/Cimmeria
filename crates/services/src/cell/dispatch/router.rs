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

    // Server-authority GM gate (#475 / CAT-N-03). For GM/debug method
    // indices, reject any caller whose `CellEntity::access_level` is below
    // GameMaster *before* the method reaches a handler. Ordinary player
    // methods pass through untouched. The gate emits its own `warn!` +
    // `onErrorCode` on rejection, so a `false` return is terminal.
    if !super::gm_gate::enforce_gm_gate(entity_id, method_index, tx, space_mgr).await {
        return;
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

    // Promoted from info! per #311 (Tier 4 follow-up to #304).
    // `info!` was noisy at the volume of cell methods that traverse the
    // router and lacked actionable framing; `warn!` is greppable as the
    // direct signal for "client called a method we don't implement".
    // Reverting to `info!` makes the dispatcher gap invisible on a
    // warn-filtered ops dashboard.
    tracing::warn!(
        entity_id,
        method_index,
        args_len = args.len(),
        "Unhandled cell method call -- no registered handler for this index; client behaviour may diverge silently"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_space_manager, LogCapture};
    use tracing::Level;

    /// **Regression guard for issue #311** (Tier 4 follow-up to #304).
    ///
    /// A cell-method dispatch with no registered handler must fire `warn!`
    /// (not `info!`) so the dispatcher gap appears on a warn-filtered ops
    /// dashboard. Reverting to `info!` fails the level check below.
    ///
    /// Bug shape: the previous `info!` was noisy and lacked actionable
    /// framing — when a player triggered an unimplemented cell method
    /// (often the case as new client features ship), the silent
    /// behavioural divergence required a player ticket to surface. This
    /// guard also pins the structured `method_index` and `args_len`
    /// fields so an ops query like `level=warn message="Unhandled cell"
    /// method_index=$N` keeps working after refactors.
    #[tokio::test]
    async fn unhandled_cell_method_warns_with_method_index_and_args_len() {
        let capture = LogCapture::install();

        let mut mgr = make_space_manager();
        let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8);
        let engine = ChainEngine::new();

        // 0xFFFF is past every cell-interface range (the highest is SGWPlayer's
        // 67–108). Guaranteed to land in the fall-through warn arm regardless
        // of future handler additions.
        let unhandled_method_index: u16 = 0xFFFF;
        let entity_id: u32 = 4242; // not present in space_mgr — span backfill skips, fall-through still runs
        let args: &[u8] = &[0x01, 0x02, 0x03];

        dispatch_cell_method(
            entity_id,
            unhandled_method_index,
            args,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let event = capture
            .find_message(Level::WARN, "Unhandled cell method call")
            .unwrap_or_else(|| {
                panic!(
                    "expected WARN for unhandled cell method; a revert to \
                     `info!` makes this test fail because the event is \
                     captured at a level below WARN. Captured: {:#?}",
                    capture.all()
                )
            });

        assert!(
            event.has_field("method_index", "65535"),
            "method_index must round-trip as the u16 we passed; got {event:#?}",
        );
        assert!(
            event.has_field("args_len", "3"),
            "args_len must be the byte count of the supplied args; got {event:#?}",
        );
        assert!(
            event.has_field("entity_id", "4242"),
            "entity_id must round-trip so ops can pivot per-player; got {event:#?}",
        );
    }
}
