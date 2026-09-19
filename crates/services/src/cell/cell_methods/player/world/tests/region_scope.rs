//! `TRIGGER_REGION` server-authority guards (H06).
//!
//! `triggerClientHintedGenericRegion` is a client-driven RPC that names a
//! region by id and asserts an enter/exit. Two things about that were taken
//! on trust until H06: that the named region belongs to the world the caller
//! is standing in, and that the caller is standing in the region at all.
//! Both are load-bearing now that region entry can fire a
//! `cross_world_teleport` chain (and, once Castle CA10 lands, a stargate
//! passage).
//!
//! These tests assert through the *logs* rather than through a fired chain:
//! `fire_enter_region` with no seeded chains produces no observable message,
//! so the `reason` field on the refusal — and its absence on the accept — is
//! what distinguishes "refused" from "accepted and nothing matched".

use super::super::*;
use super::make_mgr_with_player;
use crate::cell::space_manager::RegionData;
use crate::test_support::LogCapture;
use tracing::Level;

/// A 10×10 box centred on the origin at floor level, with a 3-unit ceiling on
/// the asymmetric fourth corner — the shape
/// `spawner::regions::load_regions_from_db` produces for a single-point
/// cylinder.
fn box_region(runtime_id: u32, world_name: &str) -> RegionData {
    RegionData {
        runtime_id,
        db_set_id: 0x7006_0001,
        tag: format!("{world_name}.H06Probe"),
        world_name: world_name.to_string(),
        height: 3.0,
        radius: 5.0,
        flags: 0,
        points: vec![
            [-5.0, 0.0, -5.0],
            [-5.0, 0.0, 5.0],
            [5.0, 0.0, 5.0],
            [5.0, 3.0, -5.0],
        ],
    }
}

/// `region_id: i32`, `b_entering: u8`, then the client's own x/y/z — which
/// the handler deliberately ignores.
fn trigger_args(region_id: i32, entering: bool, client_pos: [f32; 3]) -> Vec<u8> {
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&region_id.to_le_bytes());
    args.push(u8::from(entering));
    for v in client_pos {
        args.extend_from_slice(&v.to_le_bytes());
    }
    args
}

fn place(mgr: &mut SpaceManager, entity_id: u32, pos: [f32; 3]) {
    let e = mgr.get_entity_mut(entity_id).expect("player entity");
    e.position.x = pos[0];
    e.position.y = pos[1];
    e.position.z = pos[2];
}

