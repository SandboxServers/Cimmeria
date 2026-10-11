//! The `movement.teleport` row: one per server-authoritative move, emitted
//! from the `note_authorized_teleport` choke point.
//!
//! These need the real `castle_cellblock.nav` fixture from
//! [`super::recovery::navmesh_manager`] (an `enforce` world) and self-skip
//! without it. The emitter is
//! `crates/cell-world/src/cell/space_manager/movement_telemetry/teleport.rs`.

use tracing::Level;

use super::make_manager;
use super::recovery::{navmesh_manager, nearby_off_mesh_point, ON_MESH};
use super::telemetry_reject::player_in;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const MSG: &str = "movement.teleport";

fn teleport_rows(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == MSG)
        .collect()
}

/// A player teleported off the walkable mesh in an enforce world gets the
/// WARN, because the validator will refuse its very next packet and this
/// row is what names the teleport that caused it. Removing the call line in
/// `note_authorized_teleport` leaves no row at all.
#[test]
fn teleport_off_mesh_in_an_enforce_world_warns() {
    let Some((mut mgr, space_id, bmin, bmax)) = navmesh_manager() else {
        return;
    };
    mgr.connect_entity(100);
    if let Some(e) = mgr.get_entity_mut(100) {
        e.account_id = Some(6);
        e.player_id = Some(12);
    }
    let Some(off) = nearby_off_mesh_point(&mgr, bmin, bmax) else {
        return;
    };
    assert!(mgr.enforces_navmesh_containment(space_id));

    let capture = LogCapture::install();
    mgr.update_position_preserving_facing(100, off, [0.0; 3]);
    mgr.note_authorized_teleport(100);

    let debug = capture
        .find_message(Level::DEBUG, MSG)
        .expect("every authorized teleport writes the DEBUG row");
    assert!(
        debug.has_field("event", "authorized_teleport"),
        "{debug:#?}"
    );
    assert!(debug.has_field("navmesh_mode", "enforce"), "{debug:#?}");
    assert!(debug.has_field("dest_on_mesh", "false"), "{debug:#?}");
    assert!(debug.has_field("is_player", "true"), "{debug:#?}");
    assert!(debug.has_field("world", "Castle_CellBlock"), "{debug:#?}");
    assert!(debug.has_field("player_id", "12"), "{debug:#?}");
    assert!(debug.has_field("account_id", "6"), "{debug:#?}");

    let warn = capture
        .find_event(Level::WARN, MSG, "teleport_dest_off_mesh")
        .expect("a player put off-mesh in an enforce world must WARN");
    assert!(warn.has_field("player_id", "12"), "{warn:#?}");
    assert!(warn.has_field("world", "Castle_CellBlock"), "{warn:#?}");
}

/// On the mesh: the DEBUG row and nothing louder. A WARN here would fire on
/// every ordinary respawn and ring arrival.
#[test]
fn teleport_on_mesh_logs_debug_only() {
    let Some((mut mgr, _, _, _)) = navmesh_manager() else {
        return;
    };
    mgr.connect_entity(100);

    let capture = LogCapture::install();
    mgr.update_position_preserving_facing(100, ON_MESH, [0.0; 3]);
    mgr.note_authorized_teleport(100);

    let rows = teleport_rows(&capture);
    assert_eq!(rows.len(), 1, "one row per teleport: {rows:#?}");
    assert_eq!(rows[0].level, Level::DEBUG);
    assert!(rows[0].has_field("dest_on_mesh", "true"), "{rows:#?}");
    assert!(rows[0].has_field("entity_id", "100"), "{rows:#?}");
}

/// A meshless world reports `navmesh_mode = "none"` and leaves
/// `dest_on_mesh` out rather than guessing; it never WARNs.
#[test]
fn teleport_in_a_meshless_world_reports_mode_none() {
    let mut mgr = make_manager();
    player_in(&mut mgr, 7200, "Agnos");

    let capture = LogCapture::install();
    mgr.note_authorized_teleport(7200);

    let rows = teleport_rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].level, Level::DEBUG);
    assert!(rows[0].has_field("navmesh_mode", "none"), "{rows:#?}");
    assert!(!rows[0].fields.contains_key("dest_on_mesh"), "{rows:#?}");
}
