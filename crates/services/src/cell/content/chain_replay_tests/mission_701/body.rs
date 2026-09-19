//! CA03 — mission 701's body: steps 2399 → 2400 → 2401 → 2421 and the
//! turn-in (chains 1231-1239).
//!
//! The deferred-escort test is the one test here that does not stop at
//! `resolve_event`: a resolve-only assertion cannot tell a honoured
//! `delay_ms` from one the executor silently ran inline, so
//! `chain_1235_escort_walk_defers_then_advances` pushes the resolved
//! actions through `executor::execute_actions` and drives the queue with
//! the same rewound-`fire_at` clock `executor/tests/deferred.rs` uses.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

use super::super::super::executor::execute_actions;
use super::{
    choice_at_step, delays, engine_for, fire, interact_at_step, interact_ctx, summarized,
    with_mission, with_step, CHOICE, INTERACT, NON_JAFFA,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const COPPLEMANN: &str = "Castle_Coppleman";
const PLAYER_EID: u32 = 7701;
const PLAYER_ID: i32 = 7702;
/// The authored escort delay: 63.23 units at the 6.0 u/s NPC speed.
const ESCORT_DELAY_MS: i32 = 10_500;

/// Chain 1231 — clicking Copplemann on the opening step plays 2574
/// ("I got caught by one of those drones...").
#[tokio::test]
async fn chain_1231_shows_dialog_2574_on_step_2399() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1231).await;

    let ctx = interact_at_step(COPPLEMANN, NON_JAFFA, 2399, "active");

    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1231),
        vec!["display_dialog(2574)"],
        "step 2399 + click Copplemann must show dialog 2574 and nothing else",
    );
}

/// Chain 1231 negative — no enemy wave, no kill gate, but the step gate
/// is real: the intro dialog must not replay once the player is past
/// 2399.
#[tokio::test]
async fn chain_1231_does_not_replay_after_2399() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1231).await;

    for step in [2400, 2401, 2421] {
        let mut ctx = interact_ctx(COPPLEMANN, NON_JAFFA);
        with_step(&mut ctx, 701, 2399, "completed");
        with_step(&mut ctx, 701, step, "active");

        assert!(
            summarized(&fire(&engine, INTERACT, &ctx), 1231).is_empty(),
            "chain 1231 must not resolve while the player is on step {step}",
        );
    }
}

/// Chain 1232 — the 2574 choice advances to step 2400 ("Free Capt.
/// Copplemann from her security boot").
#[tokio::test]
async fn chain_1232_advances_to_step_2400() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1232).await;

    let ctx = choice_at_step(2574, 2399, "active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1232),
        vec!["advance_step(701, 2400)"],
        "choosing on 2574 must advance 2399 → 2400",
    );
}

/// Chain 1232 negative — a replayed 2574 choice on step 2400 must not
/// advance again. `advance_step` is unconditional in the executor, so
/// the chain's own gate is the only thing stopping a re-advance from
/// rewinding the mission.
#[tokio::test]
async fn chain_1232_does_not_advance_twice() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1232).await;

    let ctx = choice_at_step(2574, 2400, "active");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1232).is_empty(),
        "chain 1232 must not re-advance once the player is on 2400",
    );
}

/// Chain 1233 — on step 2400, clicking Copplemann launches Livewire and
/// names chain 1234 as the victory callback.
///
/// The victory chain id is asserted literally: `fire_chain_by_id` looks
/// the chain up by this number, so a typo here is a mission that can be
/// won but never progresses, with only a `warn` to show for it.
#[tokio::test]
async fn chain_1233_launches_livewire_with_victory_chain_1234() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1233).await;

    let ctx = interact_at_step(COPPLEMANN, NON_JAFFA, 2400, "active");

    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1233),
        vec!["start_minigame(Livewire, victory=[1234])"],
        "step 2400 + click Copplemann must start Livewire with 1234 as the \
         victory chain",
    );
}

/// Chain 1233 negative — the launcher must not resolve on either
/// neighbouring step. This is the gate that keeps the player from
/// replaying Livewire after they have already won it (step 2401) or
/// skipping the intro dialog entirely (step 2399), and it is the ONLY
/// gate: `fire_chain_by_id` evaluates no conditions on the victory
/// chain, so anything the launcher lets through wins the mission step.
#[tokio::test]
async fn chain_1233_does_not_launch_on_step_2399_or_2401() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1233).await;

    for step in [2399, 2401, 2421] {
        let ctx = interact_at_step(COPPLEMANN, NON_JAFFA, step, "active");

        assert!(
            summarized(&fire(&engine, INTERACT, &ctx), 1233).is_empty(),
            "the Livewire launcher must not resolve on step {step}",
        );
    }
}

