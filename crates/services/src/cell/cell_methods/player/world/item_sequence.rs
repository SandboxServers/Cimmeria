//! `Item_*` kismet sequence dispatch — the archetype-keyed animation
//! lookup shared by the equip / unequip / reload / use paths.

use crate::cell::client_methods::spawnable_entity;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Fire a `Item_*` kismet sequence (`Item_Equip` 4000 / `Item_Unequip`
/// 4001 / `Item_Reload` 4002 / `Item_Use` 4003) keyed off the player's
/// archetype-keyed "Item handling" event set. Mirrors
/// `python/cell/SGWBeing.py:getItemSequence(eventId)` + `playSequence`.
///
/// No-op (with a debug log) when the archetype, the event set, or the
/// per-event sequence is missing — matches the python's `if eventSet
/// else None` fallthrough so callers don't crash on edge entities.
pub(crate) async fn fire_item_sequence(
    entity_id: u32,
    event_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let archetype_id = space_mgr.get_entity(entity_id).and_then(|e| e.archetype_id);
    let event_set = archetype_id.and_then(crate::cell::spawner::archetype_item_event_set);
    let seq_id = event_set.and_then(|esid| space_mgr.sequence_map.get(&(esid, event_id)).copied());
    tracing::info!(
        entity_id,
        event_id,
        archetype_id = ?archetype_id,
        event_set_id = ?event_set,
        seq_id = ?seq_id,
        "fire_item_sequence: archetype-keyed sequence lookup"
    );
    let Some(seq_id) = seq_id else {
        return;
    };
    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire
    // path so animations are emitted consistently with weapon-fire and
    // reload animations).
    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&seq_id.to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.push(1);
    seq_args.extend_from_slice(&0.0f32.to_le_bytes());
    seq_args.extend_from_slice(&0u32.to_le_bytes());
    seq_args.push(0);
    seq_args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: spawnable_entity::ON_SEQUENCE,
            args: seq_args,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::make_space_manager_with_player;
    use tokio::sync::mpsc;

    /// An entity with no `archetype_id` (the default for a freshly-created
    /// entity) resolves no event set, so the per-event sequence lookup
    /// misses and `fire_item_sequence` must early-return WITHOUT emitting an
    /// `onSequence` frame. Guards the `let Some(seq_id) = seq_id else
    /// { return; }` fallthrough — a regression that dropped the guard would
    /// send a malformed `seq_id = ?` animation packet.
    #[tokio::test]
    async fn no_archetype_emits_no_sequence() {
        let mut mgr = make_space_manager_with_player(1);
        // archetype_id is left None (the create_entity default).
        let (tx, mut rx) = mpsc::channel(8);

        fire_item_sequence(1, crate::cell::spawner::EVENT_ITEM_EQUIP, &tx, &mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "missing archetype must produce no onSequence frame"
        );
        // Touch `mgr` after the call to prove the borrow released cleanly.
        let _ = mgr.get_entity_mut(1);
    }
}
