//! Mission 1200 "Meet Your Queen" — chains 6121-6127
//! (`harset_goauld_chains.sql`, Harset packet H40).
//!
//! 1200 is new authoring over fully recovered dialog data: set 1268
//! survives complete in `dialog_set_maps` / `dialog_screens` /
//! `dialog_screen_buttons` and maps one-to-one onto the two steps. The
//! regression surface is therefore the wiring, not the text:
//!
//! 1. **The archetype gate.** 1200 is Goa'uld-only and is accepted on
//!    arrival, so a wrong-archetype player must resolve nothing at all —
//!    otherwise every Jaffa and human landing on Harset silently picks
//!    up a Goa'uld story mission.
//! 2. **Step ordering.** 3584 must advance with `advance_step`, never
//!    `complete_objective`: 4139 is step 3584's only required objective
//!    and completing it would end the mission before the player ever
//!    reaches Anat (`progression.rs:176-183`).
//! 3. **The hidden optional never blocks.** Objective 5399 "Ask Ba'al
//!    for help" is `is_hidden = t, is_optional = t`. Completing it must
//!    not complete the mission, skipping it must not prevent completion,
//!    and it must not be re-offered once taken.
//! 4. **Binding discipline on template 43.** Anat is shared with mission
//!    742; the cross-mission disjointness assertion lives in
//!    `mission_742.rs`, and the guard here is the narrower one — that
//!    1200 binds Anat from exactly one site.
//!
//! Dialog evidence pinned by these tests (all live-DB, all recovered):
//! dsm 4817 -> dialog 4452 (Royal Guard, button 170 "Convince Anat's
//! Royal Jaffa to grant you an audience."), dsm 4751 -> 5435 (Anat,
//! button 171 "Flatter Anat."), dsm 6360 -> 5436 (Ba'al's advice, no
//! buttons — which is why chain 6124 triggers on `dialog_open`).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const HARSET: i32 = 57;
const CMD_CENTER: i32 = 68;
const GOAULD: i64 = 6;
/// `enum_range(NULL::resources."EArchetype")[n + 1]` — 8 is
/// `ARCHETYPE_Jaffa`, the archetype Castle's chains 1011/1012 gate on.
/// Used here purely as a wrong-archetype probe.
const JAFFA: i64 = 8;

async fn engine_with(pool: &sqlx::PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

fn actions_of(resolved: &ResolvedActions, chain_id: i64) -> Vec<Action> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a.clone())
        .collect()
}

fn ctx_step(step_id: i32, status: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        format!("mission_1200_step_{step_id}_status"),
        serde_json::json!(status),
    );
    ctx
}

/// A step context that also carries the `dialog_id` the event is about.
///
/// `Trigger::OnDialogOpen` and `OnDialogChoice` both match on
/// `event.params["dialog_id"]` (`triggers/matching.rs:140-142`), so a
/// dialog-triggered chain resolves NOTHING without this key — including
/// in the negative assertions, which would then pass for the wrong
/// reason. Every dialog test below goes through this helper so its
/// negatives stay honest: they must fail the *condition*, not the
/// trigger.
fn ctx_dialog(dialog_id: i32, step_id: i32, status: &str) -> ExecutionContext {
    let mut ctx = ctx_step(step_id, status);
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx
}

fn ctx_world_entry(world: i32, world_name: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(world);
    ctx.set_param("world_name".to_string(), serde_json::json!(world_name));
    ctx
}

// ── Arrival accept ──────────────────────────────────────────────────

/// A Goa'uld who has never taken 1200 picks it up on arrival in Harset.
/// Accept-only by design: the Royal Guard binding lives on chain 6125
/// alone so `available_interactions[209]` cannot accumulate duplicates,
/// and the Guard is in world 68 anyway.
#[tokio::test]
async fn chain_6121_accepts_1200_for_a_goauld_arriving_in_harset() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6121).await;

    let mut ctx = ctx_world_entry(HARSET, "Harset");
    ctx.set_param("archetype".to_string(), serde_json::json!(GOAULD));
    ctx.set_param(
        "mission_1200_status".to_string(),
        serde_json::json!("not_active"),
    );

    let actions = actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), 6121);
    assert!(
        actions.len() == 1 && matches!(actions[0], Action::AcceptMission { mission_id: 1200 }),
        "chain 6121 must resolve exactly one AcceptMission(1200) and bind \
         nothing — the Royal Guard bind belongs to 6125. Got {actions:?}"
    );
}

