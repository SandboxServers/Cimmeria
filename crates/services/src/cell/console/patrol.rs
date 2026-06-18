//! NPC patrol-path authoring console commands (category C) — the
//! FanMMORPG `path_*` family.
//!
//! The patrol **runtime** already exists (`CellEntity::patrol_path`,
//! `AiState::Patrol`, `cell::service::npc_ai::npc_ai_patrol`). These commands are
//! the authoring front-end: build a waypoint list at your avatar's position,
//! assign it to an NPC (which starts patrolling immediately), and record the
//! `point_set_points` / `spawnlist` seed SQL for commit.
//!
//! Path id == `point_sets.set_id`. The runtime loads patrol points from
//! `point_set_points` ordered by `point_id`; we therefore address "the Nth
//! waypoint" by `ORDER BY point_id OFFSET n` in the per-waypoint edits. Per-spawn
//! patrol assignment writes the `spawnlist.patrol_path_id` / `patrol_point_delay`
//! override columns (added to the schema), which the spawn loader
//! prefers over the template default.
//!
//! Reference: doko972/FanMMORPG `patrols.md` + `cell/commands/Resource.py`
//! (`pathAdd`…`pathSetTeleportDelay`).

use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

use super::seed;
use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const POINT_SETS_SEED: &str = "db/resources/Events/Seed/point_sets.sql";
const POINT_SET_POINTS_SEED: &str = "db/resources/Events/Seed/point_set_points.sql";
const SPAWNLIST_SEED: &str = "db/resources/Worlds/Seed/spawnlist.sql";

pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "path_add" => path_add(caller_id, args, tx, space_mgr).await,
        "path_show" => path_show(caller_id, args, tx, space_mgr).await,
        "path_clear" => path_clear(caller_id, args, tx, space_mgr).await,
        "path_assign" => path_assign(caller_id, args, target_id, tx, space_mgr).await,
        "path_unassign" => path_unassign(caller_id, target_id, tx, space_mgr).await,
        "path_set_seq" => waypoint_set(caller_id, args, "sequence_id", true, tx, space_mgr).await,
        "path_clear_seq" => {
            waypoint_set(caller_id, args, "sequence_id", false, tx, space_mgr).await
        }
        "path_set_tp" => path_set_tp(caller_id, args, tx, space_mgr).await,
        "path_clear_tp" => path_clear_tp(caller_id, args, tx, space_mgr).await,
        "path_set_tp_seq" => {
            waypoint_set(caller_id, args, "teleport_sequence_id", true, tx, space_mgr).await
        }
        "path_set_tp_delay" => path_set_tp_delay(caller_id, args, tx, space_mgr).await,
        _ => {}
    }
}

/// Parse the `path_id` arg (always positional 0).
async fn parse_path_id(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<i32> {
    super::parse_i32(caller_id, args, 0, "pathId", tx).await
}

/// Parse a non-negative waypoint `index` (positional 1). A negative value would
/// become an invalid `OFFSET -1` in the per-waypoint UPDATE subquery.
async fn parse_index(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<i32> {
    let index = super::parse_i32(caller_id, args, 1, "index", tx).await?;
    if index < 0 {
        send_gm_feedback(caller_id, "index must be >= 0", tx).await;
        return None;
    }
    Some(index)
}

/// `.path_add <pathId>` — append the caller's position as the next waypoint.
/// On the first waypoint of a path, also records the `point_sets` header row
/// (FK parent of `point_set_points`).
async fn path_add(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(pos) = space_mgr.get_entity(caller_id).map(|e| e.position) else {
        return;
    };
    let space_id = space_mgr.get_entity_space_id(caller_id);
    let world_name = space_id
        .and_then(|sid| space_mgr.spaces.get(&sid))
        .map(|s| s.world_name.clone());

    let first = space_mgr
        .patrol_authoring
        .get(&path_id)
        .is_none_or(Vec::is_empty);

    // First waypoint: ensure the point_sets header exists (FK parent).
    if first {
        let Some(world) = world_name.clone() else {
            send_gm_feedback(caller_id, "path_add: cannot resolve world.", tx).await;
            return;
        };
        let header = format!(
            "INSERT INTO resources.point_sets (set_id, name, type, world_id, shape, flags)\n\
             SELECT {path_id}, {name}, 'Patrol', w.world_id, 'Path', 1\n\
             FROM resources.worlds w WHERE w.world = {world}\n\
             ON CONFLICT (set_id) DO NOTHING;",
            name = seed::sql_str(Some(&format!("console-path-{path_id}"))),
            world = seed::sql_str(Some(&world)),
        );
        seed::record(
            caller_id,
            POINT_SETS_SEED,
            "path_add",
            &header,
            tx,
            space_mgr,
        )
        .await;
    }

    let point = format!(
        "INSERT INTO resources.point_set_points (set_id, x, y, z, yaw, pitch, roll)\n\
         VALUES ({path_id}, {x}, {y}, {z}, 0, 0, 0);",
        x = pos.x,
        y = pos.y,
        z = pos.z,
    );
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_add",
        &point,
        tx,
        space_mgr,
    )
    .await;

    let idx = {
        let buf = space_mgr.patrol_authoring.entry(path_id).or_default();
        buf.push(pos);
        buf.len() - 1
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "path_add [{path_id}] point {idx}: ({:.2}, {:.2}, {:.2})",
            pos.x, pos.y, pos.z
        ),
        tx,
    )
    .await;
}

