//! Stargate Kismet sequence emission (`Stargate_MakeGate` /
//! `Stargate_CrossGate`) and the per-world gate lookups it needs.
//!
//! We emit exactly two of the fourteen `Stargate_*` sequence events:
//!
//! - on a successful dial → `onSequence(Stargate_MakeGate)` (6100).
//! - on entering the gate region with an open dial →
//!   `onSequence(Stargate_CrossGate)` (6113), immediately before the
//!   deferred world transition.
//!
//! **This is not because the 2009 server did the same.** The owner
//! confirmed (2026-09-25, NA35) that the deprecated legacy server never
//! had working gate travel end to end, so its emission choices carry no
//! authority. The reason to hold the line at these two is a client-binary
//! fact: `FUN_005682d0` (`ghidra://SGW.exe@0x005682d0`) shows the DHD
//! dial UI collects all 7 glyphs and reports the finished address to the
//! server exactly once — there is no wire-level signal for in-progress
//! chevron selection (6106-6112) for the server to key a broadcast on.
//! `Stargate_DestroyGate` (6103) remains unemitted for a weaker reason:
//! simply unexamined, not confirmed unwanted. See
//! `docs/reverse-engineering/findings/stargate-dial-and-travel-sequences.md`.
//!
//! The one deliberate addition over the legacy-server witness behaviour
//! (which sent to `self.client` only) is fanning both sequences to every
//! witness of the dialer/crosser, not just the dialer/crosser themselves.
//! `docs/gameplay/gate-travel.md` lists that as the "Stargate witness
//! visibility" gap it closes.

use tokio::sync::mpsc;

use crate::cell::kismet::{build_on_sequence_args, KISMET_VIEW_EVENT_INVOKER};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::BSF_MOVEMENT_LOCK;
use crate::cell::space_manager::{SpaceManager, REGION_FLAG_STARGATE};
use crate::mercury::method_idx::{ON_SEQUENCE, ON_STATE_FIELD_UPDATE};

/// `ESequenceEventType.Stargate_MakeGate` — the gate opens (kawoosh).
/// `entities/defs/enumerations.xml:821`.
pub(crate) const EVENT_STARGATE_MAKE_GATE: i32 = 6100;

/// `ESequenceEventType.Stargate_CrossGate` — the player steps through.
/// `entities/defs/enumerations.xml:834`.
pub(crate) const EVENT_STARGATE_CROSS_GATE: i32 = 6113;

/// Resolve the event set of the gate the player is standing at.
///
/// `SGWPlayer.onDialGate` does `self.dialingStargate = world.stargates[0]`
/// — the sequences come from the ORIGIN world's gate prefab, not the
/// destination's. Our stargate cache is keyed by `stargate_id`, so
/// "first gate in the world" is the lowest id in that world, matching the
/// `ORDER BY stargate_id` the world's gate list was built with.
pub(crate) fn origin_gate_event_set(space_mgr: &SpaceManager, world_name: &str) -> Option<i32> {
    space_mgr
        .stargates
        .iter()
        .filter(|(_, g)| g.world_name == world_name)
        .min_by_key(|(&id, _)| id)
        .and_then(|(_, g)| g.event_set_id)
}

/// Does this world have a gate region the player can walk into?
///
/// `point_sets` rows of type `AreaSet` with `REGION_FLAG_Stargate` set —
/// `Castle.Stargate` (set 1002) and its eleven siblings. Only twelve of
/// the ~30 seeded stargates have one; on the other worlds the 2009 flow
/// has no way to complete, so `handle_dial_gate` falls back to travelling
/// on the dial itself rather than stranding the player.
pub(crate) fn world_has_stargate_region(space_mgr: &SpaceManager, world_name: &str) -> bool {
    space_mgr
        .regions_for_world(world_name)
        .iter()
        .any(|r| r.flags & REGION_FLAG_STARGATE != 0)
}