/// **Wrong archetype resolves nothing.** The headline negative: without
/// this gate every Jaffa, human and Asgard landing on Harset silently
/// acquires a Goa'uld story mission.
#[tokio::test]
async fn chain_6121_resolves_nothing_for_a_non_goauld() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6121).await;

    // Every archetype in `resources."EArchetype"` except Goa'uld,
    // addressed by the same 0-based index the loader uses.
    for archetype in [0i64, 1, 2, 3, 4, 5, 7, JAFFA] {
        let mut ctx = ctx_world_entry(HARSET, "Harset");
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
        ctx.set_param(
            "mission_1200_status".to_string(),
            serde_json::json!("not_active"),
        );

        assert!(
            actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), 6121).is_empty(),
            "chain 6121 accepted 1200 for archetype {archetype}"
        );
    }
}

/// The other two arrival gates: an already-taken mission, and the wrong
/// world. A missing `world_id` must also fail closed.
#[tokio::test]
async fn chain_6121_does_not_reaccept_or_fire_in_the_wrong_world() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6121).await;

    for status in ["active", "completed"] {
        let mut ctx = ctx_world_entry(HARSET, "Harset");
        ctx.set_param("archetype".to_string(), serde_json::json!(GOAULD));
        ctx.set_param("mission_1200_status".to_string(), serde_json::json!(status));
        assert!(
            actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), 6121).is_empty(),
            "chain 6121 re-accepted 1200 with status {status}"
        );
    }

    for world in [Some(CMD_CENTER), None] {
        let mut ctx = ctx_world_entry(HARSET, "Harset");
        ctx.world_id = world;
        ctx.set_param("archetype".to_string(), serde_json::json!(GOAULD));
        ctx.set_param(
            "mission_1200_status".to_string(),
            serde_json::json!("not_active"),
        );
        assert!(
            actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), 6121).is_empty(),
            "chain 6121 fired with world_id = {world:?}"
        );
    }
}

// ── Step 3584 -> 3585 ───────────────────────────────────────────────

/// Chain 6122: the Royal Guard is convinced. Both of step 3585's topics
/// open here because the player is already standing in world 68.
#[tokio::test]
async fn chain_6122_advances_to_3585_and_opens_anat_and_baal() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6122).await;

    let ctx = ctx_dialog(4452, 3584, "active");
    let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6122);

    assert_eq!(actions.len(), 4, "chain 6122 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::RemoveDialogSet {
            dialog_set_id: 4817,
            slot: 209
        }
    ));
    assert!(
        matches!(
            actions[1],
            Action::AdvanceStep {
                mission_id: 1200,
                step_id: 3585
            }
        ),
        "step 3584 must advance, never complete_objective(4139): 4139 is its \
         only required objective, so completing it would end 1200 before the \
         player reaches Anat. Got {:?}",
        actions[1]
    );
    assert!(matches!(
        actions[2],
        Action::AddDialogSet {
            dialog_set_id: 4751,
            slot: 43,
            ..
        }
    ));
    assert!(matches!(
        actions[3],
        Action::AddDialogSet {
            dialog_set_id: 6360,
            slot: 42,
            ..
        }
    ));

    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CompleteObjective { .. })),
        "no complete_objective on a mid-mission step"
    );
}

#[tokio::test]
async fn chain_6122_does_not_refire_once_past_step_3584() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6122).await;

    for status in ["completed", "not_active"] {
        let ctx = ctx_dialog(4452, 3584, status);
        assert!(
            actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6122).is_empty(),
            "chain 6122 re-resolved with step 3584 {status}"
        );
    }
}

// ── Step 3585: Anat, and the hidden optional ────────────────────────

/// Chain 6123: flattering Anat completes the mission and retires both
/// topics, including Ba'al's whether or not the player took it.
#[tokio::test]
async fn chain_6123_completes_1200_and_retires_both_topics() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6123).await;

    let ctx = ctx_dialog(5435, 3585, "active");
    let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6123);

    assert_eq!(actions.len(), 3, "chain 6123 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::RemoveDialogSet {
            dialog_set_id: 4751,
            slot: 43
        }
    ));
    assert!(
        matches!(
            actions[1],
            Action::RemoveDialogSet {
                dialog_set_id: 6360,
                slot: 42
            }
        ),
        "Ba'al's optional topic must be retired even when the player skipped \
         it, or the '!' stays on him forever. Got {:?}",
        actions[1]
    );
    assert!(matches!(
        actions[2],
        Action::CompleteMission { mission_id: 1200 }
    ));
}

