//! CA01 — Castle arrival and the Sgt. Gerschon handoff (chains
//! 1201-1205).
//!
//! Covers the arrival bind, both archetype branches of the offer, both
//! accept paths, and the no-re-accept guard on each.

use cimmeria_content_engine::context::ExecutionContext;

use super::{
    castle_login_ctx, choice_at_step, dialog_choice_ctx, engine_for, fire, interact_at_step,
    interact_ctx, summarized, with_mission, with_step, CHOICE, INTERACT, JAFFA, LOGIN, NON_JAFFA,
};
use crate::test_support::require_db_or_skip;

/// Chain 1201 — arriving in Castle with 701 never accepted binds the
/// Gerschon offer topic to template 149.
#[tokio::test]
async fn chain_1201_binds_gerschon_topic_on_arrival() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1201).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "not_active");

    assert_eq!(
        summarized(&fire(&engine, LOGIN, &ctx), 1201),
        vec!["add_dialog_set(3062, slot=149)"],
        "arriving in Castle before 701 is accepted must bind dialog-set map \
         3062 to Gerschon's template (149) and do nothing else",
    );
}

/// Chain 1201 negative — a player who already has 701 must not have the
/// offer topic re-bound on every login. Without the `mission_status
/// not_active` gate, Gerschon would keep the "offer" indicator for the
/// whole mission.
#[tokio::test]
async fn chain_1201_does_not_rebind_offer_once_701_is_active() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1201).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2399, "active");

    assert!(
        summarized(&fire(&engine, LOGIN, &ctx), 1201).is_empty(),
        "chain 1201 must not resolve once 701 is active",
    );
}

/// Chain 1202 — a non-Jaffa clicking Gerschon before 701 starts sees the
/// Tau'ri offer dialog 2573.
#[tokio::test]
async fn chain_1202_shows_human_offer_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1202).await;

    let ctx = interact_at_step("Castle_SgtGerschon", NON_JAFFA, 2399, "not_active");

    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1202),
        vec!["display_dialog(2573)"],
        "a non-Jaffa must get dialog 2573 from Gerschon",
    );
}

/// Chain 1202 negative — a Jaffa must NOT get the Tau'ri dialog. This is
/// the `archetype neq 8` half of D-CA13; losing it would show Jaffa
/// players a dialog written for humans ("Thank you, Sgt.").
#[tokio::test]
async fn chain_1202_does_not_fire_for_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1202).await;

    let ctx = interact_at_step("Castle_SgtGerschon", JAFFA, 2399, "not_active");

    assert!(
        summarized(&fire(&engine, INTERACT, &ctx), 1202).is_empty(),
        "chain 1202 is the `archetype neq 8` branch and must not match a Jaffa",
    );
}

/// Chain 1203 — a Jaffa clicking Gerschon sees dialog 5861 instead.
#[tokio::test]
async fn chain_1203_shows_jaffa_offer_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1203).await;

    let ctx = interact_at_step("Castle_SgtGerschon", JAFFA, 2399, "not_active");

    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1203),
        vec!["display_dialog(5861)"],
        "a Jaffa must get dialog 5861 from Gerschon",
    );
}

/// Chain 1203 negative — the Jaffa dialog must never reach a non-Jaffa.
/// Together with `chain_1202_does_not_fire_for_jaffa` this pins that the
/// two branches are mutually exclusive, so a player can never be offered
/// 701 twice in one click.
#[tokio::test]
async fn chain_1203_does_not_fire_for_non_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1203).await;

    let ctx = interact_at_step("Castle_SgtGerschon", NON_JAFFA, 2399, "not_active");

    assert!(
        summarized(&fire(&engine, INTERACT, &ctx), 1203).is_empty(),
        "chain 1203 is the `archetype eq 8` branch and must not match a non-Jaffa",
    );
}

/// Chain 1202 negative — once the player is on step 2399 (i.e. 701 is
/// accepted and running), clicking Gerschon again must not re-offer the
/// mission. This is the `step_status 2399 eq not_active` gate; the
/// Jaffa branch shares it.
#[tokio::test]
async fn gerschon_offer_does_not_reappear_once_step_2399_is_active() {
    let pool = require_db_or_skip!();

    for chain_id in [1202, 1203] {
        let engine = engine_for(&pool, chain_id).await;
        // Archetype is set to the value that WOULD match each chain, so
        // the only thing rejecting the event is the step gate.
        let archetype = if chain_id == 1203 { JAFFA } else { NON_JAFFA };
        let ctx = interact_at_step("Castle_SgtGerschon", archetype, 2399, "active");

        assert!(
            summarized(&fire(&engine, INTERACT, &ctx), chain_id as i64).is_empty(),
            "chain {chain_id} must not re-offer 701 while step 2399 is active",
        );
    }
}

