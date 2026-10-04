//! Explosive-round splash (ammo campaign AM-10, issue #1026).
//!
//! When a shot's on-hit effect is a splash
//! ([`ammo_explosive::splash_of`]: a `TCM_AERadius` effect with a
//! `SplashDamageFraction`), every other hostile within the effect's radius
//! of the target takes that share of the shot, after the target's own hit
//! has resolved.
//!
//! # Who is splashed
//!
//! The candidates come from the ground-AoE collector
//! (`dispatch::collect_ground_targets`), anchored at the primary target
//! instead of a ground click. So the splash follows the same rules as every
//! area ability (ADR decision 24): the attacker's space, alive, and
//! `combat::may_hit_in_area`, which for a player shooter is
//! `player_may_attack`: hostile-faction NPCs that are not pets, plus an
//! engaged duel partner. A friendly or neutral NPC, a pet and any other
//! player are never candidates. The primary target (already hit) and the
//! shooter are removed.
//!
//! Then line of sight from the blast, which area abilities do not check
//! (ADR decision 20): a hostile behind a wall from the target is not
//! splashed. The check uses the space's collision occluder with the
//! fire-time rule's policy: no occluder, or an `Unknown` answer, never
//! removes a target, because the navmesh ray reads furniture as walls.
//!
//! # No chaining
//!
//! Each splash target is applied with [`HitKind::Splash`]: its damage is
//! scaled by the fraction, and the per-target function runs no on-hit effect
//! for it, so a splash target never splashes. It also skips the ability's
//! other effect scripts and its pulsing effects: the splash is the shot's
//! damage only, not a second copy of its bleed or DoT. The pulsing effects
//! are scoped out of the ability before the splash hit
//! (`effect_routing::splash_scope`), so not even a DoT's first tick lands on
//! a splash target. The exception is a
//! damage script (`RangedPhysicalDamage` and its siblings, AB-06): that
//! script IS the shot's damage, so it runs on the splash target at the
//! splash fraction (`effect_scripts::apply_damage_scripts`).

use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::effects::ammo_explosive::Splash;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::navigation::LineOfSight;
use cimmeria_entity::stats::HEALTH;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::{occluder_probe, SpaceManager};

/// Which hit [`super::apply_hit`] resolves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum HitKind {
    /// A shot on its own target: the primary, or a cone or ground-AoE
    /// secondary. Runs the ammo's on-hit effect, which may splash.
    Direct,
    /// A splash target of a shot: `fraction` of the shot's damage, no on-hit
    /// effect (so no further splash), no pulsing effects and no ability
    /// scripts except its damage scripts, which run at the fraction.
    Splash { fraction: f64 },
}

impl HitKind {
    pub(super) fn is_direct(self) -> bool {
        self == HitKind::Direct
    }

    /// The factor on the hit's pre-armour damage.
    pub(super) fn damage_scale(self) -> f64 {
        match self {
            HitKind::Direct => 1.0,
            HitKind::Splash { fraction } => fraction,
        }
    }
}

/// The entities a splash of `radius` metres around `primary_id` reaches,
/// nearest first, and how many hostiles in the radius a wall hid.
pub(super) fn splash_targets(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    primary_id: u32,
    radius: f32,
) -> (Vec<u32>, u32) {
    let (Some(attacker), Some(primary)) = (
        space_mgr.get_entity(attacker_id),
        space_mgr.get_entity(primary_id),
    ) else {
        return (Vec::new(), 0);
    };
    // The launch refuses a target in another space (#906); a primary that is
    // somehow elsewhere splashes nothing rather than the attacker's space.
    if primary.space_id != attacker.space_id {
        return (Vec::new(), 0);
    }
    let anchor = primary.position;
    let in_radius = super::super::dispatch::collect_ground_targets(
        space_mgr,
        attacker_id,
        attacker.space_id,
        [anchor.x, anchor.y, anchor.z],
        radius * radius,
    );
    let occ = space_mgr.occluder_of(attacker_id);
    let blast_eye = space_mgr.eye_height_of(primary);
    let mut out = Vec::with_capacity(in_radius.len());
    let mut blocked = 0;
    for (eid, _) in in_radius {
        if eid == primary_id || eid == attacker_id {
            continue;
        }
        let Some(e) = space_mgr.get_entity(eid) else {
            continue;
        };
        if let Some(occ) = occ {
            let probe = occluder_probe(
                occ,
                anchor,
                blast_eye,
                e.position,
                space_mgr.eye_height_of(e),
            );
            if probe.result == LineOfSight::Blocked {
                blocked += 1;
                continue;
            }
        }
        out.push(eid);
    }
    (out, blocked)
}

