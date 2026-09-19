//! Terminal / quiescent states: despawn (remove from space), submit
//! (surrender + hold), and error (diagnostic hold).

use tokio::sync::mpsc;

use crate::cell::abilities::send_entity_method;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

use super::record_decision_outcome;

/// NPC despawn behavior: remove the entity from the space. Used by
/// scripted cleanup (e.g., "the boss died, his bodyguards retreat
/// off-screen"). Fans `LeftAoI` to every witness immediately.
///
/// C08b (2026-09-18): switched from the bare `SpaceManager::destroy_entity`
/// to `despawn_npc` — the bare call left the entity in every observer's
/// `witnesses` set until the next AoI tick happened to visit them (this
/// function's own doc comment used to claim immediate fanout, which was
/// false; see `content::executor::world::destroy_tagged_entity`'s doc
/// comment for the full failure-shape writeup, issue #582).
///
/// One-shot: the entity is gone by the time this returns, so any
/// subsequent tick filters skip it naturally.
pub(super) async fn npc_ai_despawn(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    record_decision_outcome("despawn");
    // Clear the movement-type cache first so the wire state is clean
    // before the destroy. The broadcast itself is dedup'd on None and
    // emits nothing — this is purely a state-clean step.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    match space_mgr.despawn_npc(npc_id, tx).await {
        DespawnOutcome::Despawned { witnesses_notified } => {
            tracing::info!(
                npc_id,
                witnesses_notified,
                "NPC AI: despawn → removed entity from space"
            );
        }
        DespawnOutcome::RefusedPlayer => {
            // Scripted AI cleanup should never target a player entity;
            // WARN loudly rather than silently no-opping.
            tracing::warn!(
                npc_id,
                "NPC AI: despawn target resolved to a player entity -- refused"
            );
        }
        DespawnOutcome::NotFound => {
            tracing::debug!(npc_id, "NPC AI: despawn target already gone");
        }
    }
}

