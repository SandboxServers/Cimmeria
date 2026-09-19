//! Same-world respawn re-registers the world's client-hinted regions.
//!
//! The reanchor burst recreates the client's pawn. In the 2026-09-18 Castle
//! playtest a respawned client sent no `triggerClientHintedGenericRegion`
//! for the remaining 28 minutes of its session (report finding H8), which
//! kills every `enter_region` chain, ring pad and stargate volume. Harset is
//! built on all three, so the list goes back out after the reanchor:
//! clear first, then one add per region of THIS world, in one batch.

use super::super::respawn::handle_respawn;
use super::make_mgr_with_player;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{RegionData, REGION_FLAG_CLIENT_HINTED};
use crate::mercury::method_idx::{ADD_CLIENT_HINTED_GENERIC_REGION, CLEAR_HINTED_REGIONS};
use tokio::sync::mpsc;

fn region(runtime_id: u32, world: &str, flags: i32) -> RegionData {
    RegionData {
        runtime_id,
        db_set_id: 9000 + runtime_id as i32,
        tag: format!("{world}.TestRegion{runtime_id}"),
        world_name: world.to_string(),
        height: 10.0,
        radius: 0.0,
        flags,
        points: vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 0.0, 4.0],
            [0.0, 0.0, 4.0],
        ],
    }
}

fn region_id_of(args: &[u8]) -> i32 {
    i32::from_le_bytes([args[0], args[1], args[2], args[3]])
}

#[tokio::test]
async fn same_world_respawn_reregisters_this_worlds_regions_after_the_reanchor() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    // Two hinted regions in the player's world, one hinted region in another
    // world, and one server-only region (flag 0) in the player's world.
    for r in [
        region(11, "Castle_CellBlock", REGION_FLAG_CLIENT_HINTED),
        region(12, "Castle_CellBlock", REGION_FLAG_CLIENT_HINTED),
        region(21, "Harset", REGION_FLAG_CLIENT_HINTED),
        region(13, "Castle_CellBlock", 0),
    ] {
        mgr.regions.insert(r.runtime_id, r);
    }

    let (tx, mut rx) = mpsc::channel(32);
    handle_respawn(1, -1, &tx, &mut mgr).await;

    let mut reanchor_at = None;
    let mut batch: Option<(usize, Vec<(u16, Vec<u8>)>)> = None;
    let mut idx = 0usize;
    while let Ok(m) = rx.try_recv() {
        match m {
            CellToBaseMsg::ReanchorPlayer { entity_id: 1, .. } => reanchor_at = Some(idx),
            CellToBaseMsg::EntityMethodCallBatch {
                entity_id: 1,
                calls,
            } => {
                assert!(batch.is_none(), "exactly one region batch is expected");
                batch = Some((idx, calls));
            }
            _ => {}
        }
        idx += 1;
    }

    let reanchor_at = reanchor_at.expect("fixture sanity: same-world respawn must reanchor");
    let (batch_at, calls) = batch.expect(
        "respawn must re-register the client-hinted regions: without them the client \
         sends no region hints for the rest of the session (playtest finding H8)",
    );
    assert!(
        batch_at > reanchor_at,
        "the regions must be queued AFTER ReanchorPlayer (reanchor at {reanchor_at}, batch at \
         {batch_at}): sent first, the pawn-recreate burst would discard them again"
    );

    assert_eq!(
        calls.first().map(|(m, a)| (*m, a.len())),
        Some((CLEAR_HINTED_REGIONS, 0)),
        "the batch must open with clearClientHintedGenericRegions so a client that kept \
         its old list does not end up with every region registered twice"
    );
    let mut added: Vec<i32> = calls[1..]
        .iter()
        .map(|(m, a)| {
            assert_eq!(*m, ADD_CLIENT_HINTED_GENERIC_REGION);
            region_id_of(a)
        })
        .collect();
    added.sort_unstable();
    assert_eq!(
        added,
        vec![11, 12],
        "exactly this world's client-hinted regions: not another world's (21), and not a \
         region without the client-hinted flag (13)"
    );
}

/// A world with no client-hinted regions sends nothing at all, not even the
/// clear: there is nothing for the client to have lost.
#[tokio::test]
async fn respawn_in_a_world_without_regions_sends_no_region_batch() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    mgr.regions
        .insert(21, region(21, "Harset", REGION_FLAG_CLIENT_HINTED));

    let (tx, mut rx) = mpsc::channel(32);
    handle_respawn(1, -1, &tx, &mut mgr).await;

    while let Ok(m) = rx.try_recv() {
        assert!(
            !matches!(m, CellToBaseMsg::EntityMethodCallBatch { .. }),
            "no region batch is expected for a world with no client-hinted regions"
        );
    }
}
