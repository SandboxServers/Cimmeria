//! The launch half of a deployable cast: validate the client's ground
//! point, stage it, and launch the ordinary cast.

use tokio::sync::mpsc;

use cimmeria_cell_catalog::cell::spawner::DeployableSpec;
use cimmeria_common::Vector3;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::navigation::LineOfSight;

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::{occluder_probe, SpaceManager};
use super::super::use_ability::handle_use_ability;
use super::feedback::{send_refusal, DeployRefusal};

/// The server's fallback range when an ability's `max_range` is 0: the rule
/// the launch range check in `handle.rs` applies.
const DEFAULT_MAX_RANGE: f32 = 30.0;

/// How far above the ground point the line-of-sight ray ends, metres. A
/// ray to the exact surface grazes it; half a metre is where a thrown
/// object would be seen landing.
pub(super) const GROUND_SIGHT_HEIGHT: f32 = 0.5;

/// The range a deployable may be placed at: the ability's `max_range`, or
/// the server default for 0. The same number the launch range check uses.
pub(super) fn deploy_max_range(ability: Option<&AbilityDef>) -> f32 {
    ability.map_or(DEFAULT_MAX_RANGE, |d| {
        if d.max_range > 0 {
            d.max_range as f32
        } else {
            DEFAULT_MAX_RANGE
        }
    })
}

/// Validate `ground` for `caster` casting `ability`, and return the point to
/// place at: the client's point, with its Y moved onto the navmesh floor
/// when a mesh covers it. Pure: no wire, no log.
///
/// Checked in order: finite; the caster is in a space; within
/// [`deploy_max_range`] of the caster (3D, like every other range check);
/// in line of sight of the caster's eye where the world has an occluder (an
/// `Unknown` answer allows, as the fire-time gate does, D-NA14); on the
/// navmesh where the world enforces containment. In an advisory world or a
/// world with no mesh an off-mesh point is kept as sent.
pub(in crate::cell::abilities) fn validate_ground_point(
    space_mgr: &SpaceManager,
    caster_id: u32,
    ability: Option<&AbilityDef>,
    ground: [f32; 3],
) -> Result<Vector3, DeployRefusal> {
    if !ground.iter().all(|c| c.is_finite()) {
        return Err(DeployRefusal::NotFinite);
    }
    let Some(caster) = space_mgr.get_entity(caster_id) else {
        return Err(DeployRefusal::NotInSpace);
    };
    let Some(space_id) = space_mgr.get_entity_space_id(caster_id) else {
        return Err(DeployRefusal::NotInSpace);
    };
    let point = Vector3::new(ground[0], ground[1], ground[2]);
    if caster.position.distance_to(&point) > deploy_max_range(ability) {
        return Err(DeployRefusal::OutOfRange);
    }
    if let Some(occ) = space_mgr.occluder_of(caster_id) {
        let probe = occluder_probe(
            occ,
            caster.position,
            space_mgr.eye_height_of(caster),
            point,
            GROUND_SIGHT_HEIGHT,
        );
        if probe.result == LineOfSight::Blocked {
            return Err(DeployRefusal::NoLineOfSight);
        }
    }
    let navmesh = space_mgr
        .spaces
        .get(&space_id)
        .and_then(|s| s.navmesh.as_ref());
    match navmesh {
        Some(nav) if nav.is_point_valid(&point) => {
            let floor = nav
                .get_height_near(point.x, point.y, point.z)
                .unwrap_or(point.y);
            Ok(Vector3::new(point.x, floor, point.z))
        }
        Some(_) if space_mgr.enforces_navmesh_containment(space_id) => {
            Err(DeployRefusal::OffNavmesh)
        }
        _ => Ok(point),
    }
}

/// Log and answer a refused deployable press. A client chooses its ground
/// point and its timing freely, so the row is DEBUG (negative-logging
/// convention, client-input refusals).
async fn refuse(
    entity_id: u32,
    ability_id: i32,
    refusal: DeployRefusal,
    stage: &'static str,
    ground: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let caster_pos = space_mgr.get_entity(entity_id).map(|e| e.position);
    tracing::debug!(
        target: "deployables.lifecycle",
        event = "deploy_refused",
        decision_outcome = "deploy_refused",
        stage,
        entity_id,
        owner_id = entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        reason = refusal.reason(),
        ground = ?ground,
        caster_pos = ?caster_pos,
        "deployable cast refused; nothing charged"
    );
    send_refusal(entity_id, id, ability_id, refusal, tx).await;
}

/// `useAbilityOnGroundTarget` for a deployable ability: validate the point,
/// stage it, and launch the cast with no target. The cast's own launch
/// answers an untrained ability (167) and every later refusal.
pub(in crate::cell::abilities) async fn handle_deploy_on_ground(
    entity_id: u32,
    ability_id: i32,
    ground: [f32; 3],
    spec: DeployableSpec,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();
    let Some(caster) = space_mgr.get_entity(entity_id) else {
        return;
    };
    // A dead caster's press is dropped silently, as every launch drops it.
    if combat::is_dead_state(caster.state_field) {
        return;
    }
    let trained = caster.abilities.has_ability(ability_id);
    // The ordinary launch refuses these two silently; a deployable is a
    // button press like any other and gets an answer (project rule).
    let early = if !trained {
        None
    } else if caster.pending_cast.is_some() {
        Some(DeployRefusal::Busy)
    } else if caster.abilities.is_on_cooldown(ability_id) {
        Some(DeployRefusal::Cooldown)
    } else {
        None
    };
    if let Some(refusal) = early {
        refuse(
            entity_id, ability_id, refusal, "launch", ground, tx, space_mgr,
        )
        .await;
        return;
    }
    if trained {
        match validate_ground_point(space_mgr, entity_id, ability_def.as_ref(), ground) {
            Ok(point) => space_mgr.deployables.stage(entity_id, ability_id, point),
            Err(refusal) => {
                refuse(
                    entity_id, ability_id, refusal, "launch", ground, tx, space_mgr,
                )
                .await;
                return;
            }
        }
    }
    let id = space_mgr.player_identity(entity_id);
    let committed = handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
    if committed {
        tracing::debug!(
            target: "deployables.lifecycle",
            event = "deploy_launched",
            entity_id,
            owner_id = entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            ability_id,
            template_id = spec.template_id,
            "deployable cast committed; the object is placed when it fires"
        );
    } else {
        // The launch refused (and answered) it: drop the point so nothing
        // can fire on it later.
        space_mgr.deployables.clear_staged(entity_id);
    }
}

/// The launch's refusal for a deployable pressed without a staged ground
/// point (a plain `useAbility` naming it). Returns true, with feedback sent,
/// when it refuses. Checked after the common validation and before the
/// cooldown is charged.
pub(in crate::cell::abilities) async fn refuse_unstaged_launch(
    entity_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    if space_mgr
        .deployables
        .staged_for(entity_id, ability_id)
        .is_some()
    {
        return false;
    }
    refuse(
        entity_id,
        ability_id,
        DeployRefusal::NoGroundPoint,
        "launch",
        [f32::NAN; 3],
        tx,
        space_mgr,
    )
    .await;
    true
}
