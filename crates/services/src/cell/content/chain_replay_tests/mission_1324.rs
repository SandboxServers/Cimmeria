//! Mission 1324 — Present Yourself (packet H20,
//! `docs/analysis/harset-rebuild/work-packets.md#h20`, seed
//! `harset_jaffa_chains.sql` chains 6301, 6303-6307).
//!
//! The first Loyalist Jaffa mission on Harset: Moh'katan (template 54,
//! tag `CmdCenter_Mohkatan`) offers it, Ba'al (template 42, tag
//! `CmdCenter_Baal`) hosts the council scene that satisfies step 3953,
//! and Moh'katan takes the turn-in on step 3954. Both NPCs stand in
//! world 68 `Harset_CmdCenter`, which is a SHARED space — so every quest
//! icon in this mission is a per-player `add_dialog_set` bind rather
//! than a `set_interaction_type`, and every bind has a `player_loaded`
//! chain that re-paints it after the cross-world hop or a relog.
//!
//! What these guards are actually protecting, in the order the risks
//! bite:
//!
//! 1. **The two click routes.** `fire_interact_tag` runs before
//!    `interactions::handle_interact` and short-circuits it when any
//!    chain matches — and `handle_interact` is the only write site for
//!    the player's `last_interaction_target` pin
//!    (`cell/interactions/dispatch/interact.rs`). A `display_dialog` on
//!    a follow-up trigger such as `dialog_choice` has no
//!    `target_entity_id` of its own and can only resolve its NPC through
//!    that pin, so an `interact_tag` chain covering the offer state
//!    would silently kill chain 6303's `display_dialog 4358`. That is
//!    why the offer has no `interact_tag` chain at all and
//!    [`clicking_mohkatan_in_the_offer_state_must_resolve_no_chain`]
//!    asserts the absence rather than a presence.
//! 2. **The archetype and world gates.** `interact_tag` does not filter
//!    by world, and the `world` condition fails closed only if it is
//!    present — dropping it is silent. Each positive here has a
//!    wrong-archetype, wrong-world and no-world counterpart.
//! 3. **State exclusivity on one NPC.** Moh'katan carries three chains
//!    in this mission plus 1326's (and later 1325's, 1343's…), all on
//!    one tag. Two of them resolving on the same click would push two
//!    `display_dialog` actions and the client would render only the
//!    second, whose button is then rejected by the #479 open-dialog
//!    gate. Every Moh'katan test asserts the *whole* engine resolved at
//!    most one `DisplayDialog`, not just that the expected chain fired.
//! 4. **`advance_step` vs `complete_objective`.** Step 3953's objective
//!    4543 is its only non-optional one, so a `complete_objective` there
//!    would complete the entire mission and orphan the return step
//!    (`cell/missions/progression.rs`, the all-required-complete
//!    branch). A structural guard asserts no chain in the packet's range
//!    uses that verb at all.
//! 5. **The bind pairing and its ordering.** Every `add_dialog_set` in
//!    the range has a matching `remove_dialog_set` on the chain that
//!    ends its state, the remove runs *before* any `complete_mission`
//!    (which awaits `fire_mission_completed` mid-list and lets a
//!    downstream chain bind the same slot), and every dsm id the chains
//!    name carries a non-zero `interaction_flags` — a zero merges
//!    nothing onto these templates' own `interaction_type = 0` and
//!    leaves the NPC unclickable with no error anywhere.
//!
//! Resolve-only is the right depth here (TESTING.md type 6): every verb
//! these chains use — `accept_mission`, `complete_mission`,
//! `advance_step`, `display_dialog`, `add_dialog_set`,
//! `remove_dialog_set` — has a shipped executor arm exercised by the
//! Castle Cellblock chains. H20 adds no arm.
//!
//! Chain-id filtering: assertions count only actions from chains
//! 6301-6345 (this file's seed range). `build_engine` loads every seed
//! file into one engine and the Goa'uld and OP-CORE lanes also register
//! `player_loaded 'Harset_CmdCenter'` chains, so a total count would
//! couple these guards to a sibling packet's authoring. The
//! *cross-engine* risks that do matter — two dialogs on one click, and
//! any chain at all matching the offer click — are asserted unfiltered.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// `archetype` id for Loyalist Jaffa. The Cellblock chains 1011/1012
/// split on the same value.
pub(super) const JAFFA: i64 = 8;
/// Any non-Jaffa archetype — used for the wrong-archetype negatives.
/// (3 is a Human archetype; the assertions only care that it is not 8.)
pub(super) const NOT_JAFFA: i64 = 3;

