//! Mission 1326 — Lan'toc (packet H22,
//! `docs/analysis/harset-rebuild/work-packets.md#h22`, seed
//! `harset_jaffa_chains.sql` chains 6331, 6333-6341).
//!
//! Moh'katan (template 54, world 68) sends the player into the Harset
//! Jaffa Zone (world 57) to present the Lan'toc loyalty rite to Jaffa
//! who once served Ra. Each of those Jaffa either swears allegiance
//! (dialog 4375) or refuses (dialog 4376); presenting the rite once
//! satisfies step 3960 whichever answer comes back, and the player
//! returns to Moh'katan for step 4603.
//!
//! Four shapes are unique to this mission and each has its own guard:
//!
//! 1. **Two branches, one step.** The accept and the refusal are two
//!    different NPCs of template 204 (`Harset_FormerRaJaffa` and
//!    `Harset_FormerRaJaffa2`), because the outcome is the NPC's and
//!    neither outcome dialog has a branchable button — the engine has no
//!    randomness primitive and no authorable condition that could pick
//!    between two dialogs on one tag. Both branches must satisfy the
//!    step, satisfy it EXACTLY once, and go inert afterwards, so the
//!    player who talks to the other Jaffa after finishing gets nothing.
//! 2. **`advance_step`, never `complete_objective 4551`.** 4551 is the
//!    only non-optional objective of the non-terminal step 3960, so
//!    `complete_objective` would trip the all-required-complete branch
//!    in `cell/missions/progression.rs` and complete the whole mission,
//!    orphaning the turn-in. The packet text asked for
//!    `complete_objective`; the engine semantics overrule it. The
//!    file-wide structural guard lives in [`super::mission_1324`]; the
//!    per-branch assertion is here.
//! 3. **A cross-world bind.** The mission is accepted in world 68 and
//!    the next icon belongs to an NPC in world 57. Binds are in-memory
//!    on the cell entity and die on the cross-world hop, so the accept
//!    chain deliberately does NOT bind the Jaffa Zone icon — chain 6334,
//!    on `player_loaded 'Harset'`, paints it on arrival and re-paints it
//!    after a relog.
//! 4. **The offer rides the native dialog path.** There is no
//!    `interact_tag` chain for the offer state: one would short-circuit
//!    `interactions::handle_interact`, which is the only writer of the
//!    `last_interaction_target` pin that chain 6333's
//!    `display_dialog 4374` needs. Same shape as 1324's chain 6303 —
//!    see [`super::mission_1324`] for the full reasoning, and
//!    [`clicking_mohkatan_in_the_lantoc_offer_state_must_resolve_no_chain`]
//!    for the guard.
//!
//! 5. **The offer is painted on two triggers, not one.** Chain 6331
//!    fires on `player_loaded`, which is an edge — but 1325 is turned in
//!    to Moh'katan in world 68, so the player is already standing where
//!    the icon belongs when the gate opens, and there is no second
//!    `player_loaded` to catch. Chain 6341 closes that window on
//!    `mission_completed '1325'`, and carries the identical condition
//!    set so the two can never disagree about who may see the offer.
//!
//! The offer chains additionally carry `mission_status 1325 eq
//! completed` and `mission_status 1324 eq completed`. Moh'katan hands out
//! 1324, 1325 and 1326 from one template and one tag; two offer chains
//! matching one right-click would both push a `display_dialog` and the
//! client would render only the second. The predecessor gate keeps
//! exactly one live, which is why the positives below set both
//! predecessors completed and one negative proves the offer stays dark
//! until they are.
//!
//! Context helpers are shared with [`super::mission_1324`] rather than
//! duplicated — the two missions share NPCs, archetype and worlds, so a
//! drifting copy of `ctx_for`/`with_step` would be a silent
//! disagreement about what state the other mission is in.

use super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_interact_tag, fire_mission_completed,
    fire_player_loaded, labels, with_mission, with_step, CMD_CENTER, CMD_CENTER_NAME, HARSET,
    HARSET_NAME, JAFFA, NOT_JAFFA,
};

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;

/// A Jaffa who has finished 1324 and 1325 and is standing in `world_id`,
/// i.e. exactly the state in which 1326 should be on offer.
fn ready_for_lantoc(world_id: i32) -> ExecutionContext {
    let ctx = ctx_for(JAFFA, Some(world_id));
    let ctx = with_mission(ctx, 1324, "completed");
    let ctx = with_mission(ctx, 1325, "completed");
    with_mission(ctx, 1326, "not_active")
}