/// Chain 1234 — the victory callback. Loaded and read the same way
/// `fire_chain_by_id` reads it, through `get_chain_actions`, because the
/// chain has no trigger row of its own (the loader gives a triggerless
/// chain an inert synthetic `OnCustomEvent`, so `resolve_event` can
/// never reach it).
#[tokio::test]
async fn chain_1234_victory_unbinds_shows_2575_and_advances_to_2401() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1234).await;

    let resolved: Vec<String> = engine
        .get_chain_actions(1234)
        .iter()
        .map(|(action, _delay)| super::summarize(action))
        .collect();

    assert_eq!(
        resolved,
        vec![
            "remove_dialog_set(3062, slot=48)",
            "display_dialog(2575)",
            "advance_step(701, 2401)",
        ],
        "the Livewire victory must unbind the in-progress topic, play 2575, \
         then advance to 2401 — in that order",
    );

    assert!(
        engine
            .get_chain_actions(1234)
            .iter()
            .all(|(_, delay)| *delay == 0),
        "no action on the victory chain may be deferred — the player is \
         looking at the minigame result when it fires",
    );
}

/// Executor-level guard for the victory dialog's speaker: dialog 2575
/// must reach the client bound to the **player**, not to Copplemann.
///
/// 2575 is one screen with `speaker_id = 0` — narration, not speech
/// ("You manage to free Capt. Copplemann from her security boot.") — and
/// `Castle.py` displays it with an explicit `displayDialog(None, 2575)`.
///
/// The trap this guards: `last_interaction_target` is a sticky pin, and
/// reaching the victory chain always means the player just clicked
/// Copplemann to launch the Livewire. So an NPC is always resolvable
/// here. Only because `executor/dialog.rs` consults the monologue cache
/// *before* the pin does the player win. Reordering those two branches
/// makes Copplemann appear to speak the player's own narration, and a
/// resolve-only test cannot see it — the action list is identical either
/// way.
#[tokio::test]
async fn chain_1234_victory_dialog_binds_the_player_not_copplemann() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1234).await;

    let mut mgr = make_space_mgr();
    // Reproduce the state chain 1233 leaves behind: the player clicked
    // Copplemann, so the pin holds an NPC.
    const COPPLEMANN_EID: u32 = 7788;
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.last_interaction_target = Some(COPPLEMANN_EID);
    }
    // 2575 is a monologue in production; mirror that here.
    mgr.monologue_dialog_ids.insert(2575);

    // Fire the victory chain exactly as the minigame callback does:
    // by id, with empty params.
    let actions = engine.get_chain_actions(1234);
    let (actions, action_delays): (Vec<_>, Vec<_>) =
        actions.into_iter().map(|(a, d)| ((1234i64, a), d)).unzip();
    let resolved = cimmeria_content_engine::chain::ResolvedActions {
        actions,
        action_delays,
        ..Default::default()
    };

    let (tx, mut rx) = mpsc::channel(32);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mut dialog_frames = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_DIALOG_DISPLAY {
                let wire_entity = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let wire_dialog = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                dialog_frames.push((wire_entity, wire_dialog));
            }
        }
    }

    assert_eq!(
        dialog_frames,
        vec![(PLAYER_EID as i32, 2575)],
        "the Livewire victory must display 2575 bound to the PLAYER \
         ({PLAYER_EID}), not to the pinned Copplemann ({COPPLEMANN_EID}) — \
         2575 is player narration and Castle.py passes a NULL speaker",
    );
}

/// Chain 1235 — the escort. Both actions must be deferred by the
/// authored 10.5 s, and the resolved list must be exactly the advance
/// plus the turn-in bind.
#[tokio::test]
async fn chain_1235_defers_both_escort_actions() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1235).await;

    let ctx = choice_at_step(2575, 2401, "active");
    let resolved = fire(&engine, CHOICE, &ctx);

    assert_eq!(
        summarized(&resolved, 1235),
        vec!["advance_step(701, 2421)", "add_dialog_set(3063, slot=48)"],
        "the escort must advance to 2421 and bind the turn-in topic (3063, \
         not the in-progress 3062)",
    );
    assert_eq!(
        delays(&resolved, 1235),
        vec![ESCORT_DELAY_MS, ESCORT_DELAY_MS],
        "both escort actions must carry the authored delay — a zero here is \
         the walk collapsing to an instant teleport of mission state",
    );
}

