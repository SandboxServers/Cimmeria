//! Spawn console commands (category B): `.spawn`, `.despawn`, `.respawnall`,
//! `.spawnrandom` here, plus `.savespawn` / `.delspawn` / `.autosavespawn` in
//! [`authoring`].
//!
//! The family splits along the persistence seam:
//!
//! - **This module — ephemeral lifecycle.** `.spawn` / `.despawn` create and
//!   destroy a runtime entity; `.respawnall` resets NPCs in place;
//!   `.spawnrandom` scatters copies. None of them read or write
//!   `resources.spawnlist`. `.spawn` / `.spawnrandom` reuse the existing
//!   cell↔base `GmSpawnNpc` round-trip, which is also what makes their
//!   feedback truthful: the request is enqueued silently and the definitive
//!   outcome line is sent by whichever side learns the real result (the base
//!   for "template not found", the cell for "spawned npc `<id>`").
//! - **[`authoring`] — spawnlist persistence.** Saving a placed entity and
//!   deleting its row are deliberately separate commands from creating and
//!   destroying one, exactly as in the legacy roster.
//!
//! Legacy reference: `deprecated/python/cell/commands/Resource.py`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

mod authoring;

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
        "spawn" => spawn_entity(caller_id, args, tx, space_mgr).await,
        "despawn" => {
            despawn_entity(
                caller_id,
                target_id.expect("Target::Mob guarantees a resolved target"),
                tx,
                space_mgr,
            )
            .await
        }
        "savespawn" => authoring::save_spawn(caller_id, target_id, tx, space_mgr).await,
        "delspawn" => authoring::del_spawn(caller_id, target_id, tx, space_mgr).await,
        "autosavespawn" => authoring::autosave(caller_id, args, tx, space_mgr).await,
        "respawnall" => respawn_all(caller_id, tx, space_mgr).await,
        "spawnrandom" => spawn_random(caller_id, args, tx, space_mgr).await,
        _ => {}
    }
}

/// The caller's space, world name, position and facing — everything a spawn
/// request needs to place an entity "right where I'm standing, facing the way
/// I'm facing" (legacy `space.createEntity(template, player.position,
/// player.rotation)`).
///
/// Facing is `e.direction.y` — the wire packs `direction` as
/// `[pitch, yaw, roll]` (`pack_angle(direction[1]) // yaw` in
/// `mercury/aoi/{create,update}.rs`), and NPC movement writes the same
/// convention directly (`npc.direction = Vector3::new(0.0, yaw, 0.0)` in
/// `cell/service/ticks/npc_movement.rs`). `direction` is never a literal
/// facing *vector* to `atan2` — a prior version of this function computed
/// `dir.x.atan2(dir.z)`, which reads pitch/roll as if they were x/z vector
/// components and produces an unrelated angle for every caller.
///
/// Returns `None` after sending the GM the specific reason, so callers can
/// simply `let Some(p) = caller_placement(..) else { return };`.
async fn caller_placement(
    cmd: &str,
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<(u32, String, [f32; 3], f32)> {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_id) else {
        send_gm_feedback(caller_id, &format!("{cmd}: you are not in a space."), tx).await;
        return None;
    };
    let Some(world_name) = space_mgr
        .spaces
        .get(&space_id)
        .map(|s| s.world_name.clone())
    else {
        send_gm_feedback(caller_id, &format!("{cmd}: cannot resolve world."), tx).await;
        return None;
    };
    let Some(e) = space_mgr.get_entity(caller_id) else {
        send_gm_feedback(caller_id, &format!("{cmd}: caller entity not found."), tx).await;
        return None;
    };
    Some((
        space_id,
        world_name,
        [e.position.x, e.position.y, e.position.z],
        e.direction.y,
    ))
}