/// Chain 1204 — choosing on dialog 2573 accepts 701, drops the Gerschon
/// bind and moves the topic onto Copplemann's template (48), in that
/// order. The order mirrors `Castle.py`'s dialogChoiceCb, where the bind
/// on 48 is nested inside the successful unbind of 149.
#[tokio::test]
async fn chain_1204_human_accept_rebinds_topic_to_copplemann() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1204).await;

    let mut ctx = dialog_choice_ctx(2573);
    with_mission(&mut ctx, 701, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1204),
        vec![
            "accept_mission(701)",
            "remove_dialog_set(3062, slot=149)",
            "add_dialog_set(3062, slot=48)",
        ],
        "accepting 701 from dialog 2573 must accept, unbind Gerschon, then \
         bind Copplemann — in that order",
    );
}

/// Chain 1204 negative — the no-re-accept guard. A second choice on 2573
/// while 701 is already active must resolve nothing at all: not just no
/// second `accept_mission`, but no second dialog-set churn either (which
/// would re-add a duplicate bind and re-push the interaction flags).
#[tokio::test]
async fn chain_1204_does_not_re_accept_on_a_second_interaction() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1204).await;

    let mut ctx = dialog_choice_ctx(2573);
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2399, "active");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1204).is_empty(),
        "chain 1204 must not resolve a second time while 701 is active",
    );
}

/// Chain 1205 — the Jaffa accept does everything 1204 does and then
/// plays the Moh'katan radio call (5862) last.
///
/// Dialog 5862 is not a monologue (Moh'katan speaks on three of its five
/// screens), and `fire_dialog_choice` stamps no `target_entity_id`, so it
/// can only reach the client through the player's
/// `last_interaction_target` pin. That pin is set before the
/// content-chain dispatch in
/// `cell_methods/player/interaction/interact.rs`, guarded there by
/// `chain_handled_interact_pins_target_for_a_later_chain_dialog`. This
/// test pins the seed side: the action must be resolved and ordered last.
#[tokio::test]
async fn chain_1205_jaffa_accept_adds_the_mohkatan_radio_call() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1205).await;

    let mut ctx = dialog_choice_ctx(5861);
    with_mission(&mut ctx, 701, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1205),
        vec![
            "accept_mission(701)",
            "remove_dialog_set(3062, slot=149)",
            "add_dialog_set(3062, slot=48)",
            "display_dialog(5862)",
        ],
        "the Jaffa accept must mirror 1204 and append dialog 5862",
    );
}

/// Chain 1205 negative — same no-re-accept guard as 1204.
#[tokio::test]
async fn chain_1205_does_not_re_accept_on_a_second_interaction() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1205).await;

    let mut ctx = dialog_choice_ctx(5861);
    with_mission(&mut ctx, 701, "active");

    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1205).is_empty(),
        "chain 1205 must not resolve a second time while 701 is active",
    );
}

/// Cross-branch guard: the two accept chains are keyed by dialog id, not
/// by archetype, because `fire_dialog_choice` carries no archetype. A
/// choice on 2573 must not also fire the Jaffa chain and vice versa —
/// otherwise a Human accept would trigger the Moh'katan radio call.
#[tokio::test]
async fn accept_chains_are_keyed_by_dialog_and_do_not_cross_fire() {
    let pool = require_db_or_skip!();

    let jaffa_chain = engine_for(&pool, 1205).await;
    let mut human_choice = dialog_choice_ctx(2573);
    with_mission(&mut human_choice, 701, "not_active");
    assert!(
        summarized(&fire(&jaffa_chain, CHOICE, &human_choice), 1205).is_empty(),
        "a choice on dialog 2573 must not fire the Jaffa accept chain",
    );

    let human_chain = engine_for(&pool, 1204).await;
    let mut jaffa_choice = dialog_choice_ctx(5861);
    with_mission(&mut jaffa_choice, 701, "not_active");
    assert!(
        summarized(&fire(&human_chain, CHOICE, &jaffa_choice), 1204).is_empty(),
        "a choice on dialog 5861 must not fire the Human accept chain",
    );
}

