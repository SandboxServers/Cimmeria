//! Ability mechanics AB-09a: a stunned or knocked-down player cannot use a
//! consumable (python's `PLAYER_STATE_Stun`, "no ability/item use"). A
//! child of `consumable_use_tests`, for its fixture.

use std::time::Instant;

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};

use super::super::consumable_use::INCAPACITATED_TEXT;
use super::*;

/// **Regression guard.** A stun's ledger entry on the user refuses a
/// slappack with the feedback line and consumes nothing; once it comes
/// off, the same use goes to the base. Without the check the use is sent
/// for consumption while stunned (`a stunned use consumes nothing` fails).
#[tokio::test]
async fn a_stunned_player_cannot_use_a_consumable() {
    let mut mgr = mgr();
    set_pool(&mut mgr, HEALTH, 200);
    let stun = TimedEffectSpec {
        effect_id: 1599,
        ability_id: 1355,
        invoker_id: 9,
        state_flags: crate::cell::combat::BSF_MOVEMENT_LOCK,
        duration_secs: Some(5.0),
        stacking: TimedStacking::PerSource,
        ..Default::default()
    };
    mgr.apply_timed_effect(PLAYER, stun, Instant::now())
        .unwrap();

    let sent = use_item(&mut mgr, &ChainEngine::new(), SLAPPACK).await;
    assert!(consumes(&sent).is_empty(), "a stunned use consumes nothing");
    let calls = method_calls(&sent);
    assert_eq!(calls.len(), 1, "the feedback line only: {calls:?}");
    assert_eq!(calls[0].1, &feedback_line(INCAPACITATED_TEXT));

    let _ = mgr.remove_timed_effects(
        PLAYER,
        crate::cell::effects::stat_buff::StatBuffRemoval::Expired,
        |_| true,
    );
    let sent = use_item(&mut mgr, &ChainEngine::new(), SLAPPACK).await;
    assert_eq!(consumes(&sent).len(), 1, "free again");
}