pub(super) const HARSET: i32 = 57;
pub(super) const CMD_CENTER: i32 = 68;
pub(super) const HARSET_NAME: &str = "Harset";
pub(super) const CMD_CENTER_NAME: &str = "Harset_CmdCenter";

/// Chain-id window owned by `harset_jaffa_chains.sql`.
pub(super) const JAFFA_CHAIN_RANGE: std::ops::RangeInclusive<i64> = 6301..=6345;

/// Build a context for a player of `archetype` standing in `world_id`.
///
/// `world_id` is the typed `ExecutionContext` field, not a param —
/// `Condition::World` reads it directly and fails closed when it is
/// `None`, which is what the no-world negatives exercise.
pub(super) fn ctx_for(archetype: i64, world_id: Option<i32>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = world_id;
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Set `mission_<id>_status`, the key `Condition::MissionStatus` reads.
pub(super) fn with_mission(mut ctx: ExecutionContext, id: i32, status: &str) -> ExecutionContext {
    ctx.set_param(format!("mission_{id}_status"), serde_json::json!(status));
    ctx
}

/// Set `mission_<id>_step_<step>_status`, the key
/// `Condition::StepStatus` reads.
pub(super) fn with_step(
    mut ctx: ExecutionContext,
    id: i32,
    step: i32,
    status: &str,
) -> ExecutionContext {
    ctx.set_param(
        format!("mission_{id}_step_{step}_status"),
        serde_json::json!(status),
    );
    ctx
}

/// Copy a context, adding one trigger-specific param.
///
/// `ExecutionContext` is deliberately not `Clone` (it carries entity
/// handles and accumulated `ActionResult`s), so the fire helpers rebuild
/// the parts a replay actually needs: the world id and the params map.
fn derive(ctx: &ExecutionContext, key: &str, value: serde_json::Value) -> ExecutionContext {
    let mut out = ExecutionContext::new();
    out.world_id = ctx.world_id;
    out.params = ctx.params.clone();
    out.set_param(key.to_string(), value);
    out
}

pub(super) fn fire_player_loaded(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    world_name: &str,
) -> ResolvedActions {
    let ctx = derive(ctx, "world_name", serde_json::json!(world_name));
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

pub(super) fn fire_interact_tag(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    tag: &str,
) -> ResolvedActions {
    let ctx = derive(ctx, "entity_tag", serde_json::json!(tag));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Replay the `mission_completed` dispatch.
///
/// `fire_mission_completed` (`content/event_dispatch/mission.rs`) runs
/// *after* `complete_mission_direct` has flipped the status, and
/// populates world, archetype and the whole mission context — so the
/// caller passes a context in which `mission_id` already reads
/// `completed`, exactly as the runtime would see it.
pub(super) fn fire_mission_completed(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    mission_id: i32,
) -> ResolvedActions {
    let ctx = derive(ctx, "mission_id", serde_json::json!(mission_id));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

pub(super) fn fire_dialog_choice(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    dialog_id: i32,
) -> ResolvedActions {
    let ctx = derive(ctx, "dialog_id", serde_json::json!(dialog_id));
    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Render `resolved` as `"<chain_id>:<verb>(<args>)"` labels, keeping
/// only the chains in `harset_jaffa_chains.sql`'s range.
///
/// Anything this packet can author but the label list does not name maps
/// to `OTHER`, so an accidentally-added action shows up as a diff rather
/// than passing silently.
pub(super) fn labels(resolved: &ResolvedActions) -> Vec<String> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| JAFFA_CHAIN_RANGE.contains(id))
        .map(|(id, action)| {
            let body = match action {
                Action::AcceptMission { mission_id } => format!("accept_mission({mission_id})"),
                Action::CompleteMission { mission_id } => format!("complete_mission({mission_id})"),
                Action::AdvanceStep {
                    mission_id,
                    step_id,
                } => format!("advance_step({mission_id}->{step_id})"),
                Action::CompleteObjective {
                    mission_id,
                    objective_id,
                } => format!("complete_objective({mission_id}/{objective_id})"),
                Action::DisplayDialog { dialog_id } => format!("display_dialog({dialog_id})"),
                Action::AddDialogSet {
                    dialog_set_id,
                    slot,
                    ..
                } => format!("add_dialog_set({dialog_set_id}@{slot})"),
                Action::RemoveDialogSet {
                    dialog_set_id,
                    slot,
                } => format!("remove_dialog_set({dialog_set_id}@{slot})"),
                other => format!("OTHER({other:?})"),
            };
            format!("{id}:{body}")
        })
        .collect()
}

/// Count `DisplayDialog` actions across the WHOLE engine, not just this
/// packet's chains.
///
/// Two chains resolving on one right-click both execute, and the second
/// `onDialogDisplay` overwrites the first on the client — the player
/// silently gets the wrong conversation and the top dialog's button is
/// rejected by the #479 gate. This assertion must stay unfiltered,
/// because the collision a sibling packet would introduce is exactly a
/// chain outside this range matching the same tag.
pub(super) fn display_dialog_count(resolved: &ResolvedActions) -> usize {
    resolved
        .actions
        .iter()
        .filter(|(_, a)| matches!(a, Action::DisplayDialog { .. }))
        .count()
}

/// A Jaffa who has never been offered 1324, standing in the Command
/// Center. Also pins 1325/1326 as un-started so 1326's offer chains
/// stay out of the way.
fn fresh_jaffa_in_command_center() -> ExecutionContext {
    let ctx = ctx_for(JAFFA, Some(CMD_CENTER));
    let ctx = with_mission(ctx, 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "not_active");
    with_mission(ctx, 1326, "not_active")
}

// ---------------------------------------------------------------
// Chain 6301 — the offer bind, and the absence that makes it work
// ---------------------------------------------------------------

/// Positive: entering the Command Center as a Jaffa who has not been
/// offered 1324 binds Moh'katan's "?" (dsm 5149, flags 134217728
/// `INT_NonAStoryMissionAvaliable`) and nothing else in the packet's
/// range.
///
/// This chain is the first paint, the relog restore AND the thing that
/// opens dialog 4357 when the player clicks — `fire_player_loaded` runs
/// on login, gate travel and the cross-world door alike.
#[tokio::test]
async fn entering_the_command_center_paints_mohkatans_offer_icon() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_player_loaded(&engine, &fresh_jaffa_in_command_center(), CMD_CENTER_NAME);

    assert_eq!(
        labels(&resolved),
        vec!["6301:add_dialog_set(5149@54)".to_string()],
        "a fresh Jaffa entering Harset_CmdCenter must bind exactly dsm \
         5149 on template 54 (Moh'katan). Anything else here means a \
         sibling 1324/1326 chain's condition set no longer excludes the \
         pre-accept state. Full resolve: {:?}",
        resolved.actions,
    );
}

/// The load-bearing ABSENCE: clicking Moh'katan while 1324 is on offer
/// must resolve NO chain anywhere in the engine.
///
/// This is the guard for the bug that nearly shipped. `fire_interact_tag`
/// returns `matched = !resolved.actions.is_empty()`, and
/// `cell_methods/player/interaction/interact.rs` calls
/// `interactions::handle_interact` only when that is false.
/// `handle_interact` is the sole write site for
/// `CellEntity::last_interaction_target`
/// (`cell/interactions/dispatch/interact.rs`), and chain 6303's
/// `display_dialog 4358` has no `target_entity_id` of its own — a
/// `dialog_choice` trigger does not stamp one — so the pin is the only
/// thing it can resolve its NPC through. Dialog 4358 is not a monologue
/// (its screens speak as 945), so with no pin the executor takes the
/// `warn!` branch and the briefing never reaches the client.
///
/// Add any `interact_tag 'CmdCenter_Mohkatan'` chain that matches the
/// offer state — however reasonable it looks — and the mission still
/// accepts while the conversation silently disappears. That failure
/// leaves no trace in the chain-resolution layer, which is why the guard
/// is here rather than in an executor test.
#[tokio::test]
async fn clicking_mohkatan_in_the_offer_state_must_resolve_no_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_interact_tag(
        &engine,
        &fresh_jaffa_in_command_center(),
        "CmdCenter_Mohkatan",
    );

    assert!(
        resolved.actions.is_empty(),
        "no chain may match a right-click on Moh'katan while 1324 is on \
         offer. A match here short-circuits `interactions::handle_interact`, \
         which never pins `last_interaction_target`, which makes chain \
         6303's `display_dialog 4358` bail with a warn — the mission \
         accepts and the briefing silently never plays. The offer dialog \
         must be opened by chain 6301's bind alone. Matched: {:?}",
        resolved.actions,
    );
}

/// Negative (wrong archetype): 1324 is Loyalist-Jaffa-only, so a Human
/// walking into the same room gets no icon. `archetype eq 8` is the gate;
/// without it, every faction would be offered the Jaffa chain — and
/// because the bind is what makes the NPC clickable at all, the bind is
/// the whole access control.
#[tokio::test]
async fn a_human_is_never_offered_1324() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(NOT_JAFFA, Some(CMD_CENTER)), 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "not_active");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME)).is_empty(),
        "archetype {NOT_JAFFA} must resolve no 1324/1326 chain on \
         Harset_CmdCenter entry"
    );
}

