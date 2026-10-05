//! Where a respawn lands: the Defeat Window's respawner, the nearest one in
//! the world, the world's start-profile point, or in place.

use cimmeria_cell_catalog::cell::respawner_fallback::nearest_valid_respawner_def;
use cimmeria_entity::movement_validation::SpaceBounds;
use cimmeria_resources::base::start_profiles::{self, StartProfiles};

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::RespawnerDef;

/// A respawner row sitting exactly at the world origin is an unauthored
/// placeholder, not a respawn point.
///
/// `load_respawners` copies `resources.respawners` in with no validation,
/// and the recovered data ships rows whose name survived but whose
/// coordinates did not (all four World 8 / Castle rows were `(0,0,0)`
/// until CA00; the two World 23 rows still are — see the KNOWN GAP
/// comment in `db/resources/Worlds/Seed/respawners.sql`). Treating those
/// as real is worse than having no respawner at all: because the row
/// *exists*, every fallback below it becomes unreachable and the player
/// is teleported to the world origin on death — usually out of bounds,
/// under the map, or in an inescapable void, with `unstuck` still
/// unimplemented. Skipping them instead degrades to the in-place /
/// Castle-default fallbacks, which are survivable.
///
/// Exact equality is the right test: (0,0,0) is a sentinel written by the
/// authoring gap, never a coordinate anyone would author deliberately, and
/// a tolerance band would start rejecting legitimately near-origin points
/// in worlds whose geometry straddles it. `-0.0 == 0.0` in IEEE-754, so
/// negative zeros are caught too.
fn is_unauthored(r: &RespawnerDef) -> bool {
    r.pos == [0.0, 0.0, 0.0]
}

/// The `reason` on the nearest-respawner log: why no named row was used.
fn fallback_reason(respawner_id: i32) -> &'static str {
    match respawner_id {
        0 => "respawner_id_zero",
        id if id < 0 => "respawner_id_unset",
        _ => "respawner_unusable",
    }
}

