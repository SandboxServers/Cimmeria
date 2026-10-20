//! Mission 689 — Prison Boot Lock (internal, hidden; C00).
//!
//! New mechanic surfaced by the v3 spec audit (no recovered Python script
//! references it at all — see `docs/analysis/castle-cellblock-rebuild/
//! work-packets.md#c00` and `audit.md`'s "New Evidence" section). Mission
//! 689 is a purely internal, `is_hidden = true` tracking mission (same
//! precedent already shipped for the Hallway0N Controllers, 682-686) whose
//! `mission_status` is what chains 1022-1025 key off:
//!
//! - Chain 1022 (`player_loaded`, `mission_status 689 eq not_active`):
//!   accepts mission 689 once per character.
//! - Chain 1023 (`player_loaded`, `mission_status 689 neq completed`):
//!   (re-)applies the movement hold at the stasis-room anchor (cellblock-
//!   boot-lock D-BL3). This is what
//!   makes "relog mid-lock re-holds" and "relog after clearing does NOT
//!   re-hold" both fall out of one condition.
//! - Chain 1024 (`interact_tag` on `Cellblock_PrisonBootControl`,
//!   `mission_status 689 neq completed`): starts a Livewire session,
//!   `on_victory_chains: [1025]`. The old `item_use 3438` trigger is gone.
//! - Chain 1025 (no trigger — invoked directly by 1024's minigame
//!   callback, same shape as chain 1017/638): releases the hold, clears the
//!   click bits, launches ability 1598 and completes mission 689.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use super::livewire_pairs::resolve_interact;
use crate::test_support::require_db_or_skip;

/// Fire `player_loaded('Castle_CellBlock')` against `chain_id` with the
/// given `mission_689_status` and assert whether it resolves ANY action for
/// that chain — `should_fire` is the expected outcome. Asserting inside the
/// helper (rather than returning the resolved actions) keeps
/// `require_db_or_skip!`'s bare `return;` valid, since every caller is a
/// `#[tokio::test]` fn returning `()`.
async fn assert_player_loaded_resolves(chain_id: i64, mission_689_status: &str, should_fire: bool) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_689_status".to_string(),
        serde_json::json!(mission_689_status),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let fired: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a)
        .collect();

    assert_eq!(
        !fired.is_empty(),
        should_fire,
        "chain {chain_id} with mission_689_status={mission_689_status:?} \
         expected to fire={should_fire}; got actions {fired:?}"
    );
}

/// Positive: mission 689 has never been accepted (`not_active`, the
/// missing-key default per content-engine.md) → chain 1022 accepts it.
#[tokio::test]
async fn live_db_chain_1022_accepts_mission_689_when_not_active() {
    assert_player_loaded_resolves(1022, "not_active", true).await;
}

/// Negative: once mission 689 is active (already accepted on a prior
/// load), chain 1022 must NOT re-fire — `accept_mission`'s own offer
/// guard would refuse it anyway (warn-logged), but the chain gate should
/// stop it from even trying on every relog.
#[tokio::test]
async fn live_db_chain_1022_does_not_refire_once_active() {
    assert_player_loaded_resolves(1022, "active", false).await;
}

/// Negative: once completed (cleared), chain 1022 must NOT re-fire either
/// — `num_repeats = 0` on the mission row already refuses a re-accept
/// server-side, but the chain gate is the first line of defense.
#[tokio::test]
async fn live_db_chain_1022_does_not_refire_once_completed() {
    assert_player_loaded_resolves(1022, "completed", false).await;
}

/// Positive (relog mid-lock, first load): mission 689 `not_active` still
/// re-applies the lock — this is the very first zone load, before chain
/// 1022's accept has committed against this same event's pre-action
/// snapshot (mission_status defaults missing keys to `not_active`, which
/// is `neq completed`).
#[tokio::test]
async fn live_db_chain_1023_holds_when_not_active() {
    assert_player_loaded_resolves(1023, "not_active", true).await;
}

/// Positive (relog mid-lock, subsequent load): mission 689 `active` (the
/// player has been locked at least once but hasn't cleared it) still
/// re-applies the lock on every relog. This is the acceptance criterion
/// "relog mid-lock must re-apply the lock."
#[tokio::test]
async fn live_db_chain_1023_holds_when_active() {
    assert_player_loaded_resolves(1023, "active", true).await;
}

/// Negative: once mission 689 is completed (Livewire cleared, chain 1025
/// ran), chain 1023 must NOT re-apply the hold. This is the acceptance
/// criterion "relog after clearing must NOT re-apply it."
#[tokio::test]
async fn live_db_chain_1023_does_not_relaunch_once_completed() {
    assert_player_loaded_resolves(1023, "completed", false).await;
}

/// Right-click on the Prison Boot Control (`interact_tag`) while the lock is
/// up starts the Livewire session, with chain 1025 as the victory chain.
#[tokio::test]
async fn live_db_chain_1024_starts_livewire_on_right_click_while_locked() {
    let pool = require_db_or_skip!();
    let actions = resolve_interact(
        &pool,
        1024,
        "Cellblock_PrisonBootControl",
        &[("mission_689_status", "active")],
    )
    .await;
    assert!(
        actions.iter().any(|(id, a)| *id == 1024
            && matches!(
                a,
                Action::StartMinigame {
                    minigame_type,
                    on_victory_chains,
                    ..
                } if minigame_type == "Livewire" && on_victory_chains == &vec![1025]
            )),
        "chain 1024 must start Livewire with victory chain 1025; actions: {actions:?}",
    );
}

/// Once mission 689 is completed, the right-click resolves nothing.
#[tokio::test]
async fn live_db_chain_1024_does_not_start_once_completed() {
    let pool = require_db_or_skip!();
    let actions = resolve_interact(
        &pool,
        1024,
        "Cellblock_PrisonBootControl",
        &[("mission_689_status", "completed")],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1024 must not start a minigame once 689 is completed; actions: {actions:?}",
    );
}

/// Chain 1025 has no trigger row (invoked directly by chain 1024's
/// `on_victory_chains`, same shape as chain 1017 for mission 638's Livewire
/// victory) — load it directly and pin its two actions: launch ability
/// 1598 (self), then complete mission 689 so chain 1023 stops re-locking.
#[tokio::test]
async fn live_db_chain_1025_launches_1598_and_completes_689() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1025)
        .await
        .expect("DB query for chain 1025 must succeed")
        .expect("chain 1025 must exist in seeded content_chains");

    assert!(
        chain.actions.iter().any(|a| matches!(
            a,
            Action::LaunchAbility {
                ability_id: 1598,
                entity_tag: None
            }
        )),
        "chain 1025 must launch ability 1598 (self) on Livewire victory; \
         actions: {:?}",
        chain.actions,
    );
    assert!(
        chain
            .actions
            .iter()
            .any(|a| matches!(a, Action::CompleteMission { mission_id: 689 })),
        "chain 1025 must complete mission 689 on Livewire victory — this is \
         what stops chain 1023 from re-locking on the next relog. \
         Actions: {:?}",
        chain.actions,
    );
}
