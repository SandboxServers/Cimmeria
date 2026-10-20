//! Mission 1360 — Frost's Letter, **the Harset half** (packet H30,
//! `db/resources/Content/Seed/harset_opcore_chains.sql` chains 6501-6502).
//!
//! Sibling to [`super::mission_1360`], which the Castle Cellblock campaign
//! owns: that file pins chain 1121 (accept 1360 on the Frost loot) and
//! this one pins the delivery at the far end of the zone hop. They are
//! separate files on purpose — two campaigns, two seed files, and a
//! failure in either should name its own owner.
//!
//! - Chain 6501 (`interact_tag 'CmdCenter_Marsh'`; `world eq 68`,
//!   `mission_status 1360 eq active`, `step_status 1360/4038 eq active`,
//!   `step_status 1361/4694 neq active`): one `complete_mission_with_item`
//!   receipt action (H30b). The letter, the "!" and 1360 are settled only
//!   by the committed base result.
//! - Chain 6502 (`player_loaded 'Harset_CmdCenter'`; `world eq 68`,
//!   `step_status 1360/4038 eq active`): re-binds DSM 120101 so the "!"
//!   survives a relog *and* the 57 -> 68 door crossing.
//!
//! Three guard shapes live here:
//!
//! 1. **Exact action shape on the happy path**: one strict receipt action
//!    with the six fixed fields, so a regression to the old four-action
//!    body fails the test.
//! 2. **No direct consume**: the executor queues the request and emits no
//!    removal, dialog or completion itself (TESTING.md type 6).
//! 3. **Adjacent negatives**: wrong world, wrong step, already completed,
//!    and the 1361-turn-in overlap that chain 6501's fourth condition
//!    exists to prevent.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::loader::{
    build_chains_from_rows, DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow,
};
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};
use cimmeria_wire::cell::messages::{
    LetterTurnInEntryEpoch, LetterTurnInSession, LetterTurnInToken,
};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;
use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` for `Harset_CmdCenter`, where Col. Marsh
/// stands. Chain 6501's `world` condition reads the typed
/// `ExecutionContext.world_id`, not a param.
const CMD_CENTER: i32 = 68;
/// `Harset` — the other side of the Command Center door. Used as the
/// wrong-world negative.
const HARSET: i32 = 57;

/// Player entity id used by the executor-level guard.
const PLAYER_EID: u32 = 7301;
/// Live Marsh entity id (tag `CmdCenter_Marsh`, template 10).
const MARSH_EID: u32 = 7302;

/// Build a context for a right-click on Marsh with the mission state the
/// caller wants to model.
///
/// `step_1361_4694` models the Praxis turn-in step, which chain 6501's
/// DISJOINTNESS condition keys on — see the seed file's note (A).
fn marsh_ctx(
    world_id: i32,
    mission_1360: &str,
    step_4038: &str,
    step_1361_4694: &str,
) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(world_id);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CmdCenter_Marsh"),
    );
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!(mission_1360),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!(step_4038),
    );
    ctx.set_param(
        "mission_1361_step_4694_status".to_string(),
        serde_json::json!(step_1361_4694),
    );
    ctx
}

/// Resolve one event against a single chain loaded from the DB.
///
/// Takes the pool rather than calling `require_db_or_skip!` itself: that
/// macro expands to a bare `return`, so it only composes in a function
/// returning `()`. Every test below opens with its own skip guard.
async fn resolve_chain(
    pool: &sqlx::PgPool,
    chain_id: i32,
    ctx: &ExecutionContext,
    tt: TriggerType,
) -> ResolvedActions {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!("chain {chain_id} must exist in seeded content_chains and load cleanly")
        });

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let event = TriggerEvent {
        trigger_type: tt,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Positive: the guaranteed first-visit state — the player walks into the
/// Command Center carrying Frost's letter with 1360 active on step 4038.
///
/// Pins the **single strict receipt action**: `complete_mission_with_item`
/// with the six fixed fields that `action_letter_turn_in.rs` accepts. The
/// old four-action body (display, remove, clear, complete) is no longer a
/// valid shape, so a regression to it fails here.
#[tokio::test]
async fn live_db_chain_6501_is_one_strict_receipt_action() {
    let ctx = marsh_ctx(CMD_CENTER, "active", "active", "not_active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;

    let actions: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();
    assert_eq!(
        actions.len(),
        1,
        "chain 6501 must resolve exactly one receipt action; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteMissionWithItem {
                mission_id: 1360,
                expected_step_id: 4038,
                item_id: 3730,
                quantity: 1,
                dialog_id: 4576,
                dialog_set_id: 120101,
                slot: 10,
            }
        ),
        "action 0 must be the strict letter receipt (1360 at 4038, item 3730, \
         blurb 4576, DSM 120101 slot 10); got {:?}",
        actions[0]
    );
}

/// Executor-level guard: the receipt action reaches the A3b turn-in path
/// (one request, processing feedback) and nothing else.
///
/// A resolve-only test cannot distinguish a wired arm from the executor's
/// `other =>` catch-all (TESTING.md type 6), so the emitted messages are
/// pinned here. The letter is consumed only by the base result, never by
/// the executor itself.
#[tokio::test]
async fn live_db_chain_6501_receipt_action_reaches_the_letter_turn_in_executor() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6501)
        .await
        .expect("DB query for chain 6501 must succeed")
        .expect("chain 6501 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = marsh_ctx(CMD_CENTER, "active", "active", "not_active");
    // `fire_interact_tag` stamps the clicked entity; the receipt binds to it.
    ctx.set_param("target_entity_id".to_string(), serde_json::json!(MARSH_EID));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);

    let mut mgr = make_cmd_center_space_mgr();
    stage_player(&mut mgr);
    let (tx, mut rx) = mpsc::channel(32);

    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &engine).await;

    let messages: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let requests = messages
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::Plugin(_)))
        .count();
    assert_eq!(
        requests, 1,
        "the receipt must queue exactly one turn-in request (A3b) before any result"
    );
    // Only the request and its processing feedback may leave the executor:
    // no item removal, dialog, completion or success frame before a result.
    let other = messages
        .iter()
        .filter(|m| {
            !matches!(
                m,
                CellToBaseMsg::Plugin(_) | CellToBaseMsg::EntityMethodCall { .. }
            )
        })
        .count();
    assert_eq!(
        other, 0,
        "the receipt action must not emit removals, dialogs or completion directly"
    );
}

/// Negative (A3a): a malformed receipt row refuses the WHOLE chain.
///
/// Dropping just the bad action would leave an ungated chain (or one with
/// stray rows) behind; the loader must keep nothing. The control builds the
/// untouched seed rows and must load, so an empty fixture cannot pass.
#[tokio::test]
async fn live_db_chain_6501_malformed_receipt_action_drops_the_whole_chain() {
    let pool = require_db_or_skip!();

    async fn build(pool: &sqlx::PgPool, mutate: impl Fn(&mut serde_json::Value)) -> usize {
        let chain: (i32, Option<String>, String, Option<i32>, bool, i32) = sqlx::query_as(
            "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains WHERE chain_id = 6501",
        )
        .fetch_one(pool)
        .await
        .expect("chain 6501 row");
        let triggers: Vec<(i32, String, Option<String>, String, bool, i32)> = sqlx::query_as(
            "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers WHERE chain_id = 6501 ORDER BY sort_order",
        )
        .fetch_all(pool)
        .await
        .expect("chain 6501 triggers");
        let conditions: Vec<(
            i32,
            String,
            Option<i32>,
            Option<String>,
            String,
            Option<String>,
            i32,
        )> = sqlx::query_as(
            "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions WHERE chain_id = 6501 ORDER BY sort_order",
        )
        .fetch_all(pool)
        .await
        .expect("chain 6501 conditions");
        let actions: Vec<(
            i32,
            String,
            Option<i32>,
            Option<String>,
            serde_json::Value,
            i32,
            i32,
        )> = sqlx::query_as(
            "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions WHERE chain_id = 6501 ORDER BY sort_order",
        )
        .fetch_all(pool)
        .await
        .expect("chain 6501 actions");

        build_chains_from_rows(
            vec![DbChainRow {
                chain_id: chain.0,
                description: chain.1,
                scope_type: chain.2,
                scope_id: chain.3,
                enabled: chain.4,
                priority: chain.5,
            }],
            triggers
                .into_iter()
                .map(|t| DbTriggerRow {
                    chain_id: t.0,
                    event_type: t.1,
                    event_key: t.2,
                    scope: t.3,
                    once: t.4,
                    sort_order: t.5,
                })
                .collect(),
            conditions
                .into_iter()
                .map(|c| DbConditionRow {
                    chain_id: c.0,
                    condition_type: c.1,
                    target_id: c.2,
                    target_key: c.3,
                    operator: c.4,
                    value: c.5,
                    sort_order: c.6,
                })
                .collect(),
            actions
                .into_iter()
                .map(|a| {
                    let mut params = a.4;
                    mutate(&mut params);
                    DbActionRow {
                        chain_id: a.0,
                        action_type: a.1,
                        target_id: a.2,
                        target_key: a.3,
                        params,
                        delay_ms: a.5,
                        sort_order: a.6,
                    }
                })
                .collect(),
        )
        .len()
    }

    assert_eq!(
        build(&pool, |_| {}).await,
        1,
        "control: untouched rows load"
    );
    assert_eq!(
        build(&pool, |p| p["quantity"] = serde_json::json!(2)).await,
        0,
        "quantity 2 must refuse the whole chain, not leave an ungated one"
    );
    assert_eq!(
        build(&pool, |p| {
            p.as_object_mut().unwrap().remove("slot");
        })
        .await,
        0,
        "a missing key must refuse the whole chain"
    );
}

/// Negative (re-fire guard): once 1360 is completed, a second right-click
/// on Marsh resolves nothing — the letter cannot be removed twice.
///
/// `MissionInstance::complete()` moves `current_step_id` into
/// `completed_steps`, so both the mission gate and the step gate flip in
/// the same transition; this models the post-completion context exactly.
#[tokio::test]
async fn live_db_chain_6501_does_not_refire_once_1360_is_completed() {
    let ctx = marsh_ctx(CMD_CENTER, "completed", "completed", "not_active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6501 must resolve nothing after 1360 completes, or a second Marsh \
         click re-runs `remove_item 3730`; got {:?}",
        resolved.actions
    );
}

/// Negative (wrong step): 1360 is active but the player is still on step
/// 4037 ("find a way to get the letter to his family") — they have not
/// reached the delivery step, so Marsh must not take the letter.
#[tokio::test]
async fn live_db_chain_6501_does_not_fire_on_the_earlier_step_4037() {
    let ctx = marsh_ctx(CMD_CENTER, "active", "not_active", "not_active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6501 must not fire while step 4038 is not the current step; got {:?}",
        resolved.actions
    );
}

/// Negative (wrong world): the same tag in world 57. `OnInteractTag` does
/// not filter by world and `fire_interact_tag` never populated a world
/// param before H07, so the `world eq 68` condition is the only thing
/// standing between this chain and a Harset-side entity that happened to
/// carry Marsh's tag.
#[tokio::test]
async fn live_db_chain_6501_does_not_fire_from_harset() {
    let ctx = marsh_ctx(HARSET, "active", "active", "not_active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6501 must not fire for a player standing in Harset (57); got {:?}",
        resolved.actions
    );
}

/// Negative (fail-closed): a dispatch site that never populated
/// `world_id` must not fire the chain either. `Condition::World` fails
/// closed on an unset id — unlike the mission conditions, which fall back
/// to `not_active` and can fail *open*.
#[tokio::test]
async fn live_db_chain_6501_fails_closed_without_a_world_context() {
    let mut ctx = marsh_ctx(CMD_CENTER, "active", "active", "not_active");
    ctx.world_id = None;
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6501 must fail closed when the dispatch site left world_id unset; got {:?}",
        resolved.actions
    );
}

/// Negative (the DISJOINTNESS condition, seed note (A)): a player who is
/// simultaneously on 1361's turn-in step 4694 and still carrying the
/// letter must not run chain 6501 on the same right-click that runs chain
/// 6527.
///
/// This is the bug shape the fourth condition exists for. Both chains key
/// on `interact_tag 'CmdCenter_Marsh'`, and `resolve_event` APPENDS every
/// matching chain's actions with no first-match break — so without the
/// gate one click would resolve both `display_dialog 4576` and
/// `display_dialog 4465`, and the client — which holds one non-tutorial
/// dialog at a time — would discard the first before the player could
/// read it, while its `remove_item` and `complete_mission` still ran.
/// Silent content loss, not a crash. (DU-08 made the server accept the
/// discarded dialog's late close, so its own chain is no longer lost;
/// the unread blurb still is.)
#[tokio::test]
async fn live_db_chain_6501_yields_to_the_praxis_turn_in() {
    let ctx = marsh_ctx(CMD_CENTER, "active", "active", "active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6501 must stand down while 1361 step 4694 is active so it cannot \
         co-fire with chain 6527 on one click; got {:?}",
        resolved.actions
    );
}

/// Restore chain 6502 re-binds Marsh's "!" on every entry to the Command
/// Center while the delivery step is open.
///
/// This chain is not just relog safety: `available_interactions` live on
/// the cell entity, and `cross_world_teleport` destroys it, so a player
/// walking from Harset into the Command Center arrives with an empty
/// binding table. Without 6502 the very first visit — the normal way to
/// reach Marsh — would find him inert.
#[tokio::test]
async fn live_db_chain_6502_rebinds_dsm_120101_on_world_entry() {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("active"),
    );

    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6502, &ctx, TriggerType::PlayerLoaded).await;
    let actions: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();
    assert_eq!(
        actions.len(),
        1,
        "chain 6502 must resolve exactly one bind; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::AddDialogSet {
                dialog_set_id: 120101,
                slot: 10,
                mission_id: Some(1360),
            }
        ),
        "chain 6502 must re-bind dsm 120101 to template slot 10 (Col Marsh) for mission \
         1360; got {:?}",
        actions[0]
    );
    assert!(
        !matches!(
            actions[0],
            Action::AddDialogSet {
                dialog_set_id: 5356,
                ..
            }
        ),
        "chain 6502 must not bind the shared stock row 5356"
    );
}

/// Relog/overlap: completing 1361 while 1360 is still at step 4038 hands the
/// letter indicator back through DSM 120101, and only in that state.
#[tokio::test]
async fn live_db_chain_6505_rebinds_dsm_120101_after_1361_completes() {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    // 6505's event_key is '1361': the trigger matches the completing mission id.
    ctx.set_param("mission_id".to_string(), serde_json::json!(1361));
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("active"),
    );

    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6505, &ctx, TriggerType::MissionCompleted).await;
    let actions: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();
    assert!(
        matches!(
            actions.as_slice(),
            [Action::AddDialogSet {
                dialog_set_id: 120101,
                slot: 10,
                mission_id: Some(1360),
            }]
        ),
        "chain 6505 must re-bind dsm 120101 to slot 10 once 1361 completes; got {actions:?}"
    );

    let mut completed = ExecutionContext::new();
    completed.world_id = Some(CMD_CENTER);
    completed.set_param("mission_id".to_string(), serde_json::json!(1361));
    completed.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!("completed"),
    );
    completed.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("completed"),
    );
    let resolved = resolve_chain(&pool, 6505, &completed, TriggerType::MissionCompleted).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6505 must emit nothing once 1360 is no longer active"
    );
}

/// Seed guard: DSM 120101 is interaction-only, and the shared stock row 5356
/// is untouched.
#[tokio::test]
async fn live_db_dsm_120101_is_interaction_only_and_5356_is_untouched() {
    let pool = require_db_or_skip!();
    let row_new: (i32, Option<i32>, i64) = sqlx::query_as(
        "SELECT dialog_set_id, dialog_id, interaction_flags::int8 \
         FROM resources.dialog_set_maps WHERE dialog_set_map_id = 120101",
    )
    .fetch_one(&pool)
    .await
    .expect("DSM 120101 must be seeded exactly once");
    assert_eq!(row_new, (1432, None, 268_435_456));

    let row_stock: (i32, Option<i32>, i64) = sqlx::query_as(
        "SELECT dialog_set_id, dialog_id, interaction_flags::int8 \
         FROM resources.dialog_set_maps WHERE dialog_set_map_id = 5356",
    )
    .fetch_one(&pool)
    .await
    .expect("stock DSM 5356 must still exist");
    assert_eq!(row_stock, (1432, Some(4576), 268_435_456));

    let last: (i64,) =
        sqlx::query_as("SELECT last_value FROM resources.dialog_set_maps_dialog_set_map_id_seq")
            .fetch_one(&pool)
            .await
            .expect("DSM sequence must exist");
    assert!(last.0 >= 120_101, "DSM sequence must reach 120101");
}

/// Seed guard: mission 1360 is one-shot in the seed (`num_repeats = 0`), so
/// the lifecycle refuses a replayed completion.
#[tokio::test]
async fn live_db_mission_1360_is_one_shot_in_the_seed() {
    let pool = require_db_or_skip!();
    let (num_repeats,): (i32,) =
        sqlx::query_as("SELECT num_repeats FROM resources.missions WHERE mission_id = 1360")
            .fetch_one(&pool)
            .await
            .expect("mission 1360 must be seeded");
    assert_eq!(num_repeats, 0, "mission 1360 must be one-shot in the seed");
}

/// Seed guard: the 567 chains (6503/6504) ship enabled with the Castle leg
/// (FZ-12, D-FZ1); item 2698 is granted by Castle chain 1397.
#[tokio::test]
async fn live_db_567_files_chains_ship_enabled() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, bool)> = sqlx::query_as(
        "SELECT chain_id, enabled FROM resources.content_chains \
         WHERE chain_id IN (6503, 6504) ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("chains 6503/6504 query must succeed");
    assert_eq!(rows, vec![(6503, true), (6504, true)]);
}

/// Restore negative: once the letter is delivered, world entry must not
/// re-paint the indicator. A stale "!" on a shared-hub NPC is exactly the
/// forgotten-clear failure the interaction-flag convention exists to stop.
#[tokio::test]
async fn live_db_chain_6502_does_not_rebind_after_delivery() {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("completed"),
    );

    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6502, &ctx, TriggerType::PlayerLoaded).await;
    assert!(
        resolved.actions.is_empty(),
        "chain 6502 must not re-bind Marsh's indicator once step 4038 is completed; got {:?}",
        resolved.actions
    );
}

/// A `Harset_CmdCenter` startup space wide enough to hold the staged
/// player. Bounds mirror `entities/spaces.xml` (`Instanced="false"`, +-400):
/// the Command Center is a SHARED space in this repo (decision D-H04), which
/// is the whole reason these chains use per-player `add_dialog_set` binds.
fn make_cmd_center_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-400" MaxX="400" MinY="-400" MaxY="400" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        "Harset_CmdCenter".to_string(),
        WorldRow::enforcing(CMD_CENTER),
    )]));
    mgr
}

/// Stage the chain-firing player and a live Marsh in the Command Center, with
/// an entered session and 1360 active at step 4038 (objective 4651), which is
/// what the A3b request path checks before it queues a Plugin request.
fn stage_player(mgr: &mut SpaceManager) {
    let space = mgr
        .create_entity(PLAYER_EID, "Harset_CmdCenter", [0.0, 0.0, 0.0], [0.0; 3])
        .expect("Harset_CmdCenter startup space must accept the player entity");
    mgr.create_entity_in_space(MARSH_EID, space, [1.0, 0.0, 0.0], [0.0; 3])
        .expect("Marsh must spawn beside the player");
    mgr.connect_entity(PLAYER_EID);
    let marsh = mgr.get_entity_mut(MARSH_EID).expect("Marsh exists");
    marsh.tag = Some("CmdCenter_Marsh".into());
    marsh.template_id = Some(10);
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    p.last_interaction_target = Some(MARSH_EID);
    p.extensions.insert(LetterTurnInSession {
        player_id: 42,
        token: LetterTurnInToken::from_bytes([8; 16]),
        generation: Instant::now(),
        epoch: LetterTurnInEntryEpoch::new(1),
    });
    p.missions.add_mission(MissionInstance::new(
        1360,
        4038,
        vec![MissionObjective {
            objective_id: 4651,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    ));
}
