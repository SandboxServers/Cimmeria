//! What a movement **reject** reports: the world it happened in, the
//! navmesh containment gate that failed and by how much, which mesh
//! build said so, and the per-entity throttle that keeps one stuck
//! player from burying everyone else's first reject.
//!
//! Also the load-time fingerprint line those per-event hashes join back
//! to. The accepted-position sampler is the sibling
//! [`super::telemetry_sampling`].
//!
//! None of these change a validation decision — they guard the fields
//! an operator reads. They need the real `castle_cellblock.nav` fixture
//! from [`super::recovery::navmesh_manager`] and self-skip without it.
//! The production gap each guard closes is in
//! `crates/services/src/cell/space_manager/movement_telemetry/`.

use std::time::{Duration, Instant};

use cimmeria_entity::movement_validation::{MovementReject, SpaceBounds};
use tracing::Level;

use super::super::super::{RejectReport, SpaceManager};
use super::make_manager;
use super::recovery::{navmesh_manager, nearby_off_mesh_point, ON_MESH};
use crate::test_support::LogCapture;

const SPAWN: [f32; 3] = [10.0, 0.0, 20.0];

/// Agnos (navmesh-less) with one entity, for the tests that only care
/// about the world label and the throttle.
fn agnos_with(entity_id: u32) -> (SpaceManager, u32) {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(entity_id, "Agnos", SPAWN, [0.0; 3])
        .unwrap();
    (mgr, space_id)
}

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

fn report_for(entity_id: u32, space_id: u32, reason: MovementReject) -> RejectReport<'static> {
    // `SpaceBounds::FALLBACK` is a const, so the borrow is 'static —
    // which keeps these helpers from needing a lifetime parameter.
    const B: &SpaceBounds = &SpaceBounds::FALLBACK;
    RejectReport {
        entity_id,
        space_id,
        reason,
        reason_label: match reason {
            MovementReject::OutOfBounds => "bounds",
            MovementReject::OffNavmesh => "navmesh",
            MovementReject::Teleport => "teleport",
        },
        position: [50_000.0, 5.0, 20.0],
        last_valid: SPAWN,
        bounds: B,
    }
}

// ── The `world` field (item 6's log half) ─────────────────────────────

/// Three days of production rejects carried `space_id` and no world, so
/// every "which worlds reject players" question needed a manual
/// space-id → world-name join against a table that only exists in the
/// running process.
#[test]
fn reject_log_names_the_world() {
    let capture = LogCapture::install();
    let (mut mgr, space_id) = agnos_with(7001);

    mgr.report_movement_reject(
        report_for(7001, space_id, MovementReject::OutOfBounds),
        Instant::now(),
    );

    let ev = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("a reject must log");
    assert!(
        ev.has_field("world", "Agnos"),
        "the reject row must name the world, not just the space id — \
         without it the 146,760-row production sample could not be split \
         by zone without a manual join: {ev:#?}"
    );
}

/// An unresolvable space still produces a row with a `world` key. An
/// absent label would silently drop the series from `sum by (world)`.
#[test]
fn reject_log_falls_back_to_unknown_world() {
    let capture = LogCapture::install();
    let (mut mgr, _) = agnos_with(7002);

    mgr.report_movement_reject(
        report_for(7002, 0xDEAD_BEEF, MovementReject::Teleport),
        Instant::now(),
    );

    let ev = capture
        .find_event(Level::WARN, "movement.validation_reject", "teleport")
        .expect("a reject must log");
    assert!(ev.has_field("world", "unknown"), "{ev:#?}");
}

// ── The navmesh diagnosis (item 2) ────────────────────────────────────

