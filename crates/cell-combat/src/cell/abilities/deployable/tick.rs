//! The deployable tick: every AoI tick, remove the deployables whose owner
//! no longer holds them or whose last pulse ran, and fire every pulse that
//! is due.

use cimmeria_cell_world::cell::duel::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::deployables::{
    deployable_verdict, despawn_deployable, scrub_orphan, DeployableDespawnReason,
    DeployableVerdict,
};
use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::super::super::combat;
use super::super::super::content_events::ContentEvents;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::damage_apply::apply_damage_to_target;
use super::super::use_ability::credit_ground_deaths;

/// Per-tick entry point, wired into the cell message loop after the pet
/// sweep. Returns at once when no deployable is live.
pub async fn deployable_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) {
    if space_mgr.deployables.is_empty() {
        return;
    }
    deployable_tick_at(Instant::now(), tx, space_mgr, events).await;
}

/// [`deployable_tick`] against an explicit clock, so tests can step through
/// a lifetime without sleeping. Returns how many pulses fired.
pub async fn deployable_tick_at(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) -> usize {
    let mut pulses = 0;
    for deployable in space_mgr.deployables.ids() {
        match deployable_verdict(space_mgr, deployable, now) {
            DeployableVerdict::Hold => {}
            DeployableVerdict::Scrub => scrub_orphan(space_mgr, deployable),
            DeployableVerdict::Despawn(reason) => {
                let _outcome = despawn_deployable(space_mgr, deployable, reason, "sweep", tx).await;
            }
            DeployableVerdict::Pulse => {
                fire_pulse(deployable, tx, space_mgr, events).await;
                pulses += 1;
                // The last pulse removes the object in the same tick, so its
                // lifetime is exactly `pulses_total * pulse_interval`.
                if space_mgr
                    .deployables
                    .get(deployable)
                    .is_some_and(|s| s.is_spent())
                {
                    let _outcome = despawn_deployable(
                        space_mgr,
                        deployable,
                        DeployableDespawnReason::Expired,
                        "sweep",
                        tx,
                    )
                    .await;
                }
            }
        }
    }
    pulses
}

/// The targets one pulse of `deployable` hits, nearest first: every live
/// entity in its space within its radius that its owner may hit in an area
/// (`combat::may_hit_in_area`, the rule the owner's own ground AoE obeys).
/// Players other than an engaged duel partner, pets and friendly NPCs are
/// never candidates.
pub(in crate::cell::abilities) fn pulse_targets(
    space_mgr: &SpaceManager,
    deployable: u32,
) -> Vec<u32> {
    let Some(state) = space_mgr.deployables.get(deployable) else {
        return Vec::new();
    };
    let (Some(object), Some(owner)) = (
        space_mgr.get_entity(deployable),
        space_mgr.get_entity(state.owner),
    ) else {
        return Vec::new();
    };
    let radius_sq = state.radius * state.radius;
    let mut hits: Vec<(u32, f32)> = combat::area_candidates(space_mgr, state.owner)
        .into_iter()
        .filter(|&eid| eid != deployable && eid != state.owner)
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            if e.space_id != object.space_id
                || combat::is_dead_state(e.state_field)
                || !combat::may_hit_in_area(owner, e, space_mgr.resources.duels())
            {
                return None;
            }
            let d = e.position.distance_squared_to(&object.position);
            (d <= radius_sq).then_some((eid, d))
        })
        .collect();
    hits.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    hits.into_iter().map(|(eid, _)| eid).collect()
}

/// `(HEALTH, FOCUS, alive)` of `eid` now.
fn vitals(space_mgr: &SpaceManager, eid: u32) -> (i32, i32, bool) {
    space_mgr.get_entity(eid).map_or((0, 0, false), |e| {
        let health = e.stats.get(HEALTH).map_or(0, |s| s.cur);
        let focus = e.stats.get(FOCUS).map_or(0, |s| s.cur);
        (
            health,
            focus,
            health > 0 && !combat::is_dead_state(e.state_field),
        )
    })
}