/// `.path_show <pathId>` — show the session-authored waypoints for a path.
async fn path_show(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let points = space_mgr
        .patrol_authoring
        .get(&path_id)
        .cloned()
        .unwrap_or_default();
    if points.is_empty() {
        send_gm_feedback(
            caller_id,
            &format!("path_show [{path_id}]: no waypoints authored this session."),
            tx,
        )
        .await;
        return;
    }
    send_gm_feedback(
        caller_id,
        &format!("path_show [{path_id}] {} point(s):", points.len()),
        tx,
    )
    .await;
    for (i, p) in points.iter().enumerate() {
        send_gm_feedback(
            caller_id,
            &format!("  [{i}] ({:.2}, {:.2}, {:.2})", p.x, p.y, p.z),
            tx,
        )
        .await;
    }
}

/// `.path_clear <pathId>` — drop the path's waypoints (buffer + DB rows).
async fn path_clear(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    space_mgr.patrol_authoring.remove(&path_id);
    // Two SINGLE statements, recorded separately: the live-write executor runs
    // each via a prepared statement (which rejects multi-command strings), and
    // each lands in its own seed file. Points first (FK child), then the header.
    let points = format!("DELETE FROM resources.point_set_points WHERE set_id = {path_id};");
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_clear",
        &points,
        tx,
        space_mgr,
    )
    .await;
    let header = format!("DELETE FROM resources.point_sets WHERE set_id = {path_id};");
    seed::record(
        caller_id,
        POINT_SETS_SEED,
        "path_clear",
        &header,
        tx,
        space_mgr,
    )
    .await;
}

/// `.path_assign <pathId> [delay]` — assign a path to the targeted NPC's spawn
/// and start it patrolling now. Applies the session waypoints to the NPC's
/// `patrol_path` in memory and records the `spawnlist` override.
async fn path_assign(
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "path_assign: target an NPC first.", tx).await;
        return;
    };
    // Default 3.0 when omitted; if supplied it must be a finite number — a bad
    // value is a typo, not a silent fallback (it would also corrupt the UPDATE).
    let delay = match args.get(1) {
        None => 3.0,
        Some(s) => match s.parse::<f32>().ok().filter(|v| v.is_finite()) {
            Some(v) => v,
            None => {
                send_gm_feedback(caller_id, "path_assign: delay must be a finite number.", tx)
                    .await;
                return;
            }
        },
    };
    let waypoints = space_mgr
        .patrol_authoring
        .get(&path_id)
        .cloned()
        .unwrap_or_default();
    if waypoints.is_empty() {
        send_gm_feedback(
            caller_id,
            &format!(
                "path_assign: path {path_id} has no authored waypoints — use .path_add first."
            ),
            tx,
        )
        .await;
        return;
    }

    // Apply in memory: the NPC starts patrolling immediately.
    let spawn_id = space_mgr.get_entity(target).and_then(|e| e.spawn_id);
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.patrol_path = waypoints;
        e.patrol_point_delay_secs = delay;
        e.patrol_next_index = 0;
        e.ai_state = AiState::Patrol;
    }

    // Persist the per-spawn override, keyed on the exact `spawn_id`. A
    // command-spawned NPC has no spawnlist row, so it patrols this session only.
    let Some(spawn_id) = spawn_id else {
        send_gm_feedback(
            caller_id,
            &format!(
                "path_assign [{path_id}] -> NPC {target}: patrolling now (in-memory only — \
                 no spawnlist row to persist the override to)."
            ),
            tx,
        )
        .await;
        return;
    };
    let sql = format!(
        "UPDATE resources.spawnlist SET patrol_path_id = {path_id}, patrol_point_delay = {delay} \
         WHERE spawn_id = {spawn_id};"
    );
    seed::record(
        caller_id,
        SPAWNLIST_SEED,
        "path_assign",
        &sql,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_id,
        &format!("path_assign [{path_id}] -> NPC {target} (delay {delay}s), patrolling now."),
        tx,
    )
    .await;
}

