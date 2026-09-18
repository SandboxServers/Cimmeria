//! Mission 702 "Rescue Dr. Zuritska" — chains 1261-1264 in
//! `db/resources/Content/Seed/castle_702_704_chains.sql` (packet CA06).
//!
//! Every chain in that file is a RECONSTRUCTION: there is no recovered
//! server script for 702-708, so these guards pin the *authoring
//! decisions* against silent seed drift rather than a ported reference.
//! The decisions worth pinning:
//!
//! * Region entry advances 2402 → 2419 and lights the cell actor; the
//!   `!` bit and its clear are a matched pair (chains 1261 / 1263) with a
//!   `player_loaded` restore (1264), because interaction flags do not
//!   survive a relog.
//! * The dialog-2577 choice is the ONLY accept point for mission 704 in
//!   the shipped data, so chain 1263 carries both the `step_status` gate
//!   and the `mission_status 704 eq not_active` accept-guard.
//! * Objective 4653 is completed by `complete_mission 702`, never by a
//!   separate `complete_objective` — a future author adding one would be
//!   double-completing the mission.
//! * `set_follow_target use_player` is what starts 704's escort. The
//!   executor-path guard for it lives in
//!   [`super::castle_702_704_executor`], because a resolve-only assertion
//!   cannot tell a wired executor arm from the `other =>` catch-all
//!   (TESTING.md type 6, PR #618).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// The `!` main-story-active bit (`INT_AStoryMissionActive`).
const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;

/// Compact label for an action, so a drifted action LIST fails on the
/// sequence rather than on a structural diff of every field. Per-action
/// invariants (ids, masks) are asserted separately.
fn label(a: &Action) -> &'static str {
    match a {
        Action::AdvanceStep { .. } => "advance_step",
        Action::SetInteractionType { .. } => "set_interaction_type",
        Action::DisplayDialog { .. } => "display_dialog",
        Action::CompleteMission { .. } => "complete_mission",
        Action::AcceptMission { .. } => "accept_mission",
        Action::SetFollowTarget { .. } => "set_follow_target",
        _ => "OTHER",
    }
}

/// Register one seeded chain and resolve a synthetic event against it.
/// Returns only the actions attributed to `chain_id`, in order.
async fn resolve_chain(
    pool: &sqlx::PgPool,
    chain_id: i32,
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> Vec<Action> {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains — \
                 castle_702_704_chains.sql missing from db/database.sql?"
            )
        });

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

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

/// Assert a `set_interaction_type` action targets `tag` with `op`/`mask`.
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
// Chain 1261 — enter Castle.InterrogationBlock on step 2402
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1261_region_entry_advances_to_2419_and_lights_the_cell_actor() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1261,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_702_step_2402_status", serde_json::json!("active")),
        ],
    )
    .await;

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec!["advance_step", "set_interaction_type"],
        "chain 1261 must advance the step before lighting the actor; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 702,
                step_id: 2419
            }
        ),
        "chain 1261 must advance 702 to step 2419; got {:?}",
        actions[0],
    );
    assert_interaction(
        &actions[1],
        "Castle_Zuritska_Cell",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1261 indicator",
    );
}

/// Adjacent wrong state: the player is already past the travel step. The
/// chain must not re-advance (which would force-complete 2419's objective
/// 4653 and skip the rescue entirely).
#[tokio::test]
async fn chain_1261_does_not_resolve_once_2419_is_the_active_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1261,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            (
                "mission_702_step_2402_status",
                serde_json::json!("completed"),
            ),
            ("mission_702_step_2419_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1261 must not re-fire once 2402 is done; got {actions:?}",
    );
}

/// A player who never accepted 702 walking through the same volume must
/// get nothing — the region is shared with mission 703's chain 1271.
#[tokio::test]
async fn chain_1261_does_not_resolve_without_the_mission() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1261,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1261 must not fire for a player without 702 active; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1262 — interact with the caged Zuritska on step 2419
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1262_interact_displays_dialog_2577() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1262,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Cell")),
            ("mission_702_step_2419_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(
        actions.len(),
        1,
        "chain 1262 must produce exactly one action; got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 2577 }),
        "chain 1262 must display dialog 2577; got {:?}",
        actions[0],
    );
}

/// Clicking the cell actor while still on the travel step must not open
/// the rescue dialog — 2577's choice is what completes the mission.
#[tokio::test]
async fn chain_1262_does_not_resolve_on_the_travel_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1262,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Cell")),
            ("mission_702_step_2402_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1262 must be gated on step 2419; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1263 — dialog 2577 choice: the rescue
// ──────────────────────────────────────────────────────────────────────

/// Helper: the full param set for a player standing on 2419 with 704 not
/// yet accepted.
fn rescue_params() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("dialog_id", serde_json::json!(2577)),
        ("mission_702_step_2419_status", serde_json::json!("active")),
        ("mission_704_status", serde_json::json!("not_active")),
    ]
}