/// Resolve `(world, position)` for the respawn target.
///
/// Priority:
///   1. Explicit `respawner_id` from the Defeat Window (must be > 0).
///   2. The respawner registered for the player's current world that is
///      nearest the death position. Taken for id 0 (the Defeat Window's
///      synthetic "Respawn Point" entry), the auto-respawn `-1`, and an id
///      that matches no usable row. Decision (@Cadacious, 2026-09-28): it
///      used to be the *first* row for the world, which in Castle_CellBlock
///      was the Stasis Chamber start room, so a death in the Mess Hall sent
///      the player back through Hallway01 and an out-of-order guard kill
///      soft-locked the chain.
///   3. The start profile's point when the world is a start world
///      (`Castle_CellBlock`, `SGC_W1`, `Dakara_E1`; Class Start v6 CS-02).
///   4. In-place at the player's current position for any other world
///      (avoids silently teleporting players cross-world).
///   5. An entity in no world goes to its own start profile's home; with
///      no profile loaded, `None` (logged at ERROR) and no respawn.
///
/// Respawners that fail [`is_unauthored`] are invisible to steps 1 and 2 —
/// an all-zero row is treated as absent at every priority, so the search
/// continues past it instead of returning the world origin.
///
/// Operational note: in-place respawn outside Castle can produce death
/// loops if the player died standing in damaging geometry (lava tile, AoE
/// pool) and no respawner is configured for that world — they'll respawn
/// at full health, take the geometry damage tick, and die again. The
/// clean fix is content-side: every world should ship at least one
/// respawner. The fallback warn log is the operator signal.
pub(super) fn resolve_respawn_target(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> Option<(String, [f32; 3])> {
    resolve_respawn_target_in(
        start_profiles::installed().as_deref(),
        respawner_id,
        entity_id,
        space_mgr,
    )
}

/// [`resolve_respawn_target`] against an explicit start-profile set
/// (`None`: none loaded).
pub(super) fn resolve_respawn_target_in(
    profiles: Option<&StartProfiles>,
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> Option<(String, [f32; 3])> {
    if respawner_id > 0 {
        match space_mgr
            .respawners
            .iter()
            .find(|r| r.respawner_id == respawner_id)
        {
            Some(r) if is_unauthored(r) => {
                // Negative-log seam, `reason` pinned by
                // `origin_respawner_warn_fires_once_on_the_explicit_id_path`.
                // `world` (not `world_name`) matches the field this
                // function's other logs already use, so one ops query
                // catches every respawn-resolution event; see the
                // worknote for the convention-vs-practice note.
                tracing::warn!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    respawner_id,
                    respawner_name = %r.name,
                    world = %r.world_name,
                    reason = "respawner_at_origin",
                    "Respawn: requested respawner is at the world origin \
                     (unauthored coordinates) — ignoring it and falling back, \
                     so the player lands at a fallback point rather than at \
                     (0,0,0); fix the row in \
                     db/resources/Worlds/Seed/respawners.sql"
                );
            }
            Some(r) => return Some((r.world_name.clone(), r.pos)),
            None => tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                respawner_id,
                respawner_name = cimmeria_names::book().respawner(respawner_id),
                reason = "respawner_not_found",
                "Respawner not found, falling back to the nearest respawner"
            ),
        }
    }

    let world_name = space_mgr.get_entity_world_name(entity_id);
    if let Some(ref wn) = world_name {
        // The player has not moved since dying (a corpse accepts no
        // movement), so its current position is the death position.
        let nearest = space_mgr.get_entity(entity_id).and_then(|e| {
            // No navmesh and the permissive AABB on purpose: a respawner row is
            // a coordinate a human authored for exactly this use, and this path
            // has never second-guessed one. The shared search still applies the
            // all-zero placeholder guard and the finite-coordinate test.
            nearest_valid_respawner_def(
                &space_mgr.respawners,
                wn,
                e.position,
                None,
                &SpaceBounds::FALLBACK,
            )
            .map(|r| {
                let p = e.position;
                let (dx, dy, dz) = (r.pos[0] - p.x, r.pos[1] - p.y, r.pos[2] - p.z);
                (r, (dx * dx + dy * dy + dz * dz).sqrt())
            })
        });
        if let Some((r, distance_m)) = nearest {
            let id = space_mgr.player_identity(entity_id);
            // Not a refusal: the Defeat Window's synthetic entry and the
            // auto-respawn timer name no respawner, so choosing one is the
            // normal path. INFO so SigNoz shows which row a death resolved to
            // (2026-09-28 playtest: id 0 sent the player to the start room,
            // far from the fight, and the run back broke the Cellblock chain).
            tracing::info!(
                target: "player.respawn",
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                account_id = id.account_id,
                account_name = id.account_name,
                player_id = id.player_id,
                player_name = id.player_name,
                requested_respawner_id = respawner_id,
                requested_respawner_name = cimmeria_names::book().respawner(respawner_id),
                respawner_id = r.respawner_id,
                respawner_name = %r.name,
                world = %wn,
                distance_m,
                reason = fallback_reason(respawner_id),
                "Respawn: no usable respawner was named -- using the one nearest \
                 the death position"
            );
            return Some((r.world_name.clone(), r.pos));
        }
        let skipped = space_mgr
            .respawners
            .iter()
            .filter(|r| r.world_name == *wn && is_unauthored(r))
            .count();
        if skipped > 0 {
            // Negative-log seam, `reason` pinned by
            // `origin_respawner_warn_fires_once_on_the_world_scan_path`.
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                world = %wn,
                skipped,
                reason = "world_respawners_all_at_origin",
                "Respawn: every respawner registered for this world is at the \
                 origin (unauthored coordinates) — falling back, so the player \
                 respawns in place rather than at (0,0,0); fix the rows in \
                 db/resources/Worlds/Seed/respawners.sql"
            );
        }
    }

    // No usable respawner. A world some start profile begins in falls back
    // to that profile's point (the Cellblock stasis room, SGC_W1, the
    // Dakara_E1 plaza); any other world respawns in place.
    match world_name.as_deref() {
        Some(world) => {
            if let Some(pos) = profiles.and_then(|p| p.start_position(world)) {
                tracing::debug!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    world,
                    "No respawner; using the start profile's point"
                );
                return Some((world.to_string(), pos));
            }
            let in_place = space_mgr
                .get_entity(entity_id)
                .map(|e| [e.position.x, e.position.y, e.position.z])?;
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                world = world,
                "No respawner configured for this world — respawning in place at current position"
            );
            Some((world.to_string(), in_place))
        }
        None => {
            // The entity is in no world: send it to its own start profile's
            // home (lock L3: never a guessed Castle_CellBlock).
            let home = space_mgr.get_entity(entity_id).and_then(|e| {
                profiles?.home_for(i32::from(e.alignment), e.archetype_id.unwrap_or(0))
            });
            match home {
                Some(p) => {
                    tracing::warn!(
                        entity_id,
                        entity_name = space_mgr.entity_names(entity_id).entity_name,
                        world = %p.world,
                        profile_id = %p.profile_id,
                        reason = "no_world",
                        "Respawn: the entity is in no world; sending it to its start profile's home"
                    );
                    Some((p.world.clone(), p.position))
                }
                None => {
                    tracing::error!(
                        entity_id,
                        entity_name = space_mgr.entity_names(entity_id).entity_name,
                        reason = "no_world_no_start_profile",
                        "Respawn refused: the entity is in no world and no start profile \
                         names a home"
                    );
                    None
                }
            }
        }
    }
}
