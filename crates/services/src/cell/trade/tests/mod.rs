//! Trade-session unit tests for the state and wire layer.
//!
//! - [`wire`] — wire-byte fixtures (the `stub_inv_items_for` info-leak
//!   sentinel guard) and the serializers' negative-log seams.
//!
//! The handler tests, which drive the inbound cell methods, are in
//! `cell_methods::player::trade::tests`.

use crate::cell::space_manager::SpaceManager;

mod wire;

/// Set up two players in the same space, separated by `dist` along the
/// X axis. Both are flagged `is_player = true` and given player IDs so
/// the full commit path can be exercised end-to-end. A copy of the
/// handler tests' fixture of the same name.
pub(super) fn make_two_players(mgr: &mut SpaceManager, a: u32, b: u32, dist: f32) {
    mgr.create_entity(a, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(b, "Agnos", [dist, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(a) {
        e.is_player = true;
        e.player_id = Some(1000);
    }
    if let Some(e) = mgr.get_entity_mut(b) {
        e.is_player = true;
        e.player_id = Some(2000);
    }
}