/// A Jaffa mid-1326 on the "present the Lan'toc" step, standing in the
/// Jaffa Zone.
fn presenting_the_lantoc() -> ExecutionContext {
    presenting_the_lantoc_in(HARSET)
}

/// The same state, in an arbitrary world — the world-68 variant is what
/// the offer chains must stay dark for once 1326 has been accepted.
fn presenting_the_lantoc_in(world_id: i32) -> ExecutionContext {
    let ctx = ctx_for(JAFFA, Some(world_id));
    let ctx = with_mission(ctx, 1324, "completed");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "active");
    let ctx = with_step(ctx, 1326, 3960, "active");
    with_step(ctx, 1326, 4603, "not_active")
}

/// The same player one beat later: the rite has been presented, the step
/// has advanced, and the player is on their way back to Moh'katan.
///
/// `step_3960` is a parameter because a completed step reads `completed`
/// in-session but `not_active` after a relog — `advance_step` persists
/// `completed_step_ids: vec![]` (`content/executor/mission.rs`). Guards
/// that must hold in both shapes drive over both.
fn lantoc_presented(world_id: i32, step_3960: &str) -> ExecutionContext {
    let ctx = ctx_for(JAFFA, Some(world_id));
    let ctx = with_mission(ctx, 1324, "completed");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "active");
    let ctx = with_step(ctx, 1326, 3960, step_3960);
    with_step(ctx, 1326, 4603, "active")
}

// ---------------------------------------------------------------
// Chains 6331 / 6333 — offer and accept
// ---------------------------------------------------------------

/// Positive: with 1324 and 1325 behind them, a Jaffa entering the
/// Command Center gets Moh'katan's 1326 "?" (dsm 5159) and nothing else.
#[tokio::test]
async fn entering_the_command_center_after_1325_paints_the_lantoc_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_player_loaded(&engine, &ready_for_lantoc(CMD_CENTER), CMD_CENTER_NAME);

    assert_eq!(
        labels(&resolved),
        vec!["6331:add_dialog_set(5159@54)".to_string()],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// The load-bearing ABSENCE, same shape as 1324's: clicking Moh'katan
/// while 1326 is on offer must resolve no chain at all, so
/// `interactions::handle_interact` runs, pins `last_interaction_target`
/// and opens dialog 4373 from the bind. Without that pin chain 6333's
/// `display_dialog 4374` bails with a warn and the briefing never plays.
#[tokio::test]
async fn clicking_mohkatan_in_the_lantoc_offer_state_must_resolve_no_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_interact_tag(&engine, &ready_for_lantoc(CMD_CENTER), "CmdCenter_Mohkatan");

    assert!(
        resolved.actions.is_empty(),
        "no chain may match a right-click on Moh'katan while 1326 is on \
         offer — the offer dialog is opened by chain 6331's bind through \
         `handle_interact`, which is also the only writer of the pin \
         chain 6333 needs. Matched: {:?}",
        resolved.actions,
    );
}

/// Negative (predecessor gate): with 1324 done but 1325 still open,
/// 1326 must stay dark. This is the condition that stops 1325's and
/// 1326's offers colliding on Moh'katan once packet H21 lands — remove
/// it and the player gets two mission offers on one right-click, of
/// which the client shows one and the other's button is rejected by the
/// #479 open-dialog gate.
#[tokio::test]
async fn the_lantoc_offer_stays_dark_until_1325_is_finished() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "completed");
    let ctx = with_mission(ctx, 1325, "active");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME)).is_empty(),
        "chain 6331 must carry `mission_status 1325 eq completed`"
    );
}

/// Negative (the other predecessor): 1325 completed while 1324 is still
/// untouched is not a state normal play can reach, but a GM grant can.
/// The `mission_status 1324 eq completed` row is what makes 6301's and
/// 6331's disjointness structural rather than an argument about which
/// mission implies which — without it both would bind to slot 54 and
/// `entries.first()` would pick by chain registration order.
#[tokio::test]
async fn the_lantoc_offer_stays_dark_if_1324_was_skipped() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "not_active");

    let resolved = fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME);
    assert_eq!(
        labels(&resolved),
        vec!["6301:add_dialog_set(5149@54)".to_string()],
        "only 1324's own offer may bind slot 54 here; 1326's must stay \
         dark so two dsm rows never share the slot. Resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (wrong archetype): Lan'toc is a Jaffa rite.
#[tokio::test]
async fn a_human_is_never_offered_lantoc() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(NOT_JAFFA, Some(CMD_CENTER)), 1324, "completed");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME)).is_empty(),
        "archetype {NOT_JAFFA} must resolve no 1326 chain"
    );
}

