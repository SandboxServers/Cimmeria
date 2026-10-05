//! The launch half of an owner-pet cast: refuse the press, with feedback and
//! no cooldown, when there is no pet to act on (pets PT-08).

use cimmeria_entity::cell_entity::PetState;
use tokio::sync::mpsc;

use cimmeria_cell_world::cell::pets::OwnerPetRefusal;

use super::super::super::super::messages::CellToBaseMsg;
use super::super::super::super::space_manager::SpaceManager;
use super::feedback::{send_refusal, DOOMED_TEXT, FEEDBACK_EFFECT_ON_ENTITY};

/// Why an owner-pet cast was refused: the `reason` of the
/// `owner_ability_refused` row, the `onErrorCode` code and the chat line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// No pet to act on (see [`OwnerPetRefusal`]).
    Pet(OwnerPetRefusal),
    /// To The Death is already running on the pet. Python's refresh would
    /// restart the 60 s timer, and with a 30 s cooldown a re-cast would keep
    /// the +400 Accuracy up forever with no death (a deliberate deviation).
    PetDoomed,
}

impl Refusal {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::Pet(r) => r.reason(),
            Self::PetDoomed => "pet_doomed",
        }
    }

    pub(super) fn code(self) -> u16 {
        match self {
            Self::Pet(r) => r.error_code(),
            Self::PetDoomed => FEEDBACK_EFFECT_ON_ENTITY,
        }
    }

    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Pet(r) => r.feedback_text(),
            Self::PetDoomed => DOOMED_TEXT,
        }
    }

    /// A client can press the ability with no pet, a dead pet or a doomed
    /// pet at will (DEBUG); a reused owner id is a server-side window no
    /// client controls (WARN). Negative-logging convention.
    pub(super) fn is_warn(self) -> bool {
        matches!(self, Self::Pet(OwnerPetRefusal::OwnerIdentityMismatch))
    }
}

/// Whether `ability_id` arms To The Death (one of its effects runs
/// `PetDeathTimer`).
pub(super) fn dooms_pet(space_mgr: &SpaceManager, ability_id: i32) -> bool {
    space_mgr.ability_defs.get(&ability_id).is_some_and(|def| {
        def.effect_ids.iter().any(|eid| {
            space_mgr
                .effect_defs
                .get(eid)
                .and_then(|e| e.script_name.as_deref())
                == Some("PetDeathTimer")
        })
    })
}

/// The pets `owner`'s cast of `ability_id` acts on, or why it may not.
pub(super) fn resolve(
    space_mgr: &SpaceManager,
    owner: u32,
    ability_id: i32,
) -> Result<Vec<u32>, Refusal> {
    let pets = space_mgr.owner_pet_targets(owner).map_err(Refusal::Pet)?;
    if dooms_pet(space_mgr, ability_id)
        && pets.iter().any(|&p| {
            space_mgr
                .get_entity(p)
                .and_then(|e| e.extensions.get::<PetState>())
                .is_some_and(|s| s.doomed_at.is_some())
        })
    {
        return Err(Refusal::PetDoomed);
    }
    Ok(pets)
}

/// Log a refusal (`owner_ability_refused`, `stage`) and answer the owner.
pub(super) async fn refuse(
    owner: u32,
    ability_id: i32,
    refusal: Refusal,
    stage: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(owner);
    let pet_ids = space_mgr.pets.pets_of(owner);
    if refusal.is_warn() {
        tracing::warn!(
            target: "pets.buff",
            event = "owner_ability_refused",
            decision_outcome = "owner_ability_refused",
            stage,
            entity_id = owner,
            entity_name = space_mgr.entity_label(owner),
            owner_id = owner,
            owner_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            ability_id,
            ability_name = cimmeria_names::book().ability(ability_id),
            pet_ids = ?pet_ids,
            reason = refusal.reason(),
            error_code = refusal.code(),
            error_name = cimmeria_names::book().error_code(refusal.code()),
            "owner ability on a pet refused"
        );
    } else {
        tracing::debug!(
            target: "pets.buff",
            event = "owner_ability_refused",
            decision_outcome = "owner_ability_refused",
            stage,
            entity_id = owner,
            entity_name = space_mgr.entity_label(owner),
            owner_id = owner,
            owner_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            ability_id,
            ability_name = cimmeria_names::book().ability(ability_id),
            pet_ids = ?pet_ids,
            reason = refusal.reason(),
            error_code = refusal.code(),
            error_name = cimmeria_names::book().error_code(refusal.code()),
            "owner ability on a pet refused"
        );
    }
    send_refusal(owner, id, ability_id, refusal.code(), refusal.text(), tx).await;
}

/// Launch-time check, after the common validation and before the cooldown
/// is charged. Returns true (feedback sent, nothing charged) when the press
/// is refused.
pub(in crate::cell::abilities::use_ability) async fn refuse_owner_pet_launch(
    owner: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    match resolve(space_mgr, owner, ability_id) {
        Ok(_) => false,
        Err(refusal) => {
            refuse(owner, ability_id, refusal, "launch", tx, space_mgr).await;
            true
        }
    }
}
