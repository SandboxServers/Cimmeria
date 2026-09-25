//! `npc_ai.leash`: enter, snap_fallback, and the aggro/leash loop (S5).

use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, ai_tick, castle_mgr, rows, NPC, PLAYER};
use crate::test_support::LogCapture;

/// An aggressive guard whose spawn is 60 from a player standing in its AoI:
/// today it aggroes (no radius), leashes on the next tick (the player is
/// past the 50 leash radius around spawn), snaps home, and aggroes again —
/// the 6-second loop NPC 100630 ran 120 times in 12 minutes.
fn looping_guard() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], Some([0.0; 3]), AiState::Idle);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.aggression = 1;
        npc.faction = 10;
    }
    mgr.create_entity(PLAYER, "Castle", [60.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.faction = 0;
    if let Some(h) = p.stats.get_mut(cimmeria_entity::stats::HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// **Acceptance: a leash loop.** Three full cycles inside the window raise
/// exactly one `event=loop` WARN carrying the count. Revert-proof: removing
/// the loop check in `detectors::leash::on_enter` (or the `on_enter` call
/// in `fight.rs`) leaves the `enter` rows and no `loop`.
#[tokio::test]
async fn an_aggro_leash_loop_is_reported() {
    let mut mgr = looping_guard();
    let logs = LogCapture::install();
    // aggro -> leash -> snap, three times.
    for _ in 0..9 {
        ai_tick(&mut mgr).await;
    }
    let enters = rows(&logs, "npc_ai.leash", "enter");
    assert_eq!(enters.len(), 3, "three leash entries: {enters:#?}");
    assert!(
        enters[0].has_field("target_to_spawn", "60.0"),
        "{:?}",
        enters[0]
    );
    assert!(enters[0].has_field("leash_distance", "50.0"));
    let loops = rows(&logs, "npc_ai.leash", "loop");
    assert_eq!(loops.len(), 1, "{loops:#?}");
    assert_eq!(loops[0].level, Level::WARN);
    assert!(loops[0].has_field("leash_count", "3"), "{:?}", loops[0]);
    assert!(loops[0].has_field("target_id", "101"));
    assert!(loops[0].has_field("world", "Castle"));

    // A fourth cycle inside the throttle window is counted, not written.
    for _ in 0..3 {
        ai_tick(&mut mgr).await;
    }
    assert_eq!(rows(&logs, "npc_ai.leash", "loop").len(), 1);
}

/// Two leashes are not a loop.
#[tokio::test]
async fn two_leashes_are_not_a_loop() {
    let mut mgr = looping_guard();
    let logs = LogCapture::install();
    for _ in 0..6 {
        ai_tick(&mut mgr).await;
    }
    assert_eq!(rows(&logs, "npc_ai.leash", "enter").len(), 2);
    assert!(rows(&logs, "npc_ai.leash", "loop").is_empty());
}

/// The instant snap is reported as the fallback it is, with how far it
/// jumped and the chase path it left behind (audit S4).
#[tokio::test]
async fn the_leash_snap_reports_its_jump_and_the_stale_path() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [20.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Leashing,
    );
    mgr.get_entity_mut(NPC)
        .unwrap()
        .nav_path
        .push_back(cimmeria_common::Vector3::new(30.0, 0.0, 0.0));
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    let snap = rows(&logs, "npc_ai.leash", "snap_fallback");
    assert_eq!(snap.len(), 1, "{:#?}", logs.all());
    for (k, v) in [
        ("snap_dist", "20.0"),
        ("stale_path_len", "1"),
        ("path_ok", "false"),
        ("snapped", "true"),
    ] {
        assert!(snap[0].has_field(k, v), "{k}={v}: {:?}", snap[0]);
    }
}