/// Negative (wrong world): the same Jaffa in the same mission state, but
/// loaded into Harset (57) rather than the Command Center (68).
///
/// Both halves of the gate are exercised: the trigger's own `world_name`
/// filter and the `world eq 68` condition. Dropping either is silent —
/// `OnPlayerLoaded` with a `None` world_name matches every world, and a
/// missing `world` condition would let a crafted context through.
#[tokio::test]
async fn the_offer_does_not_paint_in_harset_itself() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(HARSET)), 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "not_active");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, HARSET_NAME)).is_empty(),
        "1324's chains are all Command-Center-scoped; none may resolve on \
         a Harset (57) load"
    );
}

/// Negative (fail-closed): a dispatcher that never populated
/// `ExecutionContext.world_id` must not make a world-gated chain fire
/// everywhere. This is the H07 contract restated at the seed level —
/// `Condition::World` returns false for every operator when the world is
/// unknown.
#[tokio::test]
async fn the_offer_fails_closed_without_a_world_context() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, None), 1324, "not_active");
    let ctx = with_mission(ctx, 1325, "not_active");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME)).is_empty(),
        "with no world_id in context, every `world`-gated 1324 chain must \
         fail closed"
    );
}

/// Negative (already done): once 1324 is completed its offer icon must
/// never come back on a later Command Center entry — `mission_status
/// 1324 eq not_active` is what retires it.
#[tokio::test]
async fn a_completed_1324_never_re_offers() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "completed");
    let ctx = with_mission(ctx, 1325, "not_active");
    let ctx = with_mission(ctx, 1326, "not_active");

    assert!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_CENTER_NAME)).is_empty(),
        "a completed 1324 with 1325 not yet done must resolve nothing on \
         world entry — 6301 is retired and 6331 waits on 1325"
    );
}

