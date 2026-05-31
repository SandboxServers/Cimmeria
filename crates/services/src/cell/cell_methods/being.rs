//! SGWBeing interface exposed CellMethods (indices 0–1).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Set current target entity.
pub const SET_TARGET_ID: u16 = 0;
/// Set movement type (walk/run/sprint).
pub const SET_MOVEMENT_TYPE: u16 = 1;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_TARGET_ID => {
            if args.len() >= 4 {
                let target_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_id, "setTargetID");

                // Persist the player's live target so the auto-cycle loop
                // driver can read it every re-fire. `target_id == 0` is
                // the client's "deselect" sentinel — store as `None` so
                // the loop sees "no target" and stops re-firing instead
                // of trying to attack entity id 0.
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.current_target_id = if target_id > 0 { Some(target_id) } else { None };
                }

                // Send onTargetUpdate (client method 16) back to the player
                // so the client knows the target is set and enables auto-attack.
                let mut reply = Vec::with_capacity(4);
                reply.extend_from_slice(&target_id.to_le_bytes());
                let _ = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 16, // onTargetUpdate (SGWBeing interface)
                        args: reply,
                    })
                    .await;

                // Also notify witnesses so they see who we're targeting
                let witnesses = space_mgr.get_witnesses_of(entity_id);
                if !witnesses.is_empty() {
                    let mut witness_args = Vec::with_capacity(4);
                    witness_args.extend_from_slice(&target_id.to_le_bytes());
                    for witness_id in witnesses {
                        let _ = tx
                            .send(CellToBaseMsg::WitnessEntityMethod {
                                witness_id,
                                entity_id,
                                method_index: 16,
                                args: witness_args.clone(),
                            })
                            .await;
                    }
                }
            }
            true
        }
        SET_MOVEMENT_TYPE => {
            // Inbound `setMovementType(UINT8)` — the client (or a peer
            // entity in BigWorld's call-on-ghost model) is telling us
            // an entity has switched movement modes. Store the value on
            // the entity and fan out to AoI witnesses via the dedup'd
            // broadcast helper. The helper handles the
            // "already-cached, skip" case so a re-send of the same byte
            // doesn't spam the wire.
            //
            // The byte is one of `EMobMovementType` (Cover=0,
            // CombatAdvance=1, Patrol=2, Follow=3, Wander=4, Leash=5,
            // Avoid=6) per `entities/defs/enumerations.xml:1593-1604`.
            // Unknown values are stored as `None` (clear cached state)
            // and skipped on broadcast — better than persisting a
            // garbage byte that future dedup compares against.
            use cimmeria_entity::cell_entity::MobMovementType;
            if !args.is_empty() {
                let movement_type = args[0];
                let kind = match movement_type {
                    0 => Some(MobMovementType::Cover),
                    1 => Some(MobMovementType::CombatAdvance),
                    2 => Some(MobMovementType::Patrol),
                    3 => Some(MobMovementType::Follow),
                    4 => Some(MobMovementType::Wander),
                    5 => Some(MobMovementType::Leash),
                    6 => Some(MobMovementType::Avoid),
                    other => {
                        tracing::warn!(
                            entity_id,
                            movement_type = other,
                            "setMovementType: unknown EMobMovementType value, dropping"
                        );
                        None
                    }
                };
                tracing::debug!(entity_id, movement_type, ?kind, "setMovementType");
                crate::cell::abilities::broadcast_movement_type(entity_id, kind, tx, space_mgr)
                    .await;
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        mgr
    }

    /// `setTargetID(target_id)` must persist the target id on the
    /// player entity so the auto-cycle tick can read it as the live
    /// re-fire target. Pin: the auto-cycle loop driver depends on
    /// this write — without it the loop has no way to track the
    /// player's cursor selection across cooldown re-fires.
    #[tokio::test]
    async fn set_target_id_writes_current_target_to_entity() {
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(8);

        let target_id: i32 = 42;
        let handled = dispatch(1, SET_TARGET_ID, &target_id.to_le_bytes(), &tx, &mut mgr).await;
        assert!(handled);

        let p = mgr.get_entity(1).unwrap();
        assert_eq!(
            p.current_target_id,
            Some(42),
            "setTargetID must persist the target id for the auto-cycle tick to read"
        );
    }

    /// `setTargetID(0)` is the client's "deselect target" sentinel.
    /// Store as `None` so the auto-cycle tick treats it as "no target"
    /// and clears the loop — rather than attempting to fire at the
    /// non-existent entity id 0.
    #[tokio::test]
    async fn set_target_id_zero_clears_current_target() {
        let mut mgr = make_mgr();
        // Pre-condition: a target was set earlier.
        mgr.get_entity_mut(1).unwrap().current_target_id = Some(99);

        let (tx, _rx) = mpsc::channel(8);
        let handled = dispatch(1, SET_TARGET_ID, &0i32.to_le_bytes(), &tx, &mut mgr).await;
        assert!(handled);

        let p = mgr.get_entity(1).unwrap();
        assert_eq!(
            p.current_target_id, None,
            "setTargetID(0) must clear current_target_id, not store Some(0)"
        );
    }

    // ── setMovementType inbound handler ────────────────────────────────────

    /// Fixture with two co-located NPCs in a Castle space. Both
    /// connected via a player witness so the witness route fans out.
    /// Returns the manager — caller picks the entity id to drive.
    fn make_mgr_with_npc_and_witness() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // Player witness.
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        // NPC under test.
        mgr.spawn_npc(50, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    /// Inbound `setMovementType(2)` on an NPC must store `Patrol` on
    /// the entity and fan the byte out to AoI witnesses. Pin: any
    /// refactor that drops either the storage or the broadcast will
    /// fail this — the symptom would be "client never plays patrol
    /// animation despite server seeing the inbound call".
    #[tokio::test]
    async fn set_movement_type_inbound_stores_and_broadcasts() {
        use crate::cell::messages::CellToBaseMsg;
        use cimmeria_entity::cell_entity::MobMovementType;

        let mut mgr = make_mgr_with_npc_and_witness();
        let (tx, mut rx) = mpsc::channel(16);

        let handled = dispatch(50, SET_MOVEMENT_TYPE, &[2u8], &tx, &mut mgr).await;
        assert!(handled, "dispatcher must accept SET_MOVEMENT_TYPE");

        // Cache populated.
        assert_eq!(
            mgr.get_entity(50).unwrap().last_movement_type,
            Some(MobMovementType::Patrol),
            "inbound byte 2 must store MobMovementType::Patrol on the NPC",
        );

        // Witness got exactly one setMovementType packet with the
        // single-byte Patrol payload.
        let witness_sends: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| match m {
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id: 50,
                    method_index,
                    args,
                    ..
                } if method_index == SET_MOVEMENT_TYPE => Some(args),
                _ => None,
            })
            .collect();
        assert_eq!(witness_sends.len(), 1, "exactly one witness fanout");
        assert_eq!(witness_sends[0], vec![2u8], "payload pin");
    }

    /// Unknown EMobMovementType byte (anything outside 0..=6) must NOT
    /// be stored on the entity, must NOT fan out, and must surface a
    /// warn-level log. Verifies the match arm's catch-all is the
    /// failure-safe one.
    #[tokio::test]
    async fn set_movement_type_inbound_unknown_byte_is_dropped() {
        use crate::cell::messages::CellToBaseMsg;

        let mut mgr = make_mgr_with_npc_and_witness();
        let (tx, mut rx) = mpsc::channel(16);

        // 99 is well outside the enum range.
        let handled = dispatch(50, SET_MOVEMENT_TYPE, &[99u8], &tx, &mut mgr).await;
        assert!(handled);
        assert!(
            mgr.get_entity(50).unwrap().last_movement_type.is_none(),
            "unknown byte must not populate the cache",
        );
        let any_setmt = std::iter::from_fn(|| rx.try_recv().ok()).any(|m| {
            matches!(
                m,
                CellToBaseMsg::WitnessEntityMethod { method_index, .. }
                    | CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index == SET_MOVEMENT_TYPE
            )
        });
        assert!(
            !any_setmt,
            "unknown byte must not produce a wire setMovementType",
        );
    }

    /// Empty args is a malformed call. The handler short-circuits
    /// without touching state — no panic, no broadcast. Returns
    /// `true` because the method index is recognised even though the
    /// payload is invalid.
    #[tokio::test]
    async fn set_movement_type_inbound_empty_args_is_a_no_op() {
        let mut mgr = make_mgr_with_npc_and_witness();
        let (tx, _rx) = mpsc::channel(16);
        let handled = dispatch(50, SET_MOVEMENT_TYPE, &[], &tx, &mut mgr).await;
        assert!(handled, "dispatcher still claims the method index");
        assert!(
            mgr.get_entity(50).unwrap().last_movement_type.is_none(),
            "empty args must not touch the cache",
        );
    }
}