/// `.spawn <templateId>` — create one NPC from a template at the caller's exact
/// position and facing.
///
/// Legacy `Resource.spawnEntity` (`deprecated/python/cell/commands/Resource.py`):
/// look the template up, refuse an unknown one, otherwise
/// `player.space.createEntity(template, player.position, player.rotation)`.
/// Two deliberate corrections to the legacy body:
///
/// 1. **Feedback is deferred to the real creation result.** Legacy printed
///    `'Spawning entity of type <%s>'` *before* calling `createEntity`, so a
///    creation that failed still read as a success. Here the enqueue is
///    silent; the definitive `spawned npc <id> (template <n>)` line comes from
///    the cell's `GmSpawnNpcReady` handler once the entity actually exists,
///    and the base sends `spawn failed: template <n> not found` when the
///    catalog lookup misses. Same contract the native `gmSpawnByCmd` already
///    holds.
/// 2. **No autosave.** Legacy chained `saveSpawn(player, target)` on the GM's
///    `autosaveSpawns` flag, using the *stale selected target* rather than the
///    entity it had just created. Consuming the preference correctly against
///    the new entity is P11's packet; this command is purely ephemeral.
async fn spawn_entity(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(template_id) = super::parse_i32(caller_id, args, 0, "templateId", tx).await else {
        return;
    };
    // Cell-side catalog pre-check: only the base can confirm a template
    // *exists*, but a non-positive id can never name one, so reject it here
    // rather than burn a round-trip on a query that cannot match.
    if template_id <= 0 {
        send_gm_feedback(
            caller_id,
            &format!("spawn: templateId must be a positive template id (got {template_id})."),
            tx,
        )
        .await;
        return;
    }

    let Some((space_id, world_name, position, heading)) =
        caller_placement("spawn", caller_id, tx, space_mgr).await
    else {
        return;
    };
    // The caller's own position should always be finite; if it somehow isn't,
    // refuse rather than plant an un-indexable entity. Mirrors the native
    // `gmSpawnByCmd` finite check.
    if !position.iter().all(|c| c.is_finite()) || !heading.is_finite() {
        send_gm_feedback(
            caller_id,
            "spawn: your position is not finite — cannot place an entity there.",
            tx,
        )
        .await;
        return;
    }

    if let Err(e) = tx
        .send(CellToBaseMsg::GmSpawnNpc {
            entity_id: caller_id,
            template_id,
            space_id,
            world_name,
            position,
            heading,
        })
        .await
    {
        tracing::warn!(
            caller_id,
            template_id,
            error = %e,
            "spawn: GmSpawnNpc send to base failed — spawn dropped"
        );
        send_gm_feedback(
            caller_id,
            &format!("spawn: request failed for template {template_id} (cell→base channel error)."),
            tx,
        )
        .await;
    }
    // Success path is intentionally silent here — see the doc comment. The
    // round-trip owns the truthful outcome line.
}

/// `.despawn` — destroy the selected NPC/spawnable and tell its observers.
///
/// Legacy `Resource.despawnEntity` is `Atrea.destroyCellEntity(target.entityId)`
/// with a `'Despawning entity <%s>'` line printed first. Runtime-only: the
/// entity's `resources.spawnlist` row (if it has one) is left completely
/// untouched — deleting that is `.delspawn`.
///
/// The player refusal is enforced twice on purpose: the registry spec is
/// `Target::Mob`, and [`SpaceManager::despawn_npc`] independently refuses a
/// player. Legacy's own registration was `SGWSpawnableEntity`, which
/// `SGWPlayer` derives from — so the legacy command could destroy a logged-in
/// player's cell entity. That is a legacy bug, corrected per D02.
async fn despawn_entity(
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match space_mgr.despawn_npc(target, tx).await {
        DespawnOutcome::Despawned { witnesses_notified } => {
            send_gm_feedback(
                caller_id,
                &format!(
                    "despawn [{target}]: removed from the space; \
                     {witnesses_notified} observer(s) notified. \
                     Its spawnlist row (if any) is untouched — use .delspawn to remove that."
                ),
                tx,
            )
            .await;
        }
        DespawnOutcome::NotFound => {
            send_gm_feedback(
                caller_id,
                &format!("despawn [{target}]: entity is no longer in a space."),
                tx,
            )
            .await;
        }
        DespawnOutcome::RefusedPlayer => {
            send_gm_feedback(
                caller_id,
                &format!(
                    "despawn [{target}]: refused — that is a player. \
                     .despawn only removes NPCs / spawnable entities."
                ),
                tx,
            )
            .await;
        }
    }
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

    let Some((space_id, world_name, origin, heading)) =
        caller_placement("spawnrandom", caller_id, tx, space_mgr).await
    else {
        return;
    };

    let mut delivered = 0i32;
    for i in 0..count {
        // Deterministic scatter: place copies evenly on a ring within the
        // x/z range (no RNG dependency, reproducible for authoring).
        let theta = (i as f32) * std::f32::consts::TAU / (count as f32).max(1.0);
        let pos = [
            origin[0] + theta.cos() * x_range,
            origin[1],
            origin[2] + theta.sin() * z_range,
        ];
        match tx
            .send(CellToBaseMsg::GmSpawnNpc {
                entity_id: caller_id,
                template_id,
                space_id,
                world_name: world_name.clone(),
                position: pos,
                // Scattered copies share the caller's facing, matching legacy
                // `spawnRandomEntity`'s `space.createEntity(template, pos,
                // player.rotation)`.
                heading,
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
