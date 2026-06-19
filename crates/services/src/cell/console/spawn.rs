//! Spawn authoring / persistence console commands (category B):
//! `.savespawn`, `.delspawn`, `.autosavespawn`, `.respawnall`, `.spawnrandom`.
//!
//! Persistence follows the project's record-then-confirm model (see
//! [`crate::cell::console::seed`]): the command applies its effect in memory,
//! writes the live DB so it holds within the deploy, and records the canonical
//! seed SQL for a developer to confirm + commit. `.respawnall` is a pure
//! runtime reset (no authoring). `.spawnrandom` reuses the existing
//! cell↔base `GmSpawnNpc` round-trip.
//!
//! Legacy reference: `deprecated/python/cell/commands/Resource.py`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::seed;
use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Seed file the spawn rows live in.
const SPAWNLIST_SEED: &str = "db/resources/Worlds/Seed/spawnlist.sql";

pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    _engine: &ChainEngine,
) {
    match name {
        "savespawn" => save_spawn(caller_id, target_id, tx, space_mgr).await,
        "delspawn" => del_spawn(caller_id, target_id, tx, space_mgr).await,
        "autosavespawn" => autosave(caller_id, args, tx, space_mgr).await,
        "respawnall" => respawn_all(caller_id, tx, space_mgr).await,
        "spawnrandom" => spawn_random(caller_id, args, tx, space_mgr).await,
        _ => {}
    }
}

/// Yaw (radians) from a facing direction vector.
fn heading_of(dir: cimmeria_common::Vector3) -> f32 {
    dir.x.atan2(dir.z)
}

/// `.savespawn` — persist the targeted entity's current placement to
/// `resources.spawnlist`: `UPDATE` its existing row when it has a `spawn_id`,
/// otherwise `INSERT` a new one (resolving `world_id` from the world name via
/// subquery so the same statement is valid in the seed file and live).
async fn save_spawn(
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "savespawn: a target is required.", tx).await;
        return;
    };
    let Some(e) = space_mgr.get_entity(target) else {
        send_gm_feedback(caller_id, "savespawn: target not found.", tx).await;
        return;
    };
    let Some(template_id) = e.template_id else {
        send_gm_feedback(
            caller_id,
            "savespawn: target has no template (cannot persist).",
            tx,
        )
        .await;
        return;
    };
    let pos = e.position;
    let heading = heading_of(e.direction);
    let tag = e.tag.clone();
    let spawn_id = e.spawn_id;
    let space_id = e.space_id.0 as u32;
    let Some(world_name) = space_mgr
        .spaces
        .get(&space_id)
        .map(|s| s.world_name.clone())
    else {
        send_gm_feedback(caller_id, "savespawn: cannot resolve world.", tx).await;
        return;
    };

    // An already-seeded NPC updates its existing row in place; a command-spawned
    // one (no `spawn_id`) inserts a new row. Matches the legacy `saveSpawn`
    // INSERT-or-UPDATE and avoids duplicating a row on every re-save.
    let sql = match spawn_id {
        Some(id) => format!(
            "UPDATE resources.spawnlist SET x = {x}, y = {y}, z = {z}, heading = {heading}, \
             tag = {tag} WHERE spawn_id = {id};",
            x = pos.x,
            y = pos.y,
            z = pos.z,
            heading = heading,
            tag = seed::sql_str(tag.as_deref()),
        ),
        None => format!(
            "INSERT INTO resources.spawnlist (x, y, z, heading, world_id, template_id, tag)\n\
             SELECT {x}, {y}, {z}, {heading}, w.world_id, {template_id}, {tag}\n\
             FROM resources.worlds w WHERE w.world = {world};",
            x = pos.x,
            y = pos.y,
            z = pos.z,
            heading = heading,
            template_id = template_id,
            tag = seed::sql_str(tag.as_deref()),
            world = seed::sql_str(Some(&world_name)),
        ),
    };
    seed::record(caller_id, SPAWNLIST_SEED, "savespawn", &sql, tx, space_mgr).await;
}

/// `.delspawn` — remove the targeted entity in memory and record a `DELETE` of
/// its spawnlist row, keyed on the exact `spawn_id`. Command-spawned NPCs (no
/// spawnlist row) are refused.
async fn del_spawn(
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "delspawn: a target is required.", tx).await;
        return;
    };
    let Some(e) = space_mgr.get_entity(target) else {
        send_gm_feedback(caller_id, "delspawn: target not found.", tx).await;
        return;
    };
    let Some(spawn_id) = e.spawn_id else {
        send_gm_feedback(
            caller_id,
            "delspawn: target has no spawnlist row (command-spawned or a player).",
            tx,
        )
        .await;
        return;
    };

    // Key on the exact `spawn_id` — unambiguous, never matches a sibling spawn.
    let sql = format!("DELETE FROM resources.spawnlist WHERE spawn_id = {spawn_id};");
    seed::record(caller_id, SPAWNLIST_SEED, "delspawn", &sql, tx, space_mgr).await;

    // Apply in memory: despawn the entity now.
    space_mgr.destroy_entity(target);
    send_gm_feedback(
        caller_id,
        &format!("delspawn [{target}]: despawned in-memory."),
        tx,
    )
    .await;
}

