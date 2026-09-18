//! Steps 2418 / 4462 / 4469 — repair the DHD, dial Harset, walk
//! through. Chains 1356-1360.
//!
//! The last three steps are where mission 708 hands off to the engine:
//! the minigame session (2418), CA10's `stargate_dialed` (4462) and
//! CA10's `stargate_crossed` (4469). Two properties are worth a guard
//! each:
//!
//! - The Livewire launcher must be reachable ONLY on step 2418. The DHD
//!   is a permanent world object every player walks up to, so an
//!   ungated launcher would offer a hack to anyone.
//! - Dialling and crossing must stay distinct. The two triggers carry
//!   separate `TriggerType` discriminants precisely so 708 cannot
//!   complete the moment the gate opens, before the player has stepped
//!   through — see `super::super::stargate_triggers`, which pins the
//!   loader and matcher halves against a sentinel chain.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;
use tokio::sync::mpsc;

use super::super::super::executor::execute_actions;
use super::{
    actions_of, assert_no_deferred_actions, count_flag_ops, engine_for, fire,
    make_castle_space_mgr, step_ctx, JAFFA, LIVEWIRE, TAURI,
};
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7185;
const PLAYER_ID: i32 = 7186;

/// The destination world name CA10 puts in `destination_world`. A real
/// `resources.worlds.world` value, and the `event_key` both stargate
/// chains are seeded with.
const HARSET: &str = "Harset";

/// Context for a stargate event, matching what
/// `event_dispatch/stargate.rs` populates: destination world, origin
/// world, mission state and archetype.
fn gate_ctx(destination: &str, step_id: i32, archetype: i32) -> ExecutionContext {
    let mut ctx = step_ctx(step_id, archetype);
    ctx.set_param(
        "destination_world".to_string(),
        serde_json::json!(destination),
    );
    ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
    ctx
}

/// Chain 1356, executed: interacting with the DHD on step 2418 must
/// reach base as a `StartMinigame` naming Livewire and chain 1357 as its
/// victory chain.
///
/// Pushed through `execute_actions` rather than stopping at resolve
/// because `Action::StartMinigame`'s failure mode is a dropped executor
/// arm, not a mis-authored condition: a resolve-only test cannot tell a
/// wired arm from `execute_one`'s `other =>` catch-all, and a chain that
/// resolves but never launches leaves the player stuck on 2418 with a
/// lit DHD that does nothing.
#[tokio::test]
async fn chain_1356_launches_livewire_with_1357_as_its_victory_chain() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1356).await;

    let mut ctx = step_ctx(2418, TAURI);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("Castle_DHD"));

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 1356);
    let actions = actions_of(&resolved, 1356);
    assert_eq!(
        actions.len(),
        1,
        "chain 1356 must resolve exactly one action; got {actions:?}",
    );
    match actions[0] {
        Action::StartMinigame {
            minigame_type,
            on_victory_chains,
        } => {
            assert_eq!(
                minigame_type, "Livewire",
                "chain 1356 must launch Livewire (provisional per D-CA09)",
            );
            assert_eq!(
                on_victory_chains.as_slice(),
                &[1357_i64],
                "chain 1356's victory chain must be 1357 — victory chains are \
                 fired by id with no condition evaluation, so a wrong id here \
                 silently strands the mission at step 2418",
            );
        }
        other => panic!("chain 1356 must resolve a StartMinigame; got {other:?}"),
    }

    let mut mgr = make_castle_space_mgr();
    mgr.create_entity(PLAYER_EID, "Castle", [806.0, 55.0, 517.0], [0.0; 3])
        .expect("Castle startup space must accept the player entity");
    {
        let p = mgr
            .get_entity_mut(PLAYER_EID)
            .expect("player entity must exist immediately after create_entity");
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
    }
    mgr.connect_entity(PLAYER_EID);

    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mut launches = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::StartMinigame {
            entity_id,
            player_id,
            game_name,
            on_victory_chains,
            ..
        } = msg
        {
            launches.push((entity_id, player_id, game_name, on_victory_chains));
        }
    }
    assert_eq!(
        launches.len(),
        1,
        "chain 1356 must produce exactly one CellToBaseMsg::StartMinigame; zero \
         means the executor arm was lost and the action fell through the \
         `other =>` catch-all; got {launches:?}",
    );
    assert_eq!(
        launches[0],
        (
            PLAYER_EID,
            PLAYER_ID,
            "Livewire".to_string(),
            vec![1357_i64]
        ),
        "the launch must name the firing player, the Livewire game and chain \
         1357 — the victory chain is fired by id with no condition evaluation, \
         so a wrong id here silently strands the mission at step 2418",
    );
}

