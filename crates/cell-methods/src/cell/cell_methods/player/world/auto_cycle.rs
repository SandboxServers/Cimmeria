//! `setAutoCycle` toggle: BSF_AUTO_CYCLING bit management, the immediate
//! fire-on-enable path, and the user-preference state_field persist.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

/// Handle the `setAutoCycle(enabled)` toggle. Lights / clears
/// `BSF_AUTO_CYCLING`, optionally fires an immediate shot on enable, and
/// persists the deliberate button choice.
pub(super) async fn handle_set_auto_cycle(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if args.is_empty() {
        return;
    }
    let enabled = args[0] != 0;
    let id = space_mgr.player_identity(entity_id);
    if !enabled {
        tracing::info!(
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id,
            enabled,
            "setAutoCycle"
        );
    }
    if enabled {
        // The weapon's own attack, for a player who arms the loop before
        // firing anything this session: without it the button lit but the
        // loop had no ability to re-fire until a manual shot committed.
        let weapon_ability = crate::cell::abilities::ability_for_active_weapon(
            space_mgr,
            entity_id,
            crate::cell::spawner::EVENT_ITEM_RANGED,
        );
        // Light BSF_AUTO_CYCLING immediately so the button
        // highlights on the very first press; without this
        // the button looks broken until the player happens
        // to right-click an enemy. Phase 2: if the player
        // already has a target selected AND has fired any
        // ability in this session, ALSO fire that ability
        // now — the press feels like an action ("start
        // firing"), not a mode flip. `last_fired_ability_id`
        // is the simplest server-side proxy for "what
        // would the player fire?" since the wire payload
        // carries no ability id.
        let (new_state, immediate_fire) = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => return,
            };
            entity.abilities.auto_cycle = true;
            // Raw bit op — see auto_cycle module doc for why
            // BSF_AUTO_CYCLING bypasses the ref-counted helpers.
            let old = entity.state_field;
            entity.state_field |= crate::cell::combat::BSF_AUTO_CYCLING;
            let new_state = (entity.state_field != old).then_some(entity.state_field);
            let current_target_id = entity.current_target_id;
            let last_fired_ability_id = entity.abilities.last_fired_ability_id;
            let loop_ability = last_fired_ability_id.or(weapon_ability);
            let immediate_fire = loop_ability.zip(current_target_id);
            // Persist the loop's committed ability BEFORE
            // calling handle_use_ability. If that call
            // rejects (out of range / cooldown / no ammo),
            // its commit-time arm path never runs and the
            // tick driver would have no ability id to
            // re-fire with — BSF-armed but functionally
            // dead. Stashing here lets the next
            // cooldown-clear tick pick up the loop
            // regardless of immediate-fire outcome.
            // The stash does not need a target: the tick reads the live
            // one, so a target picked after the press still starts the loop.
            if let Some(ability_id) = loop_ability {
                entity.abilities.auto_cycle_ability_id = Some(ability_id);
            }
            tracing::info!(
                account_id = id.account_id,
                player_id = id.player_id,
                entity_id,
                enabled,
                current_target_id = current_target_id.unwrap_or(0),
                last_fired_ability_id = last_fired_ability_id.unwrap_or(0),
                weapon_ability_id = weapon_ability.unwrap_or(0),
                auto_cycle_ability_id = entity.abilities.auto_cycle_ability_id.unwrap_or(0),
                "setAutoCycle"
            );
            (new_state, immediate_fire)
        };
        if let Some(new_state) = new_state {
            crate::cell::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
            persist_state_field_bits(entity_id, new_state, tx, space_mgr).await;
        }
        // Gated on bit transition. CEGUI fires the Lua
        // binding 3-4× per physical click (~150µs apart);
        // without the gate every duplicate would re-attempt
        // the fire and rely on the cooldown gate inside
        // handle_use_ability to reject — wasted work + log
        // noise.
        //
        // Additionally gated on the stashed ability being
        // off cooldown. The bit-transition gate alone still
        // produces one cooldown-rejection DEBUG per
        // "arm while mid-cooldown" toggle (player manually
        // right-clicks once, then arms auto-cycle while
        // the manual shot is still cooling). The stash is
        // already persisted above, so the next
        // auto_cycle_tick after cooldown clear picks the
        // re-fire up — calling handle_use_ability here
        // when it would just reject is wasted work.
        let stash_ready = match immediate_fire {
            Some((ability_id, _)) => space_mgr
                .get_entity(entity_id)
                .is_some_and(|e| !e.abilities.is_on_cooldown(ability_id)),
            None => false,
        };
        // One line per press naming what the press did, so a "lit but no
        // shot" report reads straight off SigNoz.
        let decision = match (new_state, immediate_fire, stash_ready) {
            (None, _, _) => "already_armed",
            (Some(_), None, _) if loop_ability_missing(space_mgr, entity_id) => "no_ability",
            (Some(_), None, _) => "no_target",
            (Some(_), Some(_), false) => "on_cooldown",
            (Some(_), Some(_), true) => "fire",
        };
        tracing::info!(
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id,
            decision,
            "setAutoCycle: enable decision"
        );
        if let (Some(_), true, Some((ability_id, target_id))) =
            (new_state, stash_ready, immediate_fire)
        {
            tracing::info!(
                account_id = id.account_id,
                player_id = id.player_id,
                entity_id,
                ability_id,
                target_id,
                "setAutoCycle: immediate fire on enable (loop ability + current_target ready)"
            );
            // The immediate-fire on auto-cycle toggle ON
            // is a player-driven kill path — route through
            // the kill-credit wrapper so a tap of the
            // auto-fire button that immediately kills a
            // quest target credits the mission, matching
            // the manual-right-click path.
            let _ = crate::cell::abilities::handle_use_ability_with_kill_credit(
                entity_id,
                ability_id,
                target_id,
                &crate::cell::content::EngineEvents(engine),
                tx,
                space_mgr,
            )
            .await;
        }
    } else {
        // Explicit disable: drop the stash AND clear the bit.
        // clear_auto_cycle returns Some only on bit
        // transition — re-broadcasting for an already-off
        // player would be wire noise.
        if let Some(new_state) = crate::cell::combat::clear_auto_cycle(space_mgr, entity_id) {
            crate::cell::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
            persist_state_field_bits(entity_id, new_state, tx, space_mgr).await;
        }
    }
}

