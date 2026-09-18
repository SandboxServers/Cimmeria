//! `handle_use_ability` + content-engine kill-credit wrapper.
//!
//! Split out of the main flow: the single-target player-driven entry
//! point that resolves an ability and then fires `EntityDeath` content
//! events for any alive→dead transition (primary target + cone
//! secondaries) so kill-count missions progress.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use cimmeria_entity::stats::HEALTH;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

use super::handle::handle_use_ability;

/// `handle_use_ability` + content-engine kill-credit hook.
///
/// **Use this from every single-target player-driven path that calls
/// [`handle_use_ability`] directly.** Calls `handle_use_ability` to
/// resolve the ability, then — if the attacker is a player who just
/// transitioned a tagged NPC from alive→dead — fires the `EntityDeath`
/// content event so mission KillCount chains (e.g., "kill 5
/// Hallway_Guards") progress.
///
/// **Not** for AoE / ground-target callers: those go through
/// [`super::super::handle_use_ability_on_ground`], which returns the set of
/// every NPC that died during the cast and fires per-death
/// `fire_entity_death` at the caller layer. The AoE path is the only
/// other single canonical kill-credit fan-out today; collapsing them
/// would require returning a Vec<entity_id> from this helper too.
///
/// Why this isn't baked into `handle_use_ability` itself: NPC AI also
/// calls `handle_use_ability`, and NPC kills shouldn't fire
/// `EntityDeath` (the killer has no `player_id` — there's no mission to
/// credit). Tests that exercise `handle_use_ability` mechanics also
/// don't need to thread a `ChainEngine` through. Keeping the bare
/// function callable from those sites preserves both invariants.
///
/// Mirrors the python `useAbility` → `attemptDeath` → `_doDeath` chain
/// where the cell-side death callback was the canonical credit point.
pub async fn handle_use_ability_with_kill_credit(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Snapshot whether the target was a live NPC *before* the ability
    // resolves. Without this, hitting an already-dead corpse would
    // re-fire `fire_entity_death` and double-count mission progress on
    // every post-death swing. Player targets are excluded because PvP
    // kills don't drive mission progression today.
    //
    // The health *percentage* is snapshotted in the same pass for the
    // `entity_health_below` trigger (Harset H04). It has to happen here:
    // once `handle_use_ability` returns, the pre-hit value is gone, and
    // a downward threshold crossing can only be computed from both
    // sides of the hit.
    let (was_alive_before, pct_before) = if target_id > 0 {
        match space_mgr.get_entity(target_id as u32) {
            Some(t) if !t.is_player => (
                t.stats.get(HEALTH).is_some_and(|s| s.cur > 0),
                crate::cell::combat::health_pct(t),
            ),
            _ => (false, None),
        }
    } else {
        (false, None)
    };

    let committed = handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

    // Skip the death check when the ability was rejected pre-consume —
    // nothing was damaged, so nothing died. Also short-circuits the
    // common no-target paths (target_id == 0).
    if !committed || !was_alive_before {
        return committed;
    }

    let target_eid = target_id as u32;
    let just_died = space_mgr
        .get_entity(target_eid)
        .is_some_and(|t| t.stats.get(HEALTH).is_some_and(|s| s.cur <= 0));
    if !just_died {
        // Survived the hit — the other half of the same decision, so a
        // chain author gets exactly one of `entity_dead_tag` and
        // `entity_health_below` per hit.
        //
        // This branch is defence-in-depth, not the enforcement. The
        // authority for "a killing blow never fires a threshold chain"
        // lives in `fire_health_below_for_hit`, which drops any hit
        // whose target ends dead — it has to, because `just_died` here
        // reads health, and an effect script can heal a corpse back
        // above zero after the death transition has run.
        crate::cell::content::fire_health_below_for_hit(
            entity_id, target_eid, pct_before, engine, tx, space_mgr,
        )
        .await;
        return committed;
    }

    // Resolve the target's content-engine tag (the chain trigger key,
    // e.g. "Hallway01_Guard") and the killer's `player_id` (the
    // mission-context key). Either being absent is benign — a tagless
    // NPC just doesn't progress any chain; a player_id-less killer
    // (NPC AI shouldn't reach this helper, but be defensive) skips
    // with a warn so the unexpected case stays visible.
    let tag = match space_mgr.get_entity(target_eid).and_then(|t| t.tag.clone()) {
        Some(t) => t,
        None => return committed,
    };
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id, npc_tag = %tag,
                "handle_use_ability_with_kill_credit: killer has no player_id — skipping EntityDeath event"
            );
            return committed;
        }
    };

    crate::cell::content::fire_entity_death(entity_id, player_id, &tag, engine, tx, space_mgr)
        .await;

    // Cone AoE kill credit: drain the per-attacker scratchpad that
    // `handle_use_ability` populated with cone-secondary deaths and
    // fire `entity_death` for each tagged kill. Matches the same
    // discipline as `handle_use_ability_on_ground`.
    let cone_dead_ids: Vec<u32> = space_mgr
        .get_entity_mut(entity_id)
        .map(|att| std::mem::take(&mut att.last_aoe_deaths))
        .unwrap_or_default();
    for dead_eid in cone_dead_ids {
        let dead_tag = space_mgr.get_entity(dead_eid).and_then(|t| t.tag.clone());
        if let Some(t) = dead_tag {
            crate::cell::content::fire_entity_death(
                entity_id, player_id, &t, engine, tx, space_mgr,
            )
            .await;
        }
    }
    committed
}