/// The Livewire launcher must be unreachable on every other step. Step
/// 4462 matters most: the DHD is the dial device, and a launcher that
/// still answered there would open a minigame instead of the gate UI.
#[tokio::test]
async fn the_dhd_launcher_only_answers_on_step_2418() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1356).await;

    for step in [2415, 2416, 2417, 4462, 4469] {
        let mut ctx = step_ctx(step, TAURI);
        ctx.set_param("entity_tag".to_string(), serde_json::json!("Castle_DHD"));
        assert!(
            actions_of(&fire(&engine, TriggerType::InteractTag, &ctx), 1356).is_empty(),
            "chain 1356 must not offer Livewire at step {step}",
        );
    }

    let mut never = ExecutionContext::new();
    never.set_param("entity_tag".to_string(), serde_json::json!("Castle_DHD"));
    never.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("not_active"),
    );
    assert!(
        actions_of(&fire(&engine, TriggerType::InteractTag, &never), 1356).is_empty(),
        "a player who never accepted 708 must not be offered the DHD hack",
    );
}

/// Chain 1357 is the victory half. It has no trigger row and no
/// conditions on purpose: minigame victories are fired by id with
/// `ResolvedActions::default()` and NO condition evaluation
/// (`event_dispatch/mod.rs:53-82`), so any gate must live on the
/// launcher. Fired the same way the minigame callback does.
#[tokio::test]
async fn chain_1357_advances_to_4462_clears_livewire_and_binds_the_dial_topic() {
    let pool = require_db_or_skip!();
    let chain = super::super::super::engine_loader::load_single_chain_for_test(&pool, 1357)
        .await
        .expect("DB query for chain 1357 must succeed")
        .expect("chain 1357 must exist in seeded content_chains");

    assert!(
        chain.conditions.is_empty(),
        "chain 1357 is a minigame victory chain — conditions on it are NEVER \
         evaluated, so a condition row here is dead weight that reads like a \
         guard; got {:?}",
        chain.conditions,
    );

    let actions: Vec<&Action> = chain.actions.iter().collect();
    assert_eq!(
        actions.len(),
        3,
        "chain 1357 must resolve three actions (advance, clear Livewire, bind \
         the dial topic); got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 708,
                step_id: 4462
            }
        ),
        "chain 1357 must advance to step 4462; got {:?}",
        actions[0],
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_DHD", "~", LIVEWIRE),
        1,
        "chain 1357 must clear the DHD's Livewire bit — leaving it set would \
         offer the hack to every player in the zone forever. The DHD stays \
         DIALABLE regardless: template 162 ships interaction_type = 16 \
         (INT_Dhd). Got {actions:?}",
    );
    assert!(
        matches!(
            actions[2],
            Action::AddDialogSet {
                dialog_set_id: 3073,
                slot: 162,
                ..
            }
        ),
        "chain 1357 must bind dialog_set_map row 3073 ('Dial Harset') to \
         template 162. Inert until packet CA02 widens DialogSetMapEntry.dialog_id \
         to Option<i32> — `spawner/dialogs.rs:45` drops NULL-dialog rows at \
         load — but the seed row must be right before then. Got {:?}",
        actions[2],
    );
}

/// Chains 1358/1359: dialling Harset on step 4462 advances to 4469 and
/// plays the faction's closing line. `stargate_dialed` DOES populate
/// `archetype` (`event_dispatch/stargate.rs:96-101`), so unlike the
/// dialog halves at step 2417 the split here is a real gate.
#[tokio::test]
async fn dialling_harset_advances_to_4469_with_the_right_closing_line() {
    let pool = require_db_or_skip!();

    for (chain_id, archetype, dialog_id) in [(1358, TAURI, 5010), (1359, JAFFA, 5011)] {
        let engine = engine_for(&pool, chain_id).await;
        let ctx = gate_ctx(HARSET, 4462, archetype);

        let resolved = fire(&engine, TriggerType::StargateDialed, &ctx);
        assert_no_deferred_actions(&resolved, chain_id as i64);
        let actions = actions_of(&resolved, chain_id as i64);

        assert_eq!(
            actions.len(),
            3,
            "chain {chain_id} must resolve three actions (advance, dialog, \
             unbind the dial topic); got {actions:?}",
        );
        assert!(
            matches!(
                actions[0],
                Action::AdvanceStep {
                    mission_id: 708,
                    step_id: 4469
                }
            ),
            "chain {chain_id} must advance to step 4469; got {:?}",
            actions[0],
        );
        assert!(
            matches!(actions[1], Action::DisplayDialog { dialog_id: d } if *d == dialog_id),
            "chain {chain_id} must display dialog {dialog_id}; got {:?}",
            actions[1],
        );
        assert!(
            matches!(
                actions[2],
                Action::RemoveDialogSet {
                    dialog_set_id: 3073,
                    slot: 162
                }
            ),
            "chain {chain_id} must unbind the 'Dial Harset' topic it no longer \
             needs; got {:?}",
            actions[2],
        );
    }
}

