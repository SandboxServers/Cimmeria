//! Combat-related player method dispatch — `useAbility`,
//! `useAbilityOnGroundTarget`, `callForAid` (Defeat Window respawn),
//! the auto-`respawn` path, the `unstuck` stub, and the forward of
//! `resetMyAbilities` to the trainer respec in `vendor`.
//!
//! The respawn fork (same-world in-place reanchor vs. cross-world
//! gate-travel) lives in `cell::respawn` so this match stays a thin
//! "which entry point" dispatch. The native GM `gmRespawn` drives the same
//! fork, and the GM console sits beside these handlers rather than above them
//! in the services crate split, so the fork is one layer below both
//! (`docs/architecture/services-crate-split.md` §2H).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

use crate::cell::respawn;

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
                // Gate before the respawn is recorded anywhere, so a refused
                // call never shows up as a respawn in the friction or journal
                // rows.
                if respawn_refusal(entity_id, "callForAid", Some(respawner_id), space_mgr) {
                    return true;
                }
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
                // The receipt row (AB-T1, stage `recv`). The inbound Mercury
                // packet seq that carried the call is not here: the base
                // decodes it (`connect_loop::encrypted`) and forwards only
                // `BaseToCellMsg::CellMethodCall { entity_id, method_index,
                // args }`. Carrying it is AB-T2's plumbing.
                let who = space_mgr.player_identity(entity_id);
                tracing::debug!(
                    target: "abilities",
                    event = "use_ability_recv",
                    stage = "recv",
                    account_id = who.account_id,
                    player_id = who.player_id,
                    entity_id,
                    ability_id,
                    wire_target_id = target_id,
                    "useAbility"
                );

                // Single canonical kill-credit path — see
                // `handle_use_ability_with_kill_credit` for the
                // alive→dead detection + `fire_entity_death` wrap that
                // previously lived inline here.
                crate::cell::abilities::handle_use_ability_with_kill_credit(
                    entity_id,
                    ability_id,
                    target_id,
                    &crate::cell::content::EngineEvents(engine),
                    tx,
                    space_mgr,
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
                let who = space_mgr.player_identity(entity_id);
                tracing::debug!(
                    target: "abilities",
                    event = "use_ability_on_ground_recv",
                    stage = "recv",
                    account_id = who.account_id,
                    player_id = who.player_id,
                    entity_id,
                    ability_id,
                    x,
                    y,
                    z,
                    "useAbilityOnGroundTarget"
                );

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
                    entity_id,
                    deaths,
                    &crate::cell::content::EngineEvents(engine),
                    tx,
                    space_mgr,
                )
                .await;
            }
            true
        }

        RESPAWN => {
            if respawn_refusal(entity_id, "respawn", None, space_mgr) {
                return true;
            }
            tracing::debug!(entity_id, "respawn (auto)");
            respawn::handle_respawn(entity_id, -1, tx, space_mgr).await;
            true
        }

        UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
            true
        }

        // The trainer respec (AT-08). Its index sits in this range, but the
        // handler lives with the rest of the trainer flow in `vendor`.
        RESET_MY_ABILITIES => {
            super::vendor::handle_reset_my_abilities(entity_id, tx, space_mgr).await;
            true
        }

        _ => false,
    }
}

/// The server-side gates on the two player respawn entry points,
/// `callForAid` (67) and `respawn` (70). Returns `true` when the call is
/// refused; the caller then returns without touching the entity.
///
/// `handle_respawn` heals to full, clears every state flag and cooldown,
/// drops threat and moves the player, and on a foreign-world respawner it
/// destroys the entity and sends `GateTravel`. So the arm checks, from
/// server state only:
///
/// 1. The caller is dead (`BSF_DEAD`, set by the death path before the
///    Defeat Window opens and cleared by the respawn). HP is not the
///    authority: a corpse can hold positive HP. An unmodified client sends
///    either method only from the Defeat Window, so a living caller is a
///    forged packet, or the benign race where Release and the timer expiry
///    both fire and the first one already revived the player.
///    When `unstuck` is implemented, this widens to "dead OR a
///    server-recorded pending unstuck aid-wait", never to a wire flag.
/// 2. For `callForAid`, a positive `respawner_id` is one the Defeat Window
///    offered, per `spawner::offered_in_world` (the same predicate
///    `send_begin_aid_wait` builds the list from). Ids `<= 0` are the
///    server's own world-default fallback, and 0 is the synthetic
///    "Respawn Point" entry, so they are always accepted.
///
/// A refusal sends the client nothing: a living caller has no UI to
/// update, and a dead caller keeps the Defeat Window open to pick again.
/// It logs at DEBUG, because the client controls the input and could flood
/// a WARN index (negative-logging convention, client-input refusals).
///
/// A missing entity is not refused here: `handle_respawn`'s own not-found
/// warn stays the single seam for it.
fn respawn_refusal(
    entity_id: u32,
    method: &'static str,
    respawner_id: Option<i32>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return false;
    };
    let id = e.identity();
    if !crate::cell::combat::is_dead_state(e.state_field) {
        tracing::debug!(
            target: "player.respawn",
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            respawner_id = respawner_id.unwrap_or(-1),
            method,
            state_field = e.state_field,
            reason = "respawn_not_dead",
            "respawn request from a living player -- refused (no heal, no move); \
             only a dead player's Defeat Window may call for aid or respawn"
        );
        return true;
    }
    let Some(respawner_id) = respawner_id.filter(|&r| r > 0) else {
        return false;
    };
    let world = space_mgr.get_entity_world_name(entity_id);
    let offered = world.as_deref().is_some_and(|w| {
        crate::cell::spawner::offered_in_world(&space_mgr.respawners, w)
            .any(|r| r.respawner_id == respawner_id)
    });
    if !offered {
        tracing::debug!(
            target: "player.respawn",
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            respawner_id,
            method,
            world = ?world,
            reason = "respawner_not_offered",
            "callForAid for a respawner the Defeat Window did not offer -- refused; \
             the player stays dead with the Defeat Window open"
        );
        return true;
    }
    false
}
