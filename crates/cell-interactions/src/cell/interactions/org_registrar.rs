//! Organization registrars (ORG-05): the NPCs that open the Team and
//! Command founding dialogs.
//!
//! A registrar is keyed on seed data only: the `INT_Organization` bit on
//! `entity_templates.interaction_type` (which also gives the client its
//! organization cursor) **and** the registrar's static interaction set,
//! 7447 for Team or 7448 for Command. Those two ids are the 2009 server's
//! `INTERACTION_OrganizationRegisterTeam` / `...Command` interaction set
//! maps (`deprecated/python/common/Constants.py:47-48`), so the seed names
//! the type the way the original data did. Both are required: the bit alone
//! names no type, and `static_interaction_sets` alone is not kept in step
//! with the bits elsewhere in the seed (`spawn.rs`,
//! `static_interaction_for_flags`).
//!
//! A right-click in range forwards `OrgCellToBase::RegistrarOpen` to the
//! base, which checks D-ORG18 eligibility (the cell does not know a
//! character's Teams and Commands). An eligible player comes back as
//! `OrgBaseToCell::RegistrarEligible`, and
//! `cell_methods::organization::creation` then records the pending
//! creation and sends `launchOrganizationCreation` [135]. An ineligible
//! player gets a line from the base instead of a dialog.
//!
//! Telemetry: the `org.registrar_open` span, and the outcome row for the
//! refusals decided here (`too_far`, `base_unreachable`); the base and the
//! creation handlers write the others, so each click has exactly one row.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::org_creation::count_org_action;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::interaction_flags::INT_ORGANIZATION;
use cimmeria_entity::organization::OrgType;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use super::dispatch::{interact_range, InteractRangeFail, MAX_INTERACT_DISTANCE};
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;

/// `INTERACTION_OrganizationRegisterTeam`: the static interaction set a
/// Team registrar carries.
pub const REGISTRAR_SET_TEAM: i32 = 7447;
/// `INTERACTION_OrganizationRegisterCommand`: the static interaction set a
/// Command registrar carries.
pub const REGISTRAR_SET_COMMAND: i32 = 7448;

/// The line a click from out of range gets.
pub const REGISTRAR_TOO_FAR_TEXT: &str = "You are too far away from the registrar.";
/// The line when the request cannot reach the base.
pub const REGISTRAR_UNAVAILABLE_TEXT: &str =
    "The registrar cannot help you right now. Try again later.";

/// The organization type a registrar founds, from its seed data: the
/// `INT_Organization` bit plus exactly one of the two registrar sets.
/// `None` for anything else, including a template carrying both sets.
pub fn registrar_type(interaction_flags: i64, static_sets: &[i32]) -> Option<OrgType> {
    if interaction_flags & INT_ORGANIZATION == 0 {
        return None;
    }
    let team = static_sets.contains(&REGISTRAR_SET_TEAM);
    let command = static_sets.contains(&REGISTRAR_SET_COMMAND);
    match (team, command) {
        (true, false) => Some(OrgType::Team),
        (false, true) => Some(OrgType::Command),
        _ => None,
    }
}

/// The registrar type of entity `target`, if it is one.
fn target_registrar(space_mgr: &SpaceManager, target: u32) -> Option<OrgType> {
    space_mgr
        .get_entity(target)
        .and_then(|t| registrar_type(t.interaction_type_flags, &t.static_interaction_sets))
}

/// One `org.registrar_open` refusal row, decided on the cell.
fn rejected(
    actor: PlayerIdentity,
    entity_id: u32,
    npc_entity_id: u32,
    org_type: OrgType,
    reason: &'static str,
    distance: Option<f32>,
) {
    tracing::info!(
        target: "org",
        event = "org.registrar_open",
        outcome = "rejected",
        reason,
        account_id = actor.account_id,
        player_id = actor.player_id,
        entity_id,
        npc_entity_id,
        org_type = org_type.name(),
        distance,
        max_distance = distance.map(|_| MAX_INTERACT_DISTANCE),
        "organization registrar_open rejected"
    );
    count_org_action("registrar_open", "rejected", reason);
}