/// **The optional never blocks.** Chain 6123 carries no condition on
/// objective 5399 at all, so a player who never spoke to Ba'al finishes
/// exactly as one who did. Reverting that — adding an
/// `objective_status 5399 eq completed` gate — fails here.
#[tokio::test]
async fn the_hidden_optional_never_blocks_completion() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6123).await;

    for status in ["active", "completed", "not_active"] {
        let mut ctx = ctx_dialog(5435, 3585, "active");
        ctx.set_param(
            "mission_1200_obj_5399_status".to_string(),
            serde_json::json!(status),
        );

        let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6123);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CompleteMission { mission_id: 1200 })),
            "chain 6123 refused to complete 1200 with objective 5399 {status} \
             — the hidden optional must never gate the mission"
        );
    }
}

/// Chain 6124: Ba'al's advice completes objective 5399 and nothing else
/// mission-shaped. It must NEVER resolve a `CompleteMission` — 5399 is
/// optional, so `complete_objective` on it leaves the required 4140
/// active and the mission running (`progression.rs:176-180`).
///
/// The trigger is `dialog_open`, not `dialog_choice`: dialog 5436 has
/// zero `dialog_screen_buttons` rows and — unlike 742's button-less
/// dialogs — there is no 2009 subscription proving the client emits a
/// choice for it. Flipping this row back to `dialog_choice` fails here.
#[tokio::test]
async fn chain_6124_completes_only_the_optional_objective() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6124).await;

    let mut ctx = ctx_dialog(5436, 3585, "active");
    ctx.set_param(
        "mission_1200_obj_5399_status".to_string(),
        serde_json::json!("active"),
    );

    // `dialog_choice` must resolve nothing — the row is `dialog_open`.
    assert!(
        actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6124).is_empty(),
        "chain 6124 answered a DialogChoice event; the seed row must be \
         `dialog_open` because dialog 5436 declares no buttons"
    );

    let actions = actions_of(&fire(&engine, TriggerType::DialogOpen, &ctx), 6124);
    assert_eq!(actions.len(), 2, "chain 6124 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::CompleteObjective {
            mission_id: 1200,
            objective_id: 5399
        }
    ));
    assert!(matches!(
        actions[1],
        Action::RemoveDialogSet {
            dialog_set_id: 6360,
            slot: 42
        }
    ));
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CompleteMission { .. })),
        "the optional objective must not complete the mission"
    );
}

/// Re-opening Ba'al's dialog must not re-complete 5399, and talking to
/// him during step 3584 must do nothing at all — the advice is about
/// Anat, and objective 5399 does not exist until 3585 is active.
#[tokio::test]
async fn chain_6124_is_inert_before_step_3585_and_after_it_is_taken() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6124).await;

    // Already taken.
    let mut taken = ctx_dialog(5436, 3585, "active");
    taken.set_param(
        "mission_1200_obj_5399_status".to_string(),
        serde_json::json!("completed"),
    );
    assert!(
        actions_of(&fire(&engine, TriggerType::DialogOpen, &taken), 6124).is_empty(),
        "chain 6124 re-completed objective 5399 on a second dialog open"
    );

    // Too early — still convincing the Royal Guard.
    let mut early = ctx_dialog(5436, 3584, "active");
    early.set_param(
        "mission_1200_obj_5399_status".to_string(),
        serde_json::json!("active"),
    );
    assert!(
        actions_of(&fire(&engine, TriggerType::DialogOpen, &early), 6124).is_empty(),
        "chain 6124 fired during step 3584"
    );
}

// ── World-entry paint and restore ───────────────────────────────────

/// 6125 (Royal Guard, step 3584) and 6126 (Anat, step 3585) re-paint
/// exactly their own step on entry to the Command Center, and nothing on
/// the other's step. 6125 is also the FIRST paint, not just a restore.
#[tokio::test]
async fn world_entry_chains_paint_exactly_the_active_step() {
    let pool = require_db_or_skip!();

    // (chain, active step, other step, dsm, template)
    let cases: [(i32, i32, i32, i32, i32); 2] =
        [(6125, 3584, 3585, 4817, 209), (6126, 3585, 3584, 4751, 43)];

    for (chain_id, step, other_step, dsm, template) in cases {
        let engine = engine_with(&pool, chain_id).await;

        let mut ctx = ctx_world_entry(CMD_CENTER, "Harset_CmdCenter");
        ctx.set_param(
            format!("mission_1200_step_{step}_status"),
            serde_json::json!("active"),
        );

        let actions = actions_of(
            &fire(&engine, TriggerType::PlayerLoaded, &ctx),
            chain_id as i64,
        );
        assert_eq!(
            actions.len(),
            1,
            "chain {chain_id} must bind exactly one set; got {actions:?}"
        );
        assert!(
            matches!(
                actions[0],
                Action::AddDialogSet { dialog_set_id, slot, .. }
                    if dialog_set_id == dsm && slot == template
            ),
            "chain {chain_id} must bind dsm {dsm} on template {template}; got {:?}",
            actions[0]
        );

        // The other step active instead: nothing.
        let mut other = ctx_world_entry(CMD_CENTER, "Harset_CmdCenter");
        other.set_param(
            format!("mission_1200_step_{other_step}_status"),
            serde_json::json!("active"),
        );
        assert!(
            actions_of(
                &fire(&engine, TriggerType::PlayerLoaded, &other),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} painted its binding while step {other_step} was active"
        );

        // Wrong world: nothing.
        let mut wrong = ctx_world_entry(HARSET, "Harset");
        wrong.set_param(
            format!("mission_1200_step_{step}_status"),
            serde_json::json!("active"),
        );
        assert!(
            actions_of(
                &fire(&engine, TriggerType::PlayerLoaded, &wrong),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} fired outside world 68"
        );
    }
}

