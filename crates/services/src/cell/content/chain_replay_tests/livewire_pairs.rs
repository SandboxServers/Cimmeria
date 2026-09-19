//! Livewire launcher/victory pairs — chain-replay guards for all three
//! seeded pairs in
//! [`db/resources/Content/Seed/castle_cellblock_chains.sql`]: 1016/1017
//! (mission 638, cell door button), 1041/1042 (mission 640, ring switch),
//! and 1060/1061 (mission 641, preparation terminal). Castle packet CA04,
//! audit defect B5.
//!
//! Organised by *verb* rather than by mission, like [`super::grant_xp`] and
//! [`super::sgc_w1_move_entity`]: the risk being guarded is the
//! `start_minigame` loader arm plus the `Action::StartMinigame` executor
//! arm, which all three pairs share. `super::mission_640` still owns the
//! mission-640 flow as a whole (the transporter, the completion, the relog
//! restores); this module owns the minigame half of all three.
//!
//! Each pair gets four assertions:
//!
//! 1. The launcher resolves exactly one `StartMinigame(Livewire, …)` naming
//!    *only* its own victory chain, under its own step gate.
//! 2. The launcher resolves nothing at the adjacent wrong step.
//! 3. The resolved actions pushed through
//!    [`execute_actions`][super::super::executor::execute_actions] emit
//!    exactly one `CellToBaseMsg::StartMinigame` with the authored
//!    game name, difficulty and chain list. A resolve-only test cannot
//!    tell a wired executor arm from the `other =>` catch-all.
//! 4. The victory chain's action list is what the seed says.
//!
//! Victory chains carry no `content_triggers` row — they are invoked
//! directly through `on_victory_chains` when the minigame server reports a
//! win, so `build_chains_from_rows` gives them a never-firing
//! `OnCustomEvent` placeholder. Their assertions read the loaded `Chain`'s
//! action list directly rather than firing a synthetic event.
//!
//! The `difficulty` param (defect B5) is pinned separately, in
//! [`super::start_minigame_difficulty`] — all three pairs here run at the
//! default of 1, so the param needs a sentinel chain of its own.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Entity id of the player firing each chain.
const PLAYER_EID: u32 = 7301;
const PLAYER_ID: i32 = 42;

// ── Harness ──────────────────────────────────────────────────────────────

