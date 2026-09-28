//! The fire half of a deployable cast: place the object.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_cell_catalog::cell::spawner::DeployableSpec;
use cimmeria_cell_world::cell::deployables::{despawn_deployable, DeployableDespawnReason};
use cimmeria_common::Vector3;
use cimmeria_entity::abilities::AbilityDef;

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::use_ability::{play_ability_sequence, AbilityPhase, PhaseSequence};
use super::feedback::{send_refusal, DeployRefusal};
use super::launch::deploy_max_range;

/// Why a deployable whose warmup ended cannot be placed, if it cannot.
/// Checked before anything is spawned or replaced, so a refused fire never
/// costs the owner the object it already has.
fn fire_refusal(
    space_mgr: &SpaceManager,
    owner: u32,
    spec: DeployableSpec,
    ability: Option<&AbilityDef>,
    point: Option<Vector3>,
) -> Option<&'static str> {
    let Some(caster) = space_mgr.get_entity(owner) else {
        return Some("owner_not_found");
    };
    if combat::is_dead_state(caster.state_field) {
        return Some("owner_dead");
    }
    if space_mgr.get_entity_space_id(owner).is_none() {
        return Some("owner_not_found");
    }
    let Some(point) = point else {
        return Some("no_ground_point");
    };
    // The warmup interrupts a caster that moves, so this only trips on a
    // point staged for an earlier cast; re-checked all the same.
    if caster.position.distance_to(&point) > deploy_max_range(ability) {
        return Some("out_of_range");
    }
    if !space_mgr.spawn_templates.contains_key(&spec.template_id) {
        return Some("unknown_template");
    }
    None
}

/// Place a deployable whose warmup has completed (or that had none).
///
/// Runs in place of target resolution: the cast itself deals no damage;
/// the object's pulses do. The cooldown was charged at launch and stays
/// charged when the placement is refused here.
pub(in crate::cell::abilities) async fn fire_deploy(
    entity_id: u32,
    ability_id: i32,
    effect_seq: i32,
    spec: DeployableSpec,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let event_set_id = ability_def.as_ref().and_then(|d| d.event_set_id);
    let phase = |phase| PhaseSequence {
        phase,
        entity_id,
        ability_id,
        target_id: 0,
        instance_id: effect_seq,
        event_set_id,
    };
    let id = space_mgr.player_identity(entity_id);
    let point = space_mgr.deployables.take_staged(entity_id, ability_id);

    let refused = fire_refusal(space_mgr, entity_id, spec, ability_def.as_ref(), point);
    let placed = match (refused, point) {
        (None, Some(point)) => {
            let heading = space_mgr
                .get_entity(entity_id)
                .map_or(0.0, |e| e.direction.y);
            space_mgr
                .spawn_deployable(entity_id, spec, point, heading, Instant::now())
                .map_err(|e| e.reason())
        }
        (Some(reason), _) => Err(reason),
        (None, None) => Err("no_ground_point"),
    };
    let deployable = match placed {
        Ok(deployable) => deployable,
        Err(reason) => {
            // WARN: the launch validated all of this, the warmup's own
            // interrupts catch a dead or departed caster, and a missing
            // template is a seed defect.
            tracing::warn!(
                target: "deployables.lifecycle",
                event = "deploy_refused",
                decision_outcome = "deploy_refused",
                stage = "fire",
                entity_id,
                owner_id = entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                ability_id,
                template_id = spec.template_id,
                reason,
                "deployable refused after its warmup; nothing placed"
            );
            // The player watched the warmup: show it cancelled, then why.
            play_ability_sequence(phase(AbilityPhase::Interrupt), tx, space_mgr).await;
            send_refusal(
                entity_id,
                id,
                ability_id,
                DeployRefusal::PlacementFailed,
                tx,
            )
            .await;
            return;
        }
    };
    play_ability_sequence(phase(AbilityPhase::End), tx, space_mgr).await;

    // One active per owner per ability (`max_active`, 1 for 1012): the new
    // object replaces the oldest. Listed after the spawn, so the new one is
    // counted and is never the one removed.
    let mine = space_mgr.deployables.of_owner(entity_id, ability_id);
    let cap = spec.max_active.max(1) as usize;
    let excess = mine.len().saturating_sub(cap);
    for &old in mine.iter().filter(|&&d| d != deployable).take(excess) {
        // `despawn_deployable` logs the removal with its totals.
        let _outcome = despawn_deployable(
            space_mgr,
            old,
            DeployableDespawnReason::Replaced,
            "recast",
            tx,
        )
        .await;
    }
}
