//! Tests for the respawn fork, split by theme:
//! - [`respawn_fork`]: `handle_respawn` — the same-world in-place burst
//!   vs. the cross-world GateTravel branch, and the cell-entity state
//!   each one leaves behind.
//! - [`respawn_pets`]: the owner's pet on each branch (pets PT-02).
//! - [`respawn_regions`]: the client-hinted regions re-registered after the
//!   reanchor.
//! - [`respawn_resync`]: the client caches replayed after the reanchor.
//! - [`respawn_target`]: `resolve_respawn_target` — which (world,
//!   position) the fork is handed, including the origin-row guard and
//!   its negative-log seams.
//!
//! They moved here with the respawn core from
//! `cell_methods::player::combat::tests`, whose dispatch-routing tests keep
//! their own copy of `make_mgr_with_player`.

use crate::cell::space_manager::SpaceManager;

mod respawn_fork;
mod respawn_pets;
mod respawn_regions;
mod respawn_resync;
mod respawn_target;

/// Build a SpaceManager with one player at id=1 in the
/// Castle_CellBlock instanced space (every dispatch test sees a
/// fresh world). Caller can override is_player and stats.
fn make_mgr_with_player(world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    );
    mgr.parse_spaces_xml(&xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, world, [42.0, 1.0, 17.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}
