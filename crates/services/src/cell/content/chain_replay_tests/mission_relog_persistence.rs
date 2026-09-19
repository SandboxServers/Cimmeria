//! H50 acceptance: per-objective mission state survives a relog.
//!
//! Every other module here stops at `resolve_event`. This one runs the
//! whole loop the defect lived in:
//!
//! ```text
//! executor arm -> cell::missions::send_mission_update -> MissionUpdate
//!   -> (what the base UPSERTs into sgw_mission)
//!   -> player_init::mission_restore::build_restored_missions
//!   -> content::mission_context::populate_mission_context
//!   -> Condition::ObjectiveStatus
//! ```
//!
//! Nothing in the middle is faked. The `SavedMission` a test hydrates
//! from is rebuilt field-for-field out of the `MissionUpdate` the
//! executor actually put on the wire, so a test here cannot pass by
//! agreeing with itself — which is precisely how the defect survived
//! until now. `mission_742.rs`'s pin hand-built the hydrated instance,
//! so it could never have failed when the production shape changed.
//!
//! The mission caches come from the seeded database rather than from
//! hand-written `MissionObjectiveDef`s, because the headline fix is that
//! hydration reconstructs the objective roster *from those tables*.
//! Hand-seeding them would re-introduce the self-agreement.
//!
//! Shapes covered (the packet's acceptance list):
//!
//! | mission | shape |
//! |---|---|
//! | 742 / 2913 | objective completed on the current step |
//! | 688 / 2734 | objective completed on a step the player advanced past |
//! | 1200 / 5399 | `hidden` + `optional` carried through hydration |
//! | 742 | `MissionUpdate` emitted on a real completion, not on a no-op |
//! | 742 | a pre-H50 row holding the STEP id self-heals on login |

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use super::super::mission_context::populate_mission_context;
use crate::cell::messages::{CellToBaseMsg, SavedMission};
use crate::cell::service::base_messages::player_init::mission_restore::build_restored_missions;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_mission_defs, load_step_objectives};
use crate::test_support::require_db_or_skip;

/// The pre-relog session's cell entity.
const EID_BEFORE: u32 = 7460;
/// The post-relog session's cell entity — a *different* id on purpose,
/// so nothing can leak across the boundary in memory.
const EID_AFTER: u32 = 7461;
const PLAYER_ID: i32 = 4460;

const HARSET: i32 = 57;

// ── fixture ─────────────────────────────────────────────────────────

/// A space manager carrying the real `mission_defs` / `step_objectives`
/// caches, which is what `advance_step` and `build_restored_missions`
/// both read.
async fn mgr_with_seeded_mission_caches(pool: &sqlx::PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" Instanced="false" MinX="-1200" MaxX="1200" MinY="-1200" MaxY="1200" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.mission_defs = load_mission_defs(pool)
        .await
        .expect("load_mission_defs must succeed against the seeded DB");
    mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load_step_objectives must succeed against the seeded DB");
    mgr
}

fn stage_player(mgr: &mut SpaceManager, entity_id: u32) {
    mgr.create_entity(entity_id, "Harset", [0.0; 3], [0.0; 3])
        .expect("Harset startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(entity_id)
        .expect("player must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(entity_id);
}

/// Run one action through the real executor, exactly as a resolved chain
/// would. Returns every `CellToBaseMsg` it produced.
async fn run_action(action: Action, entity_id: u32, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    let resolved = ResolvedActions {
        actions: vec![(0, action)],
        action_delays: vec![0],
        params: std::collections::HashMap::new(),
    };
    let engine = ChainEngine::new();
    execute_actions(resolved, entity_id, PLAYER_ID, &tx, mgr, &engine).await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(m) = rx.recv().await {
        out.push(m);
    }
    out
}

fn mission_updates(msgs: &[CellToBaseMsg]) -> Vec<&CellToBaseMsg> {
    msgs.iter()
        .filter(|m| matches!(m, CellToBaseMsg::MissionUpdate { .. }))
        .collect()
}

/// Turn the last `MissionUpdate` for `mission_id` into the `SavedMission`
/// the base would hand back at the next `InitPlayerState`. This is the
/// DB round-trip minus the DB: `sgw_mission` stores exactly these
/// columns and `SavedMission` is what the loader builds from them.
fn saved_from_wire(msgs: &[CellToBaseMsg], mission_id: i32) -> SavedMission {
    msgs.iter()
        .rev()
        .find_map(|m| match m {
            CellToBaseMsg::MissionUpdate {
                mission_id: mid,
                status,
                current_step_id,
                completed_step_ids,
                completed_objective_ids,
                active_objective_ids,
                failed_objective_ids,
                repeats,
                ..
            } if *mid == mission_id => Some(SavedMission {
                mission_id: *mid,
                status: *status,
                current_step_id: *current_step_id,
                completed_step_ids: completed_step_ids.clone(),
                completed_objective_ids: completed_objective_ids.clone(),
                active_objective_ids: active_objective_ids.clone(),
                failed_objective_ids: failed_objective_ids.clone(),
                repeats: *repeats,
            }),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "no MissionUpdate for mission {mission_id} reached the base — \
                 the executor arm persisted nothing"
            )
        })
}

/// The relog: a fresh cell entity hydrated from the saved rows alone.
fn relog(mgr: &mut SpaceManager, saved: &[SavedMission]) {
    let restored = build_restored_missions(saved, mgr);
    stage_player(mgr, EID_AFTER);
    let e = mgr.get_entity_mut(EID_AFTER).unwrap();
    for m in restored {
        e.missions.add_mission(m);
    }
}

/// The context a post-relog dispatch builds for the hydrated player.
fn post_relog_context(mgr: &SpaceManager) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(HARSET);
    populate_mission_context(mgr.get_entity(EID_AFTER).unwrap(), &mut ctx);
    ctx
}