/// Chain 1235 negative — the escort must not arm from a 2575 choice on
/// any step other than 2401. 2421 is the stale-replay case (the walk
/// already finished); 2399 and 2400 are the "somehow got the dialog
/// early" case, where arming would skip the Livewire entirely.
#[tokio::test]
async fn chain_1235_only_arms_on_step_2401() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1235).await;

    for step in [2399, 2400, 2421] {
        let ctx = choice_at_step(2575, step, "active");
        assert!(
            summarized(&fire(&engine, CHOICE, &ctx), 1235).is_empty(),
            "chain 1235 must not resolve while step {step} is active",
        );
    }

    // Positive control, so the loop above cannot pass with the chain
    // permanently disabled.
    let ok = choice_at_step(2575, 2401, "active");
    assert!(
        !summarized(&fire(&engine, CHOICE, &ok), 1235).is_empty(),
        "sanity: chain 1235 must still resolve on step 2401",
    );
}

/// `onStepUpdate` — `cell::missions::ON_STEP_UPDATE`, duplicated here
/// because that constant is `pub(super)` to the missions module.
const ON_STEP_UPDATE: u16 = 81;

/// Render one cell→base message compactly so the deferred test can assert
/// the whole ordered list instead of filtering to the one variant it
/// expects. A filtered assertion cannot notice a *missing* action.
fn describe(msg: &CellToBaseMsg) -> String {
    match msg {
        CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id,
            current_step_id,
            ..
        } => format!(
            "MissionUpdate(player={player_id}, mission={mission_id}, step={current_step_id:?})"
        ),
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } if *method_index == ON_STEP_UPDATE && args.len() >= 5 => {
            let step = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            format!("onStepUpdate(step={step}, status={})", args[4])
        }
        CellToBaseMsg::EntityMethodCall { method_index, .. } => {
            format!("EntityMethodCall(method={method_index})")
        }
        CellToBaseMsg::WitnessEntityMethod {
            entity_id,
            method_index,
            ..
        } => format!("WitnessEntityMethod(entity={entity_id}, method={method_index})"),
        other => format!("{other:?}"),
    }
}

