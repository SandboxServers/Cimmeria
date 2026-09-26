//! Combat-related player method dispatch — `useAbility`,
//! `useAbilityOnGroundTarget`, `callForAid` (Defeat Window respawn),
//! the auto-`respawn` path, and a couple of unimplemented stubs.
//!
//! The respawn fork (same-world in-place reanchor vs. cross-world
//! gate-travel) lives in [`respawn`] so this match stays a thin
//! "which entry point" dispatch.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

mod respawn;

// Re-exported for the native GM `gmRespawn` handler
// (`cell_methods::gm::world`), which reuses the same respawn sequence as the
// combat Defeat-Window path rather than duplicating it.
pub(crate) use respawn::handle_respawn;

#[cfg(test)]
mod tests;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        CALL_FOR_AID => {
            if args.len() >= 4 {
                let respawner_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, respawner_id, "callForAid");
                crate::cell::playtest_friction::respawned(entity_id);
                crate::cell::player_journal::note(
                    entity_id,
                    crate::cell::player_journal::kinds::RESPAWN,
                    format!("respawner={respawner_id}"),
                );
                let snap = |m: &SpaceManager| {
                    m.get_entity(entity_id).map(|e| {
                        let h = e.stats.get(cimmeria_entity::stats::HEALTH);
                        (
                            e.state_field,
                            h.map_or(0, |s| s.cur),
                            h.map_or(0, |s| s.max),
                            [e.position.x, e.position.y, e.position.z],
                        )
                    })
                };
                let before = snap(space_mgr);
                respawn::handle_respawn(entity_id, respawner_id, tx, space_mgr).await;
                if let (Some(b), Some(a)) = (before, snap(space_mgr)) {
                    let dead = crate::cell::combat::state::BSF_DEAD;
                    tracing::info!(
                        target: "player.respawn",
                        entity_id,
                        respawner_id,
                        state_flags_before = b.0,
                        state_flags_after = a.0,
                        was_dead = b.0 & dead != 0,
                        dead_flag_cleared = b.0 & dead != 0 && a.0 & dead == 0,
                        health_before = b.1,
                        health_after = a.1,
                        health_max = a.2,
                        from = ?b.3,
                        to = ?a.3,
                        "player revived -- if the client still shows death effects, compare what was sent after this row"
                    );
                }
            }
            true
        }

        USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");

                // Single canonical kill-credit path — see
                // `handle_use_ability_with_kill_credit` for the
                // alive→dead detection + `fire_entity_death` wrap that
                // previously lived inline here.
                crate::cell::abilities::handle_use_ability_with_kill_credit(
                    entity_id, ability_id, target_id, engine, tx, space_mgr,
                )
                .await;
            }
            true
        }

        USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");

                // handle_use_ability_on_ground returns the entity IDs of every
                // NPC that died during this cast (primary + AoE secondaries).
                // We fire the content-engine death event for each, so kill-
                // count missions and other death-triggered chains advance for
                // every AoE kill — not just the primary. Empty Vec means
                // either no targets in radius, primary cast rejected, or
                // nothing died.
                let deaths = crate::cell::abilities::handle_use_ability_on_ground(
                    entity_id,
                    ability_id,
                    [x, y, z],
                    tx,
                    space_mgr,
                )
                .await;

                // Health-below drain + `entity_death` per tagged kill.
                // Shared with the warmup tick, which fires a ground cast
                // whose primary had a warmup (AT-10).
                crate::cell::abilities::credit_ground_deaths(
                    entity_id, deaths, engine, tx, space_mgr,
                )
                .await;
            }
            true
        }

        RESPAWN => {
            tracing::debug!(entity_id, "respawn (auto)");
            respawn::handle_respawn(entity_id, -1, tx, space_mgr).await;
            true
        }

        UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
            true
        }

        RESET_MY_ABILITIES => {
            tracing::info!(entity_id, "UNIMPLEMENTED: resetMyAbilities");
            true
        }

        _ => false,
    }
}
