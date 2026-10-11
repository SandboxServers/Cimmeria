//! Shutdown report: the unsaved state of every in-world player, logged before
//! the cell loop exits.
//!
//! Called from the `shutdown` arm of the message loop. One WARN per player
//! (`event = "player_state_not_persisted"`), then one INFO summary. NPCs are
//! not reported: they have no persisted state to lose.

use crate::cell::space_manager::SpaceManager;

/// Log each in-world player's unsaved state, then a summary row.
pub(super) fn report_unsaved_on_shutdown(space_mgr: &SpaceManager) {
    let mut players_in_world: usize = 0;
    let mut players_with_dirty_ammo: usize = 0;
    for eid in space_mgr.all_player_entity_ids() {
        let Some(e) = space_mgr.get_entity(eid) else {
            continue;
        };
        if e.player_id.is_none() {
            continue;
        }
        let id = e.identity();
        let dirty_ammo_slots = e.bandolier_ammo_dirty.len();
        players_in_world += 1;
        if dirty_ammo_slots > 0 {
            players_with_dirty_ammo += 1;
        }
        tracing::warn!(
            event = "player_state_not_persisted",
            reason = "server_shutdown",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = e.entity_id.0,
            entity_name = id.player_name,
            world = space_mgr.world_name_for_space(e.space_id.0 as u32),
            x = e.position.x,
            y = e.position.y,
            z = e.position.z,
            dirty_ammo_slots,
            "player state not persisted at shutdown"
        );
    }
    tracing::info!(
        event = "shutdown_unsaved_summary",
        players_in_world,
        players_with_dirty_ammo,
        "unsaved player state at shutdown"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let e = mgr.get_entity_mut(1).unwrap();
        e.is_player = true;
        e.player_id = Some(72);
        e.account_id = Some(6);
        mgr.connect_entity(1);
        mgr.spawn_npc(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr
    }

    /// One player and one NPC in the world: exactly one WARN, for the player.
    #[test]
    fn shutdown_report_warns_once_per_player_not_npc() {
        let mgr = mgr();
        let capture = LogCapture::install();
        report_unsaved_on_shutdown(&mgr);
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN && c.has_field("event", "player_state_not_persisted")
            })
            .collect();
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert!(rows[0].has_field("player_id", "72"));
        assert!(rows[0].has_field("account_id", "6"));
        assert!(rows[0].has_field("reason", "server_shutdown"));
        assert!(capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "shutdown_unsaved_summary")
                && c.has_field("players_in_world", "1")));
    }
}