/// A boxed `Send` future. [`apply_splash`] names its return type so the
/// recursion `apply_hit` -> `apply_splash` -> `apply_hit` has no opaque
/// future in its cycle: with an `async fn` here the `Send` check fails.
pub(super) type SplashFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Splash the hostiles around `primary_id` after `attacker_id`'s shot hit
/// it. Deaths join the attacker's `last_aoe_deaths`, the scratchpad the
/// kill-credit wrapper drains for cone secondaries (ADR decision 13).
pub(super) fn apply_splash<'a>(
    attacker_id: u32,
    primary_id: u32,
    ability_id: i32,
    ability_def: &'a Option<AbilityDef>,
    splash: Splash,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> SplashFuture<'a> {
    Box::pin(splash_all(
        attacker_id,
        primary_id,
        ability_id,
        ability_def,
        splash,
        tx,
        space_mgr,
    ))
}

async fn splash_all(
    attacker_id: u32,
    primary_id: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    splash: Splash,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (targets, los_blocked) = splash_targets(space_mgr, attacker_id, primary_id, splash.radius);
    let who = space_mgr.player_identity(attacker_id);
    tracing::debug!(
        target: "ammo",
        event = "ammo_splash",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id = attacker_id,
        entity_name = space_mgr.entity_label(attacker_id),
        target_entity_id = primary_id,
        target_entity_name = space_mgr.entity_label(primary_id),
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        effect_id = splash.effect_id,
        effect_name = cimmeria_names::book().effect(splash.effect_id),
        radius = splash.radius,
        fraction = splash.fraction,
        splash_count = targets.len(),
        los_blocked,
        ?targets,
        "explosive round splashed the hostiles around its target"
    );
    if targets.is_empty() {
        return;
    }
    // The splash is the shot's direct damage: not its DoT, whose first tick
    // used to land here with nothing registered behind it (AB-07).
    let scoped = super::super::effect_routing::splash_scope(&space_mgr.effect_defs, ability_def);
    let ability_def = &scoped;
    let alive_before: Vec<u32> = targets
        .iter()
        .copied()
        .filter(|&t| {
            space_mgr
                .get_entity(t)
                .is_some_and(|e| e.stats.get(HEALTH).is_some_and(|s| s.cur > 0))
        })
        .collect();
    for &secondary in &targets {
        // A fresh effect_seq per target, as the cone and ground-AoE
        // secondaries: a separate roll and a separately correlatable
        // onEffectResults.
        let seq = space_mgr
            .get_entity_mut(attacker_id)
            .map(|e| e.abilities.next_effect_id())
            .unwrap_or(0);
        // `HitKind::Splash` ends the recursion one level down: a splash
        // target runs no on-hit effect, so it never comes back here.
        super::apply_hit(
            attacker_id,
            secondary,
            ability_id,
            ability_def,
            seq as u32,
            // The primary already flushed the bandolier.
            false,
            HitKind::Splash {
                fraction: splash.fraction,
            },
            tx,
            space_mgr,
        )
        .await;
    }
    let deaths: Vec<u32> = alive_before
        .into_iter()
        .filter(|&t| {
            space_mgr
                .get_entity(t)
                .is_some_and(|e| e.stats.get(HEALTH).is_some_and(|s| s.cur <= 0))
        })
        .collect();
    if !deaths.is_empty() {
        if let Some(att) = space_mgr.get_entity_mut(attacker_id) {
            att.last_aoe_deaths.extend(deaths);
        }
    }
}
