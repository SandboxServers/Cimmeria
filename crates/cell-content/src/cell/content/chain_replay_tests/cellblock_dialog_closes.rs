//! The five Castle_CellBlock `dialog_choice` chains, fired from the
//! zero-button CLOSE path instead of a button click.
//!
//! Packet DU-02a strips every button from dialogs 2299, 4001, 5022, 3999
//! and 5023 (`base/dialog_overrides/patches_cellblock.rs`, plus the
//! matching deletion in `dialog_screen_buttons.sql`). Before the strip a
//! player committed by clicking Accept or "Receive Item" and the client
//! put that button's cooked id on the wire — 8 or 70. After it, the only
//! way out of the window is a close, and the client sends
//! `dialogButtonChoice(id, -1)` because the dialog's total button count is
//! zero (client contract F8).
//!
//! The whole packet rests on those two producing identical server
//! behaviour, which they do because `Trigger::OnDialogChoice` compares
//! `dialog_id` and nothing else (`triggers/matching.rs`) and no condition
//! type can read `button_id` (F9). "Rests on" is why it is tested rather
//! than asserted: if DU-06 lands an authorable `button_id` condition and
//! anyone puts one on these five chains, the close path silently stops
//! resolving and five Cellblock progression gates die with no error on any
//! layer.
//!
//! Each chain therefore gets the same pair:
//!
//! * fire with `button_id = -1` and assert the exact action list the seed
//!   carries — the post-strip path resolves what it is supposed to;
//! * fire with the button id the dialog used to send and assert the two
//!   resolve identically — the equivalence the strip depends on.
//!
//! What this file does NOT prove: that the dialogs are actually
//! button-less. No chain-replay context can see a button, because the
//! decision to send `-1` is made inside the 2009 client. That side is
//! guarded by the patch-versus-seed agreement tests in
//! `base/dialog_overrides/patches_cellblock.rs` and by the R1 rule of
//! `crates/content-engine/tests/it/dialog_button_linter/mod.rs`.
//!
//! The existing per-mission modules (`mission_638`, `mission_640`,
//! `mission_641`) are deliberately untouched: they cover the interact and
//! pickup halves of the same missions, and DU-02a changing nothing about
//! them is part of the claim.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;
use crate::test_support::require_db_or_skip;

/// The cooked `ButtonID` each dialog put on the wire before DU-02a
/// (F14: 8 = Accept, 70 = Receive Item), and the sentinel a close sends
/// now that the button is gone.
const CLOSE: i32 = -1;

/// Build the context `fire_dialog_choice` really populates.
///
/// `event_dispatch/dialog.rs` sets `dialog_id`, `button_id`, world
/// context and mission context — and **no `archetype`**. Reproducing that
/// omission matters here: an archetype condition on one of these chains
/// would evaluate against a missing key rather than a real value, so a
/// fixture that helpfully supplied one would hide it.
fn choice_ctx(dialog_id: i32, button_id: i32, mission_state: &[(&str, &str)]) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx.set_param("button_id".to_string(), serde_json::json!(button_id));
    for (key, value) in mission_state {
        ctx.set_param((*key).to_string(), serde_json::json!(*value));
    }
    ctx
}

fn fire(engine: &ChainEngine, ctx: &ExecutionContext) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
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
        .map(|(_, action)| action.clone())
        .collect()
}

fn flag(tag: &str, operation: &str, mask: i64) -> Action {
    Action::SetInteractionType {
        entity_tag: tag.to_string(),
        operation: operation.to_string(),
        mask,
    }
}

