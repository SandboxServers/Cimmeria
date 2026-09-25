//! Death: the one resolver every kill path funnels through, and the ordered
//! burst of methods sent to the client when an entity's HEALTH drops to zero.
//!
//! - [`resolve_death`] is the entry point. It owns the kill-site state
//!   mutations, calls [`apply_death_transition`], drains the corpse's threat
//!   list, then runs the [`side_effects`] (death animation, kill XP, player
//!   Defeat Window). It is idempotent on `BSF_DEAD`, which is what makes
//!   "exactly one death per corpse" hold across its three callers:
//!   `damage_apply`'s direct-damage arm, `damage_apply`'s post-effect-script
//!   sweep, and [`kill_npc_out_of_band`] (GM `.kill`, DoT pulse).
//! - [`apply_death_transition`] is the wire burst alone. It does **not**
//!   mutate the dying entity's state (HEALTH=0, BSF_Dead, AI state Dead) —
//!   `resolve_death` has already done that and hands the result in as
//!   `target_state`.
//!
//! The order on the wire is **load-bearing**:
//!
//! 1. (attacker only) `onTargetUpdate(0)` — drop the targeting reticle.
//! 2. (attacker only) `onStateFieldUpdate` with `BSF_InCombat` cleared — stops
//!    the client routing right-click on selected entities to `useAbility`.
//! 3. (NPC target only) `generate_loot_on_death` + `InteractionType` — the
//!    `InteractionType` update MUST land before the dead-state bit, otherwise
//!    the client locks in "shootable" cursor state on dead-state arrival and
//!    ignores the later flag change. Mirrors python `SGWMob.onDead()` which
//!    calls `setInteractionType` before the state field flip propagates.
//! 4. `onStateFieldUpdate` with the corpse's new state (dead bit set) — flips
//!    visuals + cursor.
//!
//! `apply_death_transition` handles outbound messages and the attacker's
//! BSF_InCombat clear only; the state mutations are `resolve_death`'s.

use tokio::sync::mpsc;

use crate::base::contact_list::wire::EVENT_DEATH;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::loot_drop::generate_loot_on_death;
use super::messaging::{send_entity_method, send_entity_method_to_self_and_witnesses};