async fn line(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.feedback_send_failed",
            reason = "cell_to_base_closed",
            entity_id,
            "registrar feedback could not be queued"
        );
    }
}

/// If `target_entity_id` is an organization registrar, ask the base whether
/// the player may found one and return `true`; otherwise `false`, so the
/// caller falls through to the generic dispatch.
///
/// The caller has distance-checked the target, but not that it shares the
/// player's space (`get_entity` looks in every space), so the range rule
/// is applied again here.
#[tracing::instrument(
    name = "org.registrar_open",
    level = "info",
    skip_all,
    fields(entity_id, npc_entity_id = target_entity_id)
)]
pub(crate) async fn try_open_org_registrar(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(org_type) = target_registrar(space_mgr, target_entity_id) else {
        return false;
    };
    let actor = space_mgr.player_identity(entity_id);
    if let Err(fail) = interact_range(entity_id, target_entity_id, space_mgr) {
        let distance = match fail {
            InteractRangeFail::TooFar { dist } => Some(dist),
            _ => None,
        };
        rejected(
            actor,
            entity_id,
            target_entity_id,
            org_type,
            "too_far",
            distance,
        );
        line(entity_id, REGISTRAR_TOO_FAR_TEXT, tx).await;
        return true;
    }
    let Some(player_id) = actor.player_id else {
        // Not a fully initialised player: nothing to found for.
        rejected(
            actor,
            entity_id,
            target_entity_id,
            org_type,
            "not_ready",
            None,
        );
        line(entity_id, REGISTRAR_UNAVAILABLE_TEXT, tx).await;
        return true;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::RegistrarOpen {
        player_id,
        entity_id,
        npc_entity_id: target_entity_id,
        org_type,
    });
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.registrar_forward_failed",
            reason = "cell_to_base_closed",
            account_id = actor.account_id,
            player_id,
            entity_id,
            "registrar request could not reach the base"
        );
        rejected(
            actor,
            entity_id,
            target_entity_id,
            org_type,
            "base_unreachable",
            None,
        );
        line(entity_id, REGISTRAR_UNAVAILABLE_TEXT, tx).await;
        return true;
    }
    tracing::debug!(
        target: "org",
        event = "org.registrar_forwarded",
        account_id = actor.account_id,
        player_id,
        entity_id,
        npc_entity_id = target_entity_id,
        org_type = org_type.name(),
        "registrar click forwarded to the base for the eligibility check"
    );
    true
}

/// A registrar right-clicked from beyond the interact range (or from
/// another space), which the interact range gates refuse before any
/// dispatch: write the row and the line, so the click is not silent.
/// Returns whether the target was a registrar.
pub async fn reject_registrar_out_of_range(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(org_type) = target_registrar(space_mgr, target_entity_id) else {
        return false;
    };
    let distance = match interact_range(entity_id, target_entity_id, space_mgr) {
        Err(InteractRangeFail::TooFar { dist }) => Some(dist),
        _ => None,
    };
    rejected(
        space_mgr.player_identity(entity_id),
        entity_id,
        target_entity_id,
        org_type,
        "too_far",
        distance,
    );
    line(entity_id, REGISTRAR_TOO_FAR_TEXT, tx).await;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::interaction_flags::INT_BANKER;

    #[test]
    fn the_type_needs_the_bit_and_exactly_one_registrar_set() {
        assert_eq!(
            registrar_type(INT_ORGANIZATION, &[7447]),
            Some(OrgType::Team)
        );
        assert_eq!(
            registrar_type(INT_ORGANIZATION | INT_BANKER, &[1, 7448]),
            Some(OrgType::Command)
        );
        assert_eq!(registrar_type(0, &[7447]), None, "no bit, no registrar");
        assert_eq!(registrar_type(INT_ORGANIZATION, &[]), None, "no type");
        assert_eq!(registrar_type(INT_ORGANIZATION, &[7447, 7448]), None);
    }

    /// The set ids are the 2009 constants, not a local choice.
    #[test]
    fn registrar_sets_are_the_legacy_interaction_set_maps() {
        assert_eq!((REGISTRAR_SET_TEAM, REGISTRAR_SET_COMMAND), (7447, 7448));
        assert_eq!(INT_ORGANIZATION, 64);
    }
}
