//! GM query handlers that report text back to the caller via the
//! single-recipient [`super::feedback`] channel — `gmUsers`, `testLOS`, and the
//! inspection trio `gmShowPlayer` / `gmShowTargetLocation` / `gmShowRotation`
//! (the native equivalents of FanMMORPG's `.info` / `.location` / `.rotation`).
//!
//! All read cell-side state and report it through
//! [`super::feedback::send_gm_feedback`].

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// The "subject" of a no-arg inspection command: the caller's current target if
/// it's set and in the caller's space, else the caller themselves. Mirrors
/// FanMMORPG's "target the entity, then inspect; default to self" pattern.
fn subject_or_self(caller: u32, space_mgr: &SpaceManager) -> u32 {
    let caller_space = space_mgr.get_entity(caller).map(|e| e.space_id.0);
    space_mgr
        .get_entity(caller)
        .and_then(|e| e.current_target_id)
        .and_then(|id| u32::try_from(id).ok())
        .filter(|&id| space_mgr.get_entity(id).map(|e| e.space_id.0) == caller_space)
        .unwrap_or(caller)
}

/// Short human label for an entity: NPC name → `template N` → player/entity.
fn describe(e: &CellEntity) -> String {
    e.npc_name
        .clone()
        .or_else(|| e.template_id.map(|t| format!("template {t}")))
        .unwrap_or_else(|| {
            if e.is_player {
                "player".into()
            } else {
                "entity".into()
            }
        })
}