// ---------------------------------------------------------------
// Chain 6303 — accept
// ---------------------------------------------------------------

/// Positive: the Accept button on 4357 accepts 1324, plays the
/// Castle-return conversation 4358, retires Moh'katan's "?" and paints
/// Ba'al's "!" — in that order.
///
/// Order is asserted, not just membership: `remove_dialog_set` before
/// `add_dialog_set` keeps at most one dsm bound per template, which is
/// the invariant that stops `entries.first()` in
/// `interactions/dispatch/interact.rs` handing a later state the wrong
/// dialog.
#[tokio::test]
async fn accepting_1324_moves_the_icon_from_mohkatan_to_baal() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_dialog_choice(&engine, &fresh_jaffa_in_command_center(), 4357);

    assert_eq!(
        labels(&resolved),
        vec![
            "6303:accept_mission(1324)".to_string(),
            "6303:display_dialog(4358)".to_string(),
            "6303:remove_dialog_set(5149@54)".to_string(),
            "6303:add_dialog_set(5151@42)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (re-accept guard): once 1324 is active the Accept chain must
/// not fire again. `accept_mission`'s server-side offer guard refuses a
/// re-accept authoritatively, but without the condition the chain's
/// OTHER three actions would still run — re-showing 4358 and re-shuffling
/// the binds on every stray choice.
#[tokio::test]
async fn the_accept_chain_does_not_refire_once_1324_is_active() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "active");
    assert!(
        labels(&fire_dialog_choice(&engine, &ctx, 4357)).is_empty(),
        "chain 6303 must carry `mission_status 1324 eq not_active`"
    );
}