/// `.path_unassign` — stop the targeted NPC patrolling and clear its spawn
/// override.
async fn path_unassign(
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "path_unassign: target an NPC first.", tx).await;
        return;
    };
    let spawn_id = space_mgr.get_entity(target).and_then(|e| e.spawn_id);
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.patrol_path.clear();
        e.patrol_next_index = 0;
        e.ai_state = AiState::Idle;
    }
    if let Some(spawn_id) = spawn_id {
        let sql = format!(
            "UPDATE resources.spawnlist SET patrol_path_id = NULL, patrol_point_delay = NULL \
             WHERE spawn_id = {spawn_id};"
        );
        seed::record(
            caller_id,
            SPAWNLIST_SEED,
            "path_unassign",
            &sql,
            tx,
            space_mgr,
        )
        .await;
    }
    send_gm_feedback(
        caller_id,
        &format!("path_unassign: NPC {target} stopped patrolling."),
        tx,
    )
    .await;
}

/// Shared body for the integer-valued per-waypoint edits (`path_set_seq`,
/// `path_clear_seq`, `path_set_tp_seq`). `set = false` writes NULL (clear). The
/// Nth waypoint is addressed by `ORDER BY point_id OFFSET n`.
async fn waypoint_set(
    caller_id: u32,
    args: &[&str],
    column: &str,
    set: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(index) = parse_index(caller_id, args, tx).await else {
        return;
    };
    let value = if set {
        match super::parse_i32(caller_id, args, 2, "value", tx).await {
            Some(v) => v.to_string(),
            None => return,
        }
    } else {
        "NULL".to_string()
    };
    let sql = format!(
        "UPDATE resources.point_set_points SET {column} = {value}\n\
         WHERE point_id = (SELECT point_id FROM resources.point_set_points\n\
                           WHERE set_id = {path_id} ORDER BY point_id OFFSET {index} LIMIT 1);"
    );
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_seq",
        &sql,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_id,
        &format!("path [{path_id}] point {index}: {column} = {value}"),
        tx,
    )
    .await;
}

/// `.path_set_tp <pathId> <index>` — set the waypoint's teleport destination to
/// the caller's current position.
async fn path_set_tp(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(index) = parse_index(caller_id, args, tx).await else {
        return;
    };
    let Some(pos) = space_mgr.get_entity(caller_id).map(|e| e.position) else {
        return;
    };
    let sql = format!(
        "UPDATE resources.point_set_points SET teleport_x = {x}, teleport_y = {y}, teleport_z = {z}\n\
         WHERE point_id = (SELECT point_id FROM resources.point_set_points\n\
                           WHERE set_id = {path_id} ORDER BY point_id OFFSET {index} LIMIT 1);",
        x = pos.x,
        y = pos.y,
        z = pos.z,
    );
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_set_tp",
        &sql,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_id,
        &format!(
            "path_set_tp [{path_id}] point {index} -> ({:.2}, {:.2}, {:.2})",
            pos.x, pos.y, pos.z
        ),
        tx,
    )
    .await;
}

/// `.path_clear_tp <pathId> <index>` — clear a waypoint's teleport.
async fn path_clear_tp(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(index) = parse_index(caller_id, args, tx).await else {
        return;
    };
    let sql = format!(
        "UPDATE resources.point_set_points\n\
         SET teleport_x = NULL, teleport_y = NULL, teleport_z = NULL, teleport_sequence_id = NULL, teleport_delay = NULL\n\
         WHERE point_id = (SELECT point_id FROM resources.point_set_points\n\
                           WHERE set_id = {path_id} ORDER BY point_id OFFSET {index} LIMIT 1);"
    );
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_clear_tp",
        &sql,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_id,
        &format!("path_clear_tp [{path_id}] point {index}"),
        tx,
    )
    .await;
}

/// `.path_set_tp_delay <pathId> <index> <delay>` — set the post-departure
/// teleport delay (seconds) on a waypoint.
async fn path_set_tp_delay(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(path_id) = parse_path_id(caller_id, args, tx).await else {
        return;
    };
    let Some(index) = parse_index(caller_id, args, tx).await else {
        return;
    };
    let Some(delay) = super::parse_f32(caller_id, args, 2, "delay", tx).await else {
        return;
    };
    let sql = format!(
        "UPDATE resources.point_set_points SET teleport_delay = {delay}\n\
         WHERE point_id = (SELECT point_id FROM resources.point_set_points\n\
                           WHERE set_id = {path_id} ORDER BY point_id OFFSET {index} LIMIT 1);"
    );
    seed::record(
        caller_id,
        POINT_SET_POINTS_SEED,
        "path_set_tp_delay",
        &sql,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_id,
        &format!("path_set_tp_delay [{path_id}] point {index} = {delay}s"),
        tx,
    )
    .await;
}
