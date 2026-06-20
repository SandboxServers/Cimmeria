//! Area-of-Interest propagation tick: drains `compute_aoi_changes` and
//! fans the resulting events out to BaseApp.

use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

pub(in crate::cell::service) async fn run_aoi_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let events = space_mgr.compute_aoi_changes();
    for event in events {
        if tx.send(event).await.is_err() {
            tracing::warn!("Failed to send AoI event to BaseApp (channel closed)");
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn aoi_tick_on_empty_space_manager_produces_no_messages() {
        let mut mgr = SpaceManager::new(1);
        let (tx, mut rx) = mpsc::channel(8);
        run_aoi_tick(&tx, &mut mgr).await;
        assert!(
            rx.try_recv().is_err(),
            "empty space manager must produce zero AoI events"
        );
    }

    /// AoI tick propagates entity changes to base as individual messages.
    /// When the channel is full (receiver dropped), the tick exits early
    /// rather than spinning on closed sends.
    #[tokio::test]
    async fn aoi_tick_stops_sending_when_channel_closed() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }
        mgr.connect_entity(1);
        // Force AoI changes by computing once then adding a new entity
        let _ = mgr.compute_aoi_changes();
        mgr.create_entity(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);

        // Create a channel and immediately drop the receiver
        let (tx, rx) = mpsc::channel(1);
        drop(rx);

        // Should not panic — just returns early on closed channel
        run_aoi_tick(&tx, &mut mgr).await;
    }
}