/// Load one chain, fire the close and then the old button click, and
/// assert the close resolves `expected` and the click resolves the same.
///
/// `mission_state` is whatever the chain's conditions need; an empty
/// slice means the chain is unconditional, which four of these five are.
async fn assert_close_matches_click(
    chain_id: i64,
    dialog_id: i32,
    old_button_id: i32,
    mission_state: &[(&str, &str)],
    expected: Vec<Action>,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!("chain {chain_id} must exist in seeded content_chains and load cleanly")
        });

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let close_ctx = choice_ctx(dialog_id, CLOSE, mission_state);
    let closed = fire(&engine, &close_ctx);
    let close_actions = actions_of(&closed, chain_id);

    assert_eq!(
        close_actions, expected,
        "chain {chain_id}: closing the now-button-less dialog {dialog_id} sends \
         dialogButtonChoice({dialog_id}, -1), which must resolve exactly the actions the \
         seed carries. Got {close_actions:?}",
    );
    assert_no_deferred_actions(&closed, chain_id);

    let click_ctx = choice_ctx(dialog_id, old_button_id, mission_state);
    let click_actions = actions_of(&fire(&engine, &click_ctx), chain_id);

    assert_eq!(
        close_actions, click_actions,
        "chain {chain_id}: the close (button_id -1) and the button click (button_id \
         {old_button_id}) resolved different actions. They must not: dialog_choice matches \
         on dialog id alone and no condition can read button_id, which is the only reason \
         DU-02a can strip dialog {dialog_id}'s buttons without rewriting the chain. If a \
         button_id condition has since been added, this chain needs one that accepts -1.",
    );
}

/// A wrong dialog id must resolve nothing.
///
/// Every assertion above is "these actions came back". A fixture whose
/// trigger matched everything would satisfy them all, so one negative per
/// chain pins that the match really is on `dialog_id`.
async fn assert_other_dialog_resolves_nothing(chain_id: i64, mission_state: &[(&str, &str)]) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    // 2298 is a real Cellblock dialog this packet does not strip, and no
    // chain keys a `dialog_choice` on it.
    let ctx = choice_ctx(2298, CLOSE, mission_state);
    let actions = actions_of(&fire(&engine, &ctx), chain_id);

    assert!(
        actions.is_empty(),
        "chain {chain_id} answered a close of dialog 2298, which it does not key. The \
         trigger is matching something other than dialog_id; got {actions:?}",
    );
}

// ---------------------------------------------------------------------
// Mission 638 — "Agree to escape", Human branch (dialog 2299)
// ---------------------------------------------------------------------

/// Chain 1019 is the only one of the five that is conditional: it gates
/// on step 2116 being active, so the close has to arrive in that window.
#[tokio::test]
async fn chain_1019_fires_on_the_close_of_button_less_2299() {
    assert_close_matches_click(
        1019,
        2299,
        8, // Accept
        &[("mission_638_step_2116_status", "active")],
        vec![
            Action::DisplayDialog { dialog_id: 2298 },
            Action::AcceptMission { mission_id: 639 },
            Action::CompleteMission { mission_id: 638 },
            Action::RemoveDialogSet {
                dialog_set_id: 2794,
                slot: 17,
            },
        ],
    )
    .await;
}

#[tokio::test]
async fn chain_1019_ignores_a_close_of_a_dialog_it_does_not_key() {
    assert_other_dialog_resolves_nothing(1019, &[("mission_638_step_2116_status", "active")]).await;
}

/// The step gate still holds on the close path.
///
/// Chain 1019 completes 638 and accepts 639; re-firing it would re-accept
/// a mission the player has moved past. The gate that prevents it is
/// `step_status(638, 2116) = active`, which 1019's own `complete_mission`
/// invalidates — and which a `-1` close reaches no differently from a
/// click.
#[tokio::test]
async fn chain_1019_does_not_fire_once_step_2116_is_past() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1019)
        .await
        .expect("DB query for chain 1019 must succeed")
        .expect("chain 1019 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let ctx = choice_ctx(
        2299,
        CLOSE,
        &[("mission_638_step_2116_status", "completed")],
    );
    let actions = actions_of(&fire(&engine, &ctx), 1019);

    assert!(
        actions.is_empty(),
        "chain 1019 fired on a close of 2299 after step 2116 had completed — mission 639 \
         would be accepted twice; got {actions:?}",
    );
}

// ---------------------------------------------------------------------
// Mission 641 — Marsh's first briefing (dialogs 4001 and 5022)
// ---------------------------------------------------------------------

fn briefing_accept_actions() -> Vec<Action> {
    vec![
        Action::AcceptMission { mission_id: 641 },
        flag("Preparation_SMG1A", "|", 1073741824),
        flag("Preparation_ColMarsh", "~", 8388608),
    ]
}

