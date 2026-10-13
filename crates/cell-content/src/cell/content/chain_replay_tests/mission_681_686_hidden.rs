//! Seed regression for the hidden Cellblock guard controller missions.

use crate::test_support::require_db_or_skip;

/// All six guard controllers stay out of the client tracker (#1688).
#[tokio::test]
async fn live_db_cellblock_guard_controllers_are_hidden() {
    let pool = require_db_or_skip!();
    let defs = crate::cell::spawner::load_mission_defs(&pool)
        .await
        .expect("mission defs must load");
    for mission_id in 681..=686 {
        assert!(
            defs.get(&mission_id).is_some_and(|def| def.is_hidden),
            "Cellblock controller mission {mission_id} must be hidden",
        );
    }
}