/// Apply the death-transition message sequence for a target that just died.
///
/// Ordering and side effects are described at the module level. `target_state`
/// is the corpse's already-mutated `state_field` (with `BSF_Dead` set), passed
/// in by the caller because the caller already had a mutable borrow.
///
/// `level = "info"` because death is low-frequency, high-signal, and the
/// pipeline is multi-step (reticle clear, threat fanout, state-field
/// broadcast, loot drop, respawn arm). One span surfacing all five
/// stages makes regressions like "NPC died but loot didn't drop"
/// debuggable from a single SigNoz trace instead of grepping logs.
#[tracing::instrument(
    name = "combat.death",
    level = "info",
    skip_all,
    fields(target_eid, attacker_id, attacker_is_player, target_is_player,)
)]
pub(super) async fn apply_death_transition(
    target_eid: u32,
    attacker_id: u32,
    target_state: u32,
    attacker_is_player: bool,
    target_is_player: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Discord gameplay-channel (off by default — content/debug signal). Only
    // player deaths post. Killer name + pvp/pve cause are best-effort from the
    // attacker entity; read before the mutation bursts below.
    //
    // Also sends `CellToBaseMsg::ContactListPresenceEvent` for PLAYER deaths so
    // base can fan out CM 89 (eventId=Death, data_value=0) to the deceased
    // player's contact-list watchers. NPC/mob deaths do NOT trigger a fanout —
    // the event makes no sense for mobs and would flood the channel during
    // combat.
    {
        let killer = space_mgr.get_entity(attacker_id).and_then(|e| {
            if attacker_is_player {
                e.character_name.clone()
            } else {
                e.npc_name.clone()
            }
        });
        if target_is_player {
            let character_name = space_mgr
                .get_entity(target_eid)
                .and_then(|e| e.character_name.clone())
                .unwrap_or_else(|| format!("entity:{target_eid}"));
            let cause = if attacker_is_player { "pvp" } else { "pve" };
            cimmeria_discord::emit_player_death(character_name.clone(), killer, cause);

            // Contact-list Death fanout — cell→base hop. The base handler calls
            // `fanout_contact_event` with the player's name and EVENT_DEATH.
            // data_value=0 per spec (client ignores it; shows "{Name} has died").
            let _ = tx
                .send(CellToBaseMsg::ContactListPresenceEvent {
                    player_name: character_name,
                    event_id: EVENT_DEATH,
                    data_value: 0,
                })
                .await;
        } else {
            // NPC / mob death (off by default — high volume during combat).
            let npc_name = space_mgr
                .get_entity(target_eid)
                .and_then(|e| e.npc_name.clone())
                .unwrap_or_else(|| format!("entity:{target_eid}"));
            let cause = if attacker_is_player { "player" } else { "npc" };
            let world_name = space_mgr.get_entity_world_name(target_eid);
            cimmeria_discord::emit_npc_death(npc_name, killer, cause, world_name);
        }
    }

    // Phase J: any channelled effects the dying target was running die
    // with them. `cancel_channels_from_attacker` walks every entity's
    // active_effects list looking for entries sourced by this target,
    // which is the canonical "channeller died, drop their channels"
    // semantics. Runs BEFORE the targeting/threat/loot bursts so the
    // channel-cleared wire packets land before the death sequence.
    let _ = crate::cell::effects::cancel_channels_from_attacker(
        target_eid, None, // cancel all — they're dead
        tx, space_mgr,
    )
    .await;

    // 1. Attacker side: clear targeting reticle.
    if attacker_is_player {
        send_entity_method(
            attacker_id,
            crate::mercury::method_idx::ON_TARGET_UPDATE,
            0i32.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // 1b. Release any cover slot the dying entity was holding. Without
    //     this, dead NPCs leak their reservation indefinitely — the
    //     slot stays unavailable until the corpse despawns (which is
    //     never, for static spawns). Idempotent: cheap when the entity
    //     wasn't in cover. Players also pass through here on PvP death
    //     paths; they don't reserve NPC cover slots, so the call is a
    //     no-op for them but the symmetry is cleaner than gating on
    //     `target_is_player`.
    space_mgr
        .cover
        .release_for_entity(cimmeria_common::EntityId(target_eid as i32));

    // 2. Drop the dying NPC from EVERY player's threatened_mobs set —
    //    not just the killer's. Multiple players can have the same mob on
    //    their threat lists; clearing only the killer's BSF_InCombat would
    //    leave others stuck in combat-ready cursor mode after the only mob
    //    they were threatened by died. Mirrors python `SGWPlayer.on
    //    RemovedFromThreatList` fanout from `SGWMob.onDead`.
    if !target_is_player {
        let to_broadcast =
            crate::cell::combat::clear_dead_npc_from_all_player_threat(space_mgr, target_eid);
        for (player_entity_id, new_state) in to_broadcast {
            tracing::debug!(
                player_entity_id,
                dying_npc = target_eid,
                new_state,
                "death: clearing player BSF_InCombat (last threatened mob died)"
            );
            send_entity_method(
                player_entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
            // No immediate appearance refresh: `exit_player_combat`
            // stamped the OOC timer instead of flipping the holster.
            // The deferred `holster_timer_tick` re-broadcasts
            // `BeingAppearance` after the grace window so chaining
            // mobs doesn't flicker the model (Phase 3).
        }
    }

    // 2b. Auto-cycle stop on target-death — sweep every player auto-firing
    //     at the dying entity and clear their loop. Mirrors the threat
    //     fanout above: multiple players can be auto-cycling at the same
    //     mob; one death must clear `BSF_AUTO_CYCLING` for all of them so
    //     the gun-icon button un-highlights. Applies to player targets too
    //     — a PvP scenario where one player is auto-firing at another
    //     should also stop on the target's death.
    let auto_cycle_broadcasts =
        crate::cell::combat::clear_auto_cycle_for_target(space_mgr, target_eid);
    for (player_entity_id, new_state) in auto_cycle_broadcasts {
        tracing::info!(
            player_entity_id,
            dying_target = target_eid,
            new_state,
            "death: clearing player auto-cycle loop (target died)"
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

    // 2c. Dying player's OWN auto-cycle clears. The sweep above only
    //     catches players TARGETING the dying entity. If the dying entity
    //     is itself an auto-cycling player (e.g., they got killed mid-
    //     loop), their `auto_cycle` flag + stash + `BSF_AUTO_CYCLING`
    //     stay armed. The tick will keep trying to re-fire on each
    //     cooldown tick — every invocation gets rejected by the
    //     `is_dead_state` guard in `handle_use_ability`, so it's wasted
    //     work. Worse: on respawn the player's `auto_cycle` is still
    //     true and the loop auto-resumes against the stashed target
    //     even though the player had no chance to consent.
    //
    //     Clear here so the dying player's button un-highlights on death
    //     and the post-respawn state is clean. NPCs don't have an
    //     `AbilityManager.auto_cycle`, so this branch is player-only.
    if target_is_player {
        if let Some(new_state) = crate::cell::combat::clear_auto_cycle(space_mgr, target_eid) {
            tracing::info!(
                player_entity_id = target_eid,
                new_state,
                "death: clearing dying player's own auto-cycle loop"
            );
            send_entity_method(
                target_eid,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // 3. Target side: roll loot then push interaction flags. Player targets
    //    don't loot or change interaction type.
    if !target_is_player {
        generate_loot_on_death(target_eid, space_mgr);

        let interaction_flags = space_mgr
            .get_entity(target_eid)
            .map_or(0i64, |e| e.interaction_type_flags);
        send_entity_method(
            target_eid,
            crate::mercury::method_idx::INTERACTION_TYPE,
            (interaction_flags as u64).to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // 4. Flip dead-state bit on the corpse — visuals + cursor change client-side.
    // Fan to self+witnesses so a spectator sees the entity become a corpse.
    // For NPC targets the self send is a no-op; for player targets it notifies
    // the dying player and all observers simultaneously.
    send_entity_method_to_self_and_witnesses(
        target_eid,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        target_state.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;
}

/// Resolve a death end-to-end: kill-site state mutations, the wire burst,
/// the threat drain, the death animation, kill XP, and the player Defeat
/// Window.
///
/// **This is the only place a death is allowed to happen.** Every kill
/// path funnels through it — direct ability damage, an effect script's
/// HEALTH bleed applied *after* `damage_apply`'s own lethality check
/// already ran, a DoT pulse, and a GM `.kill`. Before it existed, each
/// caller reimplemented a different subset: the effect-script path
/// reimplemented none of it (the target sat at 0 HP, still `Fighting`,
/// still shooting back, until the attacker's next shot re-ran the
/// direct-damage check), and `kill_npc_out_of_band` reimplemented
/// everything except the death animation and the XP grant.
///
/// # Exactly-once
///
/// The `BSF_DEAD` probe at the top is the idempotency guard. Everything
/// below it is non-idempotent — the loot roll, the XP payout, the threat
/// drain — so re-entering on an already-dead target would double-pay.
/// That guard is what lets `damage_apply` call this twice per hit (once
/// for direct damage, once as a post-effect-script sweep) without the
/// second call doing anything when the first already killed, and what
/// makes a swing at a corpse inert.
///
/// # Parameters
///
/// - `attacker_id` — whoever gets credit: the shooter, the DoT's invoker,
///   or the GM entity.
/// - `ability_id` — the killing ability, for the log line. `None` for
///   out-of-band kills that had no ability (GM `.kill`).
/// - `attacker_is_player` — drives the `onTargetUpdate(0)` reticle drop.
///   A GM `.kill` passes `false` because the GM was never targeting
///   through the combat HUD.
/// - `grant_xp` — pay kill XP to `attacker_id`. Combat and DoT kills pass
///   `true`; a GM `.kill` passes `false` so an admin command can't mint
///   levels. Ignored for player targets (PvP pays no XP).
///
/// Returns `true` when a death was resolved, `false` when the target was
/// missing or already carried `BSF_DEAD`.
pub(super) async fn resolve_death(
    target_eid: u32,
    attacker_id: u32,
    ability_id: Option<i32>,
    attacker_is_player: bool,
    grant_xp: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Resolved before the `&mut` borrow: `mark_npc_dead` labels its
    // `npc_ai.transition` row with it.
    let world = crate::cell::service::npc_ai::world_label(space_mgr, target_eid);
    let (target_state, target_is_player) = {
        let target = match space_mgr.get_entity_mut(target_eid) {
            Some(t) => t,
            None => return false,
        };
        if crate::cell::combat::is_dead_state(target.state_field) {
            return false;
        }

        if target.is_player {
            // Player corpses keep the cell entity alive across same-world
            // respawn (`ReanchorPlayer` reuses the entity instead of
            // destroying and re-creating). Any in-flight weapon-action
            // timer survives unless cleared here, and the per-tick
            // sweeps fire deferred actions regardless of `BSF_DEAD`.
            // Pre-fix surface: a player who dies mid-reload would have
            // `reload_completion_tick` refill their clip during the
            // Defeat Window (free reload during the dead state); a
            // player who dies during the fire-while-holstered draw
            // queue would have `pending_attack_tick` fire `useAbility`
            // against the cached target post-respawn (re-fire against
            // a possibly-dead-or-departed entity). NPCs are destroyed
            // outright on death, so the cleanup is player-only.
            target.set_state_flag(crate::cell::combat::BSF_DEAD);
            target.set_state_flag(crate::cell::combat::BSF_MOVEMENT_LOCK);
            target.clear_weapon_action_state();
        } else {
            // NPC kill: route through the canonical helper so every kill
            // path gets the same state mutations — BSF_DEAD /
            // BSF_MOVEMENT_LOCK, ai_state = Dead, respawn_at stamp,
            // last_movement_type clear, nav_path / velocity reset.
            //
            // `ai_state = Dead` is also what takes the corpse out of the
            // AI tick's admit filter. An NPC that reaches 0 HP without
            // passing through here keeps its `Fighting` state and keeps
            // swinging — exactly the playtest symptom this function was
            // extracted to kill.
            //
            // Do NOT clear `threat_list` here: `apply_death_transition`
            // calls `clear_dead_npc_from_all_player_threat`, which walks
            // this list to drain each aggroed player's `threatened_mobs`
            // and broadcast the BSF_InCombat clear. Wiping it here leaves
            // every aggroed player permanently in-combat. The drain step
            // runs below, immediately after the transition consumes it.
            //
            // Do NOT zero `interaction_type_flags` here. Python
            // `SGWMob.onDead()` OR-merges `INT_NormalLoot` and
            // preserves all other bits — content-driven bits (quest
            // tags, mission interactions) must survive death.
            //
            // `BSF_IN_COMBAT` clear stays here as a raw bit op (see
            // python `SGWMob.py:292`) — `mark_npc_dead` deliberately
            // stays out of the combat-state-machine concerns.
            crate::cell::combat::mark_npc_dead(target, &world);
            target.state_field &= !crate::cell::combat::BSF_IN_COMBAT;
        }
        (target.state_field, target.is_player)
    };

    tracing::info!(
        attacker = attacker_id,
        target = target_eid,
        ability_id = ability_id.unwrap_or(-1),
        is_npc = !target_is_player,
        "Target killed!"
    );

    apply_death_transition(
        target_eid,
        attacker_id,
        target_state,
        attacker_is_player,
        target_is_player,
        tx,
        space_mgr,
    )
    .await;

    // Deferred threat-list clear — the transition consumer reads
    // `threat_list` to fan out the BSF_InCombat clear; drain it only
    // after, or the corpse holds stale aggro entries indefinitely.
    if !target_is_player {
        if let Some(corpse) = space_mgr.get_entity_mut(target_eid) {
            corpse.threat_list.clear();
        }
    }

    side_effects::send_death_sequence(target_eid, tx, space_mgr).await;

    if grant_xp {
        side_effects::grant_kill_xp(target_eid, attacker_id, tx, space_mgr).await;
    }

    if target_is_player {
        side_effects::send_begin_aid_wait(target_eid, attacker_id, ability_id, tx, space_mgr).await;
    }

    true
}

/// Kill an NPC outside the single-hit damage pipeline (GM command, DoT
/// pulse, scripted death).
///
/// Thin NPC-only wrapper over [`resolve_death`] — it exists so callers
/// outside `abilities` (the GM `.kill` handler, the DoT pulse tick) get
/// the player-target rejection for free rather than each re-deriving it.
/// See [`resolve_death`] for the parameter semantics.
///
/// Returns `true` if a kill was applied, `false` if the target was absent,
/// a player, or already dead.
pub(crate) async fn kill_npc_out_of_band(
    target_eid: u32,
    attacker_id: u32,
    attacker_is_player: bool,
    grant_xp: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Fail closed: this entry point is NPC-only. Player corpses go through
    // the PvP/respawn path, not here.
    if space_mgr.get_entity(target_eid).is_none_or(|t| t.is_player) {
        return false;
    }

    resolve_death(
        target_eid,
        attacker_id,
        // No ability drove this kill — the log line records -1.
        None,
        attacker_is_player,
        grant_xp,
        tx,
        space_mgr,
    )
    .await
}

mod side_effects;

#[cfg(test)]
mod tests;