#[tokio::test]
async fn chain_1053_fires_on_the_close_of_button_less_4001() {
    assert_close_matches_click(1053, 4001, 8, &[], briefing_accept_actions()).await;
}

#[tokio::test]
async fn chain_1053_ignores_a_close_of_a_dialog_it_does_not_key() {
    assert_other_dialog_resolves_nothing(1053, &[]).await;
}

#[tokio::test]
async fn chain_1054_fires_on_the_close_of_button_less_5022() {
    assert_close_matches_click(1054, 5022, 8, &[], briefing_accept_actions()).await;
}

#[tokio::test]
async fn chain_1054_ignores_a_close_of_a_dialog_it_does_not_key() {
    assert_other_dialog_resolves_nothing(1054, &[]).await;
}

// ---------------------------------------------------------------------
// Mission 641 — Marsh's second briefing (dialogs 3999 and 5023)
// ---------------------------------------------------------------------

fn second_briefing_actions() -> Vec<Action> {
    vec![
        Action::AdvanceStep {
            mission_id: 641,
            step_id: 3564,
        },
        flag("Preparation_ColMarsh", "~", 8388608),
        flag("Preparation_Terminal", "|", 256),
    ]
}

/// 3999 is the one that was broken rather than merely noisy.
///
/// Its "Receive Item" buttons stopped on screen 96258, two screens short
/// of the final 96260, so a player who paged to the end had no button and
/// Done sent nothing — chain 1058 never fired and step 3564 never opened.
/// With the buttons stripped, that same Done now sends `(3999, -1)` and
/// resolves the list below.
///
/// Note what is NOT in the list: nothing is granted. The button's label
/// was a lie in both shapes — item 21 comes from chain 1055's `add_item`
/// on the locker interact, long before 3999 is reachable.
#[tokio::test]
async fn chain_1058_fires_on_the_close_of_button_less_3999() {
    assert_close_matches_click(
        1058,
        3999,
        70, // Receive Item
        &[],
        second_briefing_actions(),
    )
    .await;
}

#[tokio::test]
async fn chain_1058_ignores_a_close_of_a_dialog_it_does_not_key() {
    assert_other_dialog_resolves_nothing(1058, &[]).await;
}

#[tokio::test]
async fn chain_1059_fires_on_the_close_of_button_less_5023() {
    assert_close_matches_click(1059, 5023, 70, &[], second_briefing_actions()).await;
}

#[tokio::test]
async fn chain_1059_ignores_a_close_of_a_dialog_it_does_not_key() {
    assert_other_dialog_resolves_nothing(1059, &[]).await;
}

/// Neither second-briefing chain grants an item.
///
/// DU-02a publishes the claim that "Receive Item" never received
/// anything. That claim is load-bearing for the strip — if either chain
/// did grant, removing the button would change what the player walks away
/// with. Pinned here rather than left to the prose so a future edit that
/// moves the grant onto the dialog trips a test.
#[tokio::test]
async fn neither_second_briefing_chain_grants_an_item() {
    for (chain_id, dialog_id) in [(1058_i64, 3999_i32), (1059, 5023)] {
        let pool = require_db_or_skip!();
        let chain = load_single_chain_for_test(&pool, chain_id as i32)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

        let mut engine = ChainEngine::new();
        engine.register_chain(chain);

        let ctx = choice_ctx(dialog_id, CLOSE, &[]);
        let actions = actions_of(&fire(&engine, &ctx), chain_id);

        assert!(
            !actions.is_empty(),
            "chain {chain_id} resolved nothing, so the grant check below is vacuous",
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::GrantItem { .. } | Action::RemoveItem { .. })),
            "chain {chain_id} (dialog {dialog_id}) now touches the player's inventory. The \
             \"Receive Item\" button DU-02a stripped granted nothing; item 21 comes from \
             chain 1055's add_item on the Preparation_SMG1A locker interact. If the grant \
             has genuinely moved onto the dialog, the strip needs re-reviewing; got \
             {actions:?}",
        );
    }
}
