//! Mission 704 relog restores and shared-bit repairs — chains 1296-1298
//! and 1300/1301 in `db/resources/Content/Seed/castle_702_704_chains.sql`
//! (packet CA07). Sibling of [`super::mission_704`], which pins the
//! mission spine; this module pins the two ways a player gets their
//! interaction cursors back.
//!
//! Two distinct repairs, and the distinction is the point:
//!
//! * **Relog restore** (`player_loaded Castle`, chains 1296-1298).
//!   Interaction flags and `follow_target_id` are runtime state on the
//!   entity and do not survive a reconnect or a server restart. Without
//!   these a player who logs out mid-mission finds the actors inert.
//! * **Region re-entry repair** (`enter_region`, chains 1300/1301).
//!   `set_interaction_type` is global on the entity, so another player
//!   finishing the same step clears the bit for everyone — including a
//!   player still on it. Without these the only cure is a relog.
//!
//! Each repair is pinned to exactly one step, and the cross-step tests
//! below are what stop a drifted gate from letting a repair re-run chain
//! 1291's advance or re-arm an escort that is over.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// The `!` main-story-active bit (`INT_AStoryMissionActive`).
const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;
/// The hackable-console bit (`INT_MinigameLivewire`).
const INT_MINIGAME_LIVEWIRE: i64 = 256;

async fn load(pool: &sqlx::PgPool, chain_id: i32) -> cimmeria_content_engine::chain::Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains —                  castle_702_704_chains.sql missing from db/database.sql?"
            )
        })
}