/// Negative: and not once it is completed either — the other non-
/// `not_active` state the mission lifecycle can be in.
#[tokio::test]
async fn the_accept_chain_does_not_refire_once_1324_is_completed() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "completed");
    assert!(
        labels(&fire_dialog_choice(&engine, &ctx, 4357)).is_empty(),
        "a completed 1324 must not be re-acceptable from a replayed choice"
    );
}

// ---------------------------------------------------------------
// Chain 6304 — the council scene
// ---------------------------------------------------------------

/// Positive: on step 3953, clicking Ba'al plays the 10-screen council
/// dialog 4363, advances to 3954, and hands the icon back to Moh'katan.
///
/// `advance_step`, never `complete_objective 4543` — see the structural
/// guard below for why.
#[tokio::test]
async fn the_baal_council_advances_1324_to_the_return_step() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "active");
    let ctx = with_step(ctx, 1324, 3953, "active");
    let resolved = fire_interact_tag(&engine, &ctx, "CmdCenter_Baal");

    assert_eq!(
        display_dialog_count(&resolved),
        1,
        "one dialog per click on Ba'al. Full resolve: {:?}",
        resolved.actions,
    );
    assert_eq!(
        labels(&resolved),
        vec![
            "6304:display_dialog(4363)".to_string(),
            "6304:advance_step(1324->3954)".to_string(),
            "6304:remove_dialog_set(5151@42)".to_string(),
            "6304:add_dialog_set(120001@54)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (wrong step): Ba'al has nothing to say once the council is
/// done. The step gate is also the one-shot guard — `content_triggers
/// .once` is dead code, so a second click would otherwise replay the
/// council and re-advance the step.
///
/// Driven over both shapes step 3953 takes after the advance: `completed`
/// in-session, and `not_active` after a relog — `advance_step` persists
/// `completed_step_ids: vec![]` through the outbox
/// (`content/executor/mission.rs`), so a finished step does NOT come back
/// as `completed`. A gate written against the wrong one of those would
/// pass a single-shape test and break on relog.
#[tokio::test]
async fn clicking_baal_again_after_the_council_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for after_advance in ["completed", "not_active"] {
        let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "active");
        let ctx = with_step(ctx, 1324, 3953, after_advance);
        let ctx = with_step(ctx, 1324, 3954, "active");

        assert!(
            labels(&fire_interact_tag(&engine, &ctx, "CmdCenter_Baal")).is_empty(),
            "chain 6304's `step_status 1324/3953 eq active` is the re-fire \
             guard; with step 3953 reading {after_advance:?} it must not \
             match"
        );
    }
}

/// Negative (wrong world): Ba'al's chain is gated on world 68 like the
/// rest. `interact_tag` carries no world filter of its own, so the
/// condition is the only thing standing between a crafted interact and
/// the chain.
#[tokio::test]
async fn clicking_baal_from_the_wrong_world_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(HARSET)), 1324, "active");
    let ctx = with_step(ctx, 1324, 3953, "active");

    assert!(
        labels(&fire_interact_tag(&engine, &ctx, "CmdCenter_Baal")).is_empty(),
        "the `world eq 68` condition must reject an interact raised from \
         world 57"
    );
}

/// Negative (wrong archetype) on the council click.
#[tokio::test]
async fn a_human_cannot_trigger_the_baal_council() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(NOT_JAFFA, Some(CMD_CENTER)), 1324, "active");
    let ctx = with_step(ctx, 1324, 3953, "active");

    assert!(
        labels(&fire_interact_tag(&engine, &ctx, "CmdCenter_Baal")).is_empty(),
        "chain 6304 must be archetype-gated"
    );
}

// ---------------------------------------------------------------
// Chain 6305 — the turn-in
// ---------------------------------------------------------------

