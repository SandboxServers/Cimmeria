//! `spawn_entity` / `despawn_entity` DB-row → `Action` conversion.
//!
//! Split out of [`super::action`] rather than added to it: that file was
//! already at the 500-line soft cap from `CLAUDE.md`, and the entity
//! lifecycle verbs are a natural seam — they are the only two action types
//! that create or remove a world entity, and the only ones whose params
//! carry a full spawn descriptor rather than one or two scalars.
//!
//! [`convert_spawn_action`] is consulted by [`super::action::convert_action`]
//! before it gives up with `None`, so an unknown `action_type` still warns
//! exactly once at the caller.

use tracing::warn;

use crate::actions::Action;

use super::DbActionRow;

/// Convert a `spawn_entity` / `despawn_entity` row. Returns `None` for any
/// other `action_type` so the caller can fall through to its own
/// "unknown action" handling.
pub(super) fn convert_spawn_action(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    match row.action_type.as_str() {
        "spawn_entity" => {
            // `target_id` = entity_templates.template_id, `target_key` = the
            // spawn tag. Both are mandatory: a template-less spawn has
            // nothing to instantiate, and an untagged one is unreachable
            // forever after (no `entity_dead_tag`, no `interact_tag`, no
            // `despawn_entity`, and the executor's same-tag idempotence
            // guard has nothing to key on).
            let template_id = row.target_id?;
            let tag = row.target_key.as_deref()?.to_string();
            if tag.is_empty() {
                warn!(
                    chain_id = row.chain_id,
                    template_id, "spawn_entity: empty target_key (tag); dropping action"
                );
                return None;
            }

            // x/y/z are required and must be finite — same reasoning as
            // `cross_world_teleport`: silently spawning at the world origin
            // puts the NPC inside the floor or outside the map, which reads
            // as "the mission is broken" rather than "the seed row is
            // broken". Drop and log instead.
            let parse_coord = |key: &str| -> Option<f32> {
                let f = params.get(key)?.as_f64()? as f32;
                f.is_finite().then_some(f)
            };
            let (x, y, z) = match (parse_coord("x"), parse_coord("y"), parse_coord("z")) {
                (Some(x), Some(y), Some(z)) => (x, y, z),
                _ => {
                    warn!(
                        chain_id = row.chain_id,
                        template_id,
                        %tag,
                        ?params,
                        "spawn_entity: missing or non-finite x/y/z; \
                         dropping action to avoid spawning at (0,0,0)"
                    );
                    return None;
                }
            };

            // Heading is optional — an NPC facing the wrong way is a
            // cosmetic defect, not a broken mission, so it defaults rather
            // than dropping the row. A non-finite value still defaults
            // (NaN yaw would propagate into the direction vector).
            let heading = params
                .get("heading")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .filter(|f| f.is_finite())
                .unwrap_or(0.0);

            // `respawn_secs` is accepted on the row and deliberately
            // dropped here. A content-scoped spawn is always one-shot (the
            // respawn tick has no instance-lifetime awareness, and a
            // revived mission NPC would re-fire its `entity_dead_tag`
            // chain), so the value never reached the entity: it was parsed,
            // carried on the action, threaded through the executor and then
            // unconditionally overwritten with `None`. The PR #662 review
            // cut that dead thread. The row still loads — a mission NPC
            // that appears without respawn beats one that never appears —
            // but the author hears about it once, at load, rather than on
            // every fire.
            if params
                .get("respawn_secs")
                .and_then(|v| v.as_i64())
                .is_some()
            {
                warn!(
                    chain_id = row.chain_id,
                    template_id,
                    %tag,
                    reason = "respawn_secs_not_honoured",
                    "spawn_entity: respawn_secs is not supported for content \
                     spawns and is ignored; the NPC will spawn one-shot"
                );
            }

            // Per-spawn descriptor fields. Parsed as `Option` so the
            // executor can tell "the author said no" from "the author said
            // nothing". Both are `spawnlist` columns, not template ones.
            // `aggression: None` means "derive from faction" (NA13).
            let is_stationary = params.get("is_stationary").and_then(|v| v.as_bool());
            let aggression = params
                .get("aggression")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            let allow_shared = params.get("allow_shared").and_then(|v| v.as_bool());

            Some(Action::SpawnEntity {
                template_id,
                position: [x, y, z],
                heading,
                tag,
                is_stationary,
                aggression,
                allow_shared,
            })
        }
        "despawn_entity" => {
            // Same empty-tag gate as `spawn_entity`, for the same reason:
            // `?` catches `None` but not `Some("")`, and an empty tag can
            // never resolve an entity, so the row would be a silent no-op
            // at runtime instead of a loud drop at load.
            let entity_tag = row.target_key.as_deref()?.to_string();
            if entity_tag.is_empty() {
                warn!(
                    chain_id = row.chain_id,
                    "despawn_entity: empty target_key (tag); dropping action"
                );
                return None;
            }
            Some(Action::DespawnEntity { entity_tag })
        }
        _ => None,
    }
}
