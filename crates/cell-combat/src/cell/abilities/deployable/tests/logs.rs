//! Deployable telemetry (TESTING.md type 12): a refusal logs its `reason`
//! at DEBUG (a client picks its point and its timing at will), and the
//! spawn, each pulse and the despawn log the owner's identity, the ability
//! and the object, with the despawn carrying the damage totals.

use tracing::Level;

use crate::test_support::{Captured, LogCapture};

use super::*;

fn rows(all: &[Captured], target: &str, event: &str) -> Vec<Captured> {
    all.iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .cloned()
        .collect()
}

fn assert_owner_identity(c: &Captured) {
    assert!(c.has_field("owner_id", &OWNER.to_string()), "{c:?}");
    // `add_pet_owner`: account = entity id, player = entity id + 1000.
    assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
    assert!(
        c.has_field("player_id", &(OWNER + 1000).to_string()),
        "{c:?}"
    );
    assert!(
        c.has_field("ability_id", &DEPLOYABLE_ABILITY.to_string()),
        "{c:?}"
    );
}

/// Every refusal the launch answers logs `deploy_refused` at DEBUG with its
/// `reason`, the owner's identity and the point it was given.
#[tokio::test]
async fn refusals_log_their_reason_at_debug() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let logs = LogCapture::install();

    let far = [OWNER_POS[0] + 600.0, 0.0, OWNER_POS[2]];
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, far, &tx, &mut mgr).await;
    handle_use_ability_on_ground(
        OWNER,
        DEPLOYABLE_ABILITY,
        [f32::NAN, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;

    let refused = rows(&logs.all(), "deployables.lifecycle", "deploy_refused");
    let reasons: Vec<String> = refused
        .iter()
        .map(|c| c.fields.get("reason").cloned().unwrap_or_default())
        .collect();
    assert_eq!(reasons, ["out_of_range", "not_finite", "busy"]);
    for c in &refused {
        assert_eq!(
            c.level,
            Level::DEBUG,
            "client-driven refusals are DEBUG: {c:?}"
        );
        assert_owner_identity(c);
        assert!(c.fields.contains_key("ground"), "{c:?}");
    }
}

/// Spawn (INFO), each pulse (DEBUG) and the despawn (INFO) carry the
/// owner's identity and the object id; the despawn names its `reason` and
/// the totals.
#[tokio::test]
async fn spawn_pulse_and_despawn_carry_the_owner_and_the_totals() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(4096);
    let logs = LogCapture::install();
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, [20.0, 0.0, 10.0]);
    let spawned_at = mgr.deployables.get(d).unwrap().spawned_at;
    for n in 1..=30 {
        crate::cell::abilities::deployable_tick_at(
            spawned_at + Duration::from_secs(n),
            &tx,
            &mut mgr,
            &NoContentEvents,
        )
        .await;
    }
    let all = logs.all();

    let spawned = rows(&all, "deployables.lifecycle", "spawned");
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].level, Level::INFO);
    assert_owner_identity(&spawned[0]);
    assert!(spawned[0].has_field("entity_id", &d.to_string()));
    assert!(spawned[0].has_field("pulses_total", "30"));

    let pulses = rows(&all, "deployables.pulse", "pulse");
    assert_eq!(pulses.len(), 30);
    for p in &pulses {
        assert_eq!(p.level, Level::DEBUG);
        assert_owner_identity(p);
        assert!(p.has_field("targets", "1"), "{p:?}");
    }

    let gone = rows(&all, "deployables.lifecycle", "despawned");
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].level, Level::INFO);
    assert_owner_identity(&gone[0]);
    assert!(gone[0].has_field("reason", "expired"));
    assert!(gone[0].has_field("pulses", "30"));
    assert!(gone[0].has_field("hits", "30"));
    assert!(gone[0].fields.contains_key("focus_damage"));
    assert!(gone[0].fields.contains_key("health_damage"));
}
