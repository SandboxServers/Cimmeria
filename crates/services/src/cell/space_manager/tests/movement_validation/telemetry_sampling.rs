//! The accepted-position sampler: the positive-space counterpart to
//! `movement.validation_reject`.
//!
//! Rejects say where players are *stopped*. Nothing said where they
//! successfully walk, so a navmesh hole was only visible once somebody
//! fell into it. These guard the sample's rate limit, its
//! minimum-distance gate, its players-only scope, and the navmesh state
//! it reports — the four things that make it affordable on the colo and
//! usable as a walked-surface map.
//!
//! The reject-side guards are the sibling [`super::telemetry_reject`].

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use tracing::Level;

use super::super::super::SpaceManager;
use super::make_manager;
use super::recovery::{navmesh_manager, nearby_off_mesh_point, ON_MESH};
use crate::test_support::LogCapture;

const SPAWN: [f32; 3] = [10.0, 0.0, 20.0];

/// Create a **player** entity: `create_entity` alone leaves
/// `is_player = false` (it is stamped by `connect_entity` during world
/// entry), and the position sampler deliberately ignores non-players.
fn player_in(mgr: &mut SpaceManager, entity_id: u32, world: &str) -> u32 {
    let space_id = mgr
        .create_entity(entity_id, world, SPAWN, [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    space_id
}

// ── Accepted-position sampling (item 4) ───────────────────────────────

fn sample_rows(
    capture: &crate::test_support::LogCaptureGuard,
) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("movement.position_sample"))
        .collect()
}

/// The first accepted sample for a player emits, and carries the fields
/// a "walked cells" map needs plus the standard identity pair.
#[test]
fn position_sample_carries_world_position_and_identity() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7100, "Agnos");
    if let Some(e) = mgr.get_entity_mut(7100) {
        e.account_id = Some(6);
        e.player_id = Some(12);
    }

    mgr.sample_accepted_position_at(7100, [11.0, 0.0, 20.0], Instant::now());

    let rows = sample_rows(&capture);
    assert_eq!(rows.len(), 1, "the first accepted position must sample");
    let ev = &rows[0];
    assert!(ev.has_field("world", "Agnos"), "{ev:#?}");
    assert!(ev.has_field("entity_id", "7100"), "{ev:#?}");
    assert!(ev.has_field("account_id", "6"), "{ev:#?}");
    assert!(ev.has_field("player_id", "12"), "{ev:#?}");
    assert!(
        ev.fields.contains_key("x") && ev.fields.contains_key("z"),
        "{ev:#?}"
    );
    assert_eq!(ev.level, Level::DEBUG, "sampled telemetry is debug-level");
}

/// Rate limit: a second sample inside the 5 s window is skipped however
/// far the player moved. Without it, the accept path — which runs at the
/// client's 10 Hz — would emit 10 rows/s/player.
#[test]
fn position_sample_is_rate_limited_per_player() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7101, "Agnos");
    let t0 = Instant::now();

    mgr.sample_accepted_position_at(7101, [10.0, 0.0, 20.0], t0);
    for i in 1..10 {
        // Far apart in space, close together in time.
        mgr.sample_accepted_position_at(7101, [10.0 + i as f32 * 5.0, 0.0, 20.0], t0);
    }
    assert_eq!(sample_rows(&capture).len(), 1, "rate limit must hold");

    mgr.sample_accepted_position_at(7101, [80.0, 0.0, 20.0], t0 + Duration::from_secs(6));
    assert_eq!(
        sample_rows(&capture).len(),
        2,
        "once the window elapses, a moved player samples again"
    );
}

/// Distance gate: an AFK player parked in a safe room is not new
/// information. Without this, one stationary client would dominate the
/// walked-cells map with a single repeated point.
#[test]
fn a_standing_player_does_not_resample() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7102, "Agnos");
    let t0 = Instant::now();

    mgr.sample_accepted_position_at(7102, [10.0, 0.0, 20.0], t0);
    // Hours later, 10 cm away — still the same spot.
    mgr.sample_accepted_position_at(7102, [10.1, 0.0, 20.0], t0 + Duration::from_secs(3600));
    assert_eq!(
        sample_rows(&capture).len(),
        1,
        "a player who has not moved a metre must not resample, however \
         long they have been idle"
    );

    // Two metres away, same instant: time is satisfied, distance now is
    // too.
    mgr.sample_accepted_position_at(7102, [12.0, 0.0, 20.0], t0 + Duration::from_secs(3600));
    assert_eq!(sample_rows(&capture).len(), 2);
}

/// NPCs never sample. They outnumber players by an order of magnitude in
/// a populated zone and are already covered by `movement.npc` /
/// `npc_ai.tick`; sampling them would swamp the signal.
#[test]
fn npcs_are_never_position_sampled() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    mgr.spawn_npc(100_500, "Agnos", SPAWN, [0.0; 3]).unwrap();

    for i in 0..5 {
        mgr.sample_accepted_position_at(
            100_500,
            [10.0 + i as f32 * 10.0, 0.0, 20.0],
            Instant::now() + Duration::from_secs(i * 30),
        );
    }
    assert!(
        sample_rows(&capture).is_empty(),
        "an NPC must never produce a position sample: {:#?}",
        sample_rows(&capture)
    );
}

