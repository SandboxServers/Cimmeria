//! Server-driven auto-cycle (auto-fire) loop tick.
//!
//! Every 100 ms AoI tick, scans armed players (`auto_cycle == true` +
//! `auto_cycle_ability_id` set) and re-invokes
//! [`crate::cell::abilities::handle_use_ability`] against the live
//! [`cimmeria_entity::cell_entity::CellEntity::current_target_id`]. The
//! cooldown gate is the rate limiter; the eligibility filter
//! short-circuits to empty when nobody is auto-cycling.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Drive the server-side auto-cycle (auto-fire) loop.
///
/// For every connected player with `auto_cycle == true` and the loop
/// armed (`auto_cycle_ability_id` set):
///
/// - **Target is LIVE** — re-fires use `CellEntity::current_target_id`
///   (written by `setTargetID`). Mirrors python's
///   `self.entity().targetId` live read. Switching targets via the
///   cursor mid-loop redirects the re-fires automatically;
///   deselecting (target = 0) clears the loop.
/// - **No target / invalid target** → clear the loop. Invalid means
///   despawned, dead, or surrendered — see
///   [`crate::cell::combat::is_auto_cycle_target_valid`]. The death
///   sweep and the AI-side submit handler usually get there first; the
///   tick is the safety net for despawn / instance cleanup paths that
///   bypass the death-transition broadcast, and the *primary* stop for
///   surrender, because the AI handler only runs on the ~2 s NPC
///   cadence while this tick runs every 100 ms. Runs BEFORE the
///   cooldown gate — correctness, not rate limiting.
/// - **Out of range** → skip silently WITHOUT clearing. Leaves the
///   loop armed so a player who strafes in and out of range resumes
///   firing automatically the moment they're back in range. Critical:
///   if we let `handle_use_ability` field the range check, it emits
///   `onErrorCode(OutsideWeaponRange)` every tick (~600ms at default
///   cooldown), flooding the wire. The pre-gate here makes the loop
///   silent while out of range, exactly like cooldown.
/// - **No line of sight** (NA31, the world's occluder puts a wall
///   between the eyes) → the first blocked shot goes to
///   `handle_use_ability`, which refuses it with `onErrorCode 39`; after
///   that the loop skips silently, still armed, until the line clears
///   (`AbilityManager::auto_cycle_los_notified`).
/// - **Ability still on cooldown** → skip without clearing. Cooldown
///   clears naturally and the next tick handles re-fire.
/// - **Otherwise** → re-invoke `handle_use_ability` with the
///   loop-armed ability and the live target. Produces the same
///   `onTimerUpdate` + `onSequence` + `onEffectResults` burst as a
///   manual fire — the client cannot distinguish loop-driven from
///   manual shots.
///
/// Cadence: every 100 ms AoI tick.
///
/// `level = "debug"` — fires 10×/sec but the inner `ready` snapshot is
/// often empty (nobody armed). When armed, each fire decision becomes
/// a child event, so SigNoz operators can answer "why didn't auto-fire
/// trigger this tick?" by drilling into the span attributes (cooldown,
/// range, target alive) without grepping log text.
#[tracing::instrument(
    name = "combat.auto_cycle_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn auto_cycle_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Snapshot armed players. Drop the borrow before re-invoking
    // `handle_use_ability` (takes `&mut space_mgr`).
    let mut los_notices: Vec<(u32, bool)> = Vec::new();
    let ready: Vec<(u32, i32, i32, bool)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            if !e.abilities.auto_cycle {
                return None;
            }
            let ability_id = e.abilities.auto_cycle_ability_id?;
            // LIVE target read — mirrors python `self.entity().targetId`
            // in `abilityCooledDown`. None falls through to `target_id = 0`,
            // which the `!target_alive_or_existed` branch below treats
            // as "clear the loop".
            let target_id = e.current_target_id.unwrap_or(0);
            let target = if target_id > 0 {
                space_mgr.get_entity(target_id as u32)
            } else {
                None
            };
            let target_alive_or_existed = target
                .as_ref()
                .is_some_and(|t| crate::cell::combat::is_auto_cycle_target_valid(t));

            // Invalid target → push for clearing regardless of cooldown.
            // The clear path is correctness (BSF must un-light on
            // deselect/death/despawn even mid-cooldown); the cooldown
            // gate below is only the re-fire rate limiter. Letting
            // cooldown gate the clear would leave the loop armed for
            // up to a full cooldown AND let a player re-select a
            // different target before cooldown expires and get an
            // unintended re-fire at the new target.
            if !target_alive_or_existed {
                return Some((eid, ability_id, target_id, false));
            }

            // Line-of-sight gate (NA31). A target behind a wall gets one
            // `onErrorCode 39` (the next shot is let through so
            // `handle_use_ability` refuses it), then the loop waits armed
            // and silent until the line clears, like the range gate below.
            // Evaluated before the cooldown gate so a clear line resets
            // the one-shot notice every tick.
            let los_blocked = matches!(
                crate::cell::abilities::fire_line_of_sight(
                    space_mgr,
                    eid,
                    target_id as u32,
                    space_mgr.ability_defs.get(&ability_id),
                ),
                crate::cell::abilities::FireLos::Refused(_)
            );
            if !los_blocked && e.abilities.auto_cycle_los_notified {
                los_notices.push((eid, false));
            }
            if los_blocked && e.abilities.auto_cycle_los_notified {
                return None;
            }

            // Cooldown gate. Skip without clearing — cooldown clears
            // naturally and the next tick handles re-fire.
            if e.abilities.is_on_cooldown(ability_id) {
                return None;
            }

            // Mid-draw gate. When the player's weapon is mid-draw
            // animation (`pending_attack_at = Some`), `handle_use_ability`
            // rejects with `"weapon attack already queued (mid-draw),
            // ignoring input"`. Without this skip, every 100 ms tick
            // during the ~0.8 s draw window invokes the handler, fails
            // the gate, and produces one DEBUG line — observed as 8
            // rejections in <1 s on lomiada's 2026-06-04 18:16:08 burst.
            // The `pending_attack_tick` will fire the deferred attack
            // once the draw window elapses, then auto-cycle takes over
            // the rest of the loop. Skip-don't-error here is the same
            // pattern the cooldown + range gates use.
            if e.pending_attack_at.is_some() {
                return None;
            }

            // Range pre-gate. Mirrors the rule inside
            // `handle_use_ability` but in skip-don't-error mode.
            // Without this, every out-of-range tick would invoke the
            // handler, fail the range check, and emit
            // `onErrorCode(OutsideWeaponRange)` — at a 0.5s cooldown
            // that's an error packet every ~600 ms while out of
            // range. Loop stays armed so walking back into range
            // resumes firing on the next tick.
            let max_range = space_mgr.ability_defs.get(&ability_id).map_or(30.0, |d| {
                if d.max_range > 0 {
                    d.max_range as f32
                } else {
                    30.0
                }
            });
            if let Some(t) = target {
                if e.position.distance_to(&t.position) > max_range {
                    return None;
                }
            }

            if los_blocked {
                los_notices.push((eid, true));
            }
            Some((eid, ability_id, target_id, true))
        })
        .collect();
    for (eid, notified) in los_notices {
        if let Some(e) = space_mgr.get_entity_mut(eid) {
            e.abilities.auto_cycle_los_notified = notified;
        }
    }

    // Backfill the snapshot size so SigNoz can chart "how many auto-
    // cyclers ran per tick" — the operator-facing answer to "is auto-
    // attack actually firing?" Empty snapshots dominate but cost ~0.
    tracing::Span::current().record("ready_count", ready.len());

    for (entity_id, ability_id, target_id, target_alive) in ready {
        if !target_alive {
            // Target despawned, died without the death sweep catching
            // it, or surrendered. Clear the loop and broadcast so the
            // client un-highlights the button.
            if let Some(new_state) = crate::cell::combat::clear_auto_cycle(space_mgr, entity_id) {
                tracing::info!(
                    entity_id,
                    target_id,
                    "auto_cycle_tick: target gone or disengaged — clearing loop"
                );
                crate::cell::abilities::send_entity_method(
                    entity_id,
                    crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                    new_state.to_le_bytes().to_vec(),
                    tx,
                    space_mgr,
                )
                .await;
            }
            continue;
        }
        tracing::debug!(
            entity_id,
            ability_id,
            target_id,
            "auto_cycle_tick: re-firing"
        );
        // Route loop-driven re-fires through the kill-credit wrapper
        // so killing a quest-tagged NPC via auto-shoot advances the
        // mission's KillCount objective, matching the manual right-
        // click path. The wrapper is a no-op when the target wasn't a
        // live tagged NPC or when no kill happened this tick.
        let _ = crate::cell::abilities::handle_use_ability_with_kill_credit(
            entity_id, ability_id, target_id, engine, tx, space_mgr,
        )
        .await;
        // Commit/reject is the handler's call. A rejected re-fire
        // (e.g. out of ammo) leaves the loop armed — next tick
        // re-evaluates. A committed re-fire restarts the cooldown so
        // this same player won't fire again until that elapses.
    }
}

#[cfg(test)]
#[path = "auto_cycle_tests.rs"]
mod tests;
