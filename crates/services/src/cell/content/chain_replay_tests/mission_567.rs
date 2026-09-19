//! Mission 567 — Romney's Files, **step 4039 only** (packet H30,
//! `db/resources/Content/Seed/harset_opcore_chains.sql` chains 6503-6504).
//!
//! Both chains ship **disabled** (decision U17): step 4039 needs item 2698
//! "Romney's Files", and that item has no grant path anywhere in the seed
//! — verified across `content_actions`, `loot`, `mission_rewards`,
//! `items_event_sets`, `char_creation_choices`, `item_list_items`,
//! `blueprints` and `blueprints_components`. The Castle ledger's CA06
//! explicitly excludes it, and the Castle-side legs (steps 2000, 2012) are
//! unauthored too.
//!
//! So the value of this file is inverted relative to its siblings. It does
//! **not** guard a behaviour the player can reach; it guards the *parked*
//! state:
//!
//! 1. The rows are present and **load cleanly** — if the loader ever
//!    rejected a trigger, condition or action here it would do so silently
//!    (`warn!` + drop), and the defect would only surface on the day
//!    someone flips `enabled`. Loading is asserted separately from
//!    resolving so "row missing" and "row present but skipped" have
//!    different failure messages (TESTING.md type 6).
//! 2. The chains **resolve nothing while disabled**, even under the exact
//!    context that would otherwise satisfy every condition.
//! 3. The action list is **already correct**, so flipping `enabled` is a
//!    one-word change with no re-authoring.
//!
//! When a Castle packet grants 2698, flip `enabled` on both rows and
//! `chains_6503_and_6504_are_disabled_pending_the_2698_grant` below is the
//! test that will tell you to update this file.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` for `Harset_CmdCenter`, where Copplemann
/// (template 48, tag `CmdCenter_Copplemann`) stands.
const CMD_CENTER: i32 = 68;

/// The context that WOULD satisfy chain 6503 if it were enabled: standing
/// in the Command Center, 567 active, step 4039 current.
fn copplemann_ctx() -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CmdCenter_Copplemann"),
    );
    ctx.set_param("mission_567_status".to_string(), serde_json::json!("active"));
    ctx.set_param(
        "mission_567_step_4039_status".to_string(),
        serde_json::json!("active"),
    );
    ctx
}

/// Both chains are present, disabled, and their `enabled` flag is the only
/// thing standing between the seed and a live turn-in.
///
/// Pinning `enabled == false` (rather than just asserting no resolution)
/// is what makes the U17 handoff explicit: the day someone flips it, this
/// assertion fails and points at the item-grant precondition instead of
/// letting a half-wired mission reach a player.
#[tokio::test]
async fn chains_6503_and_6504_are_disabled_pending_the_2698_grant() {
    let pool = require_db_or_skip!();

    for chain_id in [6503, 6504] {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| {
                panic!(
                    "chain {chain_id} must EXIST in seeded content_chains and load cleanly — \
                     it is parked, not deleted (U17)"
                )
            });

        assert!(
            !chain.enabled,
            "chain {chain_id} must stay disabled until a Castle packet grants item 2698 \
             (Romney's Files). If you just enabled it deliberately, confirm the grant \
             exists and then update this test and the seed header together."
        );
    }
}

/// Chain 6503 resolves **nothing** while disabled, under the very context
/// that satisfies all three of its conditions.
///
/// `resolve_event` filters on `chain.enabled` before it even evaluates the
/// trigger, so this is the guard that proves a parked chain is inert
/// rather than merely un-reached.
#[tokio::test]
async fn chain_6503_resolves_nothing_while_disabled() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6503)
        .await
        .expect("DB query for chain 6503 must succeed")
        .expect("chain 6503 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let ctx = copplemann_ctx();
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);

    assert!(
        resolved.actions.is_empty(),
        "chain 6503 is disabled, so a right-click on Copplemann must resolve nothing \
         even with 567 active on step 4039; got {:?}",
        resolved.actions
    );
}

/// Chain 6504 (the relog restore) is inert too, so no "?" ever appears on
/// Copplemann for a step the player cannot finish. A stale turn-in
/// indicator on a shared-hub NPC would be visible to that player on every
/// Command Center visit with nothing behind the click.
#[tokio::test]
async fn chain_6504_resolves_nothing_while_disabled() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6504)
        .await
        .expect("DB query for chain 6504 must succeed")
        .expect("chain 6504 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param(
        "mission_567_step_4039_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    assert!(
        engine.resolve_event(&event, &ctx).actions.is_empty(),
        "chain 6504 is disabled, so Command Center entry must not bind Copplemann's \
         turn-in indicator"
    );
}

/// The parked chain is nonetheless **correctly authored**: the four
/// actions are exactly what the turn-in needs, in order.
///
/// This is the assertion that makes enabling a one-word change. Without
/// it, 6503 could rot — a dialog id drifting, the `remove_item` going
/// missing — and nobody would find out until a player reached a step that
/// has been unreachable for months. Reading the action list off the loaded
/// chain (rather than resolving) is what lets a disabled chain still be
/// checked.
#[tokio::test]
async fn chain_6503_is_authored_ready_for_the_day_2698_is_granted() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6503)
        .await
        .expect("DB query for chain 6503 must succeed")
        .expect("chain 6503 must exist in seeded content_chains");

    assert_eq!(
        chain.actions.len(),
        4,
        "chain 6503 must carry exactly four actions; got {:?}",
        chain.actions
    );
    assert!(
        matches!(chain.actions[0], Action::DisplayDialog { dialog_id: 2044 }),
        "action 0 must play Copplemann's blurb 2044 (\"These files are going to help us \
         out a lot. Nice work.\", dsm 2817, speaker 968); got {:?}",
        chain.actions[0]
    );
    assert!(
        matches!(
            chain.actions[1],
            Action::RemoveItem {
                item_id: 2698,
                count: 1
            }
        ),
        "action 1 must consume exactly one Romney's Files (2698); got {:?}",
        chain.actions[1]
    );
    assert!(
        matches!(
            chain.actions[2],
            Action::RemoveDialogSet {
                dialog_set_id: 2817,
                slot: 48
            }
        ),
        "action 2 must clear the dsm 2817 bind from template slot 48 (Copplemann); got {:?}",
        chain.actions[2]
    );
    assert!(
        matches!(chain.actions[3], Action::CompleteMission { mission_id: 567 }),
        "action 3 must COMPLETE 567 — step 4039 is its terminal step, so this also \
         closes objective 4652; got {:?}",
        chain.actions[3]
    );
}

/// The restore chain's single action is the matching bind. Same
/// rot-protection rationale as the test above.
#[tokio::test]
async fn chain_6504_is_authored_ready_too() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6504)
        .await
        .expect("DB query for chain 6504 must succeed")
        .expect("chain 6504 must exist in seeded content_chains");

    assert_eq!(chain.actions.len(), 1, "chain 6504 binds one dialog set");
    assert!(
        matches!(
            chain.actions[0],
            Action::AddDialogSet {
                dialog_set_id: 2817,
                slot: 48,
                mission_id: Some(567),
            }
        ),
        "chain 6504 must re-bind dsm 2817 to template slot 48 (Copplemann) for mission \
         567; got {:?}",
        chain.actions[0]
    );
}
