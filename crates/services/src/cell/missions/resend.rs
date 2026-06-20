//! Resend active mission state to the client (called during mapLoaded).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Send all active mission state to the client (called during mapLoaded).
///
/// Reference: `python/cell/MissionManager.py:559-574 resend()`
pub async fn resend_missions(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let entity = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => return,
    };

    let messages = entity.missions.serialize_resend();
    for (method_index, args) in messages {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::missions::lifecycle::accept_mission;
    use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

    fn make_objectives() -> Vec<MissionObjective> {
        vec![MissionObjective {
            objective_id: 300,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }]
    }

    #[tokio::test]
    async fn resend_sends_active_missions() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        while rx.try_recv().is_ok() {}

        resend_missions(1, &tx, &mgr).await;

        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        // 1 mission × (1 update + 1 step + 1 objective) = 3
        assert_eq!(msgs.len(), 3);
    }

    /// When the requested entity isn't in the space manager, `resend_missions`
    /// takes the `None => return` early-out and sends nothing — guards the
    /// uncovered missing-entity branch. No entity is created, so the lookup
    /// misses purely in-memory (no DB / fixtures).
    #[tokio::test]
    async fn resend_missing_entity_sends_nothing() {
        let mgr = SpaceManager::new(1);
        let (tx, mut rx) = mpsc::channel(16);

        resend_missions(999, &tx, &mgr).await;

        assert!(
            matches!(rx.try_recv(), Err(mpsc::error::TryRecvError::Empty)),
            "unknown entity must produce no messages (channel empty, not disconnected)"
        );
    }
}
