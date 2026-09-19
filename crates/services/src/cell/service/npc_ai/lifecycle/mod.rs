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
    record_decision_outcome("submit_init");
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
        return;
    }

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
    npc.nav_path.clear();
    npc.velocity = [0.0; 3];
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

    crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(
        npc_id,
        combat_exits = combat_exit_count,
        auto_cycle_exits = auto_cycle_exit_count,
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