/// Tag guard: the spawn row and `Castle.py` both spell Gerschon's tag
/// `Castle_SgtGerschon`. A trigger keyed on anything else never fires and
/// the mission is unstartable with no error anywhere — the exact silent
/// failure the region-key linter exists to catch for regions.
#[tokio::test]
async fn gerschon_chains_are_keyed_to_the_spawnlist_tag() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1202).await;

    let ctx = interact_ctx("Castle_Gerschon", NON_JAFFA);
    assert!(
        summarized(&fire(&engine, INTERACT, &ctx), 1202).is_empty(),
        "the trigger must match the exact spawnlist tag, not a near miss",
    );

    let ok = interact_at_step("Castle_SgtGerschon", NON_JAFFA, 2399, "not_active");
    assert!(
        !summarized(&fire(&engine, INTERACT, &ok), 1202).is_empty(),
        "sanity: the real tag must still match",
    );
}

/// Chain 1201 and the restore chains all key on world name `Castle`.
/// A `player_loaded` for the Cellblock must not bind Castle's topics.
#[tokio::test]
async fn arrival_bind_is_scoped_to_the_castle_world() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1201).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    with_mission(&mut ctx, 701, "not_active");

    assert!(
        summarized(&fire(&engine, LOGIN, &ctx), 1201).is_empty(),
        "chain 1201 must only fire on a login into Castle (world 8)",
    );
}

/// Sanity on the helper that every other test in this module leans on:
/// the step helper writes the key `populate_mission_context` writes.
/// If these drift the negative tests would pass for the wrong reason.
#[tokio::test]
async fn step_gate_key_matches_the_runtime_populator() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1202).await;

    let mut ctx = interact_ctx("Castle_SgtGerschon", NON_JAFFA);
    // Deliberately DO NOT set the step param: the evaluator's
    // `unwrap_or("not_active")` fallback must make the gate pass, which
    // is the real "player has never touched 701" state.
    assert_eq!(
        summarized(&fire(&engine, INTERACT, &ctx), 1202),
        vec!["display_dialog(2573)"],
        "an absent step param must read as not_active",
    );

    with_step(&mut ctx, 701, 2399, "completed");
    assert!(
        summarized(&fire(&engine, INTERACT, &ctx), 1202).is_empty(),
        "a completed step 2399 must not re-offer the mission",
    );
}

/// Documents where a player with no `archetype_id` lands.
///
/// `fire_interact_tag` only writes the `archetype` param when
/// `entity.archetype_id` is `Some`, and the condition evaluator defaults a
/// missing archetype to -1. So `archetype neq 8` is TRUE and `archetype eq
/// 8` is FALSE: an archetype-less player gets the Human branch (dialog
/// 2573), not the Jaffa one, and never gets both or neither.
///
/// That is the right failure direction — a player with no archetype is a
/// broken-state edge case, and defaulting them into the Tau'ri script is
/// far better than a silent dead end at Gerschon — but it is a default,
/// not a decision anyone made. Pinned so a change to the evaluator's
/// default (or to `neq`'s handling of a missing key) surfaces here rather
/// than as "Jaffa players can't start 701".
#[tokio::test]
async fn player_without_an_archetype_gets_the_human_branch() {
    let pool = require_db_or_skip!();

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_SgtGerschon"),
    );
    // Deliberately no `archetype` param — the shape `fire_interact_tag`
    // produces for an entity whose `archetype_id` is None.
    with_step(&mut ctx, 701, 2399, "not_active");

    let human = engine_for(&pool, 1202).await;
    assert_eq!(
        summarized(&fire(&human, INTERACT, &ctx), 1202),
        vec!["display_dialog(2573)"],
        "`archetype neq 8` must hold for a missing archetype (evaluator \
         defaults it to -1), so the player gets the Human offer",
    );

    let jaffa = engine_for(&pool, 1203).await;
    assert!(
        summarized(&fire(&jaffa, INTERACT, &ctx), 1203).is_empty(),
        "`archetype eq 8` must not match a missing archetype — the player \
         must get exactly one branch, never both",
    );
}

/// Turn-in choice must not accept 701 a third time via some other
/// mission's dialog: chain 1204's trigger is dialog 2573 only.
#[tokio::test]
async fn accept_chain_ignores_unrelated_dialog_choices() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1204).await;

    let ctx = choice_at_step(2576, 2421, "active");
    assert!(
        summarized(&fire(&engine, CHOICE, &ctx), 1204).is_empty(),
        "the turn-in dialog must not re-trigger the accept chain",
    );
}