/// `gmUsers()` — list the players in the caller's space.
///
/// Scope note: this is **space-scoped**, not all-shard. The cell only knows the
/// entities in its own spaces; a true server-wide user list would need a
/// base-side round-trip to `online_players`. For a dev tool, "who's in my
/// space" is the useful and self-contained answer.
pub(super) async fn handle_users(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        send_gm_feedback(entity_id, "gmUsers: you are not in a space.", tx).await;
        return true;
    };

    let mut players: Vec<(u32, Option<i32>)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .map(|pid| (pid, space_mgr.get_entity(pid).and_then(|e| e.player_id)))
        .collect();
    players.sort_by_key(|&(pid, _)| pid);

    let text = if players.is_empty() {
        "gmUsers: no players in your space.".to_string()
    } else {
        let list = players
            .iter()
            .map(|(pid, player_id)| match player_id {
                Some(p) => format!("{pid} (char {p})"),
                None => format!("{pid}"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("gmUsers ({} in space): {list}", players.len())
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `testLOS(INT32 aSourceEntityID, INT32 aTargetEntityID)` — report whether the
/// navmesh has line-of-sight between two entities in the caller's space.
///
/// Reuses the canonical [`SpaceManager::has_line_of_sight`] primitive (the same
/// one the NPC AI uses), which resolves the space, projects to the navmesh, and
/// raycasts — returning `true` for clear LoS (and conservatively `true` when no
/// navmesh is loaded). Both ids are validated to be in the caller's space first
/// so a typo'd id reports "not found" rather than a misleading CLEAR.
pub(super) async fn handle_test_los(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (Some(source), Some(target)) = (read_i32(args, 0), read_i32(args, 4)) else {
        send_gm_feedback(entity_id, "testLOS: need two INT32 entity ids.", tx).await;
        return true;
    };
    let (Ok(source_eid), Ok(target_eid)) = (u32::try_from(source), u32::try_from(target)) else {
        send_gm_feedback(entity_id, "testLOS: entity ids out of range.", tx).await;
        return true;
    };

    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let in_caller_space =
        |eid: u32| space_mgr.get_entity(eid).map(|e| e.space_id.0) == caller_space;
    if !in_caller_space(source_eid) || !in_caller_space(target_eid) {
        send_gm_feedback(
            entity_id,
            "testLOS: source/target not found in your space.",
            tx,
        )
        .await;
        return true;
    }

    let clear = space_mgr.has_line_of_sight(source_eid, target_eid);
    let verdict = if clear { "CLEAR" } else { "BLOCKED" };
    let text = format!("testLOS {source_eid} → {target_eid}: {verdict}");
    tracing::info!(entity_id, source_eid, target_eid, clear, "testLOS");
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmShowTargetLocation()` — report the current target's (or caller's)
/// position. Native equivalent of FanMMORPG's `.location`.
pub(super) async fn handle_show_target_location(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let subject = subject_or_self(entity_id, space_mgr);
    let text = match space_mgr.get_entity(subject) {
        Some(e) => format!(
            "loc [{subject}] {}: ({:.2}, {:.2}, {:.2})",
            describe(e),
            e.position.x,
            e.position.y,
            e.position.z
        ),
        None => "gmShowTargetLocation: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmShowRotation()` — report the current target's (or caller's) facing as a
/// heading in degrees plus the raw direction vector. Native equivalent of
/// FanMMORPG's `.rotation` / `.facing`.
pub(super) async fn handle_show_rotation(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let subject = subject_or_self(entity_id, space_mgr);
    let text = match space_mgr.get_entity(subject) {
        Some(e) => {
            let d = e.direction;
            let heading = d.x.atan2(d.z).to_degrees();
            format!(
                "rot [{subject}] {}: heading {heading:.1}° dir ({:.2}, {:.2}, {:.2})",
                describe(e),
                d.x,
                d.y,
                d.z
            )
        }
        None => "gmShowRotation: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmShowPlayer(INT32 TargetID)` — dump id / name / kind / faction / level /
/// health / position for the entity. Native equivalent of FanMMORPG's `.info`;
/// works for any entity (not just players). A `TargetID` of 0 falls back to the
/// current target, then the caller.
pub(super) async fn handle_show_player(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let subject = match read_i32(args, 0) {
        Some(id) if id > 0 => match u32::try_from(id) {
            Ok(eid) => eid,
            Err(_) => {
                send_gm_feedback(entity_id, "gmShowPlayer: target id out of range.", tx).await;
                return true;
            }
        },
        // 0 / negative / missing → current target or self.
        _ => subject_or_self(entity_id, space_mgr),
    };

    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let text = match space_mgr.get_entity(subject) {
        Some(e) if Some(e.space_id.0) == caller_space => {
            let (hp_cur, hp_max) = e.stats.get(HEALTH).map_or((0, 0), |s| (s.cur, s.max));
            let kind = if e.is_player { "player" } else { "npc" };
            format!(
                "[{subject}] {} | {kind} | faction {} | lvl {} | hp {hp_cur}/{hp_max} | ({:.1}, {:.1}, {:.1})",
                describe(e),
                e.faction,
                e.level,
                e.position.x,
                e.position.y,
                e.position.z
            )
        }
        Some(_) => "gmShowPlayer: target is in a different space.".to_string(),
        None => "gmShowPlayer: no such entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `listAbilities()` — list the known ability ids of the current target (or
/// caller). FanMMORPG-adjacent inspection helper.
pub(super) async fn handle_list_abilities(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let subject = subject_or_self(entity_id, space_mgr);
    let text = match space_mgr.get_entity(subject) {
        Some(e) => {
            let ids = e.abilities.known_ability_ids();
            if ids.is_empty() {
                format!("abilities [{subject}]: none")
            } else {
                let list = ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("abilities [{subject}] ({}): {list}", ids.len())
            }
        }
        None => "listAbilities: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmShowFlag(INT32 flagId)` — report whether state-flag bit `flagId` is set on
/// the current target (or caller), plus the raw `state_field`.
pub(super) async fn handle_show_flag(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(flag_id) = read_i32(args, 0) else {
        send_gm_feedback(entity_id, "gmShowFlag: need an INT32 flag id.", tx).await;
        return true;
    };
    if !(0..32).contains(&flag_id) {
        send_gm_feedback(
            entity_id,
            "gmShowFlag: flag id must be a bit index 0-31.",
            tx,
        )
        .await;
        return true;
    }
    let subject = subject_or_self(entity_id, space_mgr);
    let text = match space_mgr.get_entity(subject) {
        Some(e) => {
            let mask = 1u32 << flag_id;
            let set = e.has_state_flag(mask);
            format!(
                "flag [{subject}] bit {flag_id}: {} (state_field=0x{:08x})",
                if set { "SET" } else { "clear" },
                e.state_field
            )
        }
        None => "gmShowFlag: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmGetMobAttribute(INT32 TargetID, WSTRING Attribute)` — report one
/// hand-mapped attribute of the target. No reflection; a fixed set of useful
/// fields is supported.
pub(super) async fn handle_get_mob_attribute(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(target) = read_i32(args, 0) else {
        send_gm_feedback(
            entity_id,
            "gmGetMobAttribute: need INT32 target + WSTRING attr.",
            tx,
        )
        .await;
        return true;
    };
    let attr = match read_wstring(args, 4) {
        Ok((s, _)) => s,
        Err(_) => {
            send_gm_feedback(entity_id, "gmGetMobAttribute: malformed attribute.", tx).await;
            return true;
        }
    };
    let Ok(target_eid) = u32::try_from(target) else {
        send_gm_feedback(entity_id, "gmGetMobAttribute: target id out of range.", tx).await;
        return true;
    };
    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let text = match space_mgr.get_entity(target_eid) {
        Some(e) if Some(e.space_id.0) == caller_space => match attr.trim().to_lowercase().as_str() {
            "health" | "hp" => e.stats.get(HEALTH).map_or("health: n/a".into(), |s| format!("health: {}/{}", s.cur, s.max)),
            "focus" => e.stats.get(FOCUS).map_or("focus: n/a".into(), |s| format!("focus: {}/{}", s.cur, s.max)),
            "level" | "lvl" => format!("level: {}", e.level),
            "faction" => format!("faction: {}", e.faction),
            "alignment" => format!("alignment: {}", e.alignment),
            "aistate" | "ai" => format!("ai_state: {:?}", e.ai_state),
            "name" => format!("name: {}", describe(e)),
            "template" => format!("template: {:?}", e.template_id),
            "pos" | "position" => format!("pos: ({:.1}, {:.1}, {:.1})", e.position.x, e.position.y, e.position.z),
            other => format!("gmGetMobAttribute: unknown attribute '{other}' (try health/focus/level/faction/alignment/aistate/name/template/pos)"),
        },
        Some(_) => "gmGetMobAttribute: target is in a different space.".to_string(),
        None => "gmGetMobAttribute: no such entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmShowMobCount(INT32 SpaceID)` — count NPCs (non-players) in a space.
/// `SpaceID == 0` means the caller's current space.
pub(super) async fn handle_show_mob_count(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let arg = read_i32(args, 0).unwrap_or(0);
    let target_space = if arg == 0 {
        space_mgr.get_entity_space_id(entity_id)
    } else {
        u32::try_from(arg).ok()
    };
    let Some(target_space) = target_space else {
        send_gm_feedback(entity_id, "gmShowMobCount: no space.", tx).await;
        return true;
    };
    let count = space_mgr
        .all_npc_entity_ids()
        .into_iter()
        .filter(|&id| space_mgr.get_entity_space_id(id) == Some(target_space))
        .count();
    send_gm_feedback(
        entity_id,
        &format!("gmShowMobCount: {count} NPC(s) in space {target_space}"),
        tx,
    )
    .await;
    true
}

/// `gmDebugMobData(INT32 aSpaceID, INT32 target)` — dump a mob's debug data.
pub(super) async fn handle_debug_mob_data(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Args: (aSpaceID, target). We resolve the target directly; spaceId is the
    // client's hint and isn't needed once we have the entity id.
    let Some(target) = read_i32(args, 4) else {
        send_gm_feedback(
            entity_id,
            "gmDebugMobData: need INT32 spaceId + INT32 target.",
            tx,
        )
        .await;
        return true;
    };
    let Ok(target_eid) = u32::try_from(target) else {
        send_gm_feedback(entity_id, "gmDebugMobData: target id out of range.", tx).await;
        return true;
    };
    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let text = match space_mgr.get_entity(target_eid) {
        Some(e) if Some(e.space_id.0) == caller_space => {
            let (hp_cur, hp_max) = e.stats.get(HEALTH).map_or((0, 0), |s| (s.cur, s.max));
            format!(
                "mob [{target_eid}] {} | tmpl {:?} | ai {:?} | faction {} lvl {} | hp {hp_cur}/{hp_max} | threat {} | tag {:?}",
                describe(e),
                e.template_id,
                e.ai_state,
                e.faction,
                e.level,
                e.threat_list.len(),
                e.tag
            )
        }
        Some(_) => "gmDebugMobData: target is in a different space.".to_string(),
        None => "gmDebugMobData: no such entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}
