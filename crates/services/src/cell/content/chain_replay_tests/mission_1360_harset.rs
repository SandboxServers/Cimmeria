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
//!   `step_status 1361/4694 neq active`): plays Marsh's blurb 4576,
//!   removes the letter (item 3730), clears the "!" and completes 1360.
//! - Chain 6502 (`player_loaded 'Harset_CmdCenter'`; `world eq 68`,
//!   `step_status 1360/4038 eq active`): re-binds dsm 5356 so the "!"
//!   survives a relog *and* the 57 -> 68 door crossing.
//!
//! Three guard shapes live here:
//!
//! 1. **Exact action list on the happy path**, pinned as an ordered vec
//!    rather than a filtered count, so swapping `complete_mission` for
//!    `accept_mission` (the known auto-converter bug shape) or dropping
//!    the `remove_item` fails the test.
//! 2. **The letter is consumed exactly once**, asserted twice: once on
//!    the loaded chain's shape (a duplicated `remove_item` row would
//!    consume two of a stack) and once through `execute_actions`, which
//!    is the only layer that can tell a wired executor arm from the
//!    `other =>` catch-all (TESTING.md type 6).
//! 3. **Adjacent negatives** — wrong world, wrong step, already
//!    completed, and the 1361-turn-in overlap that chain 6501's fourth
//!    condition exists to prevent.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
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
/// Pins the **exact ordered action list**. The ordering itself is not
/// load-bearing at runtime (the four actions touch disjoint state), but
/// pinning it as a vec is what makes an accidental verb swap or a dropped
/// action fail loudly instead of silently changing what the player gets.
#[tokio::test]
async fn chain_6501_delivers_the_letter_on_the_first_marsh_click() {
    let ctx = marsh_ctx(CMD_CENTER, "active", "active", "not_active");
    let pool = require_db_or_skip!();
    let resolved = resolve_chain(&pool, 6501, &ctx, TriggerType::InteractTag).await;

    let actions: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();
    assert_eq!(
        actions.len(),
        4,
        "chain 6501 must resolve exactly four actions on the happy path; got {actions:?}"
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 4576 }),
        "action 0 must play Marsh's letter blurb 4576; got {:?}",
        actions[0]
    );
    assert!(
        matches!(
            actions[1],
            Action::RemoveItem {
                item_id: 3730,
                count: 1
            }
        ),
        "action 1 must consume exactly one Frost's Letter (3730); got {:?}",
        actions[1]
    );
    assert!(
        matches!(
            actions[2],
            Action::RemoveDialogSet {
                dialog_set_id: 5356,
                slot: 10
            }
        ),
        "action 2 must clear the dsm 5356 bind from template slot 10 (Marsh); got {:?}",
        actions[2]
    );
    assert!(
        matches!(actions[3], Action::CompleteMission { mission_id: 1360 }),
        "action 3 must COMPLETE 1360 — not accept it, the auto-converter bug shape; got {:?}",
        actions[3]
    );
}

/// The letter is consumed exactly once *per the seed*: a duplicated
/// `remove_item` row would be invisible in a `len() == 4` assertion if
/// some other action were dropped in the same edit, so count it directly.
/// Mirrors `mission_639::chain_1034_includes_remove_item_for_ambernol`.
#[tokio::test]
async fn chain_6501_removes_the_letter_exactly_once_in_the_seed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6501)
        .await
        .expect("DB query for chain 6501 must succeed")
        .expect("chain 6501 must exist in seeded content_chains");

    let removes = chain
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::RemoveItem {
                    item_id: 3730,
                    count: 1
                }
            )
        })
        .count();
    assert_eq!(
        removes, 1,
        "chain 6501 must carry exactly one `RemoveItem {{ item_id: 3730, count: 1 }}`. \
         Item 3730 has max_stack_size 1 so a duplicate would not double-consume today, \
         but it would emit two `RemoveInventoryItemByType` round-trips and the second \
         would log the 'no instance of this design id owned by character' warn on every \
         delivery. Full action list: {:?}",
        chain.actions,
    );
}

/// Executor-level guard: the resolved actions actually reach
/// `CellToBaseMsg::RemoveInventoryItemByType` with the letter's design id,
/// exactly once.
///
/// A resolve-only test cannot distinguish a wired `RemoveItem` arm from
/// the executor's `other =>` catch-all — that is precisely how
/// `move_entity`'s five seeded rows no-opped in production while the
/// suite stayed green (TESTING.md type 6). `remove_item`'s arm is
/// pre-existing and covered by `executor/tests/inventory_counter.rs`, but
/// the acceptance criterion for this packet is specifically *"the letter
/// is removed exactly once"*, so it is pinned end-to-end here rather than
/// inferred from two separate tests.
#[tokio::test]
async fn chain_6501_emits_exactly_one_letter_removal_through_the_executor() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 6501)
        .await
        .expect("DB query for chain 6501 must succeed")
        .expect("chain 6501 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let ctx = marsh_ctx(CMD_CENTER, "active", "active", "not_active");
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

    let removals: Vec<(i32, i32)> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| match m {
            CellToBaseMsg::RemoveInventoryItemByType { type_id, count, .. } => {
                Some((type_id, count))
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        removals,
        vec![(3730, 1)],
        "delivering the letter must emit exactly one RemoveInventoryItemByType for \
         design id 3730, count 1; got {removals:?}"
    );
}

/// Negative (re-fire guard): once 1360 is completed, a second right-click
/// on Marsh resolves nothing — the letter cannot be removed twice.
///
/// `MissionInstance::complete()` moves `current_step_id` into
/// `completed_steps`, so both the mission gate and the step gate flip in
/// the same transition; this models the post-completion context exactly.
#[tokio::test]
async fn chain_6501_does_not_refire_once_1360_is_completed() {
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
async fn chain_6501_does_not_fire_on_the_earlier_step_4037() {
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
async fn chain_6501_does_not_fire_from_harset() {
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
async fn chain_6501_fails_closed_without_a_world_context() {
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
/// `display_dialog 4465`, the second `send_dialog_display` would re-pin
/// `open_dialog_id`, and the player would never see the letter blurb
/// while its `remove_item` and `complete_mission` still ran. Silent
/// content loss, not a crash.
#[tokio::test]
async fn chain_6501_yields_to_the_praxis_turn_in() {
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
async fn chain_6502_rebinds_the_letter_indicator_on_world_entry() {
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
                dialog_set_id: 5356,
                slot: 10,
                mission_id: Some(1360),
            }
        ),
        "chain 6502 must re-bind dsm 5356 to template slot 10 (Col Marsh) for mission \
         1360; got {:?}",
        actions[0]
    );
}

/// Restore negative: once the letter is delivered, world entry must not
/// re-paint the indicator. A stale "!" on a shared-hub NPC is exactly the
/// forgotten-clear failure the interaction-flag convention exists to stop.
#[tokio::test]
async fn chain_6502_does_not_rebind_after_delivery() {
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
/// player. Bounds mirror `entities/spaces.xml`
/// (`Instanced="false"`, ±400) — the Command Center is a SHARED space in
/// this repo (decision D-H04), which is the whole reason these chains use
/// per-player `add_dialog_set` binds instead of `set_interaction_type`.
fn make_cmd_center_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-400" MaxX="400" MinY="-400" MaxY="400" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Stage the chain-firing player in the Command Center.
fn stage_player(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, "Harset_CmdCenter", [0.0, 0.0, 0.0], [0.0; 3])
        .expect("Harset_CmdCenter startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    mgr.connect_entity(PLAYER_EID);
}
