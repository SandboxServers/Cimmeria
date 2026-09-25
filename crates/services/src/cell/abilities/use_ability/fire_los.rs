//! Fire-time line of sight for PLAYER abilities (NPC AI restoration NA31,
//! decision D-NA14).
//!
//! A player's targeted ability is refused when the world's collision-geometry
//! occluder (`data/spaces/<world>.occ`, NA27) says a wall stands between the
//! player's eye and the target's. The refusal is the client's own feedback:
//! `onErrorCode(SystemID 0, InstanceID ability_id, ErrorCodeID 39)`.
//! `ErrorStrings.pak` entry `_39` (`CONDITION_FEEDBACK_LOS`) is the only LoS
//! entry with authored text, "You do not have Line of Sight to your target";
//! `_40` (`CONDITION_FEEDBACK_NoLOS`) carries its moniker as placeholder text.
//!
//! **Where it applies.** Only a player attacker, only an ability aimed at
//! another entity (`target_type_id` not `TargetSelf` or `TargetGround`), and
//! only in a space with an occluder. Everything else is allowed as before:
//!
//! - no occluder: the navmesh ray reads furniture as walls (45% of its
//!   `Blocked` answers were false on NA16's Cellblock sweep), so it must not
//!   refuse a player's shot;
//! - `Unknown` (an eye off the occluder's trimmed grid): no information, so
//!   no refusal;
//! - NPC attackers: the fight tick already ran
//!   `SpaceManager::attack_line_of_sight` in the same tick, just before it
//!   calls `handle_use_ability`, so a second check here would be redundant.
//!
//! **Tolerance.** The server's picture is up to a tick behind what the
//! player aimed at. The client draws an NPC where the server last put it,
//! about one 100 ms movement tick late, and it moves the player's own avatar
//! before the server hears about it. A shot is also aimed at a body, not a
//! point. So a blocked eye-to-eye ray is re-tried against, in order: the
//! target one tick back along its velocity, the shooter one tick ahead along
//! its own, and the target's eye moved sideways by [`BODY_HALF_WIDTH`] each
//! way. One clear ray allows the shot. That also absorbs the occluder's own
//! error, which is almost all rays grazing within 0.1 m of a wall edge
//! (NA27: 1.24% of truly clear Cellblock pairs). None of the extra rays can
//! see round a real corner more than a body width or one tick of movement.

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::{AbilityDef, TARGET_GROUND, TARGET_SELF};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::navigation::{LineOfSight, LosProbe};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{occluder_probe, SpaceManager};

/// `CONDITION_FEEDBACK_LOS`: "You do not have Line of Sight to your target".
pub(crate) const CONDITION_FEEDBACK_LOS: u16 = 39;

/// One server movement tick, seconds: how far the client's picture of a
/// moving entity can differ from the server's.
pub(crate) const LAG_TOLERANCE_SECS: f32 = 0.1;

/// Half a humanoid's shoulder width, metres: the sideways reach of the
/// body-edge rays.
pub(crate) const BODY_HALF_WIDTH: f32 = 0.35;

/// Which ray allowed a shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClearRay {
    Eye,
    TargetLagged,
    ShooterLead,
    BodyEdge,
}

/// The fire-time verdict for one `useAbility`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FireLos {
    /// The check does not apply; the label says why.
    NotChecked(&'static str),
    /// A ray was clear.
    Clear(ClearRay),
    /// The eye-to-eye ray left the occluder's grid.
    Unknown,
    /// Every ray was blocked.
    Refused(Refusal),
}

/// The evidence for a refusal: the eye-to-eye ray.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Refusal {
    pub shooter_eye: f32,
    pub target_eye: f32,
    pub probe: LosProbe,
    pub rays: u32,
}

/// Decide the fire-time line of sight of `shooter_id` using `ability` on
/// `target_id`. Pure: no wire, no log.
pub(crate) fn fire_line_of_sight(
    space_mgr: &SpaceManager,
    shooter_id: u32,
    target_id: u32,
    ability: Option<&AbilityDef>,
) -> FireLos {
    let (Some(shooter), Some(target)) = (
        space_mgr.get_entity(shooter_id),
        space_mgr.get_entity(target_id),
    ) else {
        return FireLos::NotChecked("no_entity");
    };
    if !shooter.is_player {
        return FireLos::NotChecked("npc_shooter");
    }
    if shooter_id == target_id {
        return FireLos::NotChecked("self_target");
    }
    match ability.map(|d| d.target_type_id) {
        Some(TARGET_SELF) => return FireLos::NotChecked("self_ability"),
        Some(TARGET_GROUND) => return FireLos::NotChecked("ground_ability"),
        _ => {}
    }
    if space_mgr.get_entity_space_id(shooter_id) != space_mgr.get_entity_space_id(target_id) {
        return FireLos::NotChecked("other_space");
    }
    let Some(occ) = space_mgr.occluder_of(shooter_id) else {
        return FireLos::NotChecked("no_occluder");
    };
    let (se, te) = (
        space_mgr.eye_height_of(shooter),
        space_mgr.eye_height_of(target),
    );
    let probe = |a: Vector3, b: Vector3| occluder_probe(occ, a, se, b, te);

    let eye = probe(shooter.position, target.position);
    match eye.result {
        LineOfSight::Clear => return FireLos::Clear(ClearRay::Eye),
        LineOfSight::Unknown => return FireLos::Unknown,
        LineOfSight::Blocked => {}
    }
    let mut rays = 1;
    for (kind, from, to) in alternates(shooter, target) {
        rays += 1;
        if probe(from, to).result == LineOfSight::Clear {
            return FireLos::Clear(kind);
        }
    }
    FireLos::Refused(Refusal {
        shooter_eye: se,
        target_eye: te,
        probe: eye,
        rays,
    })
}

