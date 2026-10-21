//! Mission 689 boot-lock seeds (cellblock-boot-lock BL-06): the server hold
//! (chain 1023), the click-bit set on zone load (chain 1047), the Livewire
//! victory order (chain 1025) and the mission-completed release (chain 1048).
//!
//! Goa'uld (archetype 6) are held too: no chain here has an archetype
//! condition (D-BL8, owner 2026-10-20), so the `Some(6)` cases pin that.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::movement_lock::MovementLockOp;
use cimmeria_content_engine::triggers::{Trigger, TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// The Prison Boot Control's tag (spawn 284).
const BOOT_TAG: &str = "Cellblock_PrisonBootControl";

/// Fire `player_loaded('Castle_CellBlock')` with the given
/// `mission_689_status` (and `archetype`, when given) and return the actions
/// that `chain_id` resolved.
async fn fire_player_loaded(
    pool: &PgPool,
    chain_id: i64,
    status: &str,
    archetype: Option<i64>,
) -> Vec<Action> {
    let chain = load_single_chain_for_test(pool, chain_id as i32)
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
    ctx.set_param("mission_689_status".to_string(), serde_json::json!(status));
    if let Some(archetype) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    engine
        .resolve_event(&event, &ctx)
        .actions
        .into_iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, action)| action)
        .collect()
}

/// Assert that `actions` holds the player at the stasis-room anchor (chain
/// 1023's authored hold) and does not launch ability 1597.
fn assert_hold_only(actions: &[Action], who: &str) {
    assert!(
        !actions.iter().any(|a| matches!(
            a,
            Action::LaunchAbility {
                ability_id: 1597,
                ..
            }
        )),
        "{who}: chain 1023 must not launch ability 1597 (it sets BSF_MOVEMENT_LOCK); actions: {actions:?}"
    );
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::MovementLock(MovementLockOp::Hold {
                anchor_tag,
                at,
                max_distance,
                client_flag: false,
                message: Some(_),
            }) if anchor_tag == BOOT_TAG
                && *at == [-334.231, 73.472, -228.026]
                && *max_distance == 3.0
        )),
        "{who}: chain 1023 must hold at the stasis anchor; actions: {actions:?}"
    );
}

/// Chain 1023 holds a player whose mission 689 is still `active` (not yet
/// cleared) at the stasis anchor on zone load, and does not launch ability 1597.
#[tokio::test]
async fn live_db_chain_1023_holds_at_the_stasis_anchor() {
    let pool = require_db_or_skip!();
    let actions = fire_player_loaded(&pool, 1023, "active", Some(1)).await;
    assert_hold_only(&actions, "archetype 1");
}

/// Goa'uld (archetype 6) are held too: chain 1023 has no archetype condition
/// (D-BL8), so the same hold fires for them.
#[tokio::test]
async fn live_db_chain_1023_holds_goauld_too() {
    let pool = require_db_or_skip!();
    let actions = fire_player_loaded(&pool, 1023, "active", Some(6)).await;
    assert_hold_only(&actions, "archetype 6");
}

/// The masks of the `|` (set) click-bit operations on the Prison Boot
/// Control, in authored order.
fn click_bit_sets(actions: &[Action]) -> Vec<i64> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::SetInteractionType {
                entity_tag,
                operation,
                mask,
            } if entity_tag == BOOT_TAG && operation == "|" => Some(*mask),
            _ => None,
        })
        .collect()
}

/// Zone load while the lock is up sets both click bits, for the player and
/// for a Goa'uld alike.
#[tokio::test]
async fn live_db_chain_1047_sets_click_bits_while_locked() {
    let pool = require_db_or_skip!();
    for archetype in [None, Some(6)] {
        let actions = fire_player_loaded(&pool, 1047, "not_active", archetype).await;
        assert_eq!(
            click_bit_sets(&actions),
            vec![256, 1073741824],
            "chain 1047 must set both click bits (archetype {archetype:?}); actions: {actions:?}"
        );
    }
}

/// Once mission 689 is completed, chain 1047 fires nothing: the click bits
/// are not set on a cleared cell.
#[tokio::test]
async fn live_db_chain_1047_fires_nothing_once_completed() {
    let pool = require_db_or_skip!();
    let actions = fire_player_loaded(&pool, 1047, "completed", None).await;
    assert!(
        actions.is_empty(),
        "chain 1047 must not fire once 689 is completed; actions: {actions:?}"
    );
}

/// Livewire victory: release the hold, then clear the click bits, then
/// complete mission 689. The order matters (chain 1025's own comment).
#[tokio::test]
async fn live_db_chain_1025_releases_then_clears_then_completes() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1025)
        .await
        .expect("DB query for chain 1025 must succeed")
        .expect("chain 1025 must exist in seeded content_chains");

    match chain.actions.as_slice() {
        [Action::LaunchAbility {
            ability_id: 1598,
            entity_tag: None,
        }, Action::MovementLock(MovementLockOp::Release { message: Some(_) }), Action::SetInteractionType {
            entity_tag: clear_a,
            operation: op_a,
            mask: 256,
        }, Action::SetInteractionType {
            entity_tag: clear_b,
            operation: op_b,
            mask: 1073741824,
        }, Action::CompleteMission { mission_id: 689 }]
            if clear_a == BOOT_TAG && clear_b == BOOT_TAG && op_a == "~" && op_b == "~" => {}
        other => panic!("chain 1025 actions are out of order or wrong: {other:?}"),
    }
}

/// Mission 689 completed (chain 1025's own completion, or a GM
/// `complete_mission 689`) releases any hold, silently, then clears the click
/// bits the same way chain 1025 does.
#[tokio::test]
async fn live_db_chain_1048_releases_then_clears_on_mission_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1048)
        .await
        .expect("DB query for chain 1048 must succeed")
        .expect("chain 1048 must exist in seeded content_chains");

    assert!(
        matches!(
            chain.trigger,
            Trigger::OnMissionCompleted { mission_id: 689 }
        ),
        "chain 1048 must trigger on mission 689 completion; trigger: {:?}",
        chain.trigger,
    );
    match chain.actions.as_slice() {
        [Action::MovementLock(MovementLockOp::Release { message: None }), Action::SetInteractionType {
            entity_tag: clear_a,
            operation: op_a,
            mask: 256,
        }, Action::SetInteractionType {
            entity_tag: clear_b,
            operation: op_b,
            mask: 1073741824,
        }] if clear_a == BOOT_TAG && clear_b == BOOT_TAG && op_a == "~" && op_b == "~" => {}
        other => panic!(
            "chain 1048 must be a silent release, then clear both click bits; actions: {other:?}"
        ),
    }
}