/// A minimal space with the chain-firing player staged in it. The
/// `start_minigame` arm only needs `entity_id` / `player_id`, but
/// `execute_actions` takes a `SpaceManager` regardless.
fn staged_space() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(PLAYER_EID, "Castle_CellBlock", [1.0; 3], [0.0; 3])
        .expect("Castle_CellBlock startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// Fire `interact_tag` on `tag` with the supplied `mission_<m>_step_<s>_status`
/// params and return the actions the chain resolved.
pub(super) async fn resolve_interact(
    pool: &PgPool,
    chain_id: i32,
    tag: &str,
    step_params: &[(&str, &str)],
) -> Vec<(i64, Action)> {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    for (key, value) in step_params {
        ctx.set_param(key.to_string(), serde_json::json!(value));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

/// Drain the cell→base channel and return every `StartMinigame` send as
/// `(entity_id, game_name, difficulty, on_victory_chains)`. Returning the
/// full list lets callers assert cardinality — a double-dispatch bug would
/// show as two sends and open two concurrent minigame sessions.
fn drain_minigame_starts(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> Vec<(u32, String, u32, Vec<i64>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::StartMinigame {
            entity_id,
            game_name,
            difficulty,
            on_victory_chains,
            ..
        } = msg
        {
            out.push((entity_id, game_name, difficulty, on_victory_chains));
        }
    }
    out
}

/// Push resolved actions through the executor and return the
/// `StartMinigame` traffic they produced.
pub(super) async fn execute_and_drain_starts(
    actions: Vec<(i64, Action)>,
) -> Vec<(u32, String, u32, Vec<i64>)> {
    let resolved = cimmeria_content_engine::chain::ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions,
    };
    let mut mgr = staged_space();
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;
    drain_minigame_starts(&mut rx)
}

/// The shared body for all three launcher chains.
///
/// `gate` is the `(param, value)` that satisfies the seeded `step_status`
/// condition; `wrong` is the same param set to the adjacent step's state,
/// which must resolve nothing.
async fn assert_launcher_pair(
    launcher_id: i32,
    victory_id: i64,
    tag: &str,
    gate: &[(&str, &str)],
    wrong: &[(&str, &str)],
) {
    let pool = require_db_or_skip!();

    // (1) Gated positive — exactly one StartMinigame naming only its own
    //     victory chain.
    let actions = resolve_interact(&pool, launcher_id, tag, gate).await;
    let starts: Vec<_> = actions
        .iter()
        .filter(|(id, action)| {
            *id == launcher_id as i64
                && matches!(
                    action,
                    Action::StartMinigame { minigame_type, on_victory_chains, .. }
                    if minigame_type == "Livewire" && on_victory_chains == &vec![victory_id]
                )
        })
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "chain {launcher_id} must resolve exactly one \
         StartMinigame(Livewire, on_victory_chains=[{victory_id}]) under its \
         step gate; got {actions:?}",
    );

    // (2) Wrong-step negative.
    let wrong_actions = resolve_interact(&pool, launcher_id, tag, wrong).await;
    let fired = wrong_actions
        .iter()
        .filter(|(id, _)| *id == launcher_id as i64)
        .count();
    assert_eq!(
        fired, 0,
        "chain {launcher_id} must NOT fire at the adjacent step — that would \
         re-open Livewire after the player already beat it; got \
         {wrong_actions:?}",
    );

    // (3) Executor arm — the resolve-only assertion above passes happily
    //     even when the executor drops the action into `other =>`.
    let sends = execute_and_drain_starts(actions).await;
    assert_eq!(
        sends.len(),
        1,
        "chain {launcher_id} must emit exactly one \
         CellToBaseMsg::StartMinigame; got {sends:?} (zero means the \
         executor has no Action::StartMinigame arm)",
    );
    assert_eq!(
        sends[0],
        (PLAYER_EID, "Livewire".to_string(), 1, vec![victory_id],),
        "StartMinigame must carry (firing entity, authored game name, \
         default difficulty 1, the authored victory chain)",
    );
}

/// Load a triggerless victory chain and return its action list.
async fn victory_actions(pool: &PgPool, chain_id: i32) -> Vec<Action> {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"))
        .actions
}

// ── Pair 1016/1017 — mission 638, cell door button ───────────────────────

/// Interacting with the cell door button while step 2115 is active starts
/// Livewire with victory chain 1017, and does not once the player has
/// advanced to 2116.
#[tokio::test]
async fn chain_1016_starts_livewire_and_reaches_base() {
    assert_launcher_pair(
        1016,
        1017,
        "329_CellDoorButton",
        &[("mission_638_step_2115_status", "active")],
        &[
            ("mission_638_step_2115_status", "completed"),
            ("mission_638_step_2116_status", "active"),
        ],
    )
    .await;
}

/// Chain 1017 (Livewire victory) must advance 638 to step 2116, play the
/// door sequence, and clear the button's Livewire interaction bit — in
/// that order, because the sequence is the door opening and the bit clear
/// is what stops the player re-hacking an open door.
#[tokio::test]
async fn chain_1017_victory_advances_2116_plays_sequence_and_clears_bit() {
    let pool = require_db_or_skip!();
    let actions = victory_actions(&pool, 1017).await;

    assert_eq!(
        actions.len(),
        3,
        "chain 1017 must resolve exactly 3 actions (advance + sequence + \
         bit clear); got {actions:?}",
    );
    assert!(
        matches!(
            &actions[0],
            Action::AdvanceStep {
                mission_id: 638,
                step_id: 2116
            }
        ),
        "chain 1017's first action must advance mission 638 to step 2116; \
         got {:?}",
        actions[0],
    );
    assert!(
        matches!(&actions[1], Action::PlaySequence { sequence_id: 1749 }),
        "chain 1017's second action must play sequence 1749 (the cell door \
         opening); got {:?}",
        actions[1],
    );
    assert!(
        matches!(
            &actions[2],
            Action::SetInteractionType { entity_tag, operation, mask: 256 }
            if entity_tag == "329_CellDoorButton" && operation == "~"
        ),
        "chain 1017's third action must clear the Livewire interaction bit \
         (mask 256, op '~') on 329_CellDoorButton; got {:?}",
        actions[2],
    );
}

// ── Pair 1041/1042 — mission 640, ring switch ────────────────────────────

/// Interacting with the ring switch while step 2120 is active starts
/// Livewire with victory chain 1042. `super::mission_640` pins the resolve
/// halves; this adds the executor-arm half and the difficulty field.
#[tokio::test]
async fn chain_1041_starts_livewire_and_reaches_base() {
    assert_launcher_pair(
        1041,
        1042,
        "HackTheRings_Switch",
        &[("mission_640_step_2120_status", "active")],
        &[
            ("mission_640_step_2120_status", "completed"),
            ("mission_640_step_2215_status", "active"),
        ],
    )
    .await;
}

/// Chain 1042's first action must advance 640 to step 2215. The three
/// interaction-bit swaps that follow are pinned in detail by
/// `super::mission_640::chain_1042_livewire_victory_resolves_advance_and_icon_swap`;
/// asserting the head here keeps the pair readable in one place without
/// duplicating that module.
#[tokio::test]
async fn chain_1042_victory_advances_to_step_2215() {
    let pool = require_db_or_skip!();
    let actions = victory_actions(&pool, 1042).await;

    assert!(
        matches!(
            actions.first(),
            Some(Action::AdvanceStep {
                mission_id: 640,
                step_id: 2215
            })
        ),
        "chain 1042's first action must advance mission 640 to step 2215; \
         got {actions:?}",
    );
}

// ── Pair 1060/1061 — mission 641, preparation terminal ───────────────────

/// Interacting with the preparation terminal while step 3564 is active
/// starts Livewire with victory chain 1061.
///
/// The wrong-step case here is "3564 already completed" rather than a
/// later step: chain 1061 *completes* mission 641, so 3564 is the last
/// step and there is no adjacent one to advance into.
#[tokio::test]
async fn chain_1060_starts_livewire_and_reaches_base() {
    assert_launcher_pair(
        1060,
        1061,
        "Preparation_Terminal",
        &[("mission_641_step_3564_status", "active")],
        &[("mission_641_step_3564_status", "completed")],
    )
    .await;
}

/// Chain 1060 also clears the terminal's Livewire cue the moment the
/// minigame starts (not on victory, unlike 1042) — the player has engaged
/// with it, so the "hack me" icon comes off immediately.
#[tokio::test]
async fn chain_1060_clears_the_terminal_cue_on_launch() {
    let pool = require_db_or_skip!();
    let actions = resolve_interact(
        &pool,
        1060,
        "Preparation_Terminal",
        &[("mission_641_step_3564_status", "active")],
    )
    .await;

    let clears = actions
        .iter()
        .filter(|(id, action)| {
            *id == 1060
                && matches!(
                    action,
                    Action::SetInteractionType { entity_tag, operation, mask: 256 }
                    if entity_tag == "Preparation_Terminal" && operation == "~"
                )
        })
        .count();
    assert_eq!(
        clears, 1,
        "chain 1060 must clear the Livewire cue (mask 256, op '~') on \
         Preparation_Terminal at launch time; got {actions:?}",
    );
}

/// Chain 1061 (Livewire victory) must display the closing dialog, complete
/// 641 and accept 680, in that order. Ordering matters: `complete_mission`
/// before `accept_mission` is what keeps 680 from being accepted while 641
/// is still the active mission.
#[tokio::test]
async fn chain_1061_victory_completes_641_and_accepts_680() {
    let pool = require_db_or_skip!();
    let actions = victory_actions(&pool, 1061).await;

    assert_eq!(
        actions.len(),
        5,
        "chain 1061 must resolve exactly 5 actions (dialog + complete + \
         accept + 2 interaction bits); got {actions:?}",
    );
    assert!(
        matches!(&actions[0], Action::DisplayDialog { dialog_id: 3998 }),
        "chain 1061's first action must display dialog 3998; got {:?}",
        actions[0],
    );
    assert!(
        matches!(&actions[1], Action::CompleteMission { mission_id: 641 }),
        "chain 1061's second action must complete mission 641; got {:?}",
        actions[1],
    );
    assert!(
        matches!(&actions[2], Action::AcceptMission { mission_id: 680 }),
        "chain 1061's third action must accept mission 680 — after the \
         completion, not before; got {:?}",
        actions[2],
    );
    assert!(
        matches!(
            &actions[3],
            Action::SetInteractionType { entity_tag, operation, mask: 32 }
            if entity_tag == "Preparation_RingSwitch" && operation == "|"
        ),
        "chain 1061's fourth action must set the RingNetwork bit (mask 32) \
         on Preparation_RingSwitch; got {:?}",
        actions[3],
    );
    assert!(
        matches!(
            &actions[4],
            Action::SetInteractionType { entity_tag, operation, mask: 1073741824 }
            if entity_tag == "Preparation_RingSwitch" && operation == "|"
        ),
        "chain 1061's fifth action must set the quest-highlight bit \
         (mask 2^30) on Preparation_RingSwitch; got {:?}",
        actions[4],
    );
}