/// A Castle space holding one connected player who is mid-701 on step
/// 2401 — the state a player is in when they choose on dialog 2575.
///
/// The mission instance is real rather than absent on purpose: with no
/// mission tracked, `cell::missions::advance_step` bails at its
/// "mission not found" warn and the only thing left to observe is the
/// executor's own `MissionUpdate`. Carrying the instance means the drain
/// also has to move the in-memory `current_step_id`, which is the
/// assertion that actually proves the player advanced.
fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-1200" MaxX="1200" MinY="-1200" MaxY="1200" /></Spaces>"#;
    let startup = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).expect("spaces xml must parse");
    mgr.create_startup_spaces(startup)
        .expect("Castle startup space must be created");
    mgr.create_entity(PLAYER_EID, "Castle", [0.0; 3], [0.0; 3])
        .expect("Castle space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    // Mission 701 active on step 2401 (the escort), with 2401's single
    // objective active.
    p.missions.add_mission(MissionInstance::new(
        701,
        2401,
        vec![MissionObjective {
            objective_id: 2401,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    ));
    // Seed the dialog-set cache the deferred `add_dialog_set 3063` needs.
    // A bare `SpaceManager` has an empty cache, so the action would take
    // the "dialog_set_maps cache miss" warn branch and no-op — and a test
    // that only watched for `MissionUpdate` would still pass with the
    // `add_dialog_set` row deleted from chain 1235 entirely. Values match
    // the seeded row: dialog 2576, flags 33554432
    // (`INT_AStoryMissionTurnIn`).
    mgr.dialog_set_maps.insert(
        3063,
        crate::cell::spawner::DialogSetMapEntry {
            // `Option<i32>` since CA02 widened the loader to keep
            // `dialog_id IS NULL` rows; row 3063 carries a real dialog.
            dialog_id: Some(2576),
            interaction_flags: 33_554_432,
        },
    );
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// Executor-level guard for the escort (TESTING.md type 6's
/// execute-the-actions extension).
///
/// Three things a resolve-only test cannot see:
///   1. `execute_actions` must NOT run the two actions inline — nothing
///      reaches base at click time.
///   2. A tick before the deadline must leave them queued. Without this,
///      a `delay_ms` that silently decayed to 0 would still "pass" a
///      drain test.
///   3. Once the deadline passes, the drain must fire them through the
///      real `execute_one`, producing the `MissionUpdate` that moves the
///      player to step 2421.
///
/// Time is driven by rewinding the queued entries' `fire_at`, the same
/// non-sleeping technique `executor/tests/deferred.rs` uses.
#[tokio::test]
async fn chain_1235_escort_walk_defers_then_advances_to_2421() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1235).await;

    let ctx = choice_at_step(2575, 2401, "active");
    let resolved = fire(&engine, CHOICE, &ctx);
    assert_eq!(
        resolved.actions.len(),
        2,
        "sanity: chain 1235 must resolve its two actions before we execute them",
    );

    let mut mgr = make_space_mgr();
    let (tx, mut rx) = mpsc::channel(32);
    let exec_engine = ChainEngine::new();

    assert_eq!(
        mgr.get_entity(PLAYER_EID)
            .and_then(|e| e.missions.get_mission(701))
            .and_then(|m| m.current_step_id),
        Some(2401),
        "fixture sanity: the player must start the escort on step 2401",
    );

    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    assert!(
        rx.try_recv().is_err(),
        "the escort actions must not reach base at choice time — the player \
         is supposed to walk first",
    );
    assert_eq!(
        mgr.get_entity(PLAYER_EID)
            .and_then(|e| e.missions.get_mission(701))
            .and_then(|m| m.current_step_id),
        Some(2401),
        "the player must still be on step 2401 at choice time — a deferred \
         advance that ran inline is the bug this guards",
    );
    assert_eq!(
        mgr.pending_content_actions
            .get(&PLAYER_EID)
            .map(|q| q.len()),
        Some(2),
        "both escort actions must be queued against the player entity",
    );

    // A tick well before the deadline fires nothing.
    crate::cell::content::deferred_content_action_tick(&tx, &mut mgr, &exec_engine).await;
    assert_eq!(
        mgr.pending_content_actions
            .get(&PLAYER_EID)
            .map(|q| q.len()),
        Some(2),
        "a tick inside the 10.5s window must leave the queue untouched",
    );
    assert!(
        rx.try_recv().is_err(),
        "nothing may reach base before the escort delay elapses",
    );

    // Advance the clock by rewinding the deadlines into the past.
    let past = Instant::now() - Duration::from_millis(1);
    for pending in mgr
        .pending_content_actions
        .get_mut(&PLAYER_EID)
        .expect("queue must still exist")
    {
        pending.fire_at = past;
    }

    crate::cell::content::deferred_content_action_tick(&tx, &mut mgr, &exec_engine).await;

    assert!(
        !mgr.pending_content_actions.contains_key(&PLAYER_EID),
        "the drain must empty the queue once both deadlines have passed",
    );

    // Assert the FULL ordered message list, not a filtered view. Filtering
    // to `MissionUpdate` would let the second deferred action
    // (`add_dialog_set 3063`) be deleted from chain 1235 without failing
    // anything.
    let mut drained = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        drained.push(describe(&msg));
    }
    assert_eq!(
        drained,
        vec![
            // `cell::missions::advance_step` closes the old step, opens the
            // new one, then emits one objective frame per objective of the
            // new step (none here — a bare SpaceManager has no step-objective
            // cache, and 2421's objectives are not what this test is about).
            "onStepUpdate(step=2401, status=1)".to_string(),
            "onStepUpdate(step=2421, status=0)".to_string(),
            // Then the executor's own persist message.
            "MissionUpdate(player=7702, mission=701, step=Some(2421))".to_string(),
        ],
        "draining the escort must emit exactly the advance's traffic in \
         order. Zero MissionUpdate means the AdvanceStep arm never ran; a \
         WitnessEntityMethod appearing here would mean the fixture grew an \
         NPC on template 48 and the interaction push is now in scope too.",
    );

    assert_eq!(
        mgr.get_entity(PLAYER_EID)
            .and_then(|e| e.missions.get_mission(701))
            .and_then(|m| m.current_step_id),
        Some(2421),
        "the drain must move the player's in-memory mission to step 2421, \
         not merely emit a persist message",
    );

    // The second deferred action: the turn-in topic must now be bound to
    // Copplemann's template (48) on this player, carrying row 3063's
    // dialog and flags. This is what a `MissionUpdate`-only assertion
    // could not see.
    let bound = mgr
        .get_entity(PLAYER_EID)
        .and_then(|e| e.available_interactions.get(&48).cloned())
        .unwrap_or_default();
    assert_eq!(
        bound,
        vec![(3063, Some(2576), 33_554_432i64)],
        "the drain must bind dialog-set map 3063 (dialog 2576, \
         INT_AStoryMissionTurnIn) to template 48 — deleting the \
         `add_dialog_set` action from chain 1235 fails here",
    );
}