/// Positive: on step 3954, clicking Moh'katan plays the debrief 4365,
/// clears his turn-in icon and completes the mission — in that order.
///
/// The order is the assertion that matters. `Action::CompleteMission`
/// awaits `fire_mission_completed` inside the executor arm
/// (`content/executor/mission.rs`), so a chain triggered by
/// `mission_completed '1324'` — H21 will add one to paint 1325's offer on
/// this same slot — runs its actions in the middle of this list. Removing
/// the old bind first means the incoming one is the only entry on slot 54
/// when the dust settles, rather than depending on `retain` behaviour
/// against a bind that did not exist when this chain was written.
///
/// `complete_mission` is correct here precisely because 3954 IS the
/// terminal step — the same verb on a mid-mission step would be the
/// early-completion bug the `advance_step` guard covers.
#[tokio::test]
async fn returning_to_mohkatan_completes_1324() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "active");
    let ctx = with_mission(ctx, 1325, "not_active");
    let ctx = with_mission(ctx, 1326, "not_active");
    let ctx = with_step(ctx, 1324, 3953, "completed");
    let ctx = with_step(ctx, 1324, 3954, "active");
    let resolved = fire_interact_tag(&engine, &ctx, "CmdCenter_Mohkatan");

    assert_eq!(
        display_dialog_count(&resolved),
        1,
        "the turn-in click must open exactly one dialog. Two here means \
         another chain no longer excludes the mid-mission state and the \
         player gets an offer blurb instead of the debrief. Full \
         resolve: {:?}",
        resolved.actions,
    );
    assert_eq!(
        labels(&resolved),
        vec![
            "6305:display_dialog(4365)".to_string(),
            "6305:remove_dialog_set(120001@54)".to_string(),
            "6305:complete_mission(1324)".to_string(),
        ],
        "the remove must precede the complete — see this test's doc \
         comment. Resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (already completed): a second click on Moh'katan after the
/// turn-in resolves nothing from 1324 — the step gate has flipped and
/// the offer chain is retired by `mission_status`. Both post-advance step
/// shapes are covered for the same reason as the Ba'al guard.
#[tokio::test]
async fn clicking_mohkatan_after_completing_1324_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for after in ["completed", "not_active"] {
        let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "completed");
        let ctx = with_mission(ctx, 1325, "not_active");
        let ctx = with_mission(ctx, 1326, "not_active");
        let ctx = with_step(ctx, 1324, 3953, after);
        let ctx = with_step(ctx, 1324, 3954, after);

        assert!(
            labels(&fire_interact_tag(&engine, &ctx, "CmdCenter_Mohkatan")).is_empty(),
            "1324 must be inert once completed (steps reading {after:?}); \
             1326's offer waits on 1325"
        );
    }
}

// ---------------------------------------------------------------
// Chains 6306 / 6307 — relog restore
// ---------------------------------------------------------------

/// Positive: a relog (or a walk back through the Command Center door)
/// mid-step re-paints exactly the icon that step owns, and only that one.
///
/// Binds live in `CellEntity::available_interactions`, which is in-memory
/// and destroyed on the cross-world hop — without these chains a player
/// who steps out of the Command Center and back in finds the NPC he is
/// supposed to talk to unclickable, with no error anywhere.
#[tokio::test]
async fn relog_restores_exactly_the_icon_the_active_step_owns() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Step 3953 active → Ba'al's "!" (dsm 5151 on template 42).
    let base = || {
        let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1324, "active");
        let ctx = with_mission(ctx, 1325, "not_active");
        with_mission(ctx, 1326, "not_active")
    };

    let on_3953 = with_step(
        with_step(base(), 1324, 3953, "active"),
        1324,
        3954,
        "not_active",
    );
    assert_eq!(
        labels(&fire_player_loaded(&engine, &on_3953, CMD_CENTER_NAME)),
        vec!["6306:add_dialog_set(5151@42)".to_string()],
        "mid-3953 relog must re-bind Ba'al and nothing else"
    );

    // Step 3954 active → Moh'katan's turn-in "?" (dsm 120001 on 54).
    // The outgoing step reads `not_active` after a relog, not
    // `completed`, so that is the shape used here.
    let on_3954 = with_step(
        with_step(base(), 1324, 3953, "not_active"),
        1324,
        3954,
        "active",
    );
    assert_eq!(
        labels(&fire_player_loaded(&engine, &on_3954, CMD_CENTER_NAME)),
        vec!["6307:add_dialog_set(120001@54)".to_string()],
        "mid-3954 relog must re-bind Moh'katan's turn-in and nothing else"
    );
}

