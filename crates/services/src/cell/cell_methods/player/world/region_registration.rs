//! Registering a world's client-hinted generic regions with a client.
//!
//! Region entry is client-driven: the client holds a list of trigger volumes
//! (`addClientHintedGenericRegion`) and calls
//! `triggerClientHintedGenericRegion` when its pawn crosses one. Everything
//! region-shaped on the server hangs off that call: `enter_region` content
//! chains, the ring-transporter pads, and the `REGION_FLAG_STARGATE` volume
//! that carries a player through an open gate.
//!
//! Two callers:
//!
//! - **World entry** (`player_init`): `mapLoaded` has already sent
//!   `clearClientHintedGenericRegions`, so the list is registered as is.
//! - **Same-world respawn** (`combat::respawn`): the reanchor burst's
//!   `CREATE_BASE_PLAYER` recreates the client's pawn. In the 2026-09-18
//!   Castle playtest a character that died and respawned sent zero region
//!   hints for the remaining 28 minutes of its session, while the character
//!   that never died kept sending them (report finding H8). Castle lost one
//!   mission step to that. Harset would lose its rings, its Command Center
//!   doors and its stargate, so the list is re-registered after the reanchor,
//!   the same way the inventory snapshot already is. The clear goes first so
//!   the result is identical whether or not the client kept its old list.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, REGION_FLAG_CLIENT_HINTED};
// `CLEAR_HINTED_REGIONS` is `clearClientHintedGenericRegions` (client method
// 124, no arguments), the same constant `mapLoaded` sends at world entry.
use crate::mercury::method_idx::{ADD_CLIENT_HINTED_GENERIC_REGION, CLEAR_HINTED_REGIONS};

/// Whether to lead the batch with `clearClientHintedGenericRegions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClearFirst {
    /// The caller is re-registering onto a client that may still hold a list.
    Yes,
    /// The client's list is already known to be empty (`mapLoaded` cleared it).
    No,
}

/// Send every client-hinted region of `world_name` to `entity_id`'s client as
/// ONE `EntityMethodCallBatch`, so the base packs it into a single Mercury
/// packet (PR #410: twenty-odd separate reliable packets stalled some clients
/// at login). Returns the number of regions registered.
///
/// Sends nothing, not even the clear, when the world has no client-hinted
/// regions: there is then nothing for the client to have lost.
pub(crate) async fn send_client_hinted_regions(
    entity_id: u32,
    world_name: &str,
    clear_first: ClearFirst,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> usize {
    let mut calls: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut region_count = 0usize;
    for r in space_mgr
        .regions_for_world(world_name)
        .iter()
        .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
    {
        let mut args = Vec::with_capacity(20 + r.points.len() * 12);
        args.extend_from_slice(&(r.runtime_id as i32).to_le_bytes());
        args.extend_from_slice(&r.height.to_le_bytes());
        args.extend_from_slice(&r.radius.to_le_bytes());
        args.extend_from_slice(&r.flags.to_le_bytes());
        args.extend_from_slice(&(r.points.len() as u32).to_le_bytes()); // ARRAY count
        for p in &r.points {
            args.extend_from_slice(&p[0].to_le_bytes()); // x
            args.extend_from_slice(&p[1].to_le_bytes()); // y
            args.extend_from_slice(&p[2].to_le_bytes()); // z
        }
        calls.push((ADD_CLIENT_HINTED_GENERIC_REGION, args));
        region_count += 1;
    }
    if calls.is_empty() {
        return 0;
    }
    if clear_first == ClearFirst::Yes {
        calls.insert(0, (CLEAR_HINTED_REGIONS, Vec::new()));
    }
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCallBatch { entity_id, calls })
        .await
    {
        tracing::warn!(
            entity_id,
            world = %world_name,
            region_count,
            error = %e,
            "region registration: EntityMethodCallBatch send failed -- the client has no \
             trigger volumes, so region chains, ring pads and gate crossings will not fire"
        );
    }
    region_count
}