/// Negative (wrong world): the Command-Center-scoped offer must not
/// paint on a Harset load.
#[tokio::test]
async fn the_lantoc_offer_does_not_paint_in_harset() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    assert!(
        labels(&fire_player_loaded(
            &engine,
            &ready_for_lantoc(HARSET),
            HARSET_NAME
        ))
        .is_empty(),
        "chain 6331 must carry `world eq 68` and its trigger's \
         Harset_CmdCenter name filter"
    );
}

/// Positive: the Accept button on 4373 accepts 1326, plays Moh'katan's
/// briefing 4374 and retires his "?".
///
/// No `add_dialog_set` here on purpose: the next icon belongs to the
/// Former-Ra Jaffa in world 57 and a bind made in world 68 would not
/// survive the hop. Chain 6334 paints it on arrival. If a future edit
/// adds one here it shows up as an extra label and this test fails,
/// which is the point.
#[tokio::test]
async fn accepting_lantoc_briefs_the_player_and_retires_the_offer_icon() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_dialog_choice(&engine, &ready_for_lantoc(CMD_CENTER), 4373);

    assert_eq!(
        labels(&resolved),
        vec![
            "6333:accept_mission(1326)".to_string(),
            "6333:display_dialog(4374)".to_string(),
            "6333:remove_dialog_set(5159@54)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (re-accept guard): once 1326 is active the accept chain is
/// inert. Without `mission_status 1326 eq not_active` a replayed choice
/// would re-show the briefing and re-remove the bind.
#[tokio::test]
async fn the_lantoc_accept_does_not_refire_once_1326_is_active() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    assert!(
        labels(&fire_dialog_choice(&engine, &presenting_the_lantoc(), 4373)).is_empty(),
        "chain 6333 must carry `mission_status 1326 eq not_active`"
    );
}

// ---------------------------------------------------------------
// Chain 6334 — the Jaffa Zone bind
// ---------------------------------------------------------------

/// Positive: arriving in Harset on step 3960 paints the Former-Ra
/// Jaffa's "!" (dsm 120002 on template 204).
///
/// One bind covers both spawns: `send_interaction_update_if_visible`
/// fans the merged flags to every entity of the bound template in the
/// player's AoI, and both Lan'toc Jaffa are template 204.
#[tokio::test]
async fn arriving_in_harset_on_step_3960_paints_the_jaffa_zone_icon() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_player_loaded(&engine, &presenting_the_lantoc(), HARSET_NAME);

    assert_eq!(
        labels(&resolved),
        vec!["6334:add_dialog_set(120002@204)".to_string()],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (wrong world): 6334 is the one chain in this mission gated
/// on world 57. Firing the Harset-named load with a Command Center world
/// id must resolve nothing — proving the `world eq 57` condition is
/// present and not inherited from the trigger's name filter alone.
#[tokio::test]
async fn the_jaffa_zone_bind_is_gated_on_world_57() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = ctx_for(JAFFA, Some(CMD_CENTER));
    let ctx = with_mission(ctx, 1326, "active");
    let ctx = with_step(ctx, 1326, 3960, "active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, HARSET_NAME)).is_empty(),
        "chain 6334 must carry `world eq 57`"
    );
}

/// Negative (wrong step): once the rite has been presented the Jaffa
/// Zone icon must not be re-painted on a later Harset load — the bind is
/// removed by 6337/6338 and the restore chain has to agree. Driven over
/// both post-advance shapes of step 3960.
#[tokio::test]
async fn the_jaffa_zone_icon_is_not_repainted_after_the_rite() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for step_3960 in ["completed", "not_active"] {
        assert!(
            labels(&fire_player_loaded(
                &engine,
                &lantoc_presented(HARSET, step_3960),
                HARSET_NAME
            ))
            .is_empty(),
            "chain 6334's `step_status 1326/3960 eq active` gate must \
             retire the bind once the step advances (3960 reading \
             {step_3960:?})"
        );
    }
}

