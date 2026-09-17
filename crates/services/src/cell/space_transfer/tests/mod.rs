//! Regression suite for the cross-space transfer primitive (packet P45).
//!
//! Split by the property under test:
//!
//! - [`validation`] — the validate-before-teardown contract. Every rejection
//!   shape must leave the subject entity in its origin space, at its origin
//!   position, with no `GateTravel` on the wire.
//! - [`instance_targeting`] — destination-instance resolution. The exact
//!   instance a caller names is the exact instance that reaches the wire,
//!   even when several instances of the same world are loaded; and the D15
//!   default picks the first/default loaded instance deterministically.
//!
//! The mid-transfer disconnect stages are covered on the base side, in
//! `crate::base::world_entry::gate_travel::tests::transfer`, because that is
//! where the client session state lives.

use tokio::sync::mpsc;

use super::*;
use crate::cell::space_manager::SpaceManager;

mod instance_targeting;
mod validation;

/// Worlds used by this suite:
///
/// - `Agnos` — non-instanced, has a startup space.
/// - `Castle` — non-instanced, has a startup space (the "other world" in
///   cross-world assertions).
/// - `Castle_CellBlock` — instanced: every create allocates a fresh space,
///   so this is the world that can hold several simultaneous instances.
/// - `Orphan` — declared in `spaces.xml` but non-instanced and absent from
///   `cell_spaces.xml`, so it is a known world that can never be entered.
///   Exists to pin `WorldNotLoadable` apart from `UnknownWorld`.
pub(super) const AGNOS: &str = "Agnos";
pub(super) const CASTLE: &str = "Castle";
pub(super) const INSTANCED: &str = "Castle_CellBlock";
pub(super) const ORPHAN: &str = "Orphan";

pub(super) fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        <Space WorldName="Castle_CellBlock" Instanced="true" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        <Space WorldName="Orphan" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
    </Spaces>"#;
    let startup = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" />
        <Space WorldName="Castle" />
    </Spaces>"#;
    mgr.parse_spaces_xml(spaces).unwrap();
    mgr.create_startup_spaces(startup).unwrap();
    mgr
}

/// Create a *player* entity (create + connect, so `is_player` is set) and
/// return the space it landed in.
pub(super) fn spawn_player(
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
    position: [f32; 3],
) -> u32 {
    let space_id = mgr
        .create_entity(entity_id, world, position, [0.0; 3])
        .expect("fixture entity must be creatable");
    mgr.connect_entity(entity_id);
    assert!(
        mgr.get_entity(entity_id).is_some_and(|e| e.is_player),
        "fixture must produce a player entity — the primitive is players-only (D15)"
    );
    space_id
}

/// Snapshot of everything a rejection must leave untouched.
#[derive(Debug, PartialEq)]
pub(super) struct OriginState {
    pub space_id: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub is_player: Option<bool>,
    pub space_count: usize,
}

pub(super) fn snapshot_origin(mgr: &SpaceManager, entity_id: u32) -> OriginState {
    let entity = mgr.get_entity(entity_id);
    OriginState {
        space_id: mgr.get_entity_space_id(entity_id),
        position: entity.map(|e| [e.position.x, e.position.y, e.position.z]),
        is_player: entity.map(|e| e.is_player),
        space_count: mgr.space_count(),
    }
}

/// Assert a rejected transfer changed nothing: the entity is still in its
/// origin space at its origin position, no space was created or destroyed,
/// and nothing reached the base channel.
pub(super) fn assert_origin_untouched(
    before: &OriginState,
    mgr: &SpaceManager,
    entity_id: u32,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) {
    assert_eq!(
        *before,
        snapshot_origin(mgr, entity_id),
        "a rejected transfer must leave the origin state byte-identical — \
         any drift here is a partial teardown"
    );
    match rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        Ok(msg) => panic!("rejected transfer must not emit anything to base, got {msg:?}"),
        Err(e) => panic!("unexpected channel state: {e:?}"),
    }
}