/// On a navmesh-backed space the sample reports whether the accepted
/// point is on the mesh, its height above the surface, and which mesh
/// — the three fields that make the positive samples usable as a
/// walked-surface map rather than just a position trail.
#[test]
fn position_sample_reports_navmesh_state_on_a_meshed_space() {
    let Some((mut mgr, _space_id, _bmin, _bmax)) = navmesh_manager() else {
        return; // fixture-less CI
    };
    // `navmesh_manager` uses `create_entity`, which leaves
    // `is_player = false` — the sampler skips non-players.
    mgr.connect_entity(100);
    let capture = LogCapture::install();

    mgr.sample_accepted_position_at(100, ON_MESH, Instant::now());

    let rows = sample_rows(&capture);
    assert_eq!(rows.len(), 1);
    let ev = &rows[0];
    assert!(
        ev.has_field("on_navmesh", "true"),
        "a known-walkable point must report on_navmesh=true: {ev:#?}"
    );
    assert!(ev.fields.contains_key("nav_dy"), "{ev:#?}");
    assert!(
        ev.fields.get("navmesh_hash").is_some_and(|h| h.len() == 8),
        "the sample must name the mesh build the player walked on: {ev:#?}"
    );
}

/// A meshless space must omit the navmesh fields rather than report
/// `on_navmesh = false`: nothing was checked, and a `false` here would
/// read in a query as "this player walked off the mesh".
#[test]
fn a_meshless_space_omits_the_navmesh_fields_rather_than_reporting_false() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7103, "Agnos");

    mgr.sample_accepted_position_at(7103, [11.0, 0.0, 20.0], Instant::now());

    let rows = sample_rows(&capture);
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].fields.contains_key("on_navmesh"),
        "no mesh means nothing was checked; `false` would be a false \
         accusation: {:#?}",
        rows[0]
    );
    assert!(
        !rows[0].fields.contains_key("navmesh_hash"),
        "{:#?}",
        rows[0]
    );
}

/// End-to-end through the real accept path: an accepted `EntityMove`
/// must produce a sample. Guards the `client_move.rs` hook itself, which
/// none of the direct-call tests above touch.
#[test]
fn the_accept_path_samples() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7104, "Agnos");

    let outcome = mgr.apply_client_position_update(7104, [12.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(
            outcome,
            super::super::super::ClientMoveOutcome::Accepted { .. }
        ),
        "precondition: this move must be accepted, got {outcome:?}"
    );
    assert_eq!(
        sample_rows(&capture).len(),
        1,
        "the accept path must be wired to the sampler — without the hook \
         in `SpaceManager::accept` nothing samples in production"
    );
}

/// A **rejected** move must not sample. The sampler is the
/// positive-space record; a rejected position was never written and
/// reporting it as walked would poison the map with exactly the points
/// that are not walkable.
#[test]
fn a_rejected_move_does_not_sample() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    player_in(&mut mgr, 7105, "Agnos");

    let _ = mgr.apply_client_position_update(7105, [50_000.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    assert!(
        sample_rows(&capture).is_empty(),
        "a rejected position is not a walked position: {:#?}",
        sample_rows(&capture)
    );
}

/// Sanity: the diagnosis seam agrees with the boolean one the validator
/// actually gates on, at the `SpaceManager` level too (the entity crate
/// pins the same invariant inside `NavMesh`). A meshless space returns
/// `None` rather than a verdict, which is what lets a caller say "there
/// was nothing to check" instead of inventing a gate.
#[test]
fn space_manager_diagnosis_agrees_with_is_position_valid() {
    let Some((mgr, _space_id, bmin, bmax)) = navmesh_manager() else {
        return;
    };
    let on = Vector3::new(ON_MESH[0], ON_MESH[1], ON_MESH[2]);
    assert_eq!(
        mgr.diagnose_point(100, &on).map(|v| v.valid),
        Some(mgr.is_position_valid(100, &on))
    );
    if let Some(off) = nearby_off_mesh_point(&mgr, bmin, bmax) {
        let off = Vector3::new(off[0], off[1], off[2]);
        assert_eq!(
            mgr.diagnose_point(100, &off).map(|v| v.valid),
            Some(mgr.is_position_valid(100, &off))
        );
    }

    let mut meshless = make_manager();
    meshless
        .create_entity(7106, "Agnos", SPAWN, [0.0; 3])
        .unwrap();
    assert!(
        meshless.diagnose_point(7106, &on).is_none(),
        "a meshless space has no verdict to give — `is_position_valid` \
         fails open there and a caller must be able to tell that apart \
         from a genuine pass"
    );
}
