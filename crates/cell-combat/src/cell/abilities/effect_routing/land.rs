//! Landing the effects a cast routes off its target pipeline: the user
//! halves and the beneficial area halves (AB-07), and every effect of a
//! beneficial cast (AB-01's `fire_beneficial`).
//!
//! An effect lands by running its script on its recipient with the caster
//! as source, then each recipient's dirty stats and buff timers go to it and
//! its witnesses, and a pulsing effect is registered on its recipient with
//! the caster as invoker. There is no QR roll, so a miss never drops one; no
//! threat, no in-combat state, no `onEffectResults`, and no #444 gate.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EffectDef;

use super::super::effect_plan::{
    PlanIds, PlannedEffect, PATH_ALLY_FANOUT, PATH_ROUTED_TO_USER, REASON_BENEFICIAL_CAST,
    REASON_NOT_REACHABLE, REASON_NO_SCRIPT,
};

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::messaging::WireRoute;
use super::super::wire_ledger::{self, WireCtx};

/// One effect and the entity it lands on.
#[derive(Debug, Clone)]
pub(in crate::cell::abilities) struct Landing {
    pub effect: EffectDef,
    pub recipient: u32,
    /// Why it lands there: the route `plan_cast` chose, logged on the
    /// landing's `effect_planned` row (AB-T3).
    pub route: LandingRoute,
}

/// How a [`Landing`] was routed, for its `effect_planned` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::abilities) enum LandingRoute {
    /// A user half (rule 1 or 2), with the routing `reason`.
    User(&'static str),
    /// A beneficial area half fanned out to an ally (rule 3).
    AllyArea(&'static str),
    /// An effect of a beneficial cast on its resolved target (AB-01).
    BeneficialTarget,
}

impl Landing {
    pub(in crate::cell::abilities) fn new(
        effect: &EffectDef,
        recipient: u32,
        route: LandingRoute,
    ) -> Self {
        Self {
            effect: effect.clone(),
            recipient,
            route,
        }
    }

    /// This landing's `effect_planned` row: the route's path, or `skipped`
    /// when the recipient is gone or the effect has nothing to run.
    fn plan(&self, recipient_exists: bool) -> PlannedEffect {
        let effect = &self.effect;
        if !recipient_exists {
            return PlannedEffect::skipped(effect, REASON_NOT_REACHABLE);
        }
        let script = effect.script_name.is_some();
        let pulsing = effect.is_pulsing();
        let (path, reason) = match self.route {
            LandingRoute::User(reason) => (PATH_ROUTED_TO_USER, reason),
            LandingRoute::AllyArea(reason) => (PATH_ALLY_FANOUT, reason),
            LandingRoute::BeneficialTarget => {
                return PlannedEffect::landing(
                    effect,
                    false,
                    script,
                    pulsing,
                    REASON_BENEFICIAL_CAST,
                )
            }
        };
        if !script && !pulsing {
            return PlannedEffect::skipped(effect, REASON_NO_SCRIPT);
        }
        PlannedEffect {
            path,
            reason,
            ..PlannedEffect::landing(effect, false, script, pulsing, reason)
        }
    }
}

/// Land `landings` from `caster_id` (module docs). Returns how many pulsing
/// effects were registered. Each landing logs its `effect_planned` row
/// first (AB-T3).
pub(in crate::cell::abilities) async fn land_effects(
    caster_id: u32,
    ability_id: i32,
    landings: &[Landing],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    for landing in landings {
        let exists = space_mgr.get_entity(landing.recipient).is_some();
        landing.plan(exists).log(PlanIds::of(
            space_mgr,
            caster_id,
            landing.recipient,
            ability_id,
        ));
    }
    for landing in landings {
        let Some(script_name) = landing.effect.script_name.as_deref() else {
            continue;
        };
        let mut ctx = crate::cell::effects::EffectContext {
            source_id: caster_id,
            target_id: landing.recipient,
            effect: &landing.effect,
            space_mgr,
        };
        crate::cell::effects::dispatch_by_name(script_name, &mut ctx);
    }

    let now = Instant::now();
    let mut touched: Vec<u32> = Vec::new();
    for landing in landings {
        if !touched.contains(&landing.recipient) {
            touched.push(landing.recipient);
        }
    }
    for &entity_id in &touched {
        flush_stats(entity_id, tx, space_mgr).await;
        // A ledger script queued its duration timer; send it with the stats.
        crate::cell::effects::flush_stat_buff_timers(entity_id, now, tx, space_mgr).await;
    }

    let mut pulsing = 0usize;
    for landing in landings.iter().filter(|l| l.effect.is_pulsing()) {
        if crate::cell::effects::register_active_effect(
            space_mgr,
            landing.recipient,
            caster_id,
            &landing.effect,
            now,
            tx,
        )
        .await
        {
            pulsing += 1;
        }
    }
    pulsing
}

/// Send `entity_id`'s dirty stats to it and its witnesses.
async fn flush_stats(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let stat_update = match space_mgr.get_entity_mut(entity_id) {
        Some(t) => {
            let update = t.stats.serialize_dirty();
            t.stats.clear_dirty();
            update
        }
        None => Vec::new(),
    };
    if !stat_update.is_empty() {
        wire_ledger::send(
            entity_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            stat_update,
            WireRoute::SelfAndWitnesses,
            WireCtx::new("effect_routing"),
            tx,
            space_mgr,
        )
        .await;
    }
}
