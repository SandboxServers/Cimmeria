//! The server-side gates on `callForAid` (67) and `respawn` (70):
//! a living caller is refused, and `callForAid` to a respawner the Defeat
//! Window did not offer (another world's id) is refused.
//!
//! Bug shape: without the dead gate, a living player's forged packet ran the
//! full respawn (full heal, cooldowns cleared, threat dropped, moved). Without
//! the offer check, a forged foreign-world id took `handle_respawn`'s
//! cross-world branch, destroyed the entity and sent `GateTravel`.

use super::*;
use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::spawner::RespawnerDef;
use crate::test_support::{LogCapture, LogCaptureGuard};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tracing::Level;

const WORLD: &str = "Castle_CellBlock";
const SAME_WORLD_ID: i32 = 11;
const SAME_WORLD_POS: [f32; 3] = [5.0, 6.0, 7.0];
const OTHER_WORLD_ID: i32 = 22;
const START_POS: [f32; 3] = [42.0, 1.0, 17.0];
const COOLDOWN_ABILITY: i32 = 592;
const THREAT_MOB: u32 = 50;

/// Player 1 in `WORLD` with one same-world respawner, one other-world
/// respawner, partial HEALTH/FOCUS, a running cooldown and a threatening
/// mob, so every piece of state the respawn resets is observable.
fn fixture(dead: bool) -> SpaceManager {
    let mut mgr = make_mgr_with_player(WORLD);
    mgr.respawners.push(RespawnerDef {
        respawner_id: SAME_WORLD_ID,
        world_name: WORLD.to_string(),
        name: "Cellblock Checkpoint".to_string(),
        pos: SAME_WORLD_POS,
    });
    mgr.respawners.push(RespawnerDef {
        respawner_id: OTHER_WORLD_ID,
        world_name: "Agnos".to_string(),
        name: "Agnos Checkpoint".to_string(),
        pos: [100.0, 200.0, 300.0],
    });
    let e = mgr.get_entity_mut(1).unwrap();
    e.account_id = Some(7);
    if let Some(h) = e.stats.get_mut(HEALTH) {
        h.update(0, 10, 100);
        h.clear_dirty();
    }
    if let Some(f) = e.stats.get_mut(FOCUS) {
        f.update(0, 5, 50);
        f.clear_dirty();
    }
    e.abilities
        .start_ability_cooldown(COOLDOWN_ABILITY, std::time::Duration::from_secs(60));
    e.threatened_mobs.insert(THREAT_MOB);
    if dead {
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_MOVEMENT_LOCK);
    }
    mgr
}

/// Every piece of state `handle_respawn` resets is still as the fixture
/// left it.
fn assert_untouched(mgr: &SpaceManager, state_field_before: u32) {
    let e = mgr
        .get_entity(1)
        .expect("a refused respawn must leave the entity in place (no GateTravel teardown)");
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 10, "HEALTH must not heal");
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 5, "FOCUS must not refill");
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        START_POS,
        "a refused respawn must not move the player"
    );
    assert_eq!(e.state_field, state_field_before, "state flags unchanged");
    assert!(
        e.abilities.is_on_cooldown(COOLDOWN_ABILITY),
        "a refused respawn must not clear cooldowns"
    );
    assert!(
        e.threatened_mobs.contains(&THREAT_MOB),
        "a refused respawn must not drop threat"
    );
}

fn assert_nothing_sent(rx: &mut mpsc::Receiver<CellToBaseMsg>) {
    let mut sent = Vec::new();
    while let Ok(m) = rx.try_recv() {
        sent.push(format!("{m:?}"));
    }
    assert!(
        sent.is_empty(),
        "a refused respawn must send nothing (no onEndAidWait, ReanchorPlayer, GateTravel); got {sent:#?}"
    );
}

/// Exactly one DEBUG `player.respawn` row with this `reason`, carrying the
/// caller's identity and the method.
fn assert_one_refusal(capture: &LogCaptureGuard, reason: &str, method: &str) {
    let hits: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("reason", reason))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one reason={reason} row; got {:#?}",
        capture.all()
    );
    let ev = &hits[0];
    assert_eq!(
        ev.level,
        Level::DEBUG,
        "client-input refusal logs at DEBUG: {ev:#?}"
    );
    assert_eq!(ev.target, "player.respawn", "{ev:#?}");
    assert!(ev.has_field("method", method), "{ev:#?}");
    assert!(ev.has_field("entity_id", "1"), "{ev:#?}");
    assert!(ev.has_field("account_id", "7"), "{ev:#?}");
    assert!(ev.has_field("player_id", "100"), "{ev:#?}");
}

async fn call_for_aid(mgr: &mut SpaceManager, id: i32) -> mpsc::Receiver<CellToBaseMsg> {
    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(64);
    assert!(dispatch(1, CALL_FOR_AID, &id.to_le_bytes(), &tx, mgr, &engine).await);
    rx
}

async fn respawn(mgr: &mut SpaceManager) -> mpsc::Receiver<CellToBaseMsg> {
    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(64);
    assert!(dispatch(1, RESPAWN, &[], &tx, mgr, &engine).await);
    rx
}

