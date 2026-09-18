//! Mission 704 step 2405, the Zuritska escort — chains 1291 and 1302 in
//! `db/resources/Content/Seed/castle_702_704_chains.sql` (packet CA07).
//! Sibling of [`super::mission_704`], which pins the terminal and
//! delivery steps, and of [`super::mission_704_restores`].
//!
//! The escort is the one part of 704 that can break without the player
//! doing anything wrong, so it carries two chains rather than one:
//!
//! * **1291** ends it. Reaching the Communications Room advances the
//!   step, clears the follow, arms the terminal and the workstation
//!   actor, and retires the cell actor's `!`.
//! * **1302** restarts it. `AiState::Follow` is preemptable into Fighting
//!   and `npc_ai_leash` returns to Idle, never to Follow, so one point of
//!   splash damage on the way to Level 5 ends the escort permanently.
//!   Clicking Zuritska re-issues the follow. That click is why the cell
//!   actor's `!` outlives mission 702 instead of being cleared at the
//!   rescue — see `mission_702::chain_1263_*`.
//!
//! The executor-path halves of both — does the follow actually get set,
//! does it actually get cleared — are in
//! [`super::castle_702_704_executor`].

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

/// `Castle_Zuritska_Cell`'s seeded spawn position — spawn_id 238, template
/// 168, world 8, authored by packet CA05
/// (`docs/analysis/castle-rebuild/worknotes/ca05.md`, "Spawnlist rows").
///
/// Chain 1291 walks her back here when the escort ends, as a literal
/// destination, because `MoveWaypoint` takes a parsed `[f32; 3]` and the
/// content engine has no "walk to your spawn" verb. This constant and the
/// seed row are therefore two copies of one fact;
/// `chain_1291_destination_matches_the_seeded_spawn_row` ties them to the
/// third copy, `resources.spawnlist`.
const ZURITSKA_CELL_SPAWN: [f32; 3] = [268.0, 66.79, 1042.59];

fn label(a: &Action) -> &'static str {
    match a {
        Action::AdvanceStep { .. } => "advance_step",
        Action::SetFollowTarget { .. } => "set_follow_target",
        Action::MoveWaypoint { .. } => "move_waypoint",
        Action::DisplayDialog { .. } => "display_dialog",
        Action::SetInteractionType { .. } => "set_interaction_type",
        Action::StartMinigame { .. } => "start_minigame",
        Action::GrantItem { .. } => "add_item",
        Action::RemoveItem { .. } => "remove_item",
        Action::CompleteMission { .. } => "complete_mission",
        Action::AcceptMission { .. } => "accept_mission",
        _ => "OTHER",
    }
}

async fn load(pool: &sqlx::PgPool, chain_id: i32) -> cimmeria_content_engine::chain::Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains — \
                 castle_702_704_chains.sql missing from db/database.sql?"
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
// Chain 1291 — reach the Communications Room on step 2405
// ──────────────────────────────────────────────────────────────────────

fn comms_arrival_params() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("region_key", serde_json::json!("Castle.CommsRoom")),
        ("world_name", serde_json::json!("Castle")),
        ("mission_704_step_2405_status", serde_json::json!("active")),
    ]
}

