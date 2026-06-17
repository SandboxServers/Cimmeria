//! GM query handlers that report text back to the caller via the
//! single-recipient [`super::feedback`] channel — `gmUsers`, `testLOS`, and the
//! inspection trio `gmShowPlayer` / `gmShowTargetLocation` / `gmShowRotation`
//! (the native equivalents of FanMMORPG's `.info` / `.location` / `.rotation`).
//!
//! All read cell-side state and report it through
//! [`super::feedback::send_gm_feedback`].

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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
