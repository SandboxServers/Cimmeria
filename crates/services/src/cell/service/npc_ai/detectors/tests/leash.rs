//! `npc_ai.leash`: enter, snap_fallback, and the aggro/leash loop (S5).

use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, add_threat_player, ai_tick, castle_mgr, rows, NPC, PLAYER};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// An aggressive guard at its spawn with a player 60 u out, in its AoI.
/// Before NA12 it aggroed, leashed on the next tick (the player was past the
/// 50 u radius around spawn), snapped home and aggroed again: the 6-second
/// loop NPC 100630 ran 120 times in 12 minutes. NA12 measures the NPC, so
/// this guard now simply fights.
fn looping_guard() -> SpaceManager {
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

/// NA12: the S5 scenario no longer leashes at all, so there is no `enter`
/// and no `loop`. Reverting the NPC-distance metric brings back three
/// `enter` rows and the `loop` WARN over these nine ticks.
#[tokio::test]
async fn the_s5_guard_no_longer_leashes() {
    let mut mgr = looping_guard();
    let logs = LogCapture::install();
    for _ in 0..9 {
        ai_tick(&mut mgr).await;
    }
    assert!(
        rows(&logs, "npc_ai.leash", "enter").is_empty(),
        "{:#?}",
        logs.all()
    );
    assert!(rows(&logs, "npc_ai.leash", "loop").is_empty());
}

/// One leash cycle, arranged: a Fighting NPC standing 60 u out (past its
/// band) with a player 10 u further out on its threat list. The first tick
/// leashes; with no navmesh the second snaps it home.
async fn drag_out_and_leash(mgr: &mut SpaceManager) {
    mgr.update_position_preserving_facing(NPC, [60.0, 0.0, 0.0], [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Fighting);
        npc.threat_list.insert(PLAYER, 10.0);
    }
    ai_tick(mgr).await; // Fighting -> Leashing (enter)
    ai_tick(mgr).await; // no route: snap home, Idle
}

fn leash_fixture() -> SpaceManager {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [60.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle", [70.0, 0.0, 0.0]);
    mgr
}

/// **Acceptance: a leash loop.** Three leashes inside the window raise
/// exactly one `event=loop` WARN carrying the count, and the `enter` row
/// carries NA12's reason, trigger and NPC-side distance. Revert-proof:
/// removing the loop check in `detectors::leash::on_enter` (or the
/// `on_enter` call in `leash::begin_leash`) leaves no `loop`.
#[tokio::test]
async fn an_aggro_leash_loop_is_reported() {
    let mut mgr = leash_fixture();
    let logs = LogCapture::install();
    for _ in 0..3 {
        drag_out_and_leash(&mut mgr).await;
    }
    let enters = rows(&logs, "npc_ai.leash", "enter");
    assert_eq!(enters.len(), 3, "three leash entries: {enters:#?}");
    for (k, v) in [
        ("reason", "leash_out"),
        ("trigger", "beyond_band"),
        ("npc_to_spawn", "60.0"),
        ("target_to_spawn", "70.0"),
        ("leash_distance", "50.0"),
        ("nav_path_len", "0"),
    ] {
        assert!(enters[0].has_field(k, v), "{k}={v}: {:?}", enters[0]);
    }
    let loops = rows(&logs, "npc_ai.leash", "loop");
    assert_eq!(loops.len(), 1, "{loops:#?}");
    assert_eq!(loops[0].level, Level::WARN);
    assert!(loops[0].has_field("leash_count", "3"), "{:?}", loops[0]);
    assert!(loops[0].has_field("target_id", "101"));
    assert!(loops[0].has_field("world", "Castle"));

    // A fourth cycle inside the throttle window is counted, not written.
    drag_out_and_leash(&mut mgr).await;
    assert_eq!(rows(&logs, "npc_ai.leash", "loop").len(), 1);
}

/// Two leashes are not a loop.
#[tokio::test]
async fn two_leashes_are_not_a_loop() {
    let mut mgr = leash_fixture();
    let logs = LogCapture::install();
    for _ in 0..2 {
        drag_out_and_leash(&mut mgr).await;
    }
    assert_eq!(rows(&logs, "npc_ai.leash", "enter").len(), 2);
    assert!(rows(&logs, "npc_ai.leash", "loop").is_empty());
}

/// The instant snap is reported as the fallback it is, with how far it
/// jumped. `stale_path_len` is the S4 before-picture (the chase path the
/// old snap left behind); since NA10's `snap_npc_to` it must read 0.
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
        ("stale_path_len", "0"),
        ("path_ok", "false"),
        ("snapped", "true"),
    ] {
        assert!(snap[0].has_field(k, v), "{k}={v}: {:?}", snap[0]);
    }
}