#[tokio::test]
async fn chain_1291_arrival_advances_ends_the_escort_and_arms_the_terminal() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1291,
        TriggerType::RegionEnter,
        &comms_arrival_params(),
    )
    .await;

    // Dialog 4866 is click-to-play on chain 1299, never on this chain. A
    // region-entry chain stamps no `target_entity_id`, and 4866 is not a
    // monologue (speaker 1113 on screens 96892 and 96894), so it would bind
    // through the player's `last_interaction_target` pin — which resolves to
    // the CELL Zuritska they just freed, and is empty entirely after a
    // relog. Arming the workstation actor is what makes 1299's click
    // reachable, so both facts are pinned here, before the signature check
    // narrows the failure message.
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::DisplayDialog { .. })),
        "chain 1291 must not display a dialog — a region-entry chain has no \
         NPC to bind the client's portrait lookup to. Dialog 4866 belongs on \
         chain 1299's interact_tag trigger. Got {actions:?}",
    );
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::SetInteractionType { entity_tag, .. } if entity_tag == "Castle_Zuritska_Comms"
        )),
        "chain 1291 must arm the workstation actor or chain 1299's click is \
         unreachable and the player never hears 4866; got {actions:?}",
    );

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec![
            "advance_step",
            "set_follow_target",
            "move_waypoint",
            "set_interaction_type",
            "set_interaction_type",
            "set_interaction_type",
        ],
        "chain 1291 action ordering drifted; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 704,
                step_id: 2406
            }
        ),
        "chain 1291 must advance 704 to step 2406; got {:?}",
        actions[0],
    );
    match &actions[1] {
        Action::SetFollowTarget {
            entity_tag,
            target_tag,
            use_player,
        } => {
            assert_eq!(entity_tag, "Castle_Zuritska_Cell");
            assert_eq!(
                *target_tag, None,
                "the escort CLEAR carries no target_tag — a tag here would \
                 re-point the follow instead of ending it",
            );
            assert_eq!(
                *use_player, None,
                "`use_player` must be absent on the clear; Some(true) would \
                 re-arm the follow on the player who just arrived",
            );
        }
        other => panic!("chain 1291 action 2 must be set_follow_target; got {other:?}"),
    }
    // The walk home. Order is load-bearing and is why this is asserted
    // positionally rather than with `.any()`: `set_follow_target` with no
    // target drops the NPC to Idle and clears `nav_path`, so a
    // `move_waypoint` resolved *before* it would have its path thrown away
    // and she would stand in the Comms Room forever.
    match &actions[2] {
        Action::MoveWaypoint {
            entity_tag,
            destination,
            speed,
        } => {
            assert_eq!(entity_tag, "Castle_Zuritska_Cell");
            assert_eq!(
                *destination, ZURITSKA_CELL_SPAWN,
                "chain 1291 must walk the cell actor back to her CA05 spawn \
                 position; a drifted literal here strands her in the \
                 Communications Room or inside a wall",
            );
            assert_eq!(
                *speed, 1.0,
                "no `speed` param — the default 1.0 multiplier keeps her at \
                 the template's own move_speed (0.9)",
            );
        }
        other => panic!("chain 1291 action 3 must be move_waypoint; got {other:?}"),
    }
    assert_interaction(
        &actions[3],
        "Castle_CommsTerminal",
        "|",
        INT_MINIGAME_LIVEWIRE,
        "chain 1291 terminal bit",
    );
    // Step 2406 has two interactables: the terminal and Zuritska herself
    // (chain 1299's instruction dialog). Arming only the terminal would
    // leave the player with no way to hear 4866 at all.
    assert_interaction(
        &actions[4],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1291 workstation bit",
    );
    // Arriving is what ends the escort, so it is also what retires the
    // escort-restart affordance on the CELL actor. This clear is the
    // matched pair for the `!` chain 1261 set two missions ago; chain 1263
    // deliberately leaves it alone so chain 1302 stays reachable for the
    // whole of step 2405.
    assert_interaction(
        &actions[5],
        "Castle_Zuritska_Cell",
        "~",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1291 cell-actor clear",
    );
}

#[tokio::test]
async fn chain_1291_does_not_resolve_once_the_terminal_step_is_active() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1291,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.CommsRoom")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "re-entering the room on 2406 must not re-play 4866 or re-advance; \
         got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1291_does_not_resolve_without_704() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1291,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.CommsRoom")),
            ("world_name", serde_json::json!("Castle")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "a passer-by without 704 must not arm the terminal; got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1302 — clicking Zuritska restarts a broken escort
// ──────────────────────────────────────────────────────────────────────

/// `AiState::Follow` is preemptable into Fighting, and `npc_ai_leash` ends
/// at Idle and never returns to Follow. One point of splash damage on the
/// way to Level 5 therefore ends the escort, and before this chain the only
/// recovery was a relog. Clicking Zuritska re-issues the follow.
#[tokio::test]
async fn chain_1302_interact_re_arms_the_escort_follow() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1302,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Cell")),
            ("mission_704_step_2405_status", serde_json::json!("active")),
        ],
    )
    .await;
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
                "the re-arm must point her at the clicking player, not at a tag",
            );
        }
        other => panic!("chain 1302 must re-issue set_follow_target; got {other:?}"),
    }
}

/// The escort is over at 2406. Re-arming there would drag Zuritska into the
/// Communications Room behind a player who is meant to be hacking the
/// terminal, undoing chain 1291's clear.
#[tokio::test]
async fn chain_1302_does_not_resolve_once_the_escort_is_over() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1302,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Cell")),
            (
                "mission_704_step_2405_status",
                serde_json::json!("completed"),
            ),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1302 must be gated on step 2405; got {actions:?}",
    );
}