/// Fire one pulse: apply the pulse effect to every target, as the owner.
async fn fire_pulse(
    deployable: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    events: &dyn ContentEvents,
) {
    let Some(state) = space_mgr.deployables.get(deployable).cloned() else {
        return;
    };
    let owner = state.owner;
    // The ability as the pulse sees it: its own row, with only the pulse
    // effect. The damage pipeline reads damage NVPs and scripts from every
    // effect it is given and registers any pulsing one on the target, so
    // handing it the whole ability would put the 30-pulse lifetime effect
    // on every mob hit.
    let pulse_def = space_mgr
        .ability_defs
        .get(&state.ability_id)
        .cloned()
        .map(|mut d| {
            d.effect_ids = vec![state.pulse_effect_id];
            d
        });
    if pulse_def.is_none() {
        tracing::warn!(
            target: "deployables.pulse",
            event = "pulse_skipped",
            reason = "no_ability_def",
            entity_id = deployable,
            entity_name = space_mgr.entity_label(deployable),
            deployable_id = deployable,
            deployable_name = space_mgr.entity_label(deployable),
            owner_id = owner,
            owner_name = state.owner_identity.player_name,
            account_id = state.owner_identity.account_id,
            account_name = state.owner_identity.account_name,
            player_id = state.owner_identity.player_id,
            player_name = state.owner_identity.player_name,
            ability_id = state.ability_id,
            ability_name = cimmeria_names::book().ability(state.ability_id),
            "deployable pulse has no ability definition; it lands nothing"
        );
    }

    let targets = if pulse_def.is_some() {
        pulse_targets(space_mgr, deployable)
    } else {
        Vec::new()
    };
    let mut health_damage = 0i64;
    let mut focus_damage = 0i64;
    let mut deaths = Vec::new();
    for &target in &targets {
        let (h0, f0, alive0) = vitals(space_mgr, target);
        let seq = space_mgr
            .get_entity_mut(owner)
            .map_or(0, |e| e.abilities.next_effect_id());
        apply_damage_to_target(
            owner,
            target,
            state.ability_id,
            &pulse_def,
            seq as u32,
            false,
            tx,
            space_mgr,
        )
        .await;
        let (h1, f1, alive1) = vitals(space_mgr, target);
        health_damage += i64::from((h0 - h1).max(0));
        focus_damage += i64::from((f0 - f1).max(0));
        if alive0 && !alive1 {
            deaths.push(target);
        }
    }
    let kills = deaths.len() as u32;
    // Mission kill credit and the `entity_health_below` drain, the owner's,
    // exactly as for the owner's own ground cast.
    credit_ground_deaths(owner, deaths, events, tx, space_mgr).await;

    let Some(s) = space_mgr.deployables.get_mut(deployable) else {
        return;
    };
    s.totals.pulses += 1;
    s.totals.hits += targets.len() as u32;
    s.totals.health_damage += health_damage;
    s.totals.focus_damage += focus_damage;
    s.totals.kills += kills;
    s.next_pulse_at += s.pulse_interval;
    let pulse_index = s.totals.pulses;
    let pulses_total = s.pulses_total;
    tracing::debug!(
        target: "deployables.pulse",
        event = "pulse",
        entity_id = deployable,
        entity_name = space_mgr.entity_label(deployable),
        deployable_id = deployable,
        deployable_name = space_mgr.entity_label(deployable),
        owner_id = owner,
        owner_name = state.owner_identity.player_name,
        account_id = state.owner_identity.account_id,
        account_name = state.owner_identity.account_name,
        player_id = state.owner_identity.player_id,
        player_name = state.owner_identity.player_name,
        ability_id = state.ability_id,
        ability_name = cimmeria_names::book().ability(state.ability_id),
        effect_id = state.pulse_effect_id,
        effect_name = cimmeria_names::book().effect(state.pulse_effect_id),
        pulse_index,
        pulses_total,
        targets = targets.len(),
        health_damage,
        focus_damage,
        kills,
        "deployable pulse"
    );
}