/// Chain 1236 — the turn-in topic. This chain is what replaces
/// `Castle.py`'s `dialog_set.open 3062` node, which has no dispatch site
/// in services and has never fired (D-CA04).
#[tokio::test]
async fn chain_1236_shows_turn_in_dialog_2576_on_step_2421() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1236).await;

    let ctx = interact_at_step(COPPLEMANN, NON_JAFFA, 2421, "active");

    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1236),
        vec!["display_dialog(2576)"],
        "step 2421 + click Copplemann must show the turn-in dialog 2576",
    );
}

/// Chain 1236 negative — the turn-in dialog must not be reachable on any
/// earlier step. On 2401 it would let the player take 702/703 without
/// finishing the walk; on 2399/2400 it would skip the whole mission body.
#[tokio::test]
async fn chain_1236_does_not_show_turn_in_before_step_2421() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1236).await;

    for step in [2399, 2400, 2401] {
        let ctx = interact_at_step(COPPLEMANN, NON_JAFFA, step, "active");
        assert!(
            summarized(&fire(&engine, INTERACT, &ctx), 1236).is_empty(),
            "chain 1236 must not resolve while step {step} is active",
        );
    }
}

/// Chain 1237 — the turn-in itself: drop the "?" topic, complete 701.
#[tokio::test]
async fn chain_1237_completes_701_and_clears_the_turn_in_topic() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1237).await;

    let ctx = choice_at_step(2576, 2421, "active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1237),
        vec!["remove_dialog_set(3063, slot=48)", "complete_mission(701)"],
        "the turn-in must unbind 3063 (the row chain 1235 bound) and then \
         complete 701, in that order",
    );
}

/// Chain 1237 negative — "exactly once". `MissionInstance::complete()`
/// moves 2421 out of `current_step_id` into `completed_steps`, so the
/// second choice sees `completed` and must resolve nothing. Without the
/// step gate the player could re-complete 701 and re-take 702/703.
#[tokio::test]
async fn chain_1237_completes_701_exactly_once() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1237).await;

    let mut ctx = choice_at_step(2576, 2421, "completed");
    with_mission(&mut ctx, 701, "completed");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1237).is_empty(),
        "a second choice on 2576 after 701 completed must resolve nothing",
    );
}

/// Chain 1238 — accept 702 "Rescue Dr. Zuritska" on the same choice.
#[tokio::test]
async fn chain_1238_accepts_702() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1238).await;

    let mut ctx = choice_at_step(2576, 2421, "active");
    with_mission(&mut ctx, 702, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1238),
        vec!["accept_mission(702)"],
        "the turn-in must accept 702",
    );
}

/// Chain 1238 negative — the mandatory `mission_status eq not_active`
/// gate on every `accept_mission` chain. Splitting the turn-in into
/// three chains is what lets 702's gate fail without also blocking
/// 701's completion, so this test is paired with
/// `turn_in_completes_701_even_when_702_is_already_active` below.
#[tokio::test]
async fn chain_1238_does_not_re_accept_an_active_702() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1238).await;

    let mut ctx = choice_at_step(2576, 2421, "active");
    with_mission(&mut ctx, 702, "active");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1238).is_empty(),
        "chain 1238 must not re-accept 702 when it is already active",
    );
}