/// The baseline the two refusal tests are measured against: a region in the
/// caller's own world, with the caller standing in it, is accepted.
///
/// Without this, a guard that refused *everything* would pass both negative
/// tests and nobody would notice until the zone went dead.
#[tokio::test]
async fn a_same_world_region_the_player_is_standing_in_is_accepted() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(7, box_region(7, "Castle_CellBlock"));
    place(&mut mgr, 1, [1.0, 0.5, -2.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let handled = dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(7, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(handled);
    assert!(
        capture
            .find_message(Level::INFO, "triggerClientHintedGenericRegion")
            .is_some(),
        "a legitimate region entry must reach the dispatch path"
    );
    assert!(
        capture.find_message(Level::WARN, "refused").is_none(),
        "a legitimate region entry must not be refused"
    );
}

/// `SpaceManager::get_region` is a world-global map keyed on a client-supplied
/// id. A caller in `Castle_CellBlock` naming a region seeded against `Agnos`
/// must be refused before any chain or ring transition fires.
///
/// The planted region is positioned so the caller's coordinates would pass
/// containment — the *only* thing that can refuse this is the world scope.
#[tokio::test]
async fn a_region_belonging_to_another_world_is_refused() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(7, box_region(7, "Agnos"));
    place(&mut mgr, 1, [1.0, 0.5, -2.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);
    let handled = dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(7, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(handled, "the arm still claims the method");
    assert!(
        capture
            .find_event(
                Level::WARN,
                "region belongs to world Agnos",
                "region_world_mismatch"
            )
            .is_some(),
        "a cross-world region id must be refused with the documented reason"
    );
    assert!(
        capture
            .find_message(Level::INFO, "triggerClientHintedGenericRegion")
            .is_none(),
        "the refusal must happen before the dispatch path, not after"
    );
    assert!(
        rx.try_recv().is_err(),
        "no chain and no ring transition may fire for another world's region"
    );
}

/// The forged-position case from the PR #662 review: the region is the
/// caller's own world's, but the caller is nowhere near it. The client's own
/// x/y/z in the packet claim otherwise and are ignored — the test passes the
/// region's centre as the client position precisely to prove the server does
/// not read it.
///
/// Reverting the containment gate (or swapping `entity.position` for the
/// packet's x/y/z) fails this.
#[tokio::test]
async fn an_enter_from_outside_the_volume_is_refused() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(7, box_region(7, "Castle_CellBlock"));
    place(&mut mgr, 1, [400.0, 0.0, -180.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);
    let handled = dispatch(
        1,
        TRIGGER_REGION,
        // The lie: "I am standing at the middle of the region."
        &trigger_args(7, true, [0.0, 0.0, 0.0]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(handled);
    assert!(
        capture
            .find_event(
                Level::WARN,
                "outside the 4-corner volume",
                "region_containment_failed"
            )
            .is_some(),
        "an enter event from outside the volume must be refused against the \
         server-known position, not the one the packet carries"
    );
    assert!(rx.try_recv().is_err());
}

/// The containment gate carries the 2009 slop
/// (`Config.GENERIC_REGION_CHECK_THRESHOLD = 1.5`) because the client fires
/// the hint at the instant *its* capsule crossed the boundary, while the
/// server tests the last position it accepted. A player just past the edge
/// must still get in, or region entry becomes flaky at exactly the place it
/// always happens.
#[tokio::test]
async fn an_enter_just_past_the_boundary_is_still_accepted() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(7, box_region(7, "Castle_CellBlock"));
    // 1.4 units outside the +X face of a box whose edge is at 5.0.
    place(&mut mgr, 1, [6.4, 0.0, 0.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(7, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "outside the 4-corner volume",
                "region_containment_failed"
            )
            .is_none(),
        "the movement-lag threshold must not be tightened to zero -- a player one \
         packet past the boundary is entering, not cheating"
    );
}

/// Exits are deliberately ungated, matching the Python's own
/// `# TODO: Check !entering and isPointOutsideRegion() too`. A player who has
/// left a volume is by definition outside it; gating the exit on containment
/// would refuse every legitimate "on leave" event. World scoping still
/// applies to exits.
#[tokio::test]
async fn an_exit_from_outside_the_volume_is_still_dispatched() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(7, box_region(7, "Castle_CellBlock"));
    place(&mut mgr, 1, [400.0, 0.0, -180.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(7, false, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        capture
            .find_message(Level::INFO, "triggerClientHintedGenericRegion")
            .is_some(),
        "an exit must dispatch regardless of where the player now stands"
    );
}

/// A thin door volume, crossed at a run. This is the case the tolerance
/// exists for, and the one a zero-slop containment test would break.
///
/// The client fires the hint the instant *its* pawn crosses the near face.
/// The server is testing the last position it accepted, one ~100 ms update
/// behind (`MovementValidator::MAX_SNAP_BACK_CORRECTIONS` documents the
/// rate), which at the 8.125 u/s `run_speed` puts the server-known position
/// 0.81 units short of the doorway. Refuse that and the door silently does
/// nothing — the 2026-09-18 Castle playtest's most expensive failure shape.
#[tokio::test]
async fn a_player_running_through_a_thin_volume_is_not_false_rejected() {
    // 4 units wide in X, 1 unit thick in Z: a doorway, not a room.
    fn doorway(runtime_id: u32) -> RegionData {
        RegionData {
            runtime_id,
            db_set_id: 0x7006_0002,
            tag: "Castle_CellBlock.H06Door".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 3.0,
            radius: 0.0,
            flags: 0,
            points: vec![
                [-2.0, 0.0, -0.5],
                [-2.0, 0.0, 0.5],
                [2.0, 0.0, 0.5],
                [2.0, 3.0, -0.5],
            ],
        }
    }

    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(11, doorway(11));
    // One position-update interval of running short of the near face:
    // -0.5 - (8.125 / 10) = -1.3125.
    place(&mut mgr, 1, [0.0, 0.0, -1.3125]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(11, true, [0.0, 0.0, -0.5]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "outside the 4-corner volume",
                "region_containment_failed"
            )
            .is_none(),
        "a player one position update short of a doorway is entering it, not          cheating. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::INFO, "triggerClientHintedGenericRegion")
            .is_some(),
        "the hint must reach the dispatch path"
    );
}

/// Where the 1.5-unit tolerance runs out, stated so it is a decision rather
/// than a surprise: two *consecutive* missed position updates (1.62 units of
/// staleness at run speed) fall outside the band and are refused.
///
/// The refusal is loud — `reason = region_containment_failed` with the
/// server-known position — so if a real client ever trips it, raising
/// `GENERIC_REGION_CHECK_THRESHOLD` is a one-constant change with this test
/// naming the trade-off.
#[tokio::test]
async fn two_missed_position_updates_fall_outside_the_tolerance() {
    let mut mgr = make_mgr_with_player();
    mgr.regions.insert(12, box_region(12, "Castle_CellBlock"));
    // The +X face is at 5.0; 1.63 units past the 1.5-unit band.
    place(&mut mgr, 1, [6.63, 0.0, 0.0]);

    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(12, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "outside the 4-corner volume",
                "region_containment_failed"
            )
            .is_some(),
        "1.63 units out is past the documented band and must refuse -- if this          starts firing on real clients, raise GENERIC_REGION_CHECK_THRESHOLD"
    );
}

/// Region ids are wire-encoded `i32` and stored `u32`. A negative id must be
/// refused up front rather than sign-extended into a high `u32`.
#[tokio::test]
async fn a_negative_region_id_is_refused() {
    let mut mgr = make_mgr_with_player();
    let capture = LogCapture::install();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);
    let handled = dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(-3, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(handled, "the arm still claims the method");
    assert!(
        capture
            .find_event(Level::WARN, "negative region id", "region_id_negative")
            .is_some(),
        "a negative region id must be refused with the documented reason"
    );
    assert!(rx.try_recv().is_err());
}

/// A refused hint must be visible in the player's own journal, not only in
/// the server log.
///
/// This is the 2026-09-18 Castle playtest lesson in test form: a false
/// reject and a client that never sent the hint look identical in-game (the
/// door does nothing), and the `.bug` bookmark is where the report is
/// actually written. Deleting the `player_journal::note` call in
/// `refuse_hinted_region` fails this.
#[tokio::test]
async fn a_refused_hint_is_noted_in_the_player_journal() {
    // A region id unique to this test: the journal is a process-global ring
    // keyed on entity id, so the assertion matches on the id rather than on
    // position in the ring.
    const PROBE_REGION: u32 = 0x0060_0613;

    let mut mgr = make_mgr_with_player();
    mgr.regions
        .insert(PROBE_REGION, box_region(PROBE_REGION, "Agnos"));
    place(&mut mgr, 1, [1.0, 0.5, -2.0]);

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    dispatch(
        1,
        TRIGGER_REGION,
        &trigger_args(PROBE_REGION as i32, true, [0.0; 3]),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let entries = crate::cell::player_journal::tail(1, crate::cell::player_journal::RING);
    assert!(
        entries.iter().any(|(_, _, kind, detail)| {
            *kind == crate::cell::player_journal::kinds::REGION_HINT_REFUSED
                && detail.contains(&PROBE_REGION.to_string())
                && detail.contains("region_world_mismatch")
        }),
        "the refusal must appear in the .bug journal. Journal tail: {entries:#?}"
    );
}