/// `.autosavespawn <1|0>` — toggle the per-GM autosave preference.
async fn autosave(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(on) = super::parse_bool(caller_id, args, 0, "autosave", tx).await else {
        return;
    };
    if on {
        space_mgr.autosave_spawns.insert(caller_id);
    } else {
        space_mgr.autosave_spawns.remove(&caller_id);
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "autosavespawn {} — newly placed spawns should be followed by .savespawn",
            if on { "enabled" } else { "disabled" }
        ),
        tx,
    )
    .await;
}

/// `.respawnall` — reset every NPC in the caller's space to spawn state (full
/// health, spawn position, Idle, threat cleared). Runtime-only; no persistence.
async fn respawn_all(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_id) else {
        send_gm_feedback(caller_id, "respawnall: you are not in a space.", tx).await;
        return;
    };
    let npcs: Vec<u32> = space_mgr
        .all_npc_entity_ids()
        .into_iter()
        .filter(|&id| space_mgr.get_entity_space_id(id) == Some(space_id))
        .collect();
    let mut reset = 0usize;
    for id in &npcs {
        if let Some(e) = space_mgr.get_entity_mut(*id) {
            if let Some(spawn) = e.spawn_position {
                e.position = spawn;
            }
            if let Some(h) = e.stats.get_mut(HEALTH) {
                let max = h.max;
                h.cur = max;
            }
            e.threat_list.clear();
            e.respawn_at = None;
            e.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
            reset += 1;
        }
    }
    send_gm_feedback(
        caller_id,
        &format!("respawnall: reset {reset} NPC(s) in your space."),
        tx,
    )
    .await;
}

/// `.spawnrandom <templateId> <xRange> <zRange> [count]` — spawn `count` copies
/// of a template scattered around the caller, reusing the `GmSpawnNpc`
/// cell↔base round-trip. Scatter is deterministic (a ring around the caller) to
/// avoid an RNG dependency.
async fn spawn_random(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(template_id) = super::parse_i32(caller_id, args, 0, "templateId", tx).await else {
        return;
    };
    let Some(x_range) = super::parse_f32(caller_id, args, 1, "xRange", tx).await else {
        return;
    };
    let Some(z_range) = super::parse_f32(caller_id, args, 2, "zRange", tx).await else {
        return;
    };
    let count = match args.get(3) {
        Some(s) => s.parse::<i32>().unwrap_or(1).clamp(1, 50),
        None => 1,
    };

    let Some(space_id) = space_mgr.get_entity_space_id(caller_id) else {
        send_gm_feedback(caller_id, "spawnrandom: you are not in a space.", tx).await;
        return;
    };
    let Some(world_name) = space_mgr
        .spaces
        .get(&space_id)
        .map(|s| s.world_name.clone())
    else {
        send_gm_feedback(caller_id, "spawnrandom: cannot resolve world.", tx).await;
        return;
    };
    let Some(origin) = space_mgr.get_entity(caller_id).map(|e| e.position) else {
        return;
    };

    let mut delivered = 0i32;
    for i in 0..count {
        // Deterministic scatter: place copies evenly on a ring within the
        // x/z range (no RNG dependency, reproducible for authoring).
        let theta = (i as f32) * std::f32::consts::TAU / (count as f32).max(1.0);
        let pos = [
            origin.x + theta.cos() * x_range,
            origin.y,
            origin.z + theta.sin() * z_range,
        ];
        match tx
            .send(CellToBaseMsg::GmSpawnNpc {
                entity_id: caller_id,
                template_id,
                space_id,
                world_name: world_name.clone(),
                position: pos,
            })
            .await
        {
            Ok(()) => delivered += 1,
            Err(e) => {
                // The cell→base channel is closed/full — remaining sends will
                // fail too, so stop and report the partial count rather than
                // claiming all `count` were spawned.
                tracing::warn!(
                    caller_id,
                    template_id,
                    delivered,
                    requested = count,
                    "spawnrandom: GmSpawnNpc send failed — aborting remaining spawns: {e}"
                );
                break;
            }
        }
    }
    if delivered == count {
        send_gm_feedback(
            caller_id,
            &format!("spawnrandom: requested {count} x template {template_id}"),
            tx,
        )
        .await;
    } else {
        send_gm_feedback(
            caller_id,
            &format!(
                "spawnrandom: only {delivered}/{count} spawn requests delivered for \
                 template {template_id} (cell→base channel error)"
            ),
            tx,
        )
        .await;
    }
}
