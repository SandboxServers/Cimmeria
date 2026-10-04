//! What a player is told when a deployable cast is refused.
//!
//! Every refusal sends `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id,
//! code)` and a `CHAN_FEEDBACK` chat line, both to the caster only. The chat
//! line is the route known to render: the shipped client has no Lua
//! consumer for `onErrorCode` (AT-E1).

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::super::messages::CellToBaseMsg;
use crate::cell::abilities::wire_ledger::{self, WireCtx};

/// `CONDITION_FEEDBACK_InvalidEntity`: the generic refusal.
pub(super) const CONDITION_FEEDBACK_INVALID_ENTITY: u16 = 0;
/// `CONDITION_FEEDBACK_LOS`.
pub(super) const CONDITION_FEEDBACK_LOS: u16 = 39;
/// `CONDITION_FEEDBACK_OutsideWeaponRange`, the launch range check's code.
pub(super) const CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE: u16 = 42;
/// `CONDITION_FEEDBACK_WeaponCooldownNotReady`, the pet bar's busy code.
pub(super) const CONDITION_FEEDBACK_COOLDOWN_NOT_READY: u16 = 99;

/// Why a deployable cast was refused. `reason()` is the log field, `code()`
/// and `text()` what the player is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::abilities) enum DeployRefusal {
    /// A coordinate is NaN or infinite.
    NotFinite,
    /// The point is beyond the ability's `max_range` of the caster.
    OutOfRange,
    /// The world's occluder puts a wall between the caster's eye and the
    /// point.
    NoLineOfSight,
    /// The point is off the navmesh in a world that enforces containment.
    OffNavmesh,
    /// The caster is in no space.
    NotInSpace,
    /// The ability is cooling down.
    Cooldown,
    /// Another cast is warming up.
    Busy,
    /// A plain `useAbility` named the deployable: no ground point.
    NoGroundPoint,
    /// The fire could not place the object.
    PlacementFailed,
}

impl DeployRefusal {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::NotFinite => "not_finite",
            Self::OutOfRange => "out_of_range",
            Self::NoLineOfSight => "no_line_of_sight",
            Self::OffNavmesh => "off_navmesh",
            Self::NotInSpace => "not_in_space",
            Self::Cooldown => "cooldown",
            Self::Busy => "busy",
            Self::NoGroundPoint => "no_ground_point",
            Self::PlacementFailed => "placement_failed",
        }
    }

    pub(super) fn code(self) -> u16 {
        match self {
            Self::OutOfRange => CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE,
            Self::NoLineOfSight => CONDITION_FEEDBACK_LOS,
            Self::Cooldown | Self::Busy => CONDITION_FEEDBACK_COOLDOWN_NOT_READY,
            Self::NotFinite
            | Self::OffNavmesh
            | Self::NotInSpace
            | Self::NoGroundPoint
            | Self::PlacementFailed => CONDITION_FEEDBACK_INVALID_ENTITY,
        }
    }

    pub(super) fn text(self) -> &'static str {
        match self {
            Self::OutOfRange => "That spot is out of range.",
            Self::NoLineOfSight => "You cannot see that spot.",
            Self::NotFinite | Self::OffNavmesh | Self::NotInSpace => "You cannot place that there.",
            Self::Cooldown => "That deployable is not ready yet.",
            Self::Busy => "You are already using an ability.",
            Self::NoGroundPoint => "Choose a spot on the ground to place that.",
            Self::PlacementFailed => "Your deployable could not be placed.",
        }
    }
}

/// Send the refusal's `onErrorCode` and chat line to `entity_id`.
pub(super) async fn send_refusal(
    entity_id: u32,
    id: PlayerIdentity,
    ability_id: i32,
    refusal: DeployRefusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut err = Vec::with_capacity(7);
    err.push(0u8); // SystemID: ERRORCODE_SYSTEM_Ability
    err.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    err.extend_from_slice(&refusal.code().to_le_bytes());
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, refusal.text());
    for (method_index, args) in [
        (crate::mercury::method_idx::ON_ERROR_CODE, err),
        (crate::mercury::method_idx::ON_PLAYER_COMMUNICATION, chat),
    ] {
        let row = wire_ledger::prepare(method_index, &args);
        if tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await
            .is_err()
        {
            crate::cell::abilities::metrics::wire_send_failed(
                crate::cell::abilities::metrics::WireMessage::from_method(method_index),
                crate::cell::abilities::metrics::UNKNOWN_WORLD,
            );
            tracing::warn!(
                target: "deployables.lifecycle",
                event = "refusal_feedback_send_failed",
                decision_outcome = "refusal_feedback_send_failed",
                entity_id,
                owner_id = entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                ability_id,
                method_index,
                method_name = cimmeria_wire::names::player_client_method(method_index),
                reason = refusal.reason(),
                "deployable refusal feedback could not be queued (base channel closed)"
            );
        } else {
            row.sent_to_owner_as(
                id,
                entity_id,
                WireCtx::new("deployable").reason(refusal.reason()),
            );
        }
    }
}