// ---------------------------------------------------------------
// Chains 6335 / 6336 — presenting the rite
// ---------------------------------------------------------------

/// Positive, both branches: each Former-Ra Jaffa opens exactly its own
/// outcome dialog, and exactly one dialog engine-wide.
///
/// The tags are byte-exact keys — a typo in either is silent at load
/// time and produces an NPC that is highlighted (the bind is on the
/// template, so the icon appears on both spawns regardless) but does
/// nothing when clicked. Worse, with no chain matching, the native path
/// would open `available_interactions[204].first()` = dialog 4375, so a
/// typo'd refuser tag makes the refusing Jaffa swear allegiance. Both
/// tags are therefore asserted against the dialog they must open.
#[tokio::test]
async fn each_former_ra_jaffa_opens_its_own_outcome_dialog() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for (chain_id, tag, dialog_id) in [
        (6335, "Harset_FormerRaJaffa", 4375),
        (6336, "Harset_FormerRaJaffa2", 4376),
    ] {
        let resolved = fire_interact_tag(&engine, &presenting_the_lantoc(), tag);

        assert_eq!(
            display_dialog_count(&resolved),
            1,
            "'{tag}' must open exactly one dialog. Full resolve: {:?}",
            resolved.actions,
        );
        assert_eq!(
            labels(&resolved),
            vec![format!("{chain_id}:display_dialog({dialog_id})")],
            "'{tag}' must open dialog {dialog_id} via chain {chain_id}. A \
             swap here means the accept and refusal branches were \
             transposed, which is invisible at load time. Resolve: {:?}",
            resolved.actions,
        );
    }
}

/// Negative (wrong archetype) on both branches.
#[tokio::test]
async fn a_human_cannot_present_the_lantoc() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = ctx_for(NOT_JAFFA, Some(HARSET));
    let ctx = with_mission(ctx, 1326, "active");
    let ctx = with_step(ctx, 1326, 3960, "active");

    for tag in ["Harset_FormerRaJaffa", "Harset_FormerRaJaffa2"] {
        assert!(
            labels(&fire_interact_tag(&engine, &ctx, tag)).is_empty(),
            "'{tag}' must be archetype-gated"
        );
    }
}

/// Negative (wrong world) on both branches — `interact_tag` has no world
/// filter of its own.
#[tokio::test]
async fn presenting_the_lantoc_from_the_wrong_world_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = ctx_for(JAFFA, Some(CMD_CENTER));
    let ctx = with_mission(ctx, 1326, "active");
    let ctx = with_step(ctx, 1326, 3960, "active");

    for tag in ["Harset_FormerRaJaffa", "Harset_FormerRaJaffa2"] {
        assert!(
            labels(&fire_interact_tag(&engine, &ctx, tag)).is_empty(),
            "'{tag}' must carry `world eq 57`"
        );
    }
}

/// Negative (the second Jaffa): after the rite has been presented to one
/// of them, clicking the OTHER one resolves nothing.
///
/// This is the acceptance criterion "a second interact after completion
/// resolves nothing", and it is the reason the branch chains gate on
/// `step_status 1326/3960 eq active` rather than on
/// `mission_status 1326 eq active`: the mission is still active on step
/// 4603, so a mission-level gate would leave both Jaffa live all the way
/// to the turn-in and let the player advance the step twice.
#[tokio::test]
async fn talking_to_the_other_jaffa_after_the_rite_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for step_3960 in ["completed", "not_active"] {
        for tag in ["Harset_FormerRaJaffa", "Harset_FormerRaJaffa2"] {
            let resolved = fire_interact_tag(&engine, &lantoc_presented(HARSET, step_3960), tag);
            assert!(
                labels(&resolved).is_empty(),
                "'{tag}' must be inert once step 3960 has advanced (3960 \
                 reading {step_3960:?}); got {:?}",
                resolved.actions,
            );
        }
    }
}

// ---------------------------------------------------------------
// Chains 6337 / 6338 — the two outcomes
// ---------------------------------------------------------------

