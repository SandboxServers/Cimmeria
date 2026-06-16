//! Cell-side execution of GM `/`-commands.
//!
//! Architecture: **base PARSES + AUTHORIZES → typed intent → cell EXECUTES.**
//! The base intercepts a `/`-prefixed chat line, parses it into a
//! [`GmCommandIntent`], checks the caller's access level, and ships the intent
//! to the cell via [`BaseToCellMsg::GmCommand`]. The cell owns `SpaceManager`
//! and every client-method send — including the feedback line that confirms
//! the result to the GM.
//!
//! All world mutation reuses existing combat / spawn / inventory primitives
//! rather than re-implementing them:
//! - Spawn → [`SpaceManager::spawn_npc_from_record_in_space`] (template-driven)
//! - Goto  → [`CellToBaseMsg::TeleportPlayer`] (same path as content teleport)
//! - Kill  → [`combat::mark_npc_dead`] + [`abilities::apply_death_transition`]
//! - Give  → [`CellToBaseMsg::GrantItem`]
//!
//! [`BaseToCellMsg::GmCommand`]: crate::cell::messages::BaseToCellMsg::GmCommand

mod combat_cmd;
mod feedback;
mod inventory;
mod movement;
mod query;
mod spawn;

use tokio::sync::mpsc;

use cimmeria_commands::GmCommandIntent;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;
use super::spawner::SpawnRecord;

use combat_cmd::handle_kill;
pub use feedback::send_gm_feedback;
use inventory::handle_give;
use movement::{handle_goto_coords, handle_goto_player};
use query::{handle_info, handle_who};
use spawn::handle_spawn;

/// Execute a parsed, authorized GM command against world state, then feed the
/// result back to the caller.
///
/// Every branch is defensive: a missing caller / target / unresolvable moniker
/// feeds back an error line and never panics.
pub async fn handle_gm_command(
    caller_entity_id: u32,
    intent: GmCommandIntent,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    spawn_records: &[SpawnRecord],
) {
    tracing::info!(caller_entity_id, ?intent, "Executing GM command");
    match intent {
        GmCommandIntent::Spawn { moniker, count } => {
            handle_spawn(
                caller_entity_id,
                &moniker,
                count,
                tx,
                space_mgr,
                spawn_records,
            )
            .await;
        }
        GmCommandIntent::GotoCoords(pos) => {
            handle_goto_coords(caller_entity_id, [pos.x, pos.y, pos.z], tx, space_mgr).await;
        }
        GmCommandIntent::GotoPlayer(name) => {
            handle_goto_player(caller_entity_id, &name, tx, space_mgr).await;
        }
        GmCommandIntent::Kill { target } => {
            handle_kill(caller_entity_id, target.as_deref(), tx, space_mgr).await;
        }
        GmCommandIntent::Give { item_id, count } => {
            handle_give(caller_entity_id, item_id, count, tx, space_mgr).await;
        }
        GmCommandIntent::Info => {
            handle_info(caller_entity_id, tx, space_mgr).await;
        }
        GmCommandIntent::Who => {
            handle_who(caller_entity_id, tx, space_mgr).await;
        }
    }
}

/// Shared test fixtures + decoders for the per-command handler tests. Each
/// handler module imports these via `use super::tests_common::*;` so the
/// `SpaceManager`/`SpawnRecord` builders and the `onPlayerCommunication`
/// feedback decoder live in one place rather than being copy-pasted per module.
#[cfg(test)]
pub(super) mod tests_common {
    use super::*;

    /// Build a `SpaceManager` with a non-instanced "Castle" world and a
    /// connected player at entity id 1. Non-instanced so the player and any
    /// spawned NPCs share one space (instanced worlds allocate a fresh space
    /// per `create_entity`).
    pub(in crate::cell::gm_command) fn mgr_with_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [5.0, 0.0, 5.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1); // sets is_player = true, adds to space.players
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(100);
        }
        mgr
    }

    /// A `SpawnRecord` whose `template_name` is `name`, placed in "Castle".
    pub(in crate::cell::gm_command) fn template(name: &str) -> SpawnRecord {
        SpawnRecord {
            spawn_id: 1,
            world_name: "Castle".to_string(),
            x: 99.0,
            y: 0.0,
            z: 99.0,
            heading: 0.0,
            tag: None,
            template_id: 14,
            template_name: name.to_string(),
            class: "mob".to_string(),
            static_mesh: None,
            body_set: "GLB_Components.WorldObject_Small".to_string(),
            components: None,
            flags: 0,
            interaction_type: 0,
            event_set_id: None,
            level: Some(5),
            alignment: Some(0),
            faction: Some(10),
            name_id: Some(7031),
            speaker_id: None,
            static_interaction_sets: vec![],
            has_dynamic_properties: false,
            loot_table_id: None,
            is_stationary: false,
            ability_ids: vec![],
            respawn_secs: None,
            patrol_path: vec![],
            patrol_point_delay_secs: 2.0,
            wander_radius: 0.0,
            wander_min_dwell_secs: 3.0,
            wander_max_dwell_secs: 8.0,
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
        }
    }

    pub(in crate::cell::gm_command) fn drain(
        rx: &mut mpsc::Receiver<CellToBaseMsg>,
    ) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    /// The feedback line text for the LAST `onPlayerCommunication` (method 28)
    /// addressed to `entity_id`. Decodes the Text WSTRING (after the speaker
    /// WSTRING + flags + channel).
    pub(in crate::cell::gm_command) fn feedback_text_to(
        msgs: &[CellToBaseMsg],
        entity_id: u32,
    ) -> Option<String> {
        msgs.iter().rev().find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: eid,
                method_index,
                args,
            } if *eid == entity_id
                && *method_index == crate::mercury::method_idx::ON_PLAYER_COMMUNICATION =>
            {
                Some(decode_feedback_text(args))
            }
            _ => None,
        })
    }

    /// Decode the Text field of an `onPlayerCommunication` arg buffer.
    pub(in crate::cell::gm_command) fn decode_feedback_text(args: &[u8]) -> String {
        let mut off = 0;
        let speaker_chars = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
        off += 4 + speaker_chars * 2;
        off += 1; // flags
        off += 1; // channel
        let text_chars = u32::from_le_bytes(args[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        let units: Vec<u16> = (0..text_chars)
            .map(|i| u16::from_le_bytes(args[off + i * 2..off + i * 2 + 2].try_into().unwrap()))
            .collect();
        String::from_utf16_lossy(&units)
    }
}