#[tokio::test]
async fn living_player_call_for_aid_is_refused_no_heal_no_move() {
    let mut mgr = fixture(false);
    let before = mgr.get_entity(1).unwrap().state_field;
    let capture = LogCapture::install();

    let mut rx = call_for_aid(&mut mgr, SAME_WORLD_ID).await;

    assert_untouched(&mgr, before);
    assert_nothing_sent(&mut rx);
    assert_one_refusal(&capture, "respawn_not_dead", "callForAid");
}

#[tokio::test]
async fn living_player_respawn_is_refused_no_heal_no_move() {
    let mut mgr = fixture(false);
    let before = mgr.get_entity(1).unwrap().state_field;
    let capture = LogCapture::install();

    let mut rx = respawn(&mut mgr).await;

    assert_untouched(&mgr, before);
    assert_nothing_sent(&mut rx);
    assert_one_refusal(&capture, "respawn_not_dead", "respawn");
}

/// The dead gate runs first, so a living forger naming another world's
/// respawner is refused as not dead, and never reaches the cross-world
/// branch.
#[tokio::test]
async fn living_player_call_for_aid_to_other_world_does_not_gate_travel() {
    let mut mgr = fixture(false);
    let before = mgr.get_entity(1).unwrap().state_field;
    let capture = LogCapture::install();

    let mut rx = call_for_aid(&mut mgr, OTHER_WORLD_ID).await;

    assert_untouched(&mgr, before);
    assert_nothing_sent(&mut rx);
    assert_one_refusal(&capture, "respawn_not_dead", "callForAid");
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("reason", "respawner_not_offered")),
        "the dead gate must run before the offer check"
    );
}

/// A dead player naming a respawner the Defeat Window did not offer (it is
/// in another world) stays dead, in place, with the Defeat Window open.
#[tokio::test]
async fn dead_player_call_for_aid_to_unoffered_respawner_is_refused() {
    let mut mgr = fixture(true);
    let before = mgr.get_entity(1).unwrap().state_field;
    assert_ne!(before & BSF_DEAD, 0, "fixture sanity: player is dead");
    let capture = LogCapture::install();

    let mut rx = call_for_aid(&mut mgr, OTHER_WORLD_ID).await;

    assert_untouched(&mgr, before);
    assert_nothing_sent(&mut rx);
    assert_one_refusal(&capture, "respawner_not_offered", "callForAid");
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("reason", "respawner_not_offered"))
        .unwrap();
    assert!(
        ev.has_field("respawner_id", &OTHER_WORLD_ID.to_string()),
        "{ev:#?}"
    );
    assert!(
        ev.has_field("world", &format!("{:?}", Some(WORLD))),
        "must carry the caller's world: {ev:#?}"
    );
}

/// Positive control: the gates are not over-broad. A dead player's
/// `callForAid` to an offered id, to 0 (the synthetic "Respawn Point") and
/// the timer-expiry `respawn()` all still revive.
#[tokio::test]
async fn dead_player_call_for_aid_and_respawn_still_revive() {
    for (label, call) in [
        ("callForAid(offered)", Some(SAME_WORLD_ID)),
        ("callForAid(0)", Some(0)),
        ("respawn()", None),
    ] {
        let mut mgr = fixture(true);
        let capture = LogCapture::install();

        let mut rx = match call {
            Some(id) => call_for_aid(&mut mgr, id).await,
            None => respawn(&mut mgr).await,
        };

        let e = mgr
            .get_entity(1)
            .unwrap_or_else(|| panic!("{label}: same-world respawn keeps the entity"));
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            100,
            "{label}: HEALTH at max"
        );
        assert_eq!(e.state_field & BSF_DEAD, 0, "{label}: BSF_DEAD cleared");
        assert_eq!(
            [e.position.x, e.position.y, e.position.z],
            SAME_WORLD_POS,
            "{label}: moved to the resolved same-world respawner"
        );
        let mut saw_reanchor = false;
        while let Ok(m) = rx.try_recv() {
            if matches!(m, CellToBaseMsg::ReanchorPlayer { .. }) {
                saw_reanchor = true;
            }
            assert!(
                !matches!(m, CellToBaseMsg::GateTravel { .. }),
                "{label}: same-world respawn must not gate-travel"
            );
        }
        assert!(
            saw_reanchor,
            "{label}: respawn burst reached ReanchorPlayer"
        );
        assert!(
            !capture
                .all()
                .iter()
                .any(|c| c.has_field("reason", "respawn_not_dead")
                    || c.has_field("reason", "respawner_not_offered")),
            "{label}: a legitimate respawn must not log a refusal"
        );
    }
}

/// Release clicked as the timer expires: the client sends 67 then 70. The
/// first revives; the second finds `BSF_DEAD` cleared and is refused rather
/// than running a second full reanchor.
#[tokio::test]
async fn release_then_timer_expiry_race_respawns_once() {
    let mut mgr = fixture(true);
    let mut rx = call_for_aid(&mut mgr, SAME_WORLD_ID).await;
    while rx.try_recv().is_ok() {}
    let capture = LogCapture::install();

    let mut rx = respawn(&mut mgr).await;

    assert_nothing_sent(&mut rx);
    assert_one_refusal(&capture, "respawn_not_dead", "respawn");
}