/// Whether the press left the loop with no ability to re-fire: nothing fired
/// this session and no weapon bound to a ranged attack.
fn loop_ability_missing(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_none_or(|e| e.abilities.auto_cycle_ability_id.is_none())
}

/// Fire-and-forget persist of the user-preference `state_field` bits
/// after an explicit `setAutoCycle` toggle flipped `BSF_AutoCycling`.
///
/// Masks to [`crate::cell::combat::PERSISTED_STATE_FIELD_MASK`] so
/// transient combat bits riding the same broadcast value (BSF_Dead,
/// BSF_InCombat, BSF_MovementLock) never reach the DB — a relog must
/// always be a clean combat slate (#412).
///
/// Deliberately called from the explicit toggle site only, NOT from the
/// in-combat auto-clear paths (target death, manual fire of a different
/// ability, AF_DEACTIVATE_AUTO_CYCLE): those are session mechanics, and
/// mirroring them to the DB would make the post-relog state depend on
/// whether the player's last target happened to die — the persisted
/// value tracks the player's deliberate button choice.
///
/// Silent no-op for entities without a `player_id` (NPCs / test
/// fixtures shouldn't persist).
async fn persist_state_field_bits(
    entity_id: u32,
    state_field: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field: state_field & crate::cell::combat::PERSISTED_STATE_FIELD_MASK,
        })
        .await
    {
        // Channel closed — the toggle still applies in-memory for this
        // session but won't survive the relog. Same level rationale as
        // the SystemOptionsUpdate persist failure path.
        tracing::warn!(
            entity_id,
            player_id,
            error = %e,
            "StateFieldUpdate send to base failed -- auto-cycle preference not persisted"
        );
    }
}
