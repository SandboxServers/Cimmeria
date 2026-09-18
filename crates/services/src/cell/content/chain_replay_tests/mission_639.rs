//! Mission 639 / 640 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
//! `item_use 19` consumes the ambernol vial, completes mission 639, and
//! accepts mission 640. The `remove_item` action is the load-bearing piece
//! — without it, the player keeps the vial after using it (and any chain
//! gated on "no longer holds vial" stays stuck).
//!
//! Chain 1032 is the vial-grab encounter setup: pick up the vial, destroy
//! it on the world, wake the prisoner-retrieval drone, focus its threat on
//! the triggering player, play Net'an's reaction VO, fire the cinematic,
//! advance the mission step. The seven-action ordering is canonical per
//! `python/cell/missions/Castle_CellBlock/FindAmbernol.py`.

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// C03: `launch_ability 1374` (Cure Stasis Sickness) was added to chain
/// 1034 as the first action, before `remove_item`. Investigated whether
/// this was redundant: item 19's `items_event_sets` binding `(2, 19, 1374,
/// 5)` is loaded into `space_mgr.item_event_set_abilities` but read ONLY
/// by the weapon-ability-resolution helpers in
/// `crates/services/src/cell/abilities/resolve.rs`, both of which require
/// the item to be in the ACTIVE BANDOLIER SLOT — a consumable vial used
/// from the mission-item container never reaches that code path. This test
/// pins that chain 1034 is therefore the only place ability 1374 actually
/// fires.
#[tokio::test]
async fn chain_1034_launches_cure_ability_1374() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1034)
        .await
        .expect("DB query for chain 1034 must succeed")
        .expect("chain 1034 must exist in seeded content_chains");

    assert!(
        chain.actions.iter().any(|a| matches!(
            a,
            Action::LaunchAbility {
                ability_id: 1374,
                entity_tag: None
            }
        )),
        "chain 1034 must launch ability 1374 (Cure Stasis Sickness, self) \
         when the ambernol vial is used -- the items_event_sets binding \
         for item 19 is unreachable from the consumable-use path, so this \
         chain is the only place 1374 fires. Actions: {:?}",
        chain.actions,
    );

    // Ordering: launch_ability 1374 must run before remove_item 19 (see
    // the seed comment on chain 1034) -- not load-bearing mechanically
    // (both effects are complete no-ops today, see chain 1112's
    // investigation), but pins the intuitive "cast the cure, then consume
    // the reagent" order against an accidental future re-sort.
    let launch_idx = chain
        .actions
        .iter()
        .position(|a| {
            matches!(
                a,
                Action::LaunchAbility {
                    ability_id: 1374,
                    ..
                }
            )
        })
        .expect("launch_ability 1374 must be present");
    let remove_idx = chain
        .actions
        .iter()
        .position(|a| matches!(a, Action::RemoveItem { item_id: 19, .. }))
        .expect("remove_item 19 must be present");
    assert!(
        launch_idx < remove_idx,
        "chain 1034 must launch ability 1374 BEFORE consuming the vial; \
         got launch at index {launch_idx}, remove at index {remove_idx}. \
         Actions: {:?}",
        chain.actions,
    );
}

/// C03 (decision D-CB04): chain 1112 restores the Python's unconditional
/// `player.loaded` → `launch_ability 1372` (Stasis Sickness - Stage 1),
/// gated `mission_status 639 neq completed` -- the idempotence stop the
/// 2009 script never needed. Positive: fires while 639 is `not_active`
/// (the very first load, before the mission even starts) and `active`
/// (mid-mission relog). Negative: does NOT fire once 639 is `completed`
/// (relog after cure must not re-apply).
///
/// Asserts inside the helper (rather than returning the resolved actions)
/// so `require_db_or_skip!`'s bare `return;` stays valid — every caller is
/// a `#[tokio::test]` fn returning `()`.
async fn assert_1112_player_loaded_resolves(mission_639_status: &str, should_fire: bool) {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::ChainEngine;
    use cimmeria_content_engine::context::ExecutionContext;
    use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1112)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain 1112 must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain 1112 must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_639_status".to_string(),
        serde_json::json!(mission_639_status),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let fired: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1112)
        .map(|(_, a)| a)
        .collect();
    let launches_1372 = fired.iter().any(|a| {
        matches!(
            a,
            Action::LaunchAbility {
                ability_id: 1372,
                entity_tag: None
            }
        )
    });
    assert_eq!(
        launches_1372, should_fire,
        "chain 1112 with mission_639_status={mission_639_status:?} expected \
         to launch ability 1372 = {should_fire}; got actions {fired:?}"
    );
}

#[tokio::test]
async fn chain_1112_launches_stasis_sickness_when_not_active() {
    assert_1112_player_loaded_resolves("not_active", true).await;
}

#[tokio::test]
async fn chain_1112_launches_stasis_sickness_when_active() {
    assert_1112_player_loaded_resolves("active", true).await;
}

#[tokio::test]
async fn chain_1112_does_not_relaunch_once_cured() {
    assert_1112_player_loaded_resolves("completed", false).await;
}