// ---------------------------------------------------------------
// Structural guards over the seed rows themselves
// ---------------------------------------------------------------

/// No chain in `harset_jaffa_chains.sql` may use `complete_objective`.
///
/// Every step of 1324 and 1326 has exactly one non-optional objective
/// (`mission_objectives.sql:997,999,1009,1011`), and
/// `cell/missions/progression.rs::complete_objective` calls
/// `mission.complete()` as soon as every non-optional objective of the
/// current step is done. So `complete_objective` on ANY step of these
/// missions ends the whole mission: on a mid-mission step that orphans
/// the turn-in, and on the terminal step it duplicates
/// `complete_mission`. `advance_step` force-completes the outgoing
/// step's objectives on its way, so nothing is lost.
///
/// Asserted against the DB rather than the resolved actions because the
/// mistake is an authoring one — it would be introduced as a seed row,
/// possibly on a chain no replay test covers yet. The range covers the
/// whole file (6301-6500), not just H20/H22's chains, so a later packet
/// in the same file inherits the guard.
#[tokio::test]
async fn no_harset_jaffa_chain_completes_an_objective() {
    let pool = require_db_or_skip!();
    // `content_actions.chain_id` is `integer`, so the decode target is
    // `i32`. An `i64` here would compile and pass while the query
    // returns nothing, then blow up with a `ColumnDecode` error instead
    // of this test's assertion message on the day it actually catches
    // something.
    let rows: Vec<(i32,)> = sqlx::query_as(
        "SELECT chain_id FROM resources.content_actions \
         WHERE chain_id BETWEEN 6301 AND 6500 AND action_type = 'complete_objective' \
         ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query for complete_objective rows must succeed");

    assert!(
        rows.is_empty(),
        "chains {rows:?} use `complete_objective`. Every step in the \
         Harset Loyalist Jaffa missions has exactly one non-optional \
         objective, so completing it completes the whole mission \
         (cell/missions/progression.rs). Use `advance_step` between \
         steps and `complete_mission` on the terminal one.",
    );
}

/// No chain in this file may use `set_interaction_type`.
///
/// Worlds 57 and 68 are shared persistent spaces (D-H04:
/// `Harset_CmdCenter` is `Instanced="false"`), and the executor arm
/// mutates `CellEntity::interaction_type_flags` and fans the new value
/// to every witness. One player's clear would un-click Moh'katan for
/// every other player still on the step. The per-player primitive is
/// `add_dialog_set`, which writes only the firing player's
/// `available_interactions`. This is the campaign's "shared-hub NPCs are
/// never mutated globally" guardrail expressed as a test, and it is also
/// what the `interact_tag_linter` allowlist entries for this file assume.
#[tokio::test]
async fn no_harset_jaffa_chain_sets_a_global_interaction_bit() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, target_key FROM resources.content_actions \
         WHERE chain_id BETWEEN 6301 AND 6500 AND action_type = 'set_interaction_type' \
         ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query for set_interaction_type rows must succeed");

    assert!(
        rows.is_empty(),
        "chains {rows:?} set a global interaction bit on a shared-hub \
         NPC. `set_interaction_type` fans to every witness; use \
         `add_dialog_set` so the icon is per-player.",
    );
}

/// Every `complete_mission` in this file runs AFTER the chain's own
/// `remove_dialog_set` rows.
///
/// `Action::CompleteMission` awaits `fire_mission_completed` inside the
/// executor arm, so any `mission_completed`-triggered chain executes in
/// the middle of the completing chain's action list. If the remove ran
/// last it would fire after a downstream chain had already bound the
/// next mission's icon to the same slot, and the outcome would depend on
/// `retain` semantics against a bind the author never saw. Ordering the
/// removes first makes the result independent of what any other packet
/// hangs off the completion.
#[tokio::test]
async fn every_remove_dialog_set_precedes_its_chains_complete_mission() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT c.chain_id, \
                MAX(CASE WHEN c.action_type = 'remove_dialog_set' THEN c.sort_order END), \
                MIN(CASE WHEN c.action_type = 'complete_mission'  THEN c.sort_order END) \
         FROM resources.content_actions c \
         WHERE c.chain_id BETWEEN 6301 AND 6500 \
         GROUP BY c.chain_id \
         HAVING MAX(CASE WHEN c.action_type = 'remove_dialog_set' THEN c.sort_order END) IS NOT NULL \
            AND MIN(CASE WHEN c.action_type = 'complete_mission'  THEN c.sort_order END) IS NOT NULL \
         ORDER BY c.chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query for action ordering must succeed");

    assert!(
        !rows.is_empty(),
        "no chain in 6301-6500 both removes a dialog set and completes a \
         mission — this guard has lost its subject, which means the \
         turn-in chains (6305/6340) were renumbered or gutted"
    );

    for (chain_id, last_remove, first_complete) in rows {
        assert!(
            last_remove < first_complete,
            "chain {chain_id}: `remove_dialog_set` at sort_order \
             {last_remove} runs after `complete_mission` at \
             {first_complete}. Move the removes first — \
             `fire_mission_completed` is awaited inside the complete arm, \
             so a downstream chain binds the same slot in between."
        );
    }
}