/// **The gate-reporting guard.** A navmesh reject must name which
/// containment gate failed and by how much, and which mesh build said
/// so. Reverting `report_movement_reject` to the pre-diagnosis
/// single-`reason` row drops all four fields and this fails.
///
/// Self-skips without the `castle_cellblock.nav` fixture.
#[test]
fn navmesh_reject_reports_the_gate_distances_and_mesh_hash() {
    let Some((mut mgr, space_id, bmin, bmax)) = navmesh_manager() else {
        return; // fixture-less CI
    };
    let Some(off_mesh) = nearby_off_mesh_point(&mgr, bmin, bmax) else {
        return; // whole scan window walkable — nothing to diagnose
    };
    let capture = LogCapture::install();

    let nav_bounds = SpaceBounds::new(bmin, bmax);
    mgr.report_movement_reject(
        RejectReport {
            entity_id: 100,
            space_id,
            reason: MovementReject::OffNavmesh,
            reason_label: "navmesh",
            position: off_mesh,
            last_valid: ON_MESH,
            bounds: &nav_bounds,
        },
        Instant::now(),
    );

    let ev = capture
        .find_event(Level::WARN, "movement.validation_reject", "navmesh")
        .expect("an off-navmesh reject must log");

    let gate = ev.fields.get("gate").expect(
        "a navmesh reject must name the gate that failed — \
                 'off the mesh' covers a mesh hole, a floor-clip, an \
                 over-tall jump and a point nowhere near the mesh, and \
                 those are four different bugs",
    );
    assert!(
        [
            "no_poly_in_extents",
            "horizontal",
            "below_surface",
            "above_jump_tolerance"
        ]
        .contains(&gate.as_str()),
        "gate must be one of the four stable tokens, got {gate:?}"
    );

    // `no_poly_in_extents` is the one gate with nothing to measure
    // against — the distances are correctly absent there, and present
    // everywhere else.
    if gate == "no_poly_in_extents" {
        assert!(!ev.fields.contains_key("nav_horiz_dist"), "{ev:#?}");
        assert!(!ev.fields.contains_key("nav_dy"), "{ev:#?}");
    } else {
        assert!(
            ev.fields.contains_key("nav_horiz_dist") && ev.fields.contains_key("nav_dy"),
            "a gate measured against a polygon must report both \
             distances — 'how far off' is what separates 'widen the \
             tolerance' from 'rebuild the mesh': {ev:#?}"
        );
    }

    let hash = ev.fields.get("navmesh_hash").expect(
        "a navmesh reject must name the mesh build it was \
                 judged against, or a reject from before a mesh rebuild \
                 is indistinguishable from one after it",
    );
    assert_eq!(hash.len(), 8, "short hash is 8 hex digits, got {hash:?}");
}

/// A **bounds** reject must NOT carry navmesh fields, even in a
/// navmesh-backed space: the navmesh layer never ran, so reporting a
/// gate would attribute the rejection to the wrong layer.
#[test]
fn non_navmesh_reject_reports_no_gate() {
    let Some((mut mgr, space_id, bmin, bmax)) = navmesh_manager() else {
        return;
    };
    let capture = LogCapture::install();
    let nav_bounds = SpaceBounds::new(bmin, bmax);

    mgr.report_movement_reject(
        RejectReport {
            entity_id: 100,
            space_id,
            reason: MovementReject::OutOfBounds,
            reason_label: "bounds",
            position: [bmax[0] + 10_000.0, 0.0, 0.0],
            last_valid: ON_MESH,
            bounds: &nav_bounds,
        },
        Instant::now(),
    );

    let ev = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("a bounds reject must log");
    assert!(
        !ev.fields.contains_key("gate"),
        "a bounds reject did not consult the navmesh; naming a gate \
         would blame the wrong layer: {ev:#?}"
    );
}

// ── The throttle (item 2) ─────────────────────────────────────────────

/// **The throttle guard.** One entity rejecting in a burst must produce
/// exactly one row, and the next row that does get through must say how
/// many were elided.
///
/// Reverting the `LogThrottle::admit` gate in `report_movement_reject`
/// makes this emit 6 rows instead of 2 and fails on the first assertion
/// — which is the 103,818-rows-from-one-entity shape.
#[test]
fn reject_log_throttles_a_burst_and_reports_the_suppressed_count() {
    let capture = LogCapture::install();
    let (mut mgr, space_id) = agnos_with(7003);
    let t0 = Instant::now();

    // Five rejects inside the 1 s window.
    for _ in 0..5 {
        mgr.report_movement_reject(report_for(7003, space_id, MovementReject::OutOfBounds), t0);
    }
    let after_burst = capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("movement.validation_reject"))
        .count();
    assert_eq!(
        after_burst, 1,
        "a 5-reject burst must produce ONE row, not five — one stuck \
         entity produced 71% of three days' reject volume before this \
         throttle existed"
    );

    // Past the window: the next row gets through and accounts for the
    // four it swallowed.
    mgr.report_movement_reject(
        report_for(7003, space_id, MovementReject::OutOfBounds),
        t0 + Duration::from_secs(2),
    );
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("movement.validation_reject"))
        .collect();
    assert_eq!(rows.len(), 2, "the post-window reject must emit");
    assert!(
        rows[0].has_field("suppressed", "0"),
        "the first row of an episode suppressed nothing: {:#?}",
        rows[0]
    );
    assert!(
        rows[1].has_field("suppressed", "4"),
        "the next emitted row must carry the count of rows elided since \
         the last one, or the throttle silently loses the magnitude: {:#?}",
        rows[1]
    );
}

