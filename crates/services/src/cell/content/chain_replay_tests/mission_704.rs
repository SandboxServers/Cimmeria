//! Mission 704 "Hack Communications" — the terminal and delivery steps,
//! chains 1292-1295 and 1299 in
//! `db/resources/Content/Seed/castle_702_704_chains.sql` (packet CA07).
//! The step-2405 escort (1291, 1302) is in [`super::mission_704_escort`],
//! the relog restores and shared-bit repairs (1296-1298, 1300, 1301) are in
//! [`super::mission_704_restores`], and the executor-path guards are in
//! [`super::castle_702_704_executor`].
//!
//! RECONSTRUCTION, not a port: `Castle.py` never mentions 704. The
//! authoring decisions these guards pin:
//!
//! * The Livewire launcher (1292) carries the step gate and the victory
//!   chain (1293) carries none. Victory chains are fired by id with
//!   `ResolvedActions::default()` and evaluate NO conditions
//!   (`event_dispatch/mod.rs`), so a condition row on 1293 would be a
//!   silent no-op that looks like a guard. `chain_1293_is_triggerless_and
//!   _conditionless` pins that inversion directly.
//! * Item 5029 is granted exactly once, and the guard is 1292's
//!   `step_status 2406 active` plus 1293's own advance to 2407 — not a
//!   condition on 1293.
//! * D-CA08: step state is the possession proof. Chain 1295 consumes the
//!   crystal but does NOT gate on holding it, because the cell has no
//!   inventory view to gate against.
//! * Dialog 4866 is click-to-play on chain 1299 rather than played on
//!   arrival: only an `interact_tag` trigger stamps the `target_entity_id`
//!   that binds the client's portrait to the workstation Zuritska. The
//!   arrival side of that decision is pinned in
//!   [`super::mission_704_escort`].

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{Trigger, TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// The `!` main-story-active bit (`INT_AStoryMissionActive`).
const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;
/// The hackable-console bit (`INT_MinigameLivewire`).
const INT_MINIGAME_LIVEWIRE: i64 = 256;

fn label(a: &Action) -> &'static str {
    match a {
        Action::AdvanceStep { .. } => "advance_step",
        Action::SetFollowTarget { .. } => "set_follow_target",
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
// Chain 1292 — the Livewire launcher, and its step gate
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1292_terminal_starts_livewire_naming_the_victory_chain() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1292,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_CommsTerminal")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(
        actions.len(),
        1,
        "chain 1292 must produce exactly one action; got {actions:?}",
    );
    match &actions[0] {
        Action::StartMinigame {
            minigame_type,
            difficulty,
            on_victory_chains,
        } => {
            // `difficulty` arrived with #652. Chain 1292's seed row omits it,
            // so it must come through as the loader's default of 1 — asserted
            // rather than `..`-ignored, because a stray `difficulty` param in
            // this file's seed rows would otherwise go unnoticed, and an
            // out-of-range one drops the whole action row (a launcher that
            // silently resolves nothing).
            assert_eq!(
                *difficulty, 1,
                "chain 1292 authors no difficulty, so the loader default (1) \
                 must survive to the resolved action",
            );
            assert_eq!(
                minigame_type, "Livewire",
                "D-CA09 (provisional): Livewire is the only implemented \
                 minigame; the rest are auto-win placeholders",
            );
            assert_eq!(
                on_victory_chains,
                &vec![1293_i64],
                "the launcher must name victory chain 1293 — a wrong id \
                 silently drops the crystal grant",
            );
        }
        other => panic!("chain 1292 must start a minigame; got {other:?}"),
    }
}

