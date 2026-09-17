//! Spawnlist authoring / persistence half of the spawn console family:
//! `.savespawn`, `.delspawn`, `.autosavespawn`.
//!
//! These are the commands that touch `resources.spawnlist`. They follow the
//! project's record-then-confirm model (see [`crate::cell::console::seed`]):
//! apply the effect in memory, write the live DB so it holds within the
//! deploy, and record the canonical seed SQL for a developer to confirm +
//! commit.
//!
//! The ephemeral lifecycle commands (`.spawn`, `.despawn`, `.respawnall`,
//! `.spawnrandom`) live in the parent module and never write a spawnlist row.
//!
//! Legacy reference: `deprecated/python/cell/commands/Resource.py`.

use tokio::sync::mpsc;

use crate::cell::console::{parse_bool, seed, send_gm_feedback};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

/// Seed file the spawn rows live in.
const SPAWNLIST_SEED: &str = "db/resources/Worlds/Seed/spawnlist.sql";

/// `.savespawn` — persist the targeted entity's current placement to
/// `resources.spawnlist`: `UPDATE` its existing row when it has a `spawn_id`,
/// otherwise `INSERT` a new one (resolving `world_id` from the world name via
/// subquery so the same statement is valid in the seed file and live).
pub(super) async fn save_spawn(
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
    // direction.y is yaw directly (see caller_placement's doc comment in the
    // parent module) -- not a facing vector to atan2.
    let heading = e.direction.y;
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
pub(super) async fn del_spawn(
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

    // Apply in memory: despawn the entity now. `despawn_npc` (not bare
    // `destroy_entity`) fans LeftAoI out to every current witness and scrubs
    // the target from every witness set immediately, rather than leaving
    // observers to notice on the next AoI tick.
    let outcome = space_mgr.despawn_npc(target, tx).await;
    let msg = match outcome {
        DespawnOutcome::Despawned { witnesses_notified } => {
            format!("delspawn [{target}]: despawned in-memory ({witnesses_notified} witness(es) notified).")
        }
        // Both already ruled out above (existence + spawn_id checks), but
        // despawn_npc re-checks independently (D02) -- report truthfully
        // rather than assume the earlier checks still hold.
        DespawnOutcome::NotFound => format!("delspawn [{target}]: target not found."),
        DespawnOutcome::RefusedPlayer => {
            format!("delspawn [{target}]: refused -- target is a player.")
        }
    };
    send_gm_feedback(caller_id, &msg, tx).await;
}

/// `.autosavespawn <1|0>` — toggle the per-GM autosave preference.
pub(super) async fn autosave(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(on) = parse_bool(caller_id, args, 0, "autosave", tx).await else {
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