#[tokio::test]
async fn chain_1263_completes_702_accepts_704_and_starts_the_escort() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(&pool, 1263, TriggerType::DialogChoice, &rescue_params()).await;

    // Objective 4653 is completed by the mission completion. A
    // `complete_objective` alongside it would complete 702 twice (a last
    // non-optional objective completes the mission on its own). Asserted
    // BEFORE the action-list signature below, which maps an unexpected
    // action to "OTHER" and would otherwise trip first and make this
    // check unreachable.
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CompleteObjective { .. })),
        "chain 1263 must NOT hand-complete objective 4653; \
         `complete_mission 702` does it. Got {actions:?}",
    );
    // The `!` on the cell actor must SURVIVE this chain. It is the
    // affordance mission 704's chain 1302 needs to restart an escort broken
    // by splash damage, so it is cleared on Comms Room arrival (1291), not
    // at the rescue. A clear here would silently re-introduce the
    // relog-only recovery this design replaced.
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::SetInteractionType { .. })),
        "chain 1263 must NOT touch the cell actor's indicator — it stays lit \
         through 704 step 2405 so the escort can be restarted by clicking \
         her. Chain 1291 owns the clear. Got {actions:?}",
    );

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec!["complete_mission", "accept_mission", "set_follow_target"],
        "chain 1263 action ordering drifted — the follow must be armed after \
         704 is accepted. Got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::CompleteMission { mission_id: 702 }),
        "chain 1263 must complete 702; got {:?}",
        actions[0],
    );
    assert!(
        matches!(actions[1], Action::AcceptMission { mission_id: 704 }),
        "chain 1263 must accept 704; got {:?}",
        actions[1],
    );
    match &actions[2] {
        Action::SetFollowTarget {
            entity_tag,
            target_tag,
            use_player,
        } => {
            assert_eq!(entity_tag, "Castle_Zuritska_Cell");
            assert_eq!(
                *target_tag, None,
                "the escort follows the PLAYER, so target_tag must stay None",
            );
            assert_eq!(
                *use_player,
                Some(true),
                "use_player is the only way to follow a player — players \
                 carry no spawnlist tag",
            );
        }
        other => panic!("chain 1263 action 3 must be set_follow_target; got {other:?}"),
    }
}

/// No double accept of 704: the accept-guard must refuse once 704 is live.
/// Re-firing would also re-arm the follow, snapping Zuritska back onto a
/// player who has already reached the Communications Room.
#[tokio::test]
async fn chain_1263_does_not_resolve_when_704_is_already_active() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1263,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2577)),
            ("mission_702_step_2419_status", serde_json::json!("active")),
            ("mission_704_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1263 must carry `mission_status 704 eq not_active`; got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1263_does_not_resolve_when_704_is_already_completed() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1263,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2577)),
            ("mission_702_step_2419_status", serde_json::json!("active")),
            ("mission_704_status", serde_json::json!("completed")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1263 must not re-run for a player who already finished 704; got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1263_does_not_resolve_on_the_wrong_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1263,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2577)),
            ("mission_702_step_2402_status", serde_json::json!("active")),
            ("mission_704_status", serde_json::json!("not_active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1263 must be gated on step 2419; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1264 — relog restore for step 2419
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1264_restores_the_cell_indicator_on_login() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1264,
        TriggerType::PlayerLoaded,
        &[
            ("world_name", serde_json::json!("Castle")),
            ("mission_702_step_2419_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(
        actions.len(),
        1,
        "chain 1264 must produce exactly one action; got {actions:?}",
    );
    assert_interaction(
        &actions[0],
        "Castle_Zuritska_Cell",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1264 restore",
    );
}

/// The restore must not re-light the actor for a player who already
/// freed Zuritska, or the `!` sticks forever in a shared zone.
#[tokio::test]
async fn chain_1264_does_not_restore_after_the_rescue() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1264,
        TriggerType::PlayerLoaded,
        &[
            ("world_name", serde_json::json!("Castle")),
            (
                "mission_702_step_2419_status",
                serde_json::json!("completed"),
            ),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1264 must be gated on step 2419 being active; got {actions:?}",
    );
}

/// The restore is keyed to the Castle world. Loading into any other world
/// must not touch Castle's actors.
#[tokio::test]
async fn chain_1264_does_not_restore_in_another_world() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1264,
        TriggerType::PlayerLoaded,
        &[
            ("world_name", serde_json::json!("Castle_CellBlock")),
            ("mission_702_step_2419_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1264's player_loaded key must be 'Castle'; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1265 — region re-entry repair for the shared `!` bit
// ──────────────────────────────────────────────────────────────────────

/// `set_interaction_type` is global on the entity, so another player's
/// rescue clears Zuritska's `!` for everyone — including a player still on
/// step 2419, who then cannot click her at all. Chain 1264 repairs that
/// only on a relog; 1265 repairs it on walking back into the volume.
#[tokio::test]
async fn chain_1265_repairs_the_cell_indicator_on_region_re_entry() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1265,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.InterrogationBlock")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_702_step_2419_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(
        actions.len(),
        1,
        "chain 1265 must produce exactly one action; got {actions:?}",
    );
    assert_interaction(
        &actions[0],
        "Castle_Zuritska_Cell",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1265 repair",
    );
}

/// 1261 and 1265 share a region trigger and are separated only by their
/// step gate. If either gate drifted they would both claim an entry, and
/// the repair's `|` would immediately undo 1261's own advance semantics by
/// re-lighting an actor the player has not reached yet.
#[tokio::test]
async fn chains_1261_and_1265_never_claim_the_same_region_entry() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for chain_id in [1261, 1265] {
        engine.register_chain(
            load_single_chain_for_test(&pool, chain_id)
                .await
                .expect("DB query must succeed")
                .expect("chain must exist in seeded content_chains"),
        );
    }

    for (step_2402, step_2419, expected) in [
        ("active", "not_active", 1261_i64),
        ("completed", "active", 1265_i64),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "region_key".to_string(),
            serde_json::json!("Castle.InterrogationBlock"),
        );
        ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
        ctx.set_param(
            "mission_702_step_2402_status".to_string(),
            serde_json::json!(step_2402),
        );
        ctx.set_param(
            "mission_702_step_2419_status".to_string(),
            serde_json::json!(step_2419),
        );
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
            "with 2402={step_2402} / 2419={step_2419} only chain {expected} may fire",
        );
    }
}
