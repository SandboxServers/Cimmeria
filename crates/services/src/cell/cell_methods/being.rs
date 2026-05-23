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
            if !args.is_empty() {
                let movement_type = args[0];
                tracing::debug!(entity_id, movement_type, "setMovementType");
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
}
