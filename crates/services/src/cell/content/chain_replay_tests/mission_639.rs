//! Mission 639 / 640 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
//! `item_use 19` consumes the ambernol vial, completes mission 639, and
//! accepts mission 640. The `remove_item` action is the load-bearing piece
//! — without it, the player keeps the vial after using it (and any chain
//! gated on "no longer holds vial" stays stuck).

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