/// One entity's throttle must not silence another's. Without per-entity
/// keying, a single spamming client would hide every other player's
/// first reject — strictly worse than the unthrottled log.
#[test]
fn one_entitys_throttle_does_not_silence_another() {
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    let space_id = mgr.create_entity(7004, "Agnos", SPAWN, [0.0; 3]).unwrap();
    mgr.create_entity(7005, "Agnos", SPAWN, [0.0; 3]).unwrap();
    let t0 = Instant::now();

    for _ in 0..4 {
        mgr.report_movement_reject(report_for(7004, space_id, MovementReject::OutOfBounds), t0);
    }
    mgr.report_movement_reject(report_for(7005, space_id, MovementReject::OutOfBounds), t0);

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("movement.validation_reject"))
        .collect();
    assert_eq!(rows.len(), 2, "one row per entity, not one row total");
    assert!(rows.iter().any(|r| r.has_field("entity_id", "7004")));
    assert!(
        rows.iter().any(|r| r.has_field("entity_id", "7005")),
        "the second entity's FIRST reject must not be swallowed by the \
         first entity's window: {rows:#?}"
    );
}

/// Throttle state is released on teardown, so it neither leaks nor lets
/// a recycled `entity_id` inherit a predecessor's open window (which
/// would swallow the first reject of a fresh session).
#[test]
fn telemetry_state_is_released_when_the_entity_is_destroyed() {
    let mut mgr = make_manager();
    let space_id = player_in(&mut mgr, 7006, "Agnos");
    mgr.report_movement_reject(
        report_for(7006, space_id, MovementReject::OutOfBounds),
        Instant::now(),
    );
    mgr.sample_accepted_position_at(7006, SPAWN, Instant::now());
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        2,
        "precondition: both the reject throttle and the position sample \
         must have a slot, or this test cannot prove they are both freed"
    );

    mgr.destroy_entity(7006);
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        0,
        "every per-entity telemetry slot must be dropped with the entity \
         — dropping only some of them is how the next id reuse inherits \
         a stale throttle window"
    );
}

// ── The load-time fingerprint (item 1) ────────────────────────────────

/// The `navmesh_loaded` line must carry enough to identify the mesh
/// build, not just count its polygons. Reverting it to
/// `polys = nm.poly_count()` drops every field asserted below, and with
/// them the ability to tie a session to the mesh it ran on.
///
/// Driven through the extracted emitter rather than
/// `create_space_instance`, whose `.nav` path is CWD-relative and
/// cannot resolve from the crate's test working directory.
#[test]
fn the_navmesh_load_line_identifies_the_mesh_build() {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return; // fixture-less CI
    }
    let mesh = cimmeria_entity::navigation::NavMesh::load(nav_path).unwrap();
    let capture = LogCapture::install();

    super::super::super::movement_telemetry::log_navmesh_loaded(
        4242,
        "Castle_CellBlock",
        mesh.fingerprint(),
    );

    let ev = capture
        .find_message(Level::INFO, "NavMesh loaded for space")
        .expect("space creation must log the mesh it loaded");
    assert!(ev.has_field("world", "Castle_CellBlock"), "{ev:#?}");
    assert!(ev.has_field("space_id", "4242"), "{ev:#?}");
    for field in [
        "polys",
        "verts",
        "file_bytes",
        "navmesh_hash",
        "navmesh_short_hash",
        "agent_height",
        "agent_climb",
        "agent_radius",
        "path",
    ] {
        assert!(
            ev.fields.contains_key(field),
            "the load line must carry `{field}` — a poly count alone \
             cannot answer 'which mesh build was this session running \
             on?': {ev:#?}"
        );
    }
    assert_eq!(
        ev.fields.get("navmesh_short_hash").map(String::len),
        Some(8)
    );
    assert_eq!(ev.fields.get("navmesh_hash").map(String::len), Some(16));
}
