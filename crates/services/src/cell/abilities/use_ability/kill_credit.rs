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
/// It also drains the `entity_health_below` samples the damage seam
/// queued for this cast — see
/// [`crate::cell::content::fire_pending_health_below`].
///
/// **Not** for AoE / ground-target callers: those go through
/// [`super::super::handle_use_ability_on_ground`], which returns the set of
/// every NPC that died during the cast and fires per-death
/// `fire_entity_death` at the caller layer (and drains the same
/// health-below queue itself). The AoE path is the only other single
/// canonical kill-credit fan-out today; collapsing them would require
/// returning a Vec<entity_id> from this helper too.
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
///
/// A cast with a warmup (AT-10) only launches here; nothing is damaged
/// yet, so nothing is credited. The warmup tick runs the same
/// [`is_live_npc`] + [`credit_single_target`] pair around the delayed fire.
pub async fn handle_use_ability_with_kill_credit(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let was_alive_before = is_live_npc(space_mgr, target_id);
    let committed = handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;
    credit_single_target(
        entity_id,
        target_id,
        committed,
        was_alive_before,
        engine,
        tx,
        space_mgr,
    )
    .await;
    committed
}

/// Whether `target_id` is a live NPC, sampled *before* a cast resolves.
///
/// Without the snapshot, hitting an already-dead corpse would re-fire
/// `fire_entity_death` and double-count mission progress on every
/// post-death swing. Player targets are excluded because PvP kills don't
/// drive mission progression today.
pub(super) fn is_live_npc(space_mgr: &SpaceManager, target_id: i32) -> bool {
    target_id > 0
        && space_mgr
            .get_entity(target_id as u32)
            .is_some_and(|t| !t.is_player && t.stats.get(HEALTH).is_some_and(|s| s.cur > 0))
}

/// The credit half of [`handle_use_ability_with_kill_credit`]: drain the
/// health-below samples, then fire `EntityDeath` for the primary and any
/// cone secondaries that died. `committed` is false when the cast was
/// refused before anything resolved.
pub(super) async fn credit_single_target(
    entity_id: u32,
    target_id: i32,
    committed: bool,
    was_alive_before: bool,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `entity_health_below` drain (Harset H04, reworked in the PR #662
    // review). The pre-hit percentages were sampled inside
    // `apply_damage_to_target` — once per damaged target, so cone and AoE
    // secondaries are covered, which they were not when this wrapper did
    // its own single-target snapshot. Draining before the death dispatch
    // below keeps `pct_after` as close to the hit as possible; a killing
    // blow is suppressed inside `fire_health_below_for_hit` (the corpse
    // already carries `BSF_DEAD` by now), which is what keeps
    // `entity_dead_tag` and `entity_health_below` mutually exclusive per
    // hit.
    crate::cell::content::fire_pending_health_below(engine, tx, space_mgr).await;

    // Skip the death check when the ability was rejected pre-consume —
    // nothing was damaged, so nothing died. Also short-circuits the
    // common no-target paths (target_id == 0).
    if !committed || !was_alive_before {
        return;
    }

    let target_eid = target_id as u32;
    let just_died = space_mgr
        .get_entity(target_eid)
        .is_some_and(|t| t.stats.get(HEALTH).is_some_and(|s| s.cur <= 0));
    if !just_died {
        return;
    }

    // Resolve the target's content-engine tag (the chain trigger key,
    // e.g. "Hallway01_Guard") and the killer's `player_id` (the
    // mission-context key). Either being absent is benign — a tagless
    // NPC just doesn't progress any chain; a player_id-less killer
    // (NPC AI shouldn't reach this helper, but be defensive) skips
    // with a warn so the unexpected case stays visible.
    let tag = match space_mgr.get_entity(target_eid).and_then(|t| t.tag.clone()) {
        Some(t) => t,
        None => return,
    };
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id, npc_tag = %tag,
                "handle_use_ability_with_kill_credit: killer has no player_id — skipping EntityDeath event"
            );
            return;
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
}

/// Kill credit for a ground-target cast: drain the `entity_health_below`
/// samples every wounded target queued, then fire `EntityDeath` for each
/// tagged NPC in `deaths` (the list `handle_use_ability_on_ground`
/// returns). Shared by the `useAbilityOnGroundTarget` handler and the
/// warmup tick, which fires a ground cast after its warmup (AT-10).
pub(crate) async fn credit_ground_deaths(
    entity_id: u32,
    deaths: Vec<u32>,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `entity_health_below` drain for every target this cast
    // wounded — primary and AoE secondaries alike. Before the
    // PR #662 review the trigger only existed on the
    // single-target path, so a ground cast that dragged a
    // tagged mob through its threshold lost the crossing
    // permanently (the band predicate needs `pct_before >
    // threshold`, which no later hit can satisfy). Drained
    // before the death fan-out below; a killing blow is
    // suppressed inside `fire_health_below_for_hit`.
    crate::cell::content::fire_pending_health_below(engine, tx, space_mgr).await;

    if deaths.is_empty() {
        return;
    }
    // Resolve player_id once — it doesn't change across kills.
    let player_id = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
    for dead_eid in deaths {
        let tag = space_mgr.get_entity(dead_eid).and_then(|t| t.tag.clone());
        if let Some(tag) = tag {
            match player_id {
                Some(pid) => {
                    crate::cell::content::fire_entity_death(
                        entity_id, pid, &tag, engine, tx, space_mgr,
                    )
                    .await;
                }
                None => {
                    tracing::warn!(
                        entity_id, npc_tag = %tag, dead_eid,
                        "Skipping entity_death event (ground target): killer entity has no player_id"
                    );
                }
            }
        }
    }
}