/// The tolerance rays, in the order they are tried (module docs).
fn alternates(shooter: &CellEntity, target: &CellEntity) -> Vec<(ClearRay, Vector3, Vector3)> {
    let (s, t) = (shooter.position, target.position);
    let mut out = Vec::with_capacity(4);
    if let Some(back) = along(t, target.velocity, -LAG_TOLERANCE_SECS) {
        out.push((ClearRay::TargetLagged, s, back));
    }
    if let Some(ahead) = along(s, shooter.velocity, LAG_TOLERANCE_SECS) {
        out.push((ClearRay::ShooterLead, ahead, t));
    }
    let (dx, dz) = (t.x - s.x, t.z - s.z);
    let len = (dx * dx + dz * dz).sqrt();
    if len > f32::EPSILON {
        let (px, pz) = (-dz / len * BODY_HALF_WIDTH, dx / len * BODY_HALF_WIDTH);
        out.push((ClearRay::BodyEdge, s, Vector3::new(t.x + px, t.y, t.z + pz)));
        out.push((ClearRay::BodyEdge, s, Vector3::new(t.x - px, t.y, t.z - pz)));
    }
    out
}

/// `p` moved by `v * secs` in XZ, or `None` when the entity is not moving.
fn along(p: Vector3, v: [f32; 3], secs: f32) -> Option<Vector3> {
    let speed = (v[0] * v[0] + v[2] * v[2]).sqrt();
    (speed > 0.05 && speed.is_finite())
        .then(|| Vector3::new(p.x + v[0] * secs, p.y, p.z + v[2] * secs))
}

/// The seven `onErrorCode` bytes for a line-of-sight refusal.
pub(crate) fn los_error_args(ability_id: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(0u8); // ERRORCODE_SYSTEM_Ability
    args.extend_from_slice(&ability_id.to_le_bytes());
    args.extend_from_slice(&CONDITION_FEEDBACK_LOS.to_le_bytes());
    args
}

/// Run the fire-time check and, on a refusal, log it, count it and send the
/// player the no-line-of-sight feedback. Returns `true` when the use must
/// stop here.
pub(crate) async fn refuse_without_line_of_sight(
    shooter_id: u32,
    ability_id: i32,
    target_id: u32,
    ability: Option<&AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let FireLos::Refused(r) = fire_line_of_sight(space_mgr, shooter_id, target_id, ability) else {
        return false;
    };
    let world = crate::cell::service::npc_ai::world_label(space_mgr, shooter_id);
    let (from_xyz, to_xyz) = match (
        space_mgr.get_entity(shooter_id),
        space_mgr.get_entity(target_id),
    ) {
        (Some(s), Some(t)) => (
            [s.position.x, s.position.y, s.position.z],
            [t.position.x, t.position.y, t.position.z],
        ),
        _ => ([0.0; 3], [0.0; 3]),
    };
    tracing::debug!(
        target: "abilities",
        event = "los_refused",
        entity_id = shooter_id,
        ability_id,
        target_id,
        world = %world,
        source = "occluder",
        occluder_hash = space_mgr.occluder_of(shooter_id).map(|o| o.short_hash()),
        shooter_eye = r.shooter_eye,
        target_eye = r.target_eye,
        from_xyz = ?from_xyz,
        to_xyz = ?to_xyz,
        ray_from = ?r.probe.from,
        ray_to = ?r.probe.to,
        hit_xyz = ?r.probe.hit,
        rays = r.rays,
        error_code = CONDITION_FEEDBACK_LOS,
        "useAbility refused: no line of sight to the target (onErrorCode 39)"
    );
    cimmeria_observability::counter!(
        "abilities_los_refused_total",
        "world" => world,
    );
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: shooter_id,
            method_index: crate::mercury::method_idx::ON_ERROR_CODE,
            args: los_error_args(ability_id),
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "abilities",
            event = "los_refused_send_failed",
            entity_id = shooter_id,
            ability_id,
            "useAbility: the no-line-of-sight onErrorCode could not be queued (base channel closed)"
        );
    }
    true
}