/// Positive, both branches: each outcome clears the Jaffa Zone bind and
/// advances 1326 to the return step exactly once — and neither resolves
/// a `complete_objective`.
///
/// The objective-completion assertion comes BEFORE the label
/// comparison on purpose: an unexpected action maps to `OTHER(...)` and
/// would trip the label assert first, making the more specific check
/// unreachable.
#[tokio::test]
async fn both_lantoc_outcomes_advance_the_step_exactly_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for (chain_id, dialog_id) in [(6337, 4375), (6338, 4376)] {
        let resolved = fire_dialog_choice(&engine, &presenting_the_lantoc(), dialog_id);

        let advances = resolved
            .actions
            .iter()
            .filter(|(_, a)| {
                matches!(
                    a,
                    Action::AdvanceStep {
                        mission_id: 1326,
                        step_id: 4603,
                    }
                )
            })
            .count();
        assert_eq!(
            advances, 1,
            "dialog {dialog_id} must advance 1326 to step 4603 exactly \
             once; got {advances} in {:?}",
            resolved.actions,
        );

        let completes = resolved
            .actions
            .iter()
            .filter(|(_, a)| matches!(a, Action::CompleteObjective { .. }))
            .count();
        assert_eq!(
            completes, 0,
            "dialog {dialog_id} must NOT resolve a CompleteObjective. \
             4551 is step 3960's only non-optional objective, so \
             completing it would complete the whole of 1326 and orphan \
             the turn-in step 4603 (cell/missions/progression.rs). \
             `advance_step` force-completes 4551 on its way, so the \
             objective is still completed exactly once. Got {:?}",
            resolved.actions,
        );

        assert_eq!(
            labels(&resolved),
            vec![
                format!("{chain_id}:remove_dialog_set(120002@204)"),
                format!("{chain_id}:advance_step(1326->4603)"),
            ],
            "resolve: {:?}",
            resolved.actions,
        );
    }
}

/// Negative, both branches: a replayed choice after the step has already
/// advanced resolves nothing, so the step cannot be advanced twice.
#[tokio::test]
async fn a_replayed_lantoc_outcome_cannot_advance_the_step_again() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for step_3960 in ["completed", "not_active"] {
        for dialog_id in [4375, 4376] {
            assert!(
                labels(&fire_dialog_choice(
                    &engine,
                    &lantoc_presented(HARSET, step_3960),
                    dialog_id
                ))
                .is_empty(),
                "a replayed choice on dialog {dialog_id} must resolve \
                 nothing once step 3960 is past (reading {step_3960:?})"
            );
        }
    }
}

// ---------------------------------------------------------------
// Chains 6339 / 6340 — return and turn-in
// ---------------------------------------------------------------

/// Positive: returning to the Command Center on step 4603 paints
/// Moh'katan's turn-in "?" (dsm 5161), and this is also the relog
/// restore for that state.
#[tokio::test]
async fn returning_to_the_command_center_paints_the_turn_in_icon() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_player_loaded(
        &engine,
        &lantoc_presented(CMD_CENTER, "not_active"),
        CMD_CENTER_NAME,
    );

    assert_eq!(
        labels(&resolved),
        vec!["6339:add_dialog_set(5161@54)".to_string()],
        "mid-4603 arrival or relog must re-bind Moh'katan's turn-in and \
         nothing else. Resolve: {:?}",
        resolved.actions,
    );
}

