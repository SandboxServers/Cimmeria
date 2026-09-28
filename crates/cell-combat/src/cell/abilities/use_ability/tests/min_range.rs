//! #1016: a player's cast is refused inside the ability's `min_range`.
//!
//! Turret 1205 carries `min_range = 300` UE3 units, 3 m. Before #1016 the
//! launch read only `max_range`, so the cast fired at point-blank range; the
//! 2009 Python reference (`AbilityManager.py:561`) refused it with the
//! out-of-range feedback. The warmup fire-time re-check is in
//! `warmup_interrupt.rs`.

use super::*;
use cimmeria_entity::abilities::ability_range_to_metres;

/// A 30 m ability with the turret's 3 m minimum.
fn min_range_ability() -> AbilityDef {
    let mut def = make_ability(1205, 0, 30);
    def.min_range = ability_range_to_metres(300);
    def
}

/// Revert proof: without the minimum the cast at 1 m passes the range check.
#[tokio::test]
async fn min_range_3m_refuses_a_player_target_at_1m() {
    assert!(
        fire_at_hostile(&min_range_ability(), 1.0).await,
        "a target at 1 m is inside the 3 m minimum: onErrorCode 42"
    );
}

#[tokio::test]
async fn min_range_3m_allows_a_player_target_at_5m() {
    assert!(
        !fire_at_hostile(&min_range_ability(), 5.0).await,
        "a target at 5 m is between the 3 m minimum and the 30 m reach"
    );
}

/// The refusal leaves the negative-log row the runbook greps for.
#[tokio::test]
async fn min_range_refusal_logs_reason_and_identity() {
    let logs = crate::test_support::LogCapture::install();
    assert!(fire_at_hostile(&min_range_ability(), 1.0).await);
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "abilities" && c.has_field("reason", "target_too_close"))
        .collect();
    assert_eq!(rows.len(), 1, "one refusal row; got {rows:#?}");
    let row = &rows[0];
    assert_eq!(row.level, tracing::Level::DEBUG, "{row:?}");
    assert!(row.has_field("event", "cast_refused"), "{row:?}");
    assert!(row.has_field("phase", "launch"), "{row:?}");
    assert!(row.has_field("player_id", "101"), "{row:?}");
    assert!(row.has_field("account_id", "901"), "{row:?}");
}