/// Cross-archetype and cross-destination negatives for the dial. The
/// destination key is what stops 708 advancing when the player dials
/// some other world out of curiosity; the archetype gate is what stops a
/// Jaffa getting Col. Marsh's send-off.
#[tokio::test]
async fn the_dial_chains_reject_the_wrong_archetype_and_the_wrong_destination() {
    let pool = require_db_or_skip!();
    let tauri_chain = engine_for(&pool, 1358).await;
    let jaffa_chain = engine_for(&pool, 1359).await;

    assert!(
        actions_of(
            &fire(
                &tauri_chain,
                TriggerType::StargateDialed,
                &gate_ctx(HARSET, 4462, JAFFA)
            ),
            1358
        )
        .is_empty(),
        "a Jaffa dialling Harset must not get the Tau'ri closing line",
    );
    assert!(
        actions_of(
            &fire(
                &jaffa_chain,
                TriggerType::StargateDialed,
                &gate_ctx(HARSET, 4462, TAURI)
            ),
            1359
        )
        .is_empty(),
        "a Tau'ri dialling Harset must not get the Jaffa closing line",
    );

    for (chain_id, engine, archetype) in [(1358, &tauri_chain, TAURI), (1359, &jaffa_chain, JAFFA)]
    {
        assert!(
            actions_of(
                &fire(
                    engine,
                    TriggerType::StargateDialed,
                    &gate_ctx("Castle", 4462, archetype)
                ),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} is keyed on Harset; dialling anywhere else must not \
             advance mission 708",
        );
        for step in [2418, 4469] {
            assert!(
                actions_of(
                    &fire(
                        engine,
                        TriggerType::StargateDialed,
                        &gate_ctx(HARSET, step, archetype)
                    ),
                    chain_id as i64
                )
                .is_empty(),
                "chain {chain_id} must not resolve at step {step}",
            );
        }
    }
}

/// Chain 1360: mission 708 completes when the player steps THROUGH the
/// gate, not when it opens.
///
/// The cross-trigger negative is the one that matters. Dialling arms a
/// four-second timer and opens the gate; a player who dials and walks
/// away has not left the Castle. If `stargate_dialed` also satisfied
/// this chain, 708 would complete at step 4462 and the final objective
/// would never be seen.
#[tokio::test]
async fn chain_1360_completes_708_on_crossing_and_never_on_dialling() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1360).await;

    let ctx = gate_ctx(HARSET, 4469, TAURI);
    let resolved = fire(&engine, TriggerType::StargateCrossed, &ctx);
    assert_no_deferred_actions(&resolved, 1360);
    let actions = actions_of(&resolved, 1360);
    assert_eq!(
        actions.len(),
        1,
        "chain 1360 must resolve exactly one action; got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::CompleteMission { mission_id: 708 }),
        "chain 1360 must complete mission 708; got {:?}",
        actions[0],
    );

    assert!(
        actions_of(&fire(&engine, TriggerType::StargateDialed, &ctx), 1360).is_empty(),
        "chain 1360 must NOT answer `stargate_dialed` — dialling opens the gate, \
         crossing leaves the Castle, and completing 708 on the dial would skip \
         the mission's last objective",
    );

    // Both the step gate and the destination key must hold.
    assert!(
        actions_of(
            &fire(
                &engine,
                TriggerType::StargateCrossed,
                &gate_ctx(HARSET, 4462, TAURI)
            ),
            1360
        )
        .is_empty(),
        "chain 1360 must not complete 708 while the player is still on step 4462",
    );
    assert!(
        actions_of(
            &fire(
                &engine,
                TriggerType::StargateCrossed,
                &gate_ctx("Castle", 4469, TAURI)
            ),
            1360
        )
        .is_empty(),
        "chain 1360 is keyed on Harset; crossing to any other world must not \
         complete mission 708",
    );
}
