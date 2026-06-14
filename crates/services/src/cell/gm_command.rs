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

use tokio::sync::mpsc;

use cimmeria_commands::GmCommandIntent;
use cimmeria_entity::stats::HEALTH;

use super::abilities;
use super::combat;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;
use super::spawner::SpawnRecord;

/// `onPlayerCommunication` flat ClientMethod index for SGWPlayer (28).
/// Canonical constant lives in `crate::mercury::method_idx`; the chat
/// broadcaster (`cell::chat`) uses the same index. Feedback is addressed to a
/// single entity (the GM), not fanned out to witnesses.
const ON_PLAYER_COMMUNICATION: u16 = crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// Feedback chat channel id (`EChannel.CHAN_FEEDBACK = 8`, from
/// `python/Atrea/enums.py`). Mirrors [`crate::cell::chat::CHAN_FEEDBACK`].
const CHAN_FEEDBACK: u8 = 8;

/// Hard cap on `/spawn` count — a GM typo (`/spawn drone 99999`) must not be
/// able to DoS the cell by flooding a space with entities + AoI work.
const SPAWN_COUNT_CAP: u32 = 20;

/// Hard cap on `/give` count. The count reaches `sgw_inventory.stack_size`
/// (an `integer` column with no CHECK constraint) via `GrantItem`. A
/// **negative** count would subtract from an existing stack on the
/// stack-merge path — a fat-finger (`/give 55 -5`) that silently destroys
/// items — and an absurd count is an overflow/abuse surface. The `/give`
/// handler rejects non-positive counts and clamps the upper bound here so
/// the raw client-typed value never reaches the inventory SQL (CAT-N-11
/// signed-quantity footgun; flagged by the server-authority audit).
const GIVE_COUNT_CAP: i32 = 1000;

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`.
///
/// Byte-for-byte identical to `cell::chat::serialize_on_player_communication`:
/// - Speaker: WSTRING (u32 char_count + N×2B UTF-16LE)
/// - SpeakerFlags: UINT8
/// - Channel: UINT8
/// - Text: WSTRING (u32 char_count + N×2B UTF-16LE)
///
/// Duplicated here (rather than re-exported) because the chat module's copy is
/// private and the two callers serialize for different reasons (broadcast vs.
/// single-recipient feedback); the wire shape is pinned by tests in both
/// modules so a drift in one trips its own guard.
fn serialize_on_player_communication(
    speaker: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
) -> Vec<u8> {
    let speaker_utf16: Vec<u16> = speaker.encode_utf16().collect();
    let text_utf16: Vec<u16> = text.encode_utf16().collect();

    let capacity = 4 + speaker_utf16.len() * 2 + 1 + 1 + 4 + text_utf16.len() * 2;
    let mut args = Vec::with_capacity(capacity);

    args.extend_from_slice(&(speaker_utf16.len() as u32).to_le_bytes());
    for &ch in &speaker_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }
    args.push(speaker_flags);
    args.push(channel);
    args.extend_from_slice(&(text_utf16.len() as u32).to_le_bytes());
    for &ch in &text_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }
    args
}

/// Send a single feedback line to the GM only (no witness fan-out).
///
/// Speaker is `"SYSTEM"`, flags `0`, channel `CHAN_FEEDBACK`.
pub async fn send_gm_feedback(caller_entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: caller_entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
    {
        tracing::warn!(
            caller_entity_id,
            error = %e,
            "gm_command: feedback send to base failed — GM won't see the result line"
        );
    }
}

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

/// `/spawn <moniker> [count]` — spawn `count` NPCs of the template whose
/// `template_name` matches `moniker` (case-insensitive), placed at the caller's
/// position. The moniker → template map is the loaded spawn-record set; we copy
/// a matching record's template fields and re-point its position to the caller.
async fn handle_spawn(
    caller_entity_id: u32,
    moniker: &str,
    count: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    spawn_records: &[SpawnRecord],
) {
    // Caller position + space — spawn near the GM.
    let (pos, dir, space_id) = match space_mgr.get_entity(caller_entity_id) {
        Some(e) => (
            [e.position.x, e.position.y, e.position.z],
            [e.direction.x, e.direction.y, e.direction.z],
            e.space_id.0 as u32,
        ),
        None => {
            send_gm_feedback(
                caller_entity_id,
                "Spawn failed: you are not in a space.",
                tx,
            )
            .await;
            return;
        }
    };

    // Resolve the moniker against a loaded template (by template_name).
    let Some(template) = spawn_records
        .iter()
        .find(|r| r.template_name.eq_ignore_ascii_case(moniker))
    else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawn failed: no template named '{moniker}'."),
            tx,
        )
        .await;
        return;
    };

    let n = count.clamp(1, SPAWN_COUNT_CAP);
    let mut spawned = 0u32;
    for _ in 0..n {
        let npc_id = space_mgr.allocate_npc_id();
        // Clone the resolved template record and re-point it at the caller's
        // position so the NPC spawns where the GM is standing, not at the
        // template's authored spawnlist coordinate.
        let mut record = template.clone();
        record.x = pos[0];
        record.y = pos[1];
        record.z = pos[2];
        record.heading = dir[1]; // heading is yaw (rotation.y)
        match space_mgr.spawn_npc_from_record_in_space(npc_id, &record, space_id) {
            Ok(_) => spawned += 1,
            Err(e) => {
                tracing::warn!(caller_entity_id, npc_id, moniker, "GM spawn failed: {e}");
            }
        }
    }

    if spawned == 0 {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawn failed: could not place '{moniker}'."),
            tx,
        )
        .await;
    } else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawned {spawned}x '{}'.", template.template_name),
            tx,
        )
        .await;
    }
}

/// `/goto <x> <y> <z>` — teleport the caller to an absolute coordinate in their
/// current space. Mirrors `content::executor::transport::teleport`: capture the
/// prior position, update the spatial grid, then route the authoritative snap
/// through `TeleportPlayer`.
async fn handle_goto_coords(
    caller_entity_id: u32,
    position: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some((prev_pos, space_id)) = space_mgr.get_entity(caller_entity_id).map(|e| {
        (
            [e.position.x, e.position.y, e.position.z],
            e.space_id.0 as u32,
        )
    }) else {
        send_gm_feedback(caller_entity_id, "Goto failed: you are not in a space.", tx).await;
        return;
    };

    teleport_caller(
        caller_entity_id,
        position,
        prev_pos,
        space_id,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_entity_id,
        &format!(
            "Teleported to ({:.1}, {:.1}, {:.1}).",
            position[0], position[1], position[2]
        ),
        tx,
    )
    .await;
}

/// `/goto <player>` — teleport the caller to another entity in the same space.
///
/// Players carry no character name on the cell entity (only `player_id`), so
/// `name` is resolved against (a) a numeric entity id, then (b) any entity's
/// `npc_name`. The match is restricted to player entities so `/goto` only ever
/// jumps to a player, matching the command's intent.
async fn handle_goto_player(
    caller_entity_id: u32,
    name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some((prev_pos, space_id)) = space_mgr.get_entity(caller_entity_id).map(|e| {
        (
            [e.position.x, e.position.y, e.position.z],
            e.space_id.0 as u32,
        )
    }) else {
        send_gm_feedback(caller_entity_id, "Goto failed: you are not in a space.", tx).await;
        return;
    };

    // Resolve target among players in the caller's space.
    let numeric_id: Option<u32> = name.parse::<u32>().ok();
    let target = space_mgr.all_player_entity_ids().into_iter().find(|&pid| {
        pid != caller_entity_id
            && space_mgr.get_entity_space_id(pid) == Some(space_id)
            && (Some(pid) == numeric_id
                || space_mgr
                    .get_entity(pid)
                    .and_then(|e| e.npc_name.as_deref())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name)))
    });

    let Some(target_id) = target else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Goto failed: no player '{name}' in your space."),
            tx,
        )
        .await;
        return;
    };

    let Some(dest) = space_mgr
        .get_entity(target_id)
        .map(|e| [e.position.x, e.position.y, e.position.z])
    else {
        send_gm_feedback(caller_entity_id, "Goto failed: target vanished.", tx).await;
        return;
    };

    teleport_caller(caller_entity_id, dest, prev_pos, space_id, tx, space_mgr).await;
    send_gm_feedback(
        caller_entity_id,
        &format!("Teleported to entity {target_id}."),
        tx,
    )
    .await;
}

/// Shared teleport bundle: update the spatial grid in place, then emit the
/// authoritative `TeleportPlayer` (FORCED_POSITION snap). `prev_pos` must be
/// captured BEFORE the grid update — it becomes the forced-position previous
/// reference vector (see `build_forced_position`).
async fn teleport_caller(
    caller_entity_id: u32,
    position: [f32; 3],
    prev_pos: [f32; 3],
    space_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    space_mgr.update_entity_position(caller_entity_id, position, [0, 0, 0], [0.0; 3]);
    if let Err(e) = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id: caller_entity_id,
            space_id,
            position,
            prev_pos,
        })
        .await
    {
        tracing::warn!(caller_entity_id, error = %e, "gm_command: TeleportPlayer send failed");
    }
}

/// `/kill [target_name]` — kill an NPC. Target is the named NPC (resolved by
/// `npc_name` / numeric id in the caller's space) or, when unnamed, the
/// caller's `current_target_id`. Players are never killable here.
///
/// Mirrors the canonical NPC kill path from `damage_apply`: zero HEALTH,
/// `combat::mark_npc_dead` (state flags + AI Dead + respawn stamp), clear
/// BSF_IN_COMBAT, then `apply_death_transition` for the ordered wire burst.
async fn handle_kill(
    caller_entity_id: u32,
    target_name: Option<&str>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Resolve the target entity id.
    let target_eid: Option<u32> = match target_name {
        None => space_mgr
            .get_entity(caller_entity_id)
            .and_then(|e| e.current_target_id)
            .map(|id| id as u32),
        Some(name) => {
            let space_id = space_mgr.get_entity_space_id(caller_entity_id);
            let numeric_id = name.parse::<u32>().ok();
            space_mgr.all_npc_entity_ids().into_iter().find(|&nid| {
                space_mgr.get_entity_space_id(nid) == space_id
                    && (Some(nid) == numeric_id
                        || space_mgr
                            .get_entity(nid)
                            .and_then(|e| e.npc_name.as_deref())
                            .is_some_and(|n| n.eq_ignore_ascii_case(name)))
            })
        }
    };

    let Some(target_eid) = target_eid else {
        send_gm_feedback(caller_entity_id, "Kill failed: no target.", tx).await;
        return;
    };

    // Gate: NPCs only. Also flips dead bits + AI state under one mutable borrow.
    let (is_player, name_for_feedback) = match space_mgr.get_entity_mut(target_eid) {
        Some(target) if target.is_player => (true, None),
        Some(target) => {
            if let Some(stat) = target.stats.get_mut(HEALTH) {
                stat.set_current(0);
            }
            combat::mark_npc_dead(target);
            target.state_field &= !combat::BSF_IN_COMBAT;
            (false, target.npc_name.clone())
        }
        None => {
            send_gm_feedback(caller_entity_id, "Kill failed: target not found.", tx).await;
            return;
        }
    };

    if is_player {
        send_gm_feedback(caller_entity_id, "Kill refused: target is a player.", tx).await;
        return;
    }

    let target_state = space_mgr
        .get_entity(target_eid)
        .map_or(0, |e| e.state_field);

    // Wire burst: reticle clear, threat fanout, loot + interaction flags,
    // dead-state flip. Attacker = the GM (a player), target = NPC.
    abilities::apply_death_transition(
        target_eid,
        caller_entity_id,
        target_state,
        /* attacker_is_player */ true,
        /* target_is_player */ false,
        tx,
        space_mgr,
    )
    .await;

    // Mirror damage_apply: drain the corpse's threat list now that the
    // death-transition consumer has read it (it broadcasts the BSF_InCombat
    // clear to each aggroed player first).
    if let Some(corpse) = space_mgr.get_entity_mut(target_eid) {
        corpse.threat_list.clear();
    }

    let label = name_for_feedback.unwrap_or_else(|| format!("entity {target_eid}"));
    send_gm_feedback(caller_entity_id, &format!("Killed {label}."), tx).await;
}

/// `/give <item_id> [count]` — grant an item to the caller. Base persists to
/// `sgw_inventory` and pushes the client inventory update; container 1 is the
/// backpack default (matching `content::executor::inventory::grant`).
async fn handle_give(
    caller_entity_id: u32,
    item_id: i32,
    count: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(player_id) = space_mgr
        .get_entity(caller_entity_id)
        .and_then(|e| e.player_id)
    else {
        send_gm_feedback(caller_entity_id, "Give failed: you are not a player.", tx).await;
        return;
    };

    // Reject non-positive counts and clamp the upper bound BEFORE the value
    // reaches `GrantItem` → the inventory SQL. A negative count subtracts
    // from an existing stack on the merge path (silent item destruction);
    // the column has no CHECK constraint, so the guard lives here
    // (server-authority audit, CAT-N-11). The shared `handle_grant_item`
    // sink is NOT the place to clamp — the content-engine grant path also
    // uses it with trusted counts.
    if count < 1 {
        send_gm_feedback(
            caller_entity_id,
            "Give failed: count must be a positive number.",
            tx,
        )
        .await;
        return;
    }
    let count = count.min(GIVE_COUNT_CAP);

    const DEFAULT_CONTAINER: i32 = 1; // backpack
    if let Err(e) = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id: caller_entity_id,
            player_id,
            item_id,
            container_id: DEFAULT_CONTAINER,
            count,
        })
        .await
    {
        tracing::warn!(caller_entity_id, item_id, error = %e, "gm_command: GrantItem send failed");
        send_gm_feedback(caller_entity_id, "Give failed: internal error.", tx).await;
        return;
    }

    send_gm_feedback(
        caller_entity_id,
        &format!("Gave you {count}x item {item_id}."),
        tx,
    )
    .await;
}

/// `/info` — dump a concise summary of the caller's current target (or self if
/// no target is selected).
async fn handle_info(
    caller_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let subject = space_mgr
        .get_entity(caller_entity_id)
        .and_then(|e| e.current_target_id)
        .map(|id| id as u32)
        .filter(|id| space_mgr.get_entity(*id).is_some())
        .unwrap_or(caller_entity_id);

    let Some(e) = space_mgr.get_entity(subject) else {
        send_gm_feedback(caller_entity_id, "Info failed: no entity.", tx).await;
        return;
    };

    let name = e
        .npc_name
        .clone()
        .or_else(|| e.template_id.map(|t| format!("template {t}")))
        .unwrap_or_else(|| {
            if e.is_player {
                "player".into()
            } else {
                "entity".into()
            }
        });
    let (hp_cur, hp_max) = e.stats.get(HEALTH).map_or((0, 0), |s| (s.cur, s.max));
    let text = format!(
        "[{}] {} | faction {} | hp {}/{} | pos ({:.1}, {:.1}, {:.1})",
        subject, name, e.faction, hp_cur, hp_max, e.position.x, e.position.y, e.position.z
    );
    send_gm_feedback(caller_entity_id, &text, tx).await;
}

/// `/who` — list the player entities in the caller's space.
///
/// Players carry no character name on the cell, so each entry is
/// `entity_id (player_id N)`.
async fn handle_who(
    caller_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_entity_id) else {
        send_gm_feedback(caller_entity_id, "Who failed: you are not in a space.", tx).await;
        return;
    };

    let mut entries: Vec<String> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .map(|pid| {
            let player_id = space_mgr.get_entity(pid).and_then(|e| e.player_id);
            match player_id {
                Some(p) => format!("{pid} (player_id {p})"),
                None => format!("{pid}"),
            }
        })
        .collect();
    entries.sort();

    let text = if entries.is_empty() {
        "No players in your space.".to_string()
    } else {
        format!("Players ({}): {}", entries.len(), entries.join(", "))
    };
    send_gm_feedback(caller_entity_id, &text, tx).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::math::Vector3;

    /// Build a `SpaceManager` with a non-instanced "Castle" world and a
    /// connected player at entity id 1. Non-instanced so the player and any
    /// spawned NPCs share one space (instanced worlds allocate a fresh space
    /// per `create_entity`).
    fn mgr_with_player() -> SpaceManager {
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
    fn template(name: &str) -> SpawnRecord {
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

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    /// The feedback line text for the LAST `onPlayerCommunication` (method 28)
    /// addressed to `entity_id`. Decodes the Text WSTRING (after the speaker
    /// WSTRING + flags + channel).
    fn feedback_text_to(msgs: &[CellToBaseMsg], entity_id: u32) -> Option<String> {
        msgs.iter().rev().find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: eid,
                method_index,
                args,
            } if *eid == entity_id && *method_index == ON_PLAYER_COMMUNICATION => {
                Some(decode_feedback_text(args))
            }
            _ => None,
        })
    }

    /// Decode the Text field of an `onPlayerCommunication` arg buffer.
    fn decode_feedback_text(args: &[u8]) -> String {
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

    #[tokio::test]
    async fn spawn_creates_entities_near_caller() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Jaffa Guard")];
        let (tx, mut rx) = mpsc::channel(32);

        let before = mgr.all_npc_entity_ids().len();
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "jaffa guard".to_string(), // case-insensitive match
                count: 3,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;

        let npcs = mgr.all_npc_entity_ids();
        assert_eq!(npcs.len() - before, 3, "should spawn exactly 3 NPCs");
        // Spawned at the caller's position, not the template's authored coord.
        for nid in &npcs {
            let e = mgr.get_entity(*nid).unwrap();
            assert_eq!([e.position.x, e.position.z], [5.0, 5.0]);
        }
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("spawn must feed back");
        assert!(fb.contains("Spawned 3x"), "got: {fb}");
    }

    #[tokio::test]
    async fn spawn_caps_count() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Drone")];
        let (tx, mut _rx) = mpsc::channel(64);
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "Drone".to_string(),
                count: 9999,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;
        assert_eq!(
            mgr.all_npc_entity_ids().len(),
            SPAWN_COUNT_CAP as usize,
            "count must be clamped to SPAWN_COUNT_CAP"
        );
    }

    #[tokio::test]
    async fn spawn_unknown_moniker_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Jaffa Guard")];
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "Nonexistent".to_string(),
                count: 1,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;
        assert!(mgr.all_npc_entity_ids().is_empty());
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no template named"), "got: {fb}");
    }

    #[tokio::test]
    async fn goto_coords_teleports_caller_and_emits_teleport_player() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::GotoCoords(Vector3::new(100.0, 0.0, 200.0)),
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!([e.position.x, e.position.z], [100.0, 200.0]);

        let msgs = drain(&mut rx);
        let teleport = msgs.iter().find_map(|m| match m {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                position,
                prev_pos,
                ..
            } if *entity_id == 1 => Some((*position, *prev_pos)),
            _ => None,
        });
        let (pos, prev) = teleport.expect("must emit TeleportPlayer for the caller");
        assert_eq!(pos, [100.0, 0.0, 200.0]);
        // prev_pos captured BEFORE the grid update = the spawn position.
        assert_eq!(prev, [5.0, 0.0, 5.0]);
    }

    #[tokio::test]
    async fn goto_player_jumps_to_target_by_entity_id() {
        let mut mgr = mgr_with_player();
        // Second player at id 2.
        mgr.create_entity(2, "Castle", [50.0, 0.0, 60.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);
        let (tx, mut rx) = mpsc::channel(8);

        handle_gm_command(
            1,
            GmCommandIntent::GotoPlayer("2".to_string()),
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!([e.position.x, e.position.z], [50.0, 60.0]);
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("Teleported to entity 2"), "got: {fb}");
    }

    #[tokio::test]
    async fn goto_player_unknown_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::GotoPlayer("Ghost".to_string()),
            &tx,
            &mut mgr,
            &[],
        )
        .await;
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no player 'Ghost'"), "got: {fb}");
    }

    #[tokio::test]
    async fn kill_zeroes_target_health_and_marks_dead() {
        let mut mgr = mgr_with_player();
        // Spawn an NPC co-located with the player (id 1 is at [5,0,5]) and
        // select it as the caller's current target. Co-location matters: the
        // AoI tick below only makes the player a witness of the NPC if it's
        // within AoI radius, and the corpse's dead-state flip fans out via
        // that witness link.
        let mut rec = template("Drone");
        rec.x = 5.0;
        rec.y = 0.0;
        rec.z = 5.0;
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record_in_space(npc_id, &rec, mgr.get_entity_space_id(1).unwrap())
            .unwrap();
        // Give it some health to zero out.
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            if let Some(s) = npc.stats.get_mut(HEALTH) {
                s.max = 500;
                s.set_current(500);
            }
        }
        if let Some(p) = mgr.get_entity_mut(1) {
            p.current_target_id = Some(npc_id as i32);
        }
        // Populate the player's witness set so the corpse's dead-state flip
        // fans out via WitnessEntityMethod instead of dropping at the
        // empty-witness branch of send_entity_method (same harness shape as
        // abilities::death tests).
        let _ = mgr.compute_aoi_changes();

        let (tx, mut rx) = mpsc::channel(64);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let npc = mgr.get_entity(npc_id).unwrap();
        assert_eq!(
            npc.stats.get(HEALTH).map(|s| s.cur),
            Some(0),
            "kill must zero target HEALTH"
        );
        assert_ne!(
            npc.state_field & combat::BSF_DEAD,
            0,
            "kill must set BSF_DEAD via mark_npc_dead"
        );
        // The death burst flips the corpse's state field on the wire.
        let msgs = drain(&mut rx);
        assert!(
            msgs.iter().any(|m| matches!(
                m,
                CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. }
                    | CellToBaseMsg::WitnessEntityMethod { entity_id, method_index, .. }
                    if *entity_id == npc_id
                        && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
            )),
            "death transition must emit onStateFieldUpdate for the corpse"
        );
        let fb = feedback_text_to(&msgs, 1).expect("kill must feed back");
        assert!(fb.starts_with("Killed"), "got: {fb}");
    }

    #[tokio::test]
    async fn kill_refuses_player_target() {
        let mut mgr = mgr_with_player();
        // Second player; select them as target.
        mgr.create_entity(2, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.current_target_id = Some(2);
        }
        let (tx, mut rx) = mpsc::channel(16);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        // Player target must NOT be killed.
        let p2 = mgr.get_entity(2).unwrap();
        assert_eq!(p2.state_field & combat::BSF_DEAD, 0, "player must not die");
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("is a player"), "got: {fb}");
    }

    #[tokio::test]
    async fn kill_no_target_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no target"), "got: {fb}");
    }

    #[tokio::test]
    async fn give_emits_grant_item_to_caller() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: 2,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let msgs = drain(&mut rx);
        let grant = msgs.iter().find_map(|m| match m {
            CellToBaseMsg::GrantItem {
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
            } => Some((*entity_id, *player_id, *item_id, *container_id, *count)),
            _ => None,
        });
        assert_eq!(
            grant,
            Some((1, 100, 55, 1, 2)),
            "GrantItem must target the caller"
        );
        let fb = feedback_text_to(&msgs, 1).unwrap();
        assert!(fb.contains("item 55"), "got: {fb}");
    }

    /// **HIGH-1 (CAT-N-11) regression guard.** A negative `/give` count
    /// must be REJECTED — never reach `GrantItem`, because a negative
    /// stack_size subtracts from an existing stack (silent item
    /// destruction). Reverting the `count < 1` guard trips this: the test
    /// would see a `GrantItem` emitted with count -5.
    #[tokio::test]
    async fn give_rejects_negative_count() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: -5,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let msgs = drain(&mut rx);
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GrantItem { .. })),
            "a negative count must NOT emit GrantItem (would corrupt stack_size)"
        );
        let fb = feedback_text_to(&msgs, 1).unwrap();
        assert!(fb.contains("must be a positive"), "got: {fb}");
    }

    /// **HIGH-1 upper-bound guard.** An absurd `/give` count is clamped to
    /// `GIVE_COUNT_CAP` before it reaches the inventory SQL.
    #[tokio::test]
    async fn give_clamps_absurd_count() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: 2_000_000_000,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let granted_count = drain(&mut rx).into_iter().find_map(|m| match m {
            CellToBaseMsg::GrantItem { count, .. } => Some(count),
            _ => None,
        });
        assert_eq!(
            granted_count,
            Some(GIVE_COUNT_CAP),
            "an absurd count must be clamped to GIVE_COUNT_CAP before GrantItem"
        );
    }

    #[tokio::test]
    async fn info_dumps_self_when_no_target() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(1, GmCommandIntent::Info, &tx, &mut mgr, &[]).await;
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("info must feed back to caller");
        // Self dump: entity id 1, has an hp field and a pos field.
        assert!(fb.contains("[1]"), "got: {fb}");
        assert!(fb.contains("hp"), "got: {fb}");
        assert!(fb.contains("pos"), "got: {fb}");
    }

    #[tokio::test]
    async fn who_lists_players_in_space() {
        let mut mgr = mgr_with_player();
        mgr.create_entity(2, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);
        if let Some(p) = mgr.get_entity_mut(2) {
            p.player_id = Some(200);
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(1, GmCommandIntent::Who, &tx, &mut mgr, &[]).await;
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("who must feed back");
        assert!(fb.contains("Players (2)"), "got: {fb}");
        assert!(
            fb.contains("player_id 100") && fb.contains("player_id 200"),
            "got: {fb}"
        );
    }

    /// Feedback is byte-identical to the chat module's serializer: speaker
    /// "SYSTEM", flags 0, channel 8 (CHAN_FEEDBACK), then the text WSTRING.
    #[tokio::test]
    async fn feedback_wire_shape_is_system_on_feedback_channel() {
        let (tx, mut rx) = mpsc::channel(4);
        send_gm_feedback(7, "hello", &tx).await;
        let msgs = drain(&mut rx);
        let args = match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(*entity_id, 7);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
                args.clone()
            }
            _ => panic!("expected EntityMethodCall"),
        };
        // Speaker "SYSTEM" = 6 UTF-16 chars.
        assert_eq!(u32::from_le_bytes(args[0..4].try_into().unwrap()), 6);
        let flags_off = 4 + 6 * 2;
        assert_eq!(args[flags_off], 0, "speaker flags must be 0");
        assert_eq!(
            args[flags_off + 1],
            CHAN_FEEDBACK,
            "channel must be feedback (8)"
        );
        assert_eq!(decode_feedback_text(&args), "hello");
    }
}