/// The launcher must not resolve on either neighbouring step. On 2405 the
/// terminal is not yet the objective; on 2407 a second win would re-run
/// 1293 and grant a second Data Crystal.
#[tokio::test]
async fn chain_1292_does_not_resolve_on_the_escort_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1292,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_CommsTerminal")),
            ("mission_704_step_2405_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1292 must be gated on step 2406; got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1292_does_not_resolve_on_the_delivery_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1292,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_CommsTerminal")),
            (
                "mission_704_step_2406_status",
                serde_json::json!("completed"),
            ),
            ("mission_704_step_2407_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "re-hacking the terminal on 2407 must not re-grant item 5029 — the \
         launcher gate is the ONLY single-grant guard, because victory \
         chains evaluate no conditions. Got {actions:?}",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1293 — the Livewire victory chain
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1293_victory_grants_the_crystal_and_swaps_the_cursors() {
    let pool = require_db_or_skip!();
    let chain = load(&pool, 1293).await;
    let actions = chain.actions;

    // Asserted first: the single-grant invariant is the point of the whole
    // 1292/1293 split, and the signature check below would trip on a
    // duplicated grant before this line ever ran.
    let grants = actions
        .iter()
        .filter(|a| matches!(a, Action::GrantItem { item_id: 5029, .. }))
        .count();
    assert_eq!(
        grants, 1,
        "exactly one Data Crystal grant per victory; got {grants} in {actions:?}",
    );

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec![
            "display_dialog",
            "add_item",
            "advance_step",
            "set_interaction_type",
            "set_interaction_type",
        ],
        "chain 1293 action ordering drifted; got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 2580 }),
        "chain 1293 must display the terminal read-out 2580; got {:?}",
        actions[0],
    );
    match &actions[1] {
        Action::GrantItem {
            item_id,
            count,
            container_id,
        } => {
            assert_eq!(*item_id, 5029, "must grant the Data Crystal");
            assert_eq!(*count, 1, "exactly one crystal");
            // `container: 0` is the "use the item's own container_sets[1]"
            // sentinel (`executor::inventory::grant` filters `c > 0`).
            assert_eq!(*container_id, Some(0));
        }
        other => panic!("chain 1293 action 2 must be add_item; got {other:?}"),
    }
    assert!(
        matches!(
            actions[2],
            Action::AdvanceStep {
                mission_id: 704,
                step_id: 2407
            }
        ),
        "chain 1293 must advance to the delivery step; got {:?}",
        actions[2],
    );
    assert_interaction(
        &actions[3],
        "Castle_CommsTerminal",
        "~",
        INT_MINIGAME_LIVEWIRE,
        "chain 1293 terminal clear",
    );
    assert_interaction(
        &actions[4],
        "Castle_Zuritska_Comms",
        "|",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1293 workstation indicator",
    );
}

/// The inversion that makes the launcher gate load-bearing: chain 1293
/// must carry NO trigger rows and NO condition rows. A condition here
/// would never be evaluated (`fire_chain_by_id` passes
/// `ResolvedActions::default()`), so it would read as a guard while
/// guarding nothing. A trigger row would make the chain fire on a real
/// world event as well as on victory, double-granting the crystal.
#[tokio::test]
async fn chain_1293_is_triggerless_and_conditionless() {
    let pool = require_db_or_skip!();
    let chain = load(&pool, 1293).await;

    assert!(
        chain.conditions.is_empty(),
        "chain 1293 must carry no conditions — victory chains evaluate none, \
         so any condition row is a silent no-op masquerading as a guard. \
         Got {:?}",
        chain.conditions,
    );
    match &chain.trigger {
        Trigger::OnCustomEvent { event_name } => assert_eq!(
            event_name, "__direct_invoke_1293",
            "a triggerless chain gets the loader's synthetic never-firing \
             trigger; a different event_name means a real trigger row was \
             added",
        ),
        other => panic!(
            "chain 1293 must have no content_triggers row — it is invoked \
             only via chain 1292's on_victory_chains. Got trigger {other:?}",
        ),
    }
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1299 — Zuritska's instruction, click-to-play
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1299_interact_displays_the_terminal_instruction() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1299,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Comms")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(actions.len(), 1, "got {actions:?}");
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 4866 }),
        "chain 1299 must display dialog 4866; got {:?}",
        actions[0],
    );
}

/// 1299 and 1294 share the `Castle_Zuritska_Comms` tag and are separated
/// only by their step gate. Both firing on one click would stack two
/// dialogs on the player.
#[tokio::test]
async fn chains_1299_and_1294_never_claim_the_same_click() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for chain_id in [1294, 1299] {
        engine.register_chain(load(&pool, chain_id).await);
    }

    for (step_2406, step_2407, expected) in [
        ("active", "not_active", 1299_i64),
        ("completed", "active", 1294_i64),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("Castle_Zuritska_Comms"),
        );
        ctx.set_param(
            "mission_704_step_2406_status".to_string(),
            serde_json::json!(step_2406),
        );
        ctx.set_param(
            "mission_704_step_2407_status".to_string(),
            serde_json::json!(step_2407),
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
            "with 2406={step_2406} / 2407={step_2407} only chain {expected} may fire",
        );
    }
}

