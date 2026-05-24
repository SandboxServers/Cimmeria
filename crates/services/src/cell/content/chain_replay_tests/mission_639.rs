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