fn param<'a>(ctx: &'a ExecutionContext, key: &str) -> Option<&'a str> {
    ctx.params.get(key).and_then(|v| v.as_str())
}

async fn resolve(pool: &sqlx::PgPool, chain_id: i32, ctx: &ExecutionContext) -> Vec<Action> {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine
        .resolve_event(&event, ctx)
        .actions
        .into_iter()
        .filter(|(id, _)| *id == i64::from(chain_id))
        .map(|(_, a)| a)
        .collect()
}

// ── 1. mission 742 / objective 2913 — current-step completion ───────

/// The packet's headline acceptance. Accept 742, walk to step 2504,
/// complete objective 2913, relog, and the per-objective params must
/// come back — 2913 `completed`, its two siblings `active`.
///
/// Reverting any one of the three halves breaks this: without the
/// `complete_objective` arm's `send_mission_update` nothing records
/// 2913; without the objective-id fix `active_objective_ids` holds the
/// step id; without def-driven hydration the roster is whatever the row
/// happened to contain.
#[tokio::test]
async fn objective_completed_on_the_current_step_survives_a_relog() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(Action::AcceptMission { mission_id: 742 }, EID_BEFORE, &mut mgr).await;
    run_action(
        Action::AdvanceStep {
            mission_id: 742,
            step_id: 2504,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    let msgs = run_action(
        Action::CompleteObjective {
            mission_id: 742,
            objective_id: 2913,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    let saved = saved_from_wire(&msgs, 742);
    assert_eq!(
        saved.active_objective_ids,
        vec![2913, 2914, 2915],
        "step 2504's three OBJECTIVE ids must reach the row — pre-H50 this \
         array held the step id (2504)",
    );
    assert_eq!(
        saved.completed_objective_ids,
        vec![2911, 2913],
        "both completions must reach the row: 2911 is step 2502's objective, \
         force-completed by the advance to 2504, and 2913 is the basket the \
         player just planted. 2914/2915 are untouched and must not appear.",
    );
    assert_eq!(saved.current_step_id, Some(2504));

    let mut after = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut after, &[saved]);
    let ctx = post_relog_context(&after);

    assert_eq!(
        param(&ctx, "mission_742_obj_2913_status"),
        Some("completed"),
        "`objective_status 742 2913 eq completed` must resolve after a relog",
    );
    assert_eq!(
        param(&ctx, "mission_742_obj_2914_status"),
        Some("active"),
        "2914 was never planted; it must NOT come back completed",
    );
    assert_eq!(param(&ctx, "mission_742_obj_2915_status"), Some("active"));
}

/// The gate a live player walks into: chain 6104 plants the first
/// device and is gated `objective_status 742 2913 eq active`. Across a
/// relog it must still resolve while the basket is unplanted, and must
/// be inert once it is — otherwise the player either cannot plant, or
/// can re-plant the same basket forever and drain their Scarabs.
///
/// This is the live half of the inversion of `mission_742.rs`'s H50 pin.
#[tokio::test]
async fn chain_6104_gate_tracks_the_objective_across_a_relog() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(Action::AcceptMission { mission_id: 742 }, EID_BEFORE, &mut mgr).await;
    let msgs = run_action(
        Action::AdvanceStep {
            mission_id: 742,
            step_id: 2504,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    // Relog with nothing planted.
    let mut unplanted = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut unplanted, &[saved_from_wire(&msgs, 742)]);
    let mut ctx = post_relog_context(&unplanted);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("FirstBug"));

    let actions = resolve(&pool, 6104, &ctx).await;
    assert_eq!(
        actions.len(),
        2,
        "chain 6104 must resolve against a genuinely hydrated context; got \
         {actions:?}. Before H50 the objective params did not survive the \
         round-trip and this chain was dead after every relog.",
    );
    assert!(matches!(
        actions[0],
        Action::CompleteObjective {
            mission_id: 742,
            objective_id: 2913,
        }
    ));

    // Now plant it, relog again, and the same gate must be closed.
    let planted_msgs = run_action(
        Action::CompleteObjective {
            mission_id: 742,
            objective_id: 2913,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    let mut planted = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut planted, &[saved_from_wire(&planted_msgs, 742)]);
    let mut ctx = post_relog_context(&planted);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("FirstBug"));

    assert!(
        resolve(&pool, 6104, &ctx).await.is_empty(),
        "an already-planted basket must not re-offer after a relog — the \
         player would consume a second Scarab for nothing",
    );
}

// ── 2. mission 688 / objective 2734 — prior-step completion ─────────

/// Castle chain 1109's shape. 2734 is step 2356's only objective;
/// `advance_step` to 80688 force-completes it, and it then lives *only*
/// in `completed_objectives` — the current step's roster has moved on.
/// `populate_mission_context`'s second loop is what keeps
/// `objective_status 688 2734 eq completed` matching, and it can only do
/// that if `completed_objective_ids` round-trips.
#[tokio::test]
async fn objective_completed_on_a_prior_step_survives_a_relog() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(Action::AcceptMission { mission_id: 688 }, EID_BEFORE, &mut mgr).await;
    let msgs = run_action(
        Action::AdvanceStep {
            mission_id: 688,
            step_id: 80688,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    let saved = saved_from_wire(&msgs, 688);
    assert!(
        saved.completed_objective_ids.contains(&2734),
        "advance_step force-completes the outgoing step's objectives; 2734 \
         must reach the row. Got {:?}",
        saved.completed_objective_ids,
    );
    assert!(
        saved.completed_step_ids.contains(&2356),
        "the outgoing step must reach the row too",
    );

    let mut after = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut after, &[saved]);
    let ctx = post_relog_context(&after);

    assert_eq!(
        param(&ctx, "mission_688_obj_2734_status"),
        Some("completed"),
        "`objective_status 688 2734 eq completed` must resolve after a relog",
    );
    assert!(
        !after
            .get_entity(EID_AFTER)
            .unwrap()
            .missions
            .get_mission(688)
            .unwrap()
            .active_objectives
            .iter()
            .any(|o| o.objective_id == 2734),
        "a prior step's objective must not rejoin the current roster — it \
         would then count toward step 80688's completion",
    );
}

// ── 3. mission 1200 / objective 5399 — the flag carry ───────────────

/// Mission 1200 step 3585 is the seed's canonical mixed step: 4140
/// required, 5399 `is_hidden = t, is_optional = t`. Both flags have to
/// survive hydration.
///
/// `optional` is the load-bearing one. `all_required_complete` filters on
/// it, so with the pre-H50 hardcoded `false` a relogged player who
/// skipped Ba'al could complete the last *required* objective and the
/// mission would stay open forever, waiting on an objective the design
/// says is voluntary. `hidden` matters to the client: `onObjectiveUpdate`
/// and `serialize_resend` both put it on the wire, and the journal
/// renders a hidden objective differently.
#[tokio::test]
async fn optional_and_hidden_flags_survive_a_relog() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(
        Action::AcceptMission { mission_id: 1200 },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    let msgs = run_action(
        Action::AdvanceStep {
            mission_id: 1200,
            step_id: 3585,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    let mut after = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut after, &[saved_from_wire(&msgs, 1200)]);

    let objs = after
        .get_entity(EID_AFTER)
        .unwrap()
        .missions
        .get_mission(1200)
        .unwrap()
        .active_objectives
        .clone();
    let o5399 = objs
        .iter()
        .find(|o| o.objective_id == 5399)
        .expect("5399 must be on the restored roster for step 3585");
    assert!(
        o5399.optional,
        "5399 is `is_optional = t` in resources.mission_objectives — pre-H50 \
         hydration hardcoded `optional: false` and it came back required",
    );
    assert!(
        o5399.hidden,
        "5399 is `is_hidden = t`; the client renders a hidden objective \
         differently and only learns the flag from the server",
    );
    assert!(
        !objs
            .iter()
            .find(|o| o.objective_id == 4140)
            .unwrap()
            .optional,
        "4140 is required — the flag must be read per objective, not \
         blanket-applied",
    );

    // The behaviour the flag buys: the still-open optional must not hold
    // the mission open once every required objective is done.
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::missions::complete_objective(EID_AFTER, 1200, 4140, &tx, &mut after).await;
    assert_eq!(
        after
            .get_entity(EID_AFTER)
            .unwrap()
            .missions
            .get_mission(1200)
            .unwrap()
            .status,
        MISSION_COMPLETED,
        "with 4140 (the only required objective) done, the open optional 5399 \
         must not block completion — restored as required, it did",
    );
}

/// The other direction, so the fix cannot be mistaken for "optional
/// objectives complete missions": completing the optional 5399 alone,
/// after a relog, must leave 1200 active because 4140 is still open.
#[tokio::test]
async fn completing_only_the_optional_objective_does_not_complete_the_mission() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(
        Action::AcceptMission { mission_id: 1200 },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    let msgs = run_action(
        Action::AdvanceStep {
            mission_id: 1200,
            step_id: 3585,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    let mut after = mgr_with_seeded_mission_caches(&pool).await;
    relog(&mut after, &[saved_from_wire(&msgs, 1200)]);

    let (tx, _rx) = mpsc::channel(256);
    crate::cell::missions::complete_objective(EID_AFTER, 1200, 5399, &tx, &mut after).await;
    assert_eq!(
        after
            .get_entity(EID_AFTER)
            .unwrap()
            .missions
            .get_mission(1200)
            .unwrap()
            .status,
        MISSION_ACTIVE,
        "Ba'al's advice is voluntary flavour; taking it must not end the \
         mission while Anat (4140) is still unspoken to",
    );
}

// ── 4. the MissionUpdate emission contract ──────────────────────────

/// A real completion persists exactly one `MissionUpdate`; a no-op
/// persists none.
///
/// The no-op half is the half that matters. `complete_objective` against
/// an objective that is not on the current step's roster returns without
/// mutating anything (`MissionInstance::complete_objective` matches by
/// id). Persisting there would UPSERT the unchanged row, which is
/// harmless for the data but fatal for every guard above: a dead
/// executor arm and a live one would produce the same DB traffic.
#[tokio::test]
async fn mission_update_is_emitted_on_a_real_completion_and_not_on_a_no_op() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;
    stage_player(&mut mgr, EID_BEFORE);

    run_action(Action::AcceptMission { mission_id: 742 }, EID_BEFORE, &mut mgr).await;
    run_action(
        Action::AdvanceStep {
            mission_id: 742,
            step_id: 2504,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;

    let real = run_action(
        Action::CompleteObjective {
            mission_id: 742,
            objective_id: 2913,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    assert_eq!(
        mission_updates(&real).len(),
        1,
        "a real objective completion must persist exactly one MissionUpdate \
         — pre-H50 this arm persisted none at all",
    );

    // 2734 belongs to mission 688's step 2356; it is not on 742's roster.
    let no_op = run_action(
        Action::CompleteObjective {
            mission_id: 742,
            objective_id: 2734,
        },
        EID_BEFORE,
        &mut mgr,
    )
    .await;
    assert!(
        mission_updates(&no_op).is_empty(),
        "an objective that is not on the current step's roster must not \
         persist anything; got {no_op:?}",
    );
}

// ── 5. the self-heal ────────────────────────────────────────────────

/// Every `sgw_mission` row written before H50 holds the STEP id in
/// `active_objective_ids`, and the repo does not do DB migrations — so
/// hydration is the only place those rows can be repaired.
///
/// This is the shape the 2026-09-18 lane hit as "a server restart during
/// step 2504 left all three bug baskets dark for everyone": the restart
/// re-read rows whose objective array was `[2504]`, and every basket
/// chain is gated `objective_status 742 29xx eq active`, which the
/// evaluator answered `not_active` for. Def-driven reconstruction turns
/// that row back into the real roster on first login.
#[tokio::test]
async fn a_pre_h50_row_holding_the_step_id_self_heals_on_login() {
    let pool = require_db_or_skip!();
    let mut mgr = mgr_with_seeded_mission_caches(&pool).await;

    // Verbatim what the old accept / advance_step arms wrote.
    let legacy = SavedMission {
        mission_id: 742,
        status: MISSION_ACTIVE,
        current_step_id: Some(2504),
        completed_step_ids: vec![2502, 2503],
        completed_objective_ids: vec![],
        active_objective_ids: vec![2504],
        failed_objective_ids: vec![],
        repeats: 0,
    };
    relog(&mut mgr, &[legacy]);
    let mut ctx = post_relog_context(&mgr);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("FirstBug"));

    assert_eq!(
        param(&ctx, "mission_742_obj_2913_status"),
        Some("active"),
        "the roster must be rebuilt from resources.mission_objectives",
    );
    assert!(
        param(&ctx, "mission_742_obj_2504_status").is_none(),
        "the step id must not survive as a pseudo-objective — nothing can \
         complete it, so `all_required_complete` would be false forever",
    );
    assert_eq!(
        resolve(&pool, 6104, &ctx).await.len(),
        2,
        "the baskets must light up again on the first login after the fix",
    );
}
