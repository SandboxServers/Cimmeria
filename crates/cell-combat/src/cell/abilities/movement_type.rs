//! NPC movement-type bookkeeping: the `last_movement_type` cache and its
//! `movement.movement_type` rows. Nothing here goes on the wire (NA10).

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// 1-in-N for the `movement.movement_type` `deduped` TRACE row, which fires
/// once per NPC per AI tick. Prime, so the fixed NPC iteration order cannot
/// phase-lock the sample onto one NPC.
pub(crate) const MOVEMENT_TYPE_DEDUPED_SAMPLE_EVERY: u64 = 53;
static MOVEMENT_TYPE_DEDUPED_SAMPLER: crate::firehose::FirehoseSampler =
    crate::firehose::FirehoseSampler::new(MOVEMENT_TYPE_DEDUPED_SAMPLE_EVERY);

/// Record an NPC's movement type (`EMobMovementType`) in the dedup cache
/// `last_movement_type`. **Nothing goes on the wire.**
///
/// The client has no server-to-client movement-type message. `setMovementType`
/// is a cell method only (`SGWBeing.def`, `<Exposed/>`, client to server; the
/// client has `Event_NetOut_SetMovementType` and no NetIn twin). This function
/// used to send a `WitnessEntityMethod` with method index `1` and a one-byte
/// payload. For a witness, client method `1` of every NPC entity type is
/// `onSequence` (`client_methods::spawnable_entity::ON_SEQUENCE`), the Kismet
/// sequence trigger that attack animations use. So every Fighting, Patrol,
/// Leash or Follow entry sent each witness a truncated `onSequence`. The
/// client handler once thought to be the movement-type animation switch,
/// `0x00deb660`, is the GM path visualiser for `SGWGmPlayer.onShowPath`
/// (NA10, 2026-09-25; see
/// `docs/reverse-engineering/findings/npc-movement-pathfinding.md`).
///
/// What the client animates comes from the velocity on each `EntityMoved`.
/// To stop an NPC, zero its velocity (`npc_ai::stop_npc_movement`). No
/// movement type is needed.
///
/// The cache is kept, and callers still report their state here, so that
/// `last_movement_type` stays a truthful "what is this NPC doing" field for
/// telemetry and the debug bookmark. `kind = None` clears it. Each change logs
/// `movement.movement_type outcome=suppressed` (or `cleared`) at DEBUG.
///
/// `tx` is unused now and kept so the dozen call sites stay unchanged. A
/// future GM `onShowPath` feature is the correct way to show a movement type
/// on a client.
pub async fn broadcast_movement_type(
    entity_id: u32,
    kind: Option<cimmeria_entity::cell_entity::MobMovementType>,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // NPC-only guard. Players have no movement-type concept; a caller
    // routing this at a player is a bug worth seeing.
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    if is_player {
        tracing::warn!(
            target: "movement.movement_type",
            entity_id,
            ?kind,
            "broadcast_movement_type called on a player entity — no-op (movement type is NPC-only)"
        );
        return;
    }

    let last = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.last_movement_type);
    if last == kind {
        // Hot path: every AI tick (2 s) re-asserts the current kind, once per
        // NPC. NA25 exports this target, so the row is sampled 1-in-N with
        // the skipped count; ~75 rows/s at 150 NPCs becomes ~1.4.
        if let Some(suppressed) = MOVEMENT_TYPE_DEDUPED_SAMPLER.admit() {
            tracing::trace!(
                target: "movement.movement_type",
                entity_id,
                ?kind,
                outcome = "deduped",
                sampled_1_in = MOVEMENT_TYPE_DEDUPED_SAMPLER.every(),
                suppressed,
                "movement type unchanged"
            );
        }
        return;
    }
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.last_movement_type = kind;
    }
    match kind {
        None => tracing::debug!(
            target: "movement.movement_type",
            entity_id,
            prior_kind = ?last,
            outcome = "cleared",
            "movement type cache cleared"
        ),
        Some(k) => tracing::debug!(
            target: "movement.movement_type",
            entity_id,
            kind = ?k,
            kind_byte = k as u8,
            prior_kind = ?last,
            outcome = "suppressed",
            "movement type recorded; not sent (no client receiver exists, index 1 is onSequence)"
        ),
    }
}