/// Send one gate `onSequence` to the dialer and every player witnessing
/// them.
///
/// `source_id` / `target_id` in the payload are the DIALER's entity id,
/// not a gate entity's — the gate is a client-side Kismet prefab with no
/// server entity, and `SGWPlayer.py:2112` / `:2124` pass `self.entityId`
/// for both fields. The fan-out therefore uses the dialer's witness list.
///
/// Returns the resolved sequence id, or `None` when the gate's event set
/// has no sequence for `event_id` (nothing is sent in that case).
pub(crate) async fn send_gate_sequence(
    entity_id: u32,
    event_set_id: Option<i32>,
    event_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<i32> {
    let Some(event_set_id) = event_set_id else {
        tracing::warn!(
            entity_id,
            event_id,
            reason = "gate_event_set_missing",
            "gate sequence: origin gate has no event_set_id — \
             stargates.event_set_id is NULL for this world's gate, so the \
             client plays no gate animation"
        );
        return None;
    };
    let Some(&seq_id) = space_mgr.sequence_map.get(&(event_set_id, event_id)) else {
        tracing::warn!(
            entity_id,
            event_set_id,
            event_id,
            reason = "gate_sequence_unmapped",
            "gate sequence not in event_sets_sequences map — kismet \
             sequence will not play"
        );
        return None;
    };

    let args = build_on_sequence_args(seq_id, entity_id, KISMET_VIEW_EVENT_INVOKER);
    // The observee is the dialer, so the idbase is theirs. Looked up
    // rather than assumed true: a GM `gmDHD` against a non-player entity
    // would otherwise encode method 1 under the wrong idbase.
    let entity_is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);

    // Sorted + deduped so the emission order is deterministic (the
    // witness list comes out of a HashMap scan) and so the dialer can
    // never be sent the same frame twice when they are somehow already
    // in their own witness list.
    let mut targets = space_mgr.get_witnesses_of(entity_id);
    targets.push(entity_id);
    targets.sort_unstable();
    targets.dedup();

    let witness_count = targets.len();
    for witness_id in targets {
        if let Err(e) = tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: ON_SEQUENCE,
                args: args.clone(),
                entity_is_player,
            })
            .await
        {
            tracing::warn!(
                witness_id,
                entity_id,
                event_id,
                seq_id,
                "gate sequence: cell→base send failed — this witness will \
                 not see the gate animation: {e}"
            );
        }
    }

    tracing::info!(
        entity_id,
        event_set_id,
        event_id,
        seq_id,
        witness_count,
        "Sent stargate onSequence"
    );
    Some(seq_id)
}

/// Set or clear `BSF_MovementLock` on the crossing entity for the
/// `CROSSING_CINEMATIC_HOLD` window, notifying the owning client on an
/// actual bit transition. Ref-counted, matching
/// `ring_transport::wire_helpers::update_state_flag` (kept separate rather
/// than reused across modules — that helper is `pub(super)` to
/// `ring_transport`, and duplicating five lines here is cheaper than
/// widening its visibility for one caller outside that module).
///
/// `BSF_MOVEMENT_LOCK` already has other writers (death, the ring
/// transporter, the `Stun` effect script), so this MUST go through
/// `CellEntity::set_state_flag`/`unset_state_flag` rather than a raw
/// `|=`/`&=` — see the doc comment on those methods for why a raw write
/// desyncs the counter from the bit and leaves it stuck.
pub(crate) async fn set_crossing_movement_lock(
    entity_id: u32,
    set: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let changed = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            let transitioned = if set {
                e.set_state_flag(BSF_MOVEMENT_LOCK)
            } else {
                e.unset_state_flag(BSF_MOVEMENT_LOCK)
            };
            transitioned.then_some(e.state_field)
        }
        None => return,
    };
    let Some(new_state) = changed else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STATE_FIELD_UPDATE,
            args: new_state.to_le_bytes().to_vec(),
        })
        .await
    {
        tracing::warn!(
            entity_id,
            set,
            error = %e,
            reason = "cell_to_base_send_failed",
            "gate crossing: movement-lock update could not be enqueued — \
             the client keeps the stale lock state until the world \
             transition (or a relog) corrects it"
        );
    }
}