/// Chain 1034: regression guard for an actual production bug — the seed
/// file was updated to add the `remove_item` action, but a stale local
/// DB without a re-seed surfaced as "ambernol use no longer removes the
/// vial". This test would have failed in CI on the broken seed.
#[tokio::test]
async fn chain_1034_includes_remove_item_for_ambernol() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1034)
        .await
        .expect("DB query for chain 1034 must succeed")
        .expect("chain 1034 must exist in seeded content_chains");

    let remove_vial_actions = chain
        .actions
        .iter()
        .filter(
            |a| matches!(a, Action::RemoveItem { item_id, count } if *item_id == 19 && *count == 1),
        )
        .count();
    // Pin `== 1` rather than `>= 1` so a future seed change that
    // accidentally duplicates the remove action (causing a stack-of-2
    // vials to vanish in one use) fails this guard.
    assert_eq!(
        remove_vial_actions, 1,
        "chain 1034 must include exactly one `RemoveItem {{ item_id: 19, count: 1 }}` \
         so the ambernol vial is consumed on use; got {remove_vial_actions} \
         matching actions. Full action list: {:?}",
        chain.actions,
    );
}

/// Chain 1032: the seven-action vial-grab sequence. Replay guard pinned to
/// the canonical ordering from `FindAmbernol.py:21-35 + :151-159`:
///
/// 1. `add_item 19`        — `inventory.pickedUpItem(19, 1)`
/// 2. `destroy_entity`     — `destroyCellEntity(vial)`
/// 3. `set_aggression 1`   — `drone.setAggression(1)` (durable behavior bit)
/// 4. `generate_threat`    — `drone.threatGenerated(player, 1000)` (focuses drone)
/// 5. `display_dialog 2297`— Net'an's reaction VO
/// 6. `play_sequence 10001`— cinematic
/// 7. `advance_step 2144`  — mission step transition
///
/// Two of these were historically missing (`generate_threat` and
/// `display_dialog`) — they were dropped during the auto-conversion of the
/// Python chain and surfaced as "drone doesn't aggro until 2s later" and
/// "Net'an's line never plays" in the encounter walkthrough. This test
/// guards against regression of the curated re-insert; the bug shape the
/// guard catches is a future seed change that drops either action back.
#[tokio::test]
async fn chain_1032_seven_actions_match_python_ordering() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1032)
        .await
        .expect("DB query for chain 1032 must succeed")
        .expect("chain 1032 must exist in seeded content_chains");

    // Build a label-only signature so a future addition of a sibling
    // action surfaces as a mismatch on this list rather than on a
    // per-action structural comparison (which would also break on cosmetic
    // changes like swapping container ID or threat magnitude).
    let signature: Vec<&'static str> = chain
        .actions
        .iter()
        .map(|a| match a {
            Action::GrantItem { .. } => "add_item",
            Action::DestroyTaggedEntity { .. } => "destroy_entity",
            Action::SetAggression { .. } => "set_aggression",
            Action::GenerateThreat { .. } => "generate_threat",
            Action::DisplayDialog { .. } => "display_dialog",
            Action::PlaySequence { .. } => "play_sequence",
            Action::AdvanceStep { .. } => "advance_step",
            _ => "OTHER",
        })
        .collect();

    let expected = vec![
        "add_item",
        "destroy_entity",
        "set_aggression",
        "generate_threat",
        "display_dialog",
        "play_sequence",
        "advance_step",
    ];
    assert_eq!(
        signature, expected,
        "chain 1032 action ordering drifted from the canonical Python sequence. \
         Got {signature:?}, expected {expected:?}. \
         Full action list: {:?}",
        chain.actions,
    );

    // Per-action invariants the encounter relies on:
    //
    // - SetAggression must target the drone tag with level=1. Without the
    //   level, auto-aggro never kicks in; without the tag, the wrong NPC
    //   gets the behavior bit.
    let aggression = chain
        .actions
        .iter()
        .find_map(|a| match a {
            Action::SetAggression { entity_tag, level } => Some((entity_tag.as_str(), *level)),
            _ => None,
        })
        .expect("chain 1032 must include a SetAggression action");
    assert_eq!(
        aggression,
        ("ArmYourself_PrisonerRetrievalUnit", 1),
        "chain 1032 SetAggression must target the drone with level=1",
    );

    // - GenerateThreat must target the drone tag with magnitude 1000 — this
    //   is what focuses the drone on the player who grabbed the vial rather
    //   than whichever player happens to be closest (auto-aggro from
    //   set_aggression alone picks the closest, which is wrong in a party).
    let threat = chain
        .actions
        .iter()
        .find_map(|a| match a {
            Action::GenerateThreat {
                entity_tag,
                threat_level,
            } => Some((entity_tag.as_deref(), *threat_level)),
            _ => None,
        })
        .expect("chain 1032 must include a GenerateThreat action");
    assert_eq!(
        threat,
        (Some("ArmYourself_PrisonerRetrievalUnit"), 1000),
        "chain 1032 GenerateThreat must focus the drone with threat=1000",
    );

    // - DisplayDialog must be dialog 2297 (Net'an's reaction line).
    let dialog_id = chain
        .actions
        .iter()
        .find_map(|a| match a {
            Action::DisplayDialog { dialog_id } => Some(*dialog_id),
            _ => None,
        })
        .expect("chain 1032 must include a DisplayDialog action");
    assert_eq!(
        dialog_id, 2297,
        "chain 1032 DisplayDialog must reference dialog 2297 (Net'an reaction)",
    );
}
