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

use cimmeria_entity::cell_entity::BandolierItem;
use tokio::sync::mpsc;

use super::*;
use crate::cell::space_manager::SpaceManager;

mod departure_cleanup;
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

/// `player_id` given to fixture players, derived from their entity id so two
/// fixture players never collide.
pub(super) fn fixture_player_id(entity_id: u32) -> i32 {
    0x7000_0E00 + entity_id as i32
}

/// Create a *player* entity (create + connect, so `is_player` is set) and
/// return the space it landed in.
///
/// Also sets `player_id` and seeds one dirty bandolier slot. Neither
/// `create_entity` nor `connect_entity` sets `player_id` (it defaults to
/// `None`), and the transfer's pre-teardown bandolier flush is gated on it —
/// so without this the *only* statement in the primitive that can mutate
/// anything before teardown would be dead in every test, and the
/// "a rejection changes nothing" claim would be vacuous for it.
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
    let entity = mgr
        .get_entity_mut(entity_id)
        .expect("fixture entity must exist immediately after create");
    entity.player_id = Some(fixture_player_id(entity_id));
    entity.bandolier_items.insert(
        DIRTY_SLOT,
        BandolierItem {
            instance_id: 0x7000_0E10 + entity_id as i32,
            item_id: 21,
            clip_size: 30,
            default_ammo_type: 1,
            current_ammo: 17,
            cur_ammo_type: 1,
        },
    );
    entity.bandolier_ammo_dirty.insert(DIRTY_SLOT);
    assert!(
        mgr.get_entity(entity_id).is_some_and(|e| e.is_player),
        "fixture must produce a player entity — the primitive is players-only (D15)"
    );
    space_id
}

/// Bandolier slot the fixture marks dirty.
pub(super) const DIRTY_SLOT: i32 = 0;

/// Pull the `GateTravel` off the channel, skipping the pre-teardown cleanup
/// traffic (the bandolier flush, and any trade-cancel notifications) that
/// legitimately precedes it. `departure_cleanup` is where that ordering is
/// asserted; everywhere else it is noise.
pub(super) fn expect_gate_travel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> CellToBaseMsg {
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::GateTravel { .. }) {
            return msg;
        }
    }
    panic!("no GateTravel was emitted");
}

/// Is the fixture's seeded ammo write still pending?
pub(super) fn ammo_still_dirty(mgr: &SpaceManager, entity_id: u32) -> bool {
    mgr.get_entity(entity_id)
        .is_some_and(|e| e.bandolier_ammo_dirty.contains(&DIRTY_SLOT))
}

/// Snapshot of everything a rejection must leave untouched.
///
/// `ammo_dirty` and `trade_partner` are here because they are the two pieces
/// of state the primitive mutates on the *accepted* path before teardown —
/// without them a rejection test would be asserting that nothing changed in
/// fields nothing ever writes.
#[derive(Debug, PartialEq)]
pub(super) struct OriginState {
    pub space_id: Option<u32>,
    pub position: Option<[f32; 3]>,
    pub is_player: Option<bool>,
    pub space_count: usize,
    pub ammo_dirty: Option<bool>,
    pub trade_partner: Option<Option<u32>>,
}

pub(super) fn snapshot_origin(mgr: &SpaceManager, entity_id: u32) -> OriginState {
    let entity = mgr.get_entity(entity_id);
    OriginState {
        space_id: mgr.get_entity_space_id(entity_id),
        position: entity.map(|e| [e.position.x, e.position.y, e.position.z]),
        is_player: entity.map(|e| e.is_player),
        space_count: mgr.space_count(),
        ammo_dirty: entity.map(|e| e.bandolier_ammo_dirty.contains(&DIRTY_SLOT)),
        trade_partner: entity.map(|e| e.trade_partner_entity_id),
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