/// 1302 and mission 702's chain 1262 share the `Castle_Zuritska_Cell` tag
/// and are separated only by their step gates, which chain 1263 flips in
/// one action list. Both firing on one click would open the rescue dialog
/// again mid-escort.
#[tokio::test]
async fn chains_1262_and_1302_never_claim_the_same_click() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for chain_id in [1262, 1302] {
        engine.register_chain(load(&pool, chain_id).await);
    }

    for (step_2419, step_2405, expected) in [
        ("active", "not_active", 1262_i64),
        ("completed", "active", 1302_i64),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("Castle_Zuritska_Cell"),
        );
        ctx.set_param(
            "mission_702_step_2419_status".to_string(),
            serde_json::json!(step_2419),
        );
        ctx.set_param(
            "mission_704_step_2405_status".to_string(),
            serde_json::json!(step_2405),
        );
        let event = TriggerEvent {
            trigger_type: TriggerType::InteractTag,
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
            "with 702/2419={step_2419} and 704/2405={step_2405} only chain \
             {expected} may fire",
        );
    }
}

// ──────────────────────────────────────────────────────────────────────
// Cross-packet drift guard: chain 1291's literal vs CA05's spawn row
// ──────────────────────────────────────────────────────────────────────

/// Chain 1291's `move_waypoint` destination is a **second copy** of
/// `Castle_Zuritska_Cell`'s spawn position, which CA05 owns in
/// `resources.spawnlist` (spawn_id 238). `MoveWaypoint` takes a parsed
/// `[f32; 3]` and the content engine has no "walk to your spawn" verb, so the
/// duplication is forced; this test is what stops the two copies drifting.
/// If CA05's position moves and this seed row does not, Zuritska walks to
/// wherever she used to live — which, in a room bounded by prefab geometry,
/// means inside a wall.
///
/// **Two modes, deliberately, and the active one depends on a sibling PR.**
/// CA05's spawnlist row ships on PR #667, not here. Until that merges the
/// row is absent and this test asserts only what is knowable without it: that
/// the chain resolves a `move_waypoint` at CA05's *documented* coordinate and
/// that nothing else claims the tag. Once #667 lands, the row appears and the
/// equality assertion below activates with no edit to this file — the test
/// becomes the drift guard its name promises. The mode is printed in the
/// failure message either way, so a green run is never ambiguous about which
/// half ran.
///
/// This is the only conditional assertion in the 702-704 suite. It is here
/// rather than in CA05's own tests because the duplicated literal is *this*
/// packet's, so the drift is this packet's to catch.
#[tokio::test]
async fn chain_1291_destination_matches_the_seeded_spawn_row() {
    let pool = require_db_or_skip!();

    let actions = resolve_chain(
        &pool,
        1291,
        TriggerType::RegionEnter,
        &comms_arrival_params(),
    )
    .await;
    let dest = actions
        .iter()
        .find_map(|a| match a {
            Action::MoveWaypoint {
                entity_tag,
                destination,
                ..
            } if entity_tag == "Castle_Zuritska_Cell" => Some(*destination),
            _ => None,
        })
        .expect(
            "chain 1291 must carry a move_waypoint for Castle_Zuritska_Cell --              without it the escort ends with her standing in the Comms Room",
        );
    assert_eq!(
        dest, ZURITSKA_CELL_SPAWN,
        "chain 1291's destination drifted from the coordinate CA05 documented",
    );

    let rows: Vec<(f32, f32, f32)> = sqlx::query_as(
        "SELECT x, y, z FROM resources.spawnlist          WHERE tag = $1 AND world_id = 8",
    )
    .bind("Castle_Zuritska_Cell")
    .fetch_all(&pool)
    .await
    .expect("spawnlist query must succeed");

    assert!(
        rows.len() <= 1,
        "exactly one spawn row may claim Castle_Zuritska_Cell; {} do, so \
         `move_waypoint`'s tag lookup is nondeterministic",
        rows.len(),
    );

    // Absent row = pre-#667 mode; the assertions above already ran and this
    // half activates the moment CA05's spawnlist lands.
    if let Some(&(x, y, z)) = rows.first() {
        assert_eq!(
            [x, y, z],
            ZURITSKA_CELL_SPAWN,
            "DRIFT: CA05's spawn row for Castle_Zuritska_Cell is {:?} but \
             chain 1291 walks her to {ZURITSKA_CELL_SPAWN:?}. Update the \
             `move_waypoint` row in castle_702_704_chains.sql and this \
             constant together.",
            [x, y, z],
        );
    }
}