/// Register one seeded chain and resolve a synthetic event against it.
async fn resolve_chain(
    pool: &sqlx::PgPool,
    chain_id: i32,
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> Vec<Action> {
    let mut engine = ChainEngine::new();
    engine.register_chain(load(pool, chain_id).await);

    let mut ctx = ExecutionContext::new();
    for (k, v) in params {
        ctx.set_param((*k).to_string(), v.clone());
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine
        .resolve_event(&event, &ctx)
        .actions
        .into_iter()
        .filter(|(id, _)| *id == chain_id as i64)
        .map(|(_, a)| a)
        .collect()
}

fn assert_interaction(a: &Action, tag: &str, op: &str, mask: i64, what: &str) {
    match a {
        Action::SetInteractionType {
            entity_tag,
            operation,
            mask: m,
        } => {
            assert_eq!(entity_tag, tag, "{what}: wrong entity_tag");
            assert_eq!(operation, op, "{what}: wrong op");
            assert_eq!(*m, mask, "{what}: wrong mask");
        }
        other => panic!("{what}: expected set_interaction_type, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────────
// Chains 1296 / 1297 / 1298 — relog restores
// ──────────────────────────────────────────────────────────────────────

/// Fire `player_loaded Castle` at `chain_id` with exactly one of 704's
/// three steps active. The key is matched to a literal rather than
/// formatted so a typo'd step number fails loudly here instead of
/// silently producing a param no condition ever reads (which would make
/// every negative assertion pass for the wrong reason).
async fn restore_actions(pool: &sqlx::PgPool, chain_id: i32, active_step: i32) -> Vec<Action> {
    let key = match active_step {
        2405 => "mission_704_step_2405_status",
        2406 => "mission_704_step_2406_status",
        2407 => "mission_704_step_2407_status",
        other => panic!("{other} is not a step of mission 704"),
    };
    resolve_chain(
        pool,
        chain_id,
        TriggerType::PlayerLoaded,
        &[
            ("world_name", serde_json::json!("Castle")),
            (key, serde_json::json!("active")),
        ],
    )
    .await
}

#[tokio::test]
async fn chain_1296_restores_the_escort_follow_on_login() {
    let pool = require_db_or_skip!();
    let actions = restore_actions(&pool, 1296, 2405).await;
    assert_eq!(actions.len(), 1, "got {actions:?}");
    match &actions[0] {
        Action::SetFollowTarget {
            entity_tag,
            target_tag,
            use_player,
        } => {
            assert_eq!(entity_tag, "Castle_Zuritska_Cell");
            assert_eq!(*target_tag, None);
            assert_eq!(
                *use_player,
                Some(true),
                "the relog restore must re-bind the follow to the returning \
                 player's NEW entity id — a stale id from before the relog \
                 resolves to nothing and the follow silently drops to Idle",
            );
        }
        other => panic!("chain 1296 must re-arm the follow; got {other:?}"),
    }
}

/// Step 2406 has two interactables, so its restore must re-arm both. A
/// restore that only re-armed the terminal would leave a relogged player
/// unable to click Zuritska for the instruction dialog.
#[tokio::test]
async fn chain_1297_restores_both_2406_bits_on_login() {
    let pool = require_db_or_skip!();
    let actions = restore_actions(&pool, 1297, 2406).await;
    assert_eq!(actions.len(), 2, "got {actions:?}");
    assert_interaction(
        &actions[0],
        "Castle_CommsTerminal",
        "|",
        INT_MINIGAME_LIVEWIRE,
        "chain 1297 terminal restore",
    );
    assert_interaction(
        &actions[1],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1297 workstation restore",
    );
}

#[tokio::test]
async fn chain_1298_restores_the_workstation_indicator_on_login() {
    let pool = require_db_or_skip!();
    let actions = restore_actions(&pool, 1298, 2407).await;
    assert_eq!(actions.len(), 1, "got {actions:?}");
    assert_interaction(
        &actions[0],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1298 restore",
    );
}

/// Each restore is pinned to exactly one step. Logging in on 2407 must
/// not re-arm the escort or the terminal — the escort is over and the
/// terminal was cleared at victory.
#[tokio::test]
async fn the_restores_do_not_cross_steps() {
    let pool = require_db_or_skip!();
    for (chain_id, wrong_step) in [(1296, 2407), (1297, 2405), (1298, 2406)] {
        let actions = restore_actions(&pool, chain_id, wrong_step).await;
        assert!(
            actions.is_empty(),
            "chain {chain_id} must not restore while step {wrong_step} is the \
             active one; got {actions:?}",
        );
    }
}

/// The restores are keyed to the Castle world.
#[tokio::test]
async fn chain_1297_does_not_restore_in_another_world() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1297,
        TriggerType::PlayerLoaded,
        &[
            ("world_name", serde_json::json!("Castle_CellBlock")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1297's player_loaded key must be 'Castle'; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chains 1300 / 1301 — region re-entry repair for the shared bits
// ──────────────────────────────────────────────────────────────────────

/// Fire `enter_region Castle.CommsRoom` at `chain_id` with one of 704's
/// steps active.
async fn comms_reentry_actions(
    pool: &sqlx::PgPool,
    chain_id: i32,
    active_step: i32,
) -> Vec<Action> {
    let key = match active_step {
        2405 => "mission_704_step_2405_status",
        2406 => "mission_704_step_2406_status",
        2407 => "mission_704_step_2407_status",
        other => panic!("{other} is not a step of mission 704"),
    };
    resolve_chain(
        pool,
        chain_id,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.CommsRoom")),
            ("world_name", serde_json::json!("Castle")),
            (key, serde_json::json!("active")),
        ],
    )
    .await
}

/// Another player's Livewire victory or delivery clears these bits for
/// everyone. Chains 1297/1298 repair that only on a relog; 1300 and 1301
/// repair it on walking back into the room.
#[tokio::test]
async fn chain_1300_repairs_both_2406_bits_on_region_re_entry() {
    let pool = require_db_or_skip!();
    let actions = comms_reentry_actions(&pool, 1300, 2406).await;
    assert_eq!(actions.len(), 2, "got {actions:?}");
    assert_interaction(
        &actions[0],
        "Castle_CommsTerminal",
        "|",
        INT_MINIGAME_LIVEWIRE,
        "chain 1300 terminal repair",
    );
    assert_interaction(
        &actions[1],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1300 workstation repair",
    );
}

#[tokio::test]
async fn chain_1301_repairs_the_delivery_bit_on_region_re_entry() {
    let pool = require_db_or_skip!();
    let actions = comms_reentry_actions(&pool, 1301, 2407).await;
    assert_eq!(actions.len(), 1, "got {actions:?}");
    assert_interaction(
        &actions[0],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1301 repair",
    );
}

/// 1291, 1300 and 1301 all trigger on `Castle.CommsRoom` and are separated
/// only by their step gates. If any gate drifted, a re-entry would re-run
/// 1291's advance and clear the escort a second time.
#[tokio::test]
async fn the_comms_room_chains_never_claim_the_same_entry() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for chain_id in [1291, 1300, 1301] {
        engine.register_chain(load(&pool, chain_id).await);
    }

    for (active_step, expected) in [(2405, 1291_i64), (2406, 1300_i64), (2407, 1301_i64)] {
        let key = match active_step {
            2405 => "mission_704_step_2405_status",
            2406 => "mission_704_step_2406_status",
            _ => "mission_704_step_2407_status",
        };
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "region_key".to_string(),
            serde_json::json!("Castle.CommsRoom"),
        );
        ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
        ctx.set_param(key.to_string(), serde_json::json!("active"));
        let event = TriggerEvent {
            trigger_type: TriggerType::RegionEnter,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let mut claiming: Vec<i64> = engine
            .resolve_event(&event, &ctx)
            .actions
            .iter()
            .map(|(id, _)| *id)
            .collect();
        claiming.sort_unstable();
        claiming.dedup();
        assert_eq!(
            claiming,
            vec![expected],
            "on step {active_step} only chain {expected} may claim a Comms Room entry",
        );
    }
}
