//! Stargate Kismet sequence emission (`Stargate_MakeGate` /
//! `Stargate_CrossGate`) and the per-world gate lookups it needs.
//!
//! Evidence: `deprecated/python/cell/SGWPlayer.py:2105-2129`. The 2009
//! server emitted exactly two of the fifteen `Stargate_*` sequence events:
//!
//! - `gateDialTimerExpired` → `onSequence(Stargate_MakeGate)` (6100),
//!   four seconds after a successful dial.
//! - `stargatePassed` → `onSequence(Stargate_CrossGate)` (6113), on
//!   entering the gate region, immediately before `moveTo`.
//!
//! `cancelDialing` emits nothing — `Stargate_DestroyGate` (6103) is never
//! sent, and neither are the seven chevron events (6106-6112), even
//! though every gate's event set defines sequences for all of them. Per
//! decision D-CA10 we emit the same two and no more.
//!
//! The one deliberate addition over 2009 is the witness fan-out: the
//! Python sent to `self.client` only, so a second player standing at the
//! gate saw nothing. `docs/gameplay/gate-travel.md` lists that as the
//! "Stargate witness visibility" gap.

use tokio::sync::mpsc;

use crate::cell::kismet::{build_on_sequence_args, KISMET_VIEW_EVENT_INVOKER};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, REGION_FLAG_STARGATE};
use crate::mercury::method_idx::ON_SEQUENCE;

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