// ──────────────────────────────────────────────────────────────────────
// Chain 1294 / 1295 — delivering the crystal
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn chain_1294_interact_displays_the_delivery_briefing() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1294,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Comms")),
            ("mission_704_step_2407_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert_eq!(actions.len(), 1, "got {actions:?}");
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 2581 }),
        "chain 1294 must display dialog 2581; got {:?}",
        actions[0],
    );
}

#[tokio::test]
async fn chain_1294_does_not_resolve_before_the_crystal_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1294,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_Zuritska_Comms")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1294 must be gated on step 2407; got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1295_delivery_consumes_the_crystal_completes_704_accepts_706() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1295,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2581)),
            ("mission_704_step_2407_status", serde_json::json!("active")),
            ("mission_706_status", serde_json::json!("not_active")),
        ],
    )
    .await;

    let signature: Vec<&str> = actions.iter().map(label).collect();
    assert_eq!(
        signature,
        vec![
            "remove_item",
            "set_interaction_type",
            "complete_mission",
            "accept_mission",
        ],
        "chain 1295 action ordering drifted; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::RemoveItem {
                item_id: 5029,
                count: 1
            }
        ),
        "the crystal is handed over, so it must be consumed explicitly — \
         `UseInventoryItem` no longer auto-consumes. Got {:?}",
        actions[0],
    );
    assert_interaction(
        &actions[1],
        "Castle_Zuritska_Comms",
        "~",
        INT_A_STORY_MISSION_ACTIVE,
        "chain 1295 workstation clear",
    );
    assert!(
        matches!(actions[2], Action::CompleteMission { mission_id: 704 }),
        "got {:?}",
        actions[2],
    );
    assert!(
        matches!(actions[3], Action::AcceptMission { mission_id: 706 }),
        "got {:?}",
        actions[3],
    );

    // D-CA08: step state is the possession proof. A `HasItem` condition
    // would never evaluate true — the cell carries no inventory view.
    let chain = load(&pool, 1295).await;
    assert!(
        !chain.conditions.iter().any(|c| matches!(
            c,
            cimmeria_content_engine::conditions::Condition::HasItem { .. }
        )),
        "chain 1295 must not gate on HasItem (D-CA08); got {:?}",
        chain.conditions,
    );
}

#[tokio::test]
async fn chain_1295_does_not_resolve_when_706_is_already_active() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1295,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2581)),
            ("mission_704_step_2407_status", serde_json::json!("active")),
            ("mission_706_status", serde_json::json!("active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1295 must carry `mission_status 706 eq not_active` — a second \
         run would also consume another crystal. Got {actions:?}",
    );
}

/// The repeat-player case. `mission_status ... eq not_active` is false for
/// `completed` as well as for `active`, so a player who has already
/// finished 706 and somehow re-reaches this dialog neither re-consumes a
/// crystal nor re-accepts. Mirrors `mission_702`'s pair on chain 1263.
#[tokio::test]
async fn chain_1295_does_not_resolve_when_706_is_already_completed() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1295,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2581)),
            ("mission_704_step_2407_status", serde_json::json!("active")),
            ("mission_706_status", serde_json::json!("completed")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1295 must not re-run for a player who already finished 706 — \
         it would consume another Data Crystal. Got {actions:?}",
    );
}

#[tokio::test]
async fn chain_1295_does_not_resolve_on_the_wrong_step() {
    let pool = require_db_or_skip!();
    let actions = resolve_chain(
        &pool,
        1295,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2581)),
            ("mission_704_step_2406_status", serde_json::json!("active")),
            ("mission_706_status", serde_json::json!("not_active")),
        ],
    )
    .await;
    assert!(
        actions.is_empty(),
        "chain 1295 must be gated on step 2407; got {actions:?}",
    );
}