/// Chain 1239 — accept 703 "Payback". This is the RECONSTRUCTION arm
/// (D-CA05): `Castle.py` accepts only 702, and the evidence for 703 is
/// dialog 2576's plural "Take Missions" button plus dialog 2577
/// assuming 703 is live. Held in its own chain so it can be withdrawn
/// with one `enabled = false`.
#[tokio::test]
async fn chain_1239_accepts_703() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1239).await;

    let mut ctx = choice_at_step(2576, 2421, "active");
    with_mission(&mut ctx, 703, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1239),
        vec!["accept_mission(703)"],
        "the turn-in must also accept 703 per D-CA05",
    );
}

/// Chain 1239 negative — same not-active gate.
#[tokio::test]
async fn chain_1239_does_not_re_accept_an_active_703() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1239).await;

    let mut ctx = choice_at_step(2576, 2421, "active");
    with_mission(&mut ctx, 703, "active");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1239).is_empty(),
        "chain 1239 must not re-accept 703 when it is already active",
    );
}

/// Both accept chains carry TWO gates: the step and the target mission's
/// `not_active`. The tests above exercise only the mission half, so a
/// dropped `step_status` row would go unnoticed and a stray 2576 choice
/// at any point in the mission would hand out 702/703 early.
///
/// Each case here leaves the mission `not_active` — so the mission gate
/// passes — and varies only the step.
#[tokio::test]
async fn accept_chains_also_require_step_2421() {
    let pool = require_db_or_skip!();

    for (chain_id, mission) in [(1238, 702), (1239, 703)] {
        let engine = engine_for(&pool, chain_id).await;

        for step in [2399, 2400, 2401] {
            let mut ctx = choice_at_step(2576, step, "active");
            with_mission(&mut ctx, mission, "not_active");
            assert!(
                summarized(&fire(&engine, CHOICE, &ctx), chain_id as i64).is_empty(),
                "chain {chain_id} must not accept {mission} while step {step} \
                 is active — the step gate is the other half of its guard",
            );
        }

        // And the already-completed turn-in step: a replayed 2576 choice
        // after 701 finished must not re-offer the follow-ups.
        let mut done = choice_at_step(2576, 2421, "completed");
        with_mission(&mut done, 701, "completed");
        with_mission(&mut done, mission, "not_active");
        assert!(
            summarized(&fire(&engine, CHOICE, &done), chain_id as i64).is_empty(),
            "chain {chain_id} must not accept {mission} once step 2421 is \
             completed",
        );

        // Positive control.
        let mut ok = choice_at_step(2576, 2421, "active");
        with_mission(&mut ok, mission, "not_active");
        assert!(
            !summarized(&fire(&engine, CHOICE, &ok), chain_id as i64).is_empty(),
            "sanity: chain {chain_id} must still accept {mission} on step 2421",
        );
    }
}

/// The reason the turn-in is three chains and not one: a player who
/// somehow already holds 702 (a GM grant, a future alternate route) must
/// still be able to finish 701. Folding the accepts into chain 1237
/// would make 702's `not_active` gate block the completion too, which is
/// a permanent soft-stuck on the last step of the mission.
#[tokio::test]
async fn turn_in_completes_701_even_when_702_is_already_active() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1237).await;

    let mut ctx = choice_at_step(2576, 2421, "active");
    with_mission(&mut ctx, 702, "active");
    with_mission(&mut ctx, 703, "active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1237),
        vec!["remove_dialog_set(3063, slot=48)", "complete_mission(701)"],
        "701's completion must not depend on 702/703 state",
    );
}

/// Tag guard for the Copplemann chains. The spawnlist tag is
/// `Castle_Coppleman` with a single 'n' even though the character is
/// named "Copplemann"; a trigger spelled the prose way never fires.
#[tokio::test]
async fn copplemann_chains_are_keyed_to_the_spawnlist_tag() {
    let pool = require_db_or_skip!();

    for (chain_id, step) in [(1231, 2399), (1233, 2400), (1236, 2421)] {
        let engine = engine_for(&pool, chain_id).await;

        let wrong = interact_at_step("Castle_Copplemann", NON_JAFFA, step, "active");
        assert!(
            summarized(&fire(&engine, INTERACT, &wrong), chain_id as i64).is_empty(),
            "chain {chain_id} must not match the double-n spelling",
        );

        let right = interact_at_step(COPPLEMANN, NON_JAFFA, step, "active");
        assert!(
            !summarized(&fire(&engine, INTERACT, &right), chain_id as i64).is_empty(),
            "sanity: chain {chain_id} must match the real spawnlist tag",
        );
    }
}