/// Chain 6127 restores Ba'al's advice topic only while 5399 is still
/// outstanding — the extra `objective_status` gate 6126 deliberately
/// does not carry, because Anat's topic must come back until the mission
/// is over while Ba'al's must not be re-offered once taken.
///
/// **H50 landed 2026-09-19**: this chain's `objective_status` gate is
/// now satisfiable from a real post-relog context. 5399 round-trips
/// through `sgw_mission` and comes back `hidden` + `optional` from
/// `resources.mission_objectives`; see
/// `optional_and_hidden_flags_survive_a_relog` in
/// [`super::mission_relog_persistence`], which drives 1200 step 3585
/// through the whole loop. The assertions below stay hand-seeded so they
/// test this chain's gate in isolation.
#[tokio::test]
async fn chain_6127_restores_baals_advice_only_while_it_is_outstanding() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6127).await;

    let mut outstanding = ctx_world_entry(CMD_CENTER, "Harset_CmdCenter");
    outstanding.set_param(
        "mission_1200_step_3585_status".to_string(),
        serde_json::json!("active"),
    );
    outstanding.set_param(
        "mission_1200_obj_5399_status".to_string(),
        serde_json::json!("active"),
    );

    let actions = actions_of(
        &fire(&engine, TriggerType::PlayerLoaded, &outstanding),
        6127,
    );
    assert!(
        actions.len() == 1
            && matches!(
                actions[0],
                Action::AddDialogSet {
                    dialog_set_id: 6360,
                    slot: 42,
                    mission_id: Some(1200),
                }
            ),
        "chain 6127 resolved {actions:?}"
    );

    let mut taken = outstanding;
    taken.set_param(
        "mission_1200_obj_5399_status".to_string(),
        serde_json::json!("completed"),
    );
    assert!(
        actions_of(&fire(&engine, TriggerType::PlayerLoaded, &taken), 6127).is_empty(),
        "chain 6127 re-offered advice the player had already taken"
    );
}

/// Only one site binds Anat for 1200. If a second `add_dialog_set 4751`
/// row is ever added on a `player_loaded` chain, the world-entry path
/// would push two identical entries into `available_interactions[43]`.
/// Benign today (`retain` removes every copy, `.first()` picks the same
/// dialog) but it stops being benign the moment a second dsm shares the
/// template — which is exactly what mission 742 does.
#[tokio::test]
async fn exactly_one_world_entry_chain_binds_anat_for_1200() {
    let pool = require_db_or_skip!();

    let mut binders = Vec::new();
    for chain_id in [6121i32, 6125, 6126, 6127] {
        let engine = engine_with(&pool, chain_id).await;

        let mut ctx = ctx_world_entry(CMD_CENTER, "Harset_CmdCenter");
        ctx.set_param("archetype".to_string(), serde_json::json!(GOAULD));
        ctx.set_param(
            "mission_1200_status".to_string(),
            serde_json::json!("not_active"),
        );
        for step in [3584, 3585] {
            ctx.set_param(
                format!("mission_1200_step_{step}_status"),
                serde_json::json!("active"),
            );
        }
        ctx.set_param(
            "mission_1200_obj_5399_status".to_string(),
            serde_json::json!("active"),
        );

        if actions_of(
            &fire(&engine, TriggerType::PlayerLoaded, &ctx),
            chain_id as i64,
        )
        .iter()
        .any(|a| matches!(a, Action::AddDialogSet { slot: 43, .. }))
        {
            binders.push(chain_id);
        }
    }

    assert_eq!(
        binders,
        vec![6126],
        "exactly chain 6126 may bind Anat on world entry for 1200; got {binders:?}"
    );
}