/// NPC submit behavior: the NPC surrenders. Disengages *both* sides of
/// the fight and holds position. The AI tick keeps admitting Submit on
/// every pass (the snapshot filter permits it), so the handler early-outs
/// once there is nothing left to clean up. Content authors destroy or
/// transition the NPC when they're done with it.
///
/// # Both sides, not just the NPC
///
/// Clearing the NPC's own `threat_list` in place is not enough, and was
/// the original shape of this handler. The attacker's combat state lives
/// on the *attacker*: every player who aggroed this NPC holds it in
/// `threatened_mobs`, and that set — not the `BSF_InCombat` bit — is what
/// gates the weapon-drawn posture and `regen_tick`. Wiping `threat_list`
/// without the player-side scrub strands every attacker in combat
/// permanently: bit stuck set, weapon stays drawn, health and focus never
/// regenerate again for the rest of the session.
///
/// So this routes through the same [`combat::clear_dead_npc_from_all_player_threat`]
/// the death path uses, and through [`combat::clear_auto_cycle_for_target`]
/// for the auto-fire loop. What it deliberately does *not* borrow from the
/// death path: no `onTargetUpdate(0)` reticle drop, no loot roll, no
/// `InteractionType` push and no dead-state flip. The NPC is alive and
/// still selectable — only the automatic aggression on both sides stops.
pub(super) async fn npc_ai_submit(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use crate::cell::combat;
    // Cheap probe: skip the whole pass once there is nothing to clean.
    //
    // `threat_list` doubles as the *re-engage* probe. A player cannot arm
    // an auto-cycle loop at this NPC without first landing a hit, and the
    // damage path repopulates `threat_list` on every hit — so a non-empty
    // list is exactly "somebody engaged since the last cleanup", and the
    // sweeps below run again for them.
    let needs_cleanup = space_mgr.get_entity(npc_id).is_some_and(|e| {
        e.last_movement_type.is_some() || !e.threat_list.is_empty() || e.aggression > 0
    });
    if !needs_cleanup {
        // Not silent, and not `submit_init` either: a parked NPC would
        // otherwise post a "first-entry combat-clear" outcome on every
        // 2 s tick for the rest of the space's life, which makes the
        // `npc_ai_decisions_total{submit_init}` counter unreadable and
        // the `npc_ai.tick` row lie about what the handler did.
        record_decision_outcome("submit_hold");
        return;
    }
    record_decision_outcome("submit_init");

    // Who the NPC surrenders *to*, captured before the scrub drains the
    // list. Highest threat, matching how `npc_ai_fight` picks its target
    // — for the 1v1 duel this is always the duelist. `f32` threat values
    // are finite (`generate_threat` adds damage totals), so
    // `partial_cmp` cannot see a NaN; `unwrap_or(Equal)` keeps the
    // comparator total anyway rather than panicking if that ever changes.
    let surrender_to = space_mgr.get_entity(npc_id).and_then(|e| {
        e.threat_list
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&player_eid, _)| player_eid)
    });

    // Player-side scrub FIRST — it reads this NPC's `threat_list` to find
    // who to drop it from, so clearing the list before the call would
    // silently scrub nobody. Same deferred-drain ordering contract the
    // death path has: the transition consumes the list, the caller drains
    // it afterwards.
    let combat_exits = combat::clear_dead_npc_from_all_player_threat(space_mgr, npc_id);

    // Entity vanished between the probe and here (despawn racing the AI
    // tick). The scrub above already ran, which is the half that matters
    // for the players.
    let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
        return;
    };
    npc.threat_list.clear();
    // `nav_path` + `velocity` together are what stop the client
    // extrapolating the NPC along its stale chase vector — the
    // "walks off facing the wrong way" shape from the 2026-09-18 Castle
    // playtest (findings H4b / H6). Zeroed velocity reaches every
    // witness on the next 100 ms AoI tick: `compute_aoi_changes` pushes
    // an `EntityMoved` carrying position, direction and velocity for
    // every entity still in view, whether or not it moved. No extra
    // fan-out is needed here, and none of it is conditional on the
    // NPC having a path.
    npc.nav_path.clear();
    npc.velocity = [0.0; 3];
    // Raw clear, not `unset_state_flag`. `BSF_IN_COMBAT` has no
    // ref-counted enter path on the NPC side — nothing ever calls
    // `set_state_flag(BSF_IN_COMBAT)` — so the counter entry does not
    // exist and `unset_state_flag` would `return false` without
    // touching `state_field`. Matches the death paths.
    npc.state_field &= !combat::BSF_IN_COMBAT;
    // Cosmetic: the fast-retry sweep already drops this NPC from
    // `pending_ai_retries` the moment it sees a non-Fighting state, so a
    // stale deadline is unreachable — but leaving it set makes the
    // entity lie about itself. The respawn tick clears it for the same
    // reason.
    npc.ai_retry_at = None;

    // Disarm, not a marker. The durable "this NPC surrendered" fact is
    // `ai_state == Submit`; `aggression` is the separate switch that
    // makes an *idle* NPC seed threat on a passing player unprompted,
    // and while the NPC sits in Submit nothing reads it at all. It is
    // zeroed here for the paths that can push a submitted NPC back to
    // Idle behind our back — a content `set_npc_ai_state idle`, a
    // `set_follow_target` that resolves to nothing, the GM console, and
    // the respawn tick after somebody kills the NPC anyway. Without
    // this, any of them hands back a hostile-on-sight mob.
    //
    // Pure runtime state: no template column, no `spawnlist` column, no
    // persistence, and the respawn tick never re-seeds it — so the clear
    // cannot leak to the database, and equally cannot be silently
    // undone. Note the name collides with python's `EMobAggressionLevel`
    // (`SGWMob.def` `Aggression`, INT8), which runs the *opposite* way
    // (low = hostile, 3 = neutral) and is a CELL_PUBLIC wire property.
    // These are unrelated fields.
    //
    // Deliberately NOT a faction flip. `faction` is what gates whether a
    // player may target the NPC with an offensive ability at all, so
    // flipping it off the hostile sentinel would make the surrendered
    // NPC unattackable — and it doubles as the NPC-versus-NPC aggro
    // filter, so the flip would also read this NPC as an ally to every
    // other mob's idle scan. Explicit attacks stay legal; only the
    // automatic paths shut off.
    npc.aggression = 0;

    // Any channel the NPC was running dies with its willingness to
    // fight — otherwise a surrendered NPC keeps pulsing its debuff onto
    // the player who just accepted the surrender. `None` cancels all of
    // them; the same call guards the death path.
    let _ = crate::cell::effects::cancel_channels_from_attacker(npc_id, None, tx, space_mgr).await;

    // A submitted NPC never leaves this state on its own, so a cover slot
    // it was holding when it surrendered would be reserved forever — the
    // same leak `apply_death_transition` releases for a corpse.
    // Idempotent: a no-op for an NPC that was never in cover.
    space_mgr
        .cover
        .release_for_entity(cimmeria_common::EntityId(npc_id as i32));

    // Turn to face whoever the NPC surrendered to. Without this the yaw
    // freezes wherever the last translation left it — an NPC that broke
    // off a chase ends up kneeling at right angles to the player it just
    // gave up to (2026-09-18 Castle playtest, findings H4b and H6: the
    // AI writes `direction` only as a side effect of movement, and
    // attack-in-place clears `nav_path`, so nothing re-faces a
    // stationary NPC). Reuses `fight::face_target` so surrender and the
    // combat re-face share one atan2(dx, dz) convention. Costs no wire
    // traffic of its own — the AoI tick's `EntityMoved` carries
    // direction every pass.
    if let Some(player_eid) = surrender_to {
        if let (Some(npc_pos), Some(player_pos)) = (
            space_mgr.get_entity(npc_id).map(|e| e.position),
            space_mgr.get_entity(player_eid).map(|e| e.position),
        ) {
            super::fight::face_target(space_mgr, npc_id, npc_pos, player_pos);
        }
    }

    // Push the `BSF_InCombat` clear to each attacker whose last threat
    // source this NPC was. No appearance refresh here on purpose:
    // `exit_player_combat` stamped the OOC holster timer instead of
    // flipping the posture, and `holster_timer_tick` re-broadcasts
    // `BeingAppearance` after the grace window.
    let combat_exit_count = combat_exits.len();
    for (player_entity_id, new_state) in combat_exits {
        tracing::debug!(
            player_entity_id,
            npc_id,
            new_state,
            "NPC AI: submit — clearing attacker BSF_InCombat (surrendered NPC was their last threat)"
        );
        send_entity_method(
            player_entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            new_state.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // Stop every auto-fire loop currently pointed at this NPC. Without
    // it the loop keeps re-firing on its own cadence and kills the NPC
    // that just surrendered. The sweep matches the player's LIVE
    // `current_target_id`, so a player who has already switched cursor
    // to something else keeps their loop.
    //
    // This is the immediate stop; the durable one is the target-validity
    // gate in `auto_cycle_tick`, which refuses a submitted target on
    // every 100 ms pass. Both exist because this handler only runs on
    // the ~2 s AI cadence — long enough for several re-fires.
    let auto_cycle_exits = combat::clear_auto_cycle_for_target(space_mgr, npc_id);
    let auto_cycle_exit_count = auto_cycle_exits.len();
    for (player_entity_id, new_state) in auto_cycle_exits {
        send_entity_method(
            player_entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            new_state.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // `None` is the codebase-wide "stopped" convention — every other AI
    // stop path (fight→idle, leash arrival, patrol/wander/investigate
    // dwell) uses it. Be clear about what it does: it clears the
    // server-side cache and sends NOTHING on the wire, so the client
    // keeps playing whatever animation the last `setMovementType`
    // selected. The enum has no "stopped" discriminant to send instead
    // (`Cover`/`CombatAdvance`/`Patrol`/`Follow`/`Wander`/`Leash`/`Avoid`
    // are all *how am I moving* kinds), so inventing one here would be a
    // wire change on a guess. What actually stops the NPC visibly moving
    // is the zeroed velocity above, which the AoI tick transmits.
    // Coupling `setMovementType` to path start/stop is the repo-wide fix
    // (2026-09-18 playtest, recommended change 9) and is not H08's.
    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(
        npc_id,
        combat_exits = combat_exit_count,
        auto_cycle_exits = auto_cycle_exit_count,
        surrender_to = ?surrender_to,
        "NPC AI: submit → both sides disengaged, holding"
    );
}

/// NPC error behavior: diagnostic fallback. Halts AI work (no
/// pathfind, no broadcast cadence). Logged once per entry so a stuck
/// NPC doesn't fill the log stream. Used by the `enterErrorAIState`
/// slash command and by the AI tick when it catches an unrecoverable
/// inconsistency (future).
pub(super) async fn npc_ai_error(
    npc_id: u32,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    record_decision_outcome("error_hold");
    // No-op per tick — Error is a quiescent diagnostic state. The
    // entry log is emitted by whatever transitioned the NPC into
    // Error (typically the content action or the slash command).
    tracing::debug!(npc_id, "NPC AI: error state — holding");
}

// Must stay the LAST item in the file — clippy denies
// `items_after_test_module` workspace-wide.
#[cfg(test)]
mod tests;
