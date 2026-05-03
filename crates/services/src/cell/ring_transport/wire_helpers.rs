//! Wire-level helpers used by the ring-transport effect dispatcher: builds
//! the byte payloads for the client methods we send during a ring trip
//! (`onSequence`, `onStateFieldUpdate`, `onVisible`, `onRingTransporterList`).
//!
//! Kept separate from [`super::dispatch`] so the dispatcher only deals in
//! `Effect → mpsc::Sender<CellToBaseMsg>` and doesn't accumulate byte-level
//! plumbing.

use tokio::sync::mpsc;

use super::regions::RingRegion;
use super::transporter::RegionEvent;
use super::wire::build_on_ring_transporter_list;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::{
    ON_RING_TRANSPORTER_LIST, ON_SEQUENCE, ON_STATE_FIELD_UPDATE, ON_VISIBLE,
};

/// `BSF_MovementLock` — bit 6 of `state_field`. See
/// `crates/entity/src/cell_entity/` for the full bit layout.
pub const BSF_MOVEMENT_LOCK: u32 = 1 << 6;

/// Re-export for tests (see `super::tests`) and external consumers that
/// dispatch on the destination-list method id.
pub const METHOD_ON_RING_TRANSPORTER_LIST: u16 = ON_RING_TRANSPORTER_LIST;

/// KISMET_VIEW_EventInvoker viewType passed to `onSequence`. Same value used
/// elsewhere for region-driven kismet — the camera follows the triggering
/// player.
const KISMET_VIEW_EVENT_INVOKER: u8 = 3;

/// Build an `onSequence` payload (matches the layout in
/// `cell/content/executor.rs::PlaySequence` and `cell_methods/player/world.rs`).
fn build_on_sequence_args(seq_id: i32, entity_id: u32) -> Vec<u8> {
    let mut args = Vec::with_capacity(26);
    args.extend_from_slice(&seq_id.to_le_bytes()); // KismetEventSetSeqID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
    args.push(1); // PrimaryTarget = true
    args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count = 0
    args.push(KISMET_VIEW_EVENT_INVOKER); // ViewType
    args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
    args
}

pub(super) async fn send_play_sequence(
    entity_id: u32,
    event_set_id: i32,
    region_event: RegionEvent,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let event_id = region_event.event_id();
    let seq_id = match space_mgr.sequence_map.get(&(event_set_id, event_id)) {
        Some(&id) => id,
        None => {
            tracing::warn!(
                event_set_id,
                event_id,
                "ring sequence not in event_sets_sequences map — kismet sequence will not play"
            );
            return;
        }
    };
    let args = build_on_sequence_args(seq_id, entity_id);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_SEQUENCE,
            args,
        })
        .await;
}

pub(super) async fn update_state_flag(
    entity_id: u32,
    flag: u32,
    set: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let new_state = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            if set {
                e.state_field |= flag;
            } else {
                e.state_field &= !flag;
            }
            e.state_field
        }
        None => return,
    };
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STATE_FIELD_UPDATE,
            args: new_state.to_le_bytes().to_vec(),
        })
        .await;
}

/// Broadcasts a visibility change to every witness of the entity (and the
/// entity itself) so the avatar fades for nearby players too. Asymmetric
/// because BigWorld's hide/show primitives are different:
///
/// - **Hide** sends `BASEMSG_ENTITY_INVISIBLE (0x0B)` alone — the engine-
///   level visual hide that keeps AoI bookkeeping intact. Mirror of C++
///   `ClientHandler::leaveAoI(id, deleteEntity=false)` per
///   `client_handler.cpp:516-528`.
/// - **Show** sends entity method `onVisible(1)` (index 8) — mirror of
///   `ClientHandler::enterAoI` per `client_handler.cpp:507-514`. The
///   original Python `setVisible(True)` path also plays a
///   `SEQUENCE_VisibilitySafety` here; we don't yet (sequence id not in
///   the seed data we've audited).
///
/// Sending `onVisible(0)` was wrong — the engine doesn't use it for hide;
/// only `0x0B` works.
pub(super) async fn send_visible(
    entity_id: u32,
    visible: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let mut targets: std::collections::HashSet<u32> =
        space_mgr.get_witnesses_of(entity_id).into_iter().collect();
    targets.insert(entity_id);
    for witness_id in targets {
        let msg = if visible {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: ON_VISIBLE,
                args: vec![1],
            }
        } else {
            CellToBaseMsg::EntityInvisible {
                witness_id,
                entity_id,
            }
        };
        let _ = tx.send(msg).await;
    }
}

pub(super) async fn send_destination_list(
    entity_id: u32,
    source_region_id: i32,
    destinations: &[i32],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let source = match space_mgr.ring_regions.get(&source_region_id) {
        Some(r) => r,
        None => {
            tracing::warn!(
                source_region_id,
                "send_destination_list: source region not in cache"
            );
            return;
        }
    };
    let dests: Vec<&RingRegion> = destinations
        .iter()
        .filter_map(|id| {
            let r = space_mgr.ring_regions.get(id);
            if r.is_none() {
                tracing::warn!(
                    invalid_id = id,
                    "ring destination id not in cache — skipping"
                );
            }
            r
        })
        .collect();

    let payload = build_on_ring_transporter_list(source, &dests);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: METHOD_ON_RING_TRANSPORTER_LIST,
            args: payload,
        })
        .await;
    tracing::info!(
        entity_id,
        source_region_id,
        destination_count = dests.len(),
        "Sent onRingTransporterList"
    );
}