/// Every dsm id these chains bind carries the bit it is bound FOR, and
/// carries a dialog exactly where a click has to open one.
///
/// Two independent failure modes, both silent:
///
/// * `interaction_flags = 0` merges nothing onto templates 42/54/204,
///   whose own `entity_templates.interaction_type` is 0. The chain
///   resolves, the action executes, and the NPC stays scenery — the
///   client never even sends the click. That is exactly why dsm 120001
///   exists instead of the shipped row 6791, which binds the right
///   dialog with zero flags.
/// * A missing `dialog_id` on a row that the OFFER states depend on.
///   The offer dialogs 4357 and 4373 are opened by Route B —
///   `handle_interact` walking `available_interactions` — so those two
///   rows must carry a dialog or the offer click does nothing. The
///   turn-in and mid-mission rows are opened by their own Route A
///   chains instead, so a dialog there is incidental.
///
/// The loader itself no longer discriminates: since Castle packet CA02,
/// `cell/spawner/dialogs.rs::load_dialog_set_maps` KEEPS NULL-dialog
/// rows as `dialog_id: None` so a bind can raise an indicator with no
/// dialog behind it. dsm 120002 is deliberately one of those; its guard
/// lives in [`super::mission_1326`].
#[tokio::test]
async fn every_bound_dialog_set_map_carries_its_bit_and_needed_dialog() {
    let pool = require_db_or_skip!();

    // (dsm_id, expected interaction_flags, must a click open a dialog
    // through Route B?) for every id bound by chains 6301-6345 except
    // 120002, which mission_1326.rs pins.
    const BOUND: [(i32, i64, bool); 5] = [
        (5149, 134_217_728, true), // 1324 offer "?"   INT_NonAStoryMissionAvaliable
        (5151, 268_435_456, false), // 1324 council "!" INT_NonAStoryMissionActive (chain 6304 opens it)
        (120_001, 536_870_912, false), // 1324 turn-in "?" INT_NonAStoryMissionTurnIn (chain 6305)
        (5159, 134_217_728, true),  // 1326 offer "?"
        (5161, 536_870_912, false), // 1326 turn-in "?" (chain 6340)
    ];

    for (dsm_id, expected_flags, route_b) in BOUND {
        let row: Option<(Option<i32>, i64)> = sqlx::query_as(
            "SELECT dialog_id, interaction_flags FROM resources.dialog_set_maps \
             WHERE dialog_set_map_id = $1",
        )
        .bind(dsm_id)
        .fetch_optional(&pool)
        .await
        .expect("dialog_set_maps query must succeed");

        let (dialog_id, flags) = row.unwrap_or_else(|| {
            panic!("dialog_set_map {dsm_id} must exist — chains 6301-6345 bind it")
        });

        assert_eq!(
            flags, expected_flags,
            "dialog_set_map {dsm_id} must carry interaction_flags \
             {expected_flags}. Templates 42/54/204 all have \
             `entity_templates.interaction_type = 0`, so a zero here \
             leaves the NPC unclickable with no error anywhere"
        );
        if route_b {
            assert!(
                dialog_id.is_some(),
                "dialog_set_map {dsm_id} backs an OFFER state, whose \
                 dialog is opened by `handle_interact` walking the \
                 player's binds (Route B). With a NULL dialog_id that \
                 walk steps over the row and the offer click does \
                 nothing — no chain, no dialog, no log line."
            );
        }
    }
}