/// Positive: clicking Moh'katan on step 4603 plays the turn-in dialog
/// 4377, clears the bind and completes 1326 — one dialog engine-wide,
/// remove before complete (see [`super::mission_1324`] for why the order
/// is load-bearing).
#[tokio::test]
async fn returning_to_mohkatan_completes_1326() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_interact_tag(
        &engine,
        &lantoc_presented(CMD_CENTER, "not_active"),
        "CmdCenter_Mohkatan",
    );

    assert_eq!(
        display_dialog_count(&resolved),
        1,
        "the turn-in click must open exactly one dialog. Full resolve: {:?}",
        resolved.actions,
    );
    assert_eq!(
        labels(&resolved),
        vec![
            "6340:display_dialog(4377)".to_string(),
            "6340:remove_dialog_set(5161@54)".to_string(),
            "6340:complete_mission(1326)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative: once 1326 is completed, Moh'katan resolves nothing from
/// this packet — the turn-in step gate has flipped and the offer chain
/// is retired by `mission_status 1326 eq not_active`.
#[tokio::test]
async fn clicking_mohkatan_after_completing_1326_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for after in ["completed", "not_active"] {
        let ctx = ctx_for(JAFFA, Some(CMD_CENTER));
        let ctx = with_mission(ctx, 1324, "completed");
        let ctx = with_mission(ctx, 1325, "completed");
        let ctx = with_mission(ctx, 1326, "completed");
        let ctx = with_step(ctx, 1326, 3960, after);
        let ctx = with_step(ctx, 1326, 4603, after);

        assert!(
            labels(&fire_interact_tag(&engine, &ctx, "CmdCenter_Mohkatan")).is_empty(),
            "1326 must be inert once completed (steps reading {after:?})"
        );
    }
}

/// dsm 120002 is an INTERACTION-ONLY row: the right bit, and no dialog.
///
/// The `dialog_id IS NULL` half is the guard that matters, and it is a
/// guard rather than a description of the seed. One bind lights BOTH
/// Lan'toc Jaffa, because `send_interaction_update_if_visible` fans the
/// merged flags to every entity of the bound template in the player's
/// AoI and both spawns are template 204. That is intended — either Jaffa
/// satisfies the step. What is NOT acceptable is the row naming a
/// dialog: `handle_interact` opens the first bound entry that has one,
/// so in any state where chains 6335/6336 stop matching, a dialog on
/// this row would open on BOTH NPCs and the Jaffa scripted to REFUSE
/// would swear allegiance instead. With NULL, that path opens nothing.
/// This is the 2026-09-18 Castle playtest lesson (two NPCs of one
/// identity, one of them showing the other's cue) applied structurally.
///
/// NULL is only available because Castle packet CA02 taught
/// `cell/spawner/dialogs.rs::load_dialog_set_maps` to keep such rows as
/// `dialog_id: None` instead of dropping them. Before CA02 this bind
/// would have been a silent cache miss — so this assertion is also the
/// tripwire if that loader behaviour is ever reverted.
///
/// A zero `interaction_flags` would merge nothing onto template 204's own
/// `interaction_type = 0` and leave both Jaffa unclickable, with the
/// chains still perfectly correct and the mission unplayable.
#[tokio::test]
async fn the_lantoc_jaffa_bind_is_interaction_only_and_carries_its_bit() {
    let pool = require_db_or_skip!();

    let row: Option<(i32, Option<i32>, i64)> = sqlx::query_as(
        "SELECT dialog_set_id, dialog_id, interaction_flags \
         FROM resources.dialog_set_maps WHERE dialog_set_map_id = 120002",
    )
    .fetch_optional(&pool)
    .await
    .expect("dialog_set_maps query must succeed");

    let (set_id, dialog_id, flags) =
        row.expect("dialog_set_map 120002 must exist — chain 6334 binds it");

    assert_eq!(
        set_id, 1393,
        "120002 belongs to the shipped Lan'toc dialog set 1393"
    );
    assert_eq!(
        dialog_id, None,
        "120002 must have a NULL dialog_id. Both Lan'toc Jaffa are \
         template 204 and one bind lights both, so a dialog here is \
         reachable from EITHER of them through `handle_interact` — and \
         the refusing Jaffa would open the oath. The outcome dialogs \
         belong to chains 6335/6336, keyed per tag."
    );
    assert_eq!(
        flags, 268_435_456,
        "120002 must carry INT_NonAStoryMissionActive; template 204 has \
         `entity_templates.interaction_type = 0`, so a zero here leaves \
         both Lan'toc Jaffa unclickable"
    );
}

/// Dialogs 4375 and 4376 must stay button-less, or chains 6337/6338 die.
///
/// A dialog with zero `dialog_screen_buttons` rows sends
/// `Event_NetOut_DialogButtonChoice` with `ButtonId = -1` when the player
/// closes it; a dialog that HAS buttons sends nothing on close and fires
/// only on a click. Both outcome dialogs are button-less today, which is
/// the only reason a `dialog_choice` trigger can close step 3960 at all.
///
/// Adding a button to either one — a perfectly reasonable-looking content
/// edit — would silently strand the mission on step 3960 with no error
/// on any layer. The guard is at the DB level rather than on resolved
/// actions because the break is in the client's send decision, which no
/// chain-replay context can model.
#[tokio::test]
async fn the_lantoc_outcome_dialogs_stay_button_less() {
    let pool = require_db_or_skip!();

    for dialog_id in [4375, 4376] {
        let (buttons,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM resources.dialog_screen_buttons b \
             JOIN resources.dialog_screens s ON s.screen_id = b.screen_id \
             WHERE s.dialog_id = $1",
        )
        .bind(dialog_id)
        .fetch_one(&pool)
        .await
        .expect("dialog_screen_buttons query must succeed");

        assert_eq!(
            buttons, 0,
            "dialog {dialog_id} now has {buttons} button(s). Chains \
             6337/6338 fire on `dialog_choice '{dialog_id}'`, which the \
             client only sends on CLOSE for a button-less dialog. With a \
             button present it sends on the click instead and the \
             close-path choice disappears, so step 3960 can never \
             advance. Move the chain to the new button's flow or keep \
             the dialog button-less."
        );
    }
}

// ---------------------------------------------------------------
// Chain 6341 — the offer edge-closer
// ---------------------------------------------------------------

/// Positive: completing 1325 paints 1326's offer icon immediately,
/// without making the player leave the Command Center and come back.
///
/// This is the Castle playtest's edge-trigger finding (H9) applied here.
/// Chain 6331 fires on `player_loaded`, an EDGE — but 1325 is turned in
/// TO MOH'KATAN, IN WORLD 68, so the player is already standing where the
/// icon belongs at the moment the gate opens, and there is no second
/// `player_loaded` to catch. With 6331 alone the "?" would appear only
/// after a world round-trip, which reads as the next mission not
/// existing.
///
/// `fire_mission_completed` runs after `complete_mission_direct` has
/// flipped the status (`content/executor/mission.rs`), so the chain sees
/// 1325 as `completed` and can carry 6331's exact condition set.
#[tokio::test]
async fn completing_1325_paints_the_lantoc_offer_without_a_world_hop() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_mission_completed(&engine, &ready_for_lantoc(CMD_CENTER), 1325);

    assert_eq!(
        labels(&resolved),
        vec!["6341:add_dialog_set(5159@54)".to_string()],
        "completing 1325 in world 68 must bind dsm 5159 on slot 54 right \
         away. An empty result here means chain 6341 was dropped and \
         1326's offer is invisible until the player walks out of the \
         Command Center and back in. Resolve: {:?}",
        resolved.actions,
    );
}

