//! Action execution — dispatches resolved content engine actions against the
//! game state (missions, items, dialogs, interactions, etc.).
//!
//! Each match arm forwards to a per-family handler in a sibling module:
//!
//! - [`ability_granter`] — `GmAbilityBulk`, the Debug Area ability granter
//!   and reset NPCs (GM-gated `gmGiveAllAbilities` / `gmResetAbilities`)
//! - [`bark`]      — `NpcBark`, the non-modal companion line (client method 28)
//! - [`black_market`] — `OpenBlackMarket`, open the client Black Market window
//!   (`onBMOpen`, client method 90)
//! - [`mission`]   — accept/advance/complete/abandon, advance step, complete objective
//! - [`inventory`] — grant/remove items, bandolier seeding
//! - [`dialog`]    — display, add/remove dialog set, add dialog
//! - [`stats`]     — `Action::ChangeStat`
//! - [`spawn`]     — `SpawnEntity` / `DespawnEntity` / `DestroyTaggedEntity`
//!   (the last two share `despawn_by_tag`)
//! - [`world`]     — interaction-type/visibility/move/threat/aggression
//! - [`counter`]   — increment/reset
//! - [`transport`] — teleport, ring transporter
//! - [`stargate`]  — `GrantStargateAddress` (cell, client method 66, the base)
//! - [`mail`]      — `SendSystemMail`, forwarded to the base's mail writer
//! - [`loot`]      — `OpenLoot`, a loot window on a live container
//! - [`deferred`]  — `content_actions.delay_ms > 0` scheduling/tick-drain (C08a)
//! - [`once_gate`] — `content_triggers.once`: fire once per entity, then disarm
//!
//! - [`dispatch`]  — [`execute_one`], the per-action `match`; single-arm
//!   actions with no shared helpers (PlaySequence, StartMinigame,
//!   SystemMessage, SendMessage, SetActiveSlot, TriggerChain, fallback) stay
//!   inline there; `LaunchAbility`/`ApplyEffect` forward to
//!   [`super::effect_apply`].

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ResolvedActions;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

mod ability_granter;
mod bark;
mod black_market;
mod counter;
mod deferred;
mod dialog;
mod dispatch;
mod inventory;
mod loot;
mod mail;
mod mission;
mod once_gate;
mod spawn;
mod stargate;
mod stats;
mod transport;
mod world;

use dispatch::execute_one;

#[cfg(test)]
mod tests;

// Re-export `item_container` so the parent module's test suite (which
// imports `super::executor::item_container`) keeps working without
// touching the call site. Only the parent's `#[cfg(test)]` block reads
// it through this path, so gate the re-export on `cfg(test)` to keep
// the unused-imports lint happy on release builds.
#[cfg(test)]
pub(super) use inventory::item_container;

// `deferred_content_action_tick` is the cell-tick-facing entry point for
// C08a's delayed-action drain; re-exported so `content/mod.rs` can expose
// it to `cell::service::message_loop` at the same flat
// `crate::cell::content::<fn>` depth as `build_engine` and the `fire_*`
// dispatchers.
pub use deferred::deferred_content_action_tick;

/// Execute resolved actions from the content engine against the game state.
///
/// `level = "info"` because chain firings are low-rate and high-signal
/// — every "player did X, missions did Y" sequence shows up as one
/// span containing the action vector. The `actions_len` field gives a
/// quick "how complex is this chain?" view in SigNoz.
///
/// Actions with `delay_ms == 0` (the overwhelming majority) run inline,
/// in order, exactly as before C08a. Actions with `delay_ms > 0` are
/// queued via `SpaceManager::schedule_content_action` instead of run —
/// `deferred_content_action_tick` fires them later from the cell tick.
/// See `deferred` module docs for why a per-entity queue was chosen over
/// a spawned timer task.
#[tracing::instrument(
    name = "content.execute_actions",
    level = "info",
    skip_all,
    fields(entity_id, player_id, actions_len = resolved.actions.len()),
)]
// `pub` for the `test-support` re-export in `content/mod.rs`; without the
// feature nothing outside this crate names it.
#[cfg_attr(not(any(test, feature = "test-support")), allow(unreachable_pub))]
pub async fn execute_actions(
    resolved: ResolvedActions,
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    // Destructure so the inner loop can move `actions` while later
    // action branches still read trigger-time `params`. `Action::RemoveItem`
    // looks up `instance_id` here to consume the exact stack the
    // player clicked on `useItem`. `action_delays` is index-aligned with
    // `actions` (a parallel vec, not a wider tuple — see the field doc on
    // `ResolvedActions`); a missing index means delay_ms == 0.
    let ResolvedActions {
        actions,
        action_delays,
        params,
    } = resolved;
    // Fire-once chains (#802): drop a spent one, record a first fire.
    let (actions, action_delays) =
        once_gate::gate_for_entity(actions, action_delays, entity_id, space_mgr, engine);
    if !actions.is_empty() {
        // One ordered line per resolved action list: item grants, step
        // advances, dialogs and their delays as the executor will run them.
        let order: Vec<String> = actions
            .iter()
            .enumerate()
            .map(|(i, (chain_id, action))| {
                let d = action_delays.get(i).copied().unwrap_or(0);
                let k = crate::cell::player_journal::action_kind(action);
                if d > 0 {
                    format!("{chain_id}:{k}+{d}ms")
                } else {
                    format!("{chain_id}:{k}")
                }
            })
            .collect();
        crate::cell::player_journal::note(
            entity_id,
            crate::cell::player_journal::kinds::ACTION_LIST,
            order.join(" > "),
        );
    }
    for (i, (chain_id, action)) in actions.into_iter().enumerate() {
        let delay_ms = action_delays.get(i).copied().unwrap_or(0);
        if delay_ms > 0 {
            tracing::debug!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                delay_ms,
                action = ?action,
                "Content: deferring action"
            );
            space_mgr.schedule_content_action(
                entity_id,
                chain_id,
                action,
                player_id,
                delay_ms,
                params.clone(),
            );
            continue;
        }
        execute_one(
            chain_id, action, entity_id, player_id, &params, tx, space_mgr, engine,
        )
        .await;
    }
}
