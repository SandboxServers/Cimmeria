//! Following the owner: keep the pet in `Follow` on its owner, and teleport
//! it back beside the owner when it has fallen too far behind (D-PT07).
//!
//! The walking itself is the ordinary follow handler (`npc_ai::follow`) with
//! `follow_target_id = owner`; this file only arms it and decides the
//! teleport. The move itself is PT-02's
//! [`crate::cell::pets::on_owner_teleported`] with
//! `OwnerPath::PetLeftBehind`: the same grounded spot behind the owner, the
//! same immediate `EntityMoved` to the pet's witnesses, the same
//! `pets.lifecycle event=owner_teleported` row (from, to, distance,
//! grounding). It is a same-space position write, not `onPlayerTeleport`.

use cimmeria_entity::cell_entity::PetState;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;

use crate::cell::space_manager::SpaceManager;

use super::super::leash::policy::horizontal_distance;

/// Inner edge of the follow band, in world units (D-PT07).
pub(super) const PET_FOLLOW_MIN_DISTANCE: f32 = 2.0;
/// Outer edge of the follow band, in world units (D-PT07).
pub(super) const PET_FOLLOW_MAX_DISTANCE: f32 = 5.0;
/// Horizontal owner distance beyond which the pet teleports back (D-PT07).
/// Above the NPC perception radius, so an unrouted pet does not trail far.
pub(super) const PET_TELEPORT_DISTANCE: f32 = 40.0;
/// Height difference to the owner that counts as another floor. The same
/// "a storey, not a jump" value the follow handler replans at.
pub(super) const PET_FLOOR_BAND: f32 = 4.0;
/// At most one teleport per this interval (D-PT07, `SGWPet.lastTeleportTime`).
/// PT-02 stamps the same clock when the owner teleports.
pub(super) const PET_TELEPORT_MIN_INTERVAL: Duration = Duration::from_secs(5);

/// Whether a pet at `pet` is left behind by an owner at `owner`: past the
/// teleport distance, or on another floor. The label is the `reason` on the
/// teleport rows.
pub(super) fn left_behind(pet: &Vector3, owner: &Vector3) -> Option<&'static str> {
    if horizontal_distance(pet, owner) > PET_TELEPORT_DISTANCE {
        Some("distance")
    } else if (owner.y - pet.y).abs() > PET_FLOOR_BAND {
        Some("floor_band")
    } else {
        None
    }
}

/// Put the pet in `Follow` on its owner with the pet follow band. Writes the
/// state only when it changes, so a pet already following logs nothing.
pub(super) fn arm_follow(space_mgr: &mut SpaceManager, pet_id: u32, owner_id: u32) {
    let world = super::super::world_label(space_mgr, pet_id);
    let Some(pet) = space_mgr.get_entity_mut(pet_id) else {
        return;
    };
    pet.follow_target_id = Some(owner_id);
    pet.follow_min_distance = PET_FOLLOW_MIN_DISTANCE;
    pet.follow_max_distance = PET_FOLLOW_MAX_DISTANCE;
    let from = pet.ai_state();
    if from == AiState::Follow {
        return;
    }
    super::super::set_ai_state_on(
        pet,
        &world,
        AiState::Follow,
        super::super::AiTransitionReason::PetFollow,
    );
    let id = super::owner_identity(space_mgr, pet_id, owner_id);
    let names = space_mgr.entity_names(pet_id);
    tracing::debug!(
        target: "pets.ai",
        entity_id = pet_id,
        event = "follow_armed",
        decision_outcome = "pet_follow_armed",
        pet_id,
        owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_name = names.entity_name,
        pet_name = names.entity_name,
        template_id = names.template_id,
        template_name = names.template_name,
        owner_name = id.player_name,
        account_name = id.account_name,
        player_name = id.player_name,
        from = from.label(),
        "pet: following its owner"
    );
}

/// What [`teleport_if_left_behind`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TeleportCheck {
    /// Close enough and on the owner's floor.
    NotNeeded,
    /// Too far, but the last teleport was under [`PET_TELEPORT_MIN_INTERVAL`]
    /// ago; the pet walks instead.
    RateLimited,
    /// Moved beside the owner.
    Teleported,
}

/// Teleport the pet beside its owner when it is more than
/// [`PET_TELEPORT_DISTANCE`] away or on another floor, unless it teleported
/// (or was moved with its owner) under [`PET_TELEPORT_MIN_INTERVAL`] ago.
///
/// The move goes through `on_owner_teleported`, which places every live pet
/// of the owner, not just this one: an owner with two pets gets both beside
/// it, and both clocks are stamped.
pub(super) async fn teleport_if_left_behind(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    owner_id: u32,
    now: Instant,
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
) -> TeleportCheck {
    let (Some(pet), Some(owner)) = (space_mgr.get_entity(pet_id), space_mgr.get_entity(owner_id))
    else {
        return TeleportCheck::NotNeeded;
    };
    let Some(reason) = left_behind(&pet.position, &owner.position) else {
        return TeleportCheck::NotNeeded;
    };
    let distance = pet.position.distance_to(&owner.position);
    let dy = owner.position.y - pet.position.y;
    let last = pet
        .extensions
        .get::<PetState>()
        .and_then(|p| p.last_teleport_at);
    let id = super::owner_identity(space_mgr, pet_id, owner_id);
    let names = space_mgr.entity_names(pet_id);
    if let Some(last) =
        last.filter(|t| now.saturating_duration_since(*t) < PET_TELEPORT_MIN_INTERVAL)
    {
        tracing::debug!(
            target: "pets.ai",
            entity_id = pet_id,
            event = "teleport_rate_limited",
            decision_outcome = "pet_teleport_rate_limited",
            pet_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            entity_name = names.entity_name,
            pet_name = names.entity_name,
            template_id = names.template_id,
            template_name = names.template_name,
            owner_name = id.player_name,
            account_name = id.account_name,
            player_name = id.player_name,
            reason,
            distance,
            dy,
            since_last_ms = now.saturating_duration_since(last).as_millis() as u64,
            "pet: left behind, but teleported too recently -- walking"
        );
        return TeleportCheck::RateLimited;
    }
    // The decision row (why); PT-02's `owner_teleported` row is the move
    // (from, to, grounding).
    tracing::debug!(
        target: "pets.ai",
        entity_id = pet_id,
        event = "teleported",
        decision_outcome = "pet_teleported",
        pet_id,
        owner_id,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_name = names.entity_name,
        pet_name = names.entity_name,
        template_id = names.template_id,
        template_name = names.template_name,
        owner_name = id.player_name,
        account_name = id.account_name,
        player_name = id.player_name,
        reason,
        distance,
        dy,
        "pet: left behind, teleporting it beside its owner"
    );
    crate::cell::pets::on_owner_teleported(
        owner_id,
        crate::cell::pets::OwnerPath::PetLeftBehind,
        tx,
        space_mgr,
    )
    .await;
    TeleportCheck::Teleported
}