/// Negative: the edge-closer carries the same gates as 6331, so it
/// cannot paint for the wrong archetype, the wrong world, or a player
/// who has somehow completed 1325 without 1324.
///
/// The world gate is the one worth spelling out: if a later packet moves
/// 1325's turn-in out of world 68 this chain goes quiet rather than
/// binding an icon onto an NPC the player cannot see, and chain 6331
/// still paints it on the next Command Center entry. Fail-closed to the
/// slower path, never to a wrong one.
#[tokio::test]
async fn the_lantoc_edge_closer_carries_the_same_gates_as_the_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Wrong archetype.
    let ctx = with_mission(ctx_for(NOT_JAFFA, Some(CMD_CENTER)), 1324, "completed");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "not_active");
    assert!(
        labels(&fire_mission_completed(&engine, &ctx, 1325)).is_empty(),
        "chain 6341 must be archetype-gated like 6331"
    );

    // Wrong world.
    assert!(
        labels(&fire_mission_completed(
            &engine,
            &ready_for_lantoc(HARSET),
            1325
        ))
        .is_empty(),
        "chain 6341 must carry `world eq 68`"
    );

    // 1324 skipped (reachable only by a GM grant).
    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "completed");
    let ctx = with_mission(ctx, 1326, "not_active");
    assert!(
        labels(&fire_mission_completed(&engine, &ctx, 1325)).is_empty(),
        "chain 6341 must carry `mission_status 1324 eq completed`"
    );

    // 1326 already taken — a replayed completion must not re-paint an
    // offer the player has moved past.
    assert!(
        labels(&fire_mission_completed(
            &engine,
            &presenting_the_lantoc_in(CMD_CENTER),
            1325
        ))
        .is_empty(),
        "chain 6341 must carry `mission_status 1326 eq not_active`"
    );
}

/// Negative: completing a DIFFERENT mission must not paint 1326's offer.
///
/// `OnMissionCompleted` matches on the mission id in `event_key`, so a
/// NULL or mistyped key would turn this into a wildcard that binds the
/// Lan'toc icon every time the player finishes anything at all.
#[tokio::test]
async fn completing_some_other_mission_does_not_paint_the_lantoc_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    assert!(
        labels(&fire_mission_completed(
            &engine,
            &ready_for_lantoc(CMD_CENTER),
            1324
        ))
        .is_empty(),
        "chain 6341's trigger must name mission 1325 specifically"
    );
}
