//! The bank bit and the rules for an item entering a Team or Command vault
//! (bank-vault BV-07; D-BV08, D-BV12).

use cimmeria_entity::inventory::INV_BANK;
use cimmeria_entity::known_names;

use super::super::super::grant::item_allows_container;
use super::super::bank_rules::{is_mission_item, MoveRefusal};
use super::super::container_policy::MoveEnd;
use super::super::{MoveCtx, MoveRequest};
use super::refusal::OrgMoveRefusal;
use super::MoveTx;
use crate::base::world_entry::methods::inventory::org_vault::access::{BankBit, OrgVaultActor};

/// The actor's rank holds `bit`, or the move is refused.
pub(super) fn has_bit(actor: &OrgVaultActor, bit: BankBit) -> Result<(), OrgMoveRefusal> {
    if actor.access.permissions().contains(bit.permission()) {
        Ok(())
    } else {
        Err(OrgMoveRefusal::MissingPermission(bit))
    }
}

/// The rules for an item entering a shared vault: never a bound item,
/// never a mission item, and only what the personal vault (17) would take.
/// `from_container` is where it sits now (the mission bag marks a mission
/// item). Fails closed when the mission lookup errors.
pub(super) async fn entering_vault(
    tx: &mut MoveTx,
    req: &MoveRequest,
    ctx: &MoveCtx<'_>,
    type_id: i32,
    bound: bool,
    from_container: i32,
) -> Result<(), OrgMoveRefusal> {
    if bound {
        return Err(OrgMoveRefusal::BoundItem);
    }
    match is_mission_item(tx, type_id, from_container).await {
        Ok(false) => {}
        Ok(true) => return Err(OrgMoveRefusal::Shared(MoveRefusal::MissionItem)),
        Err(e) => {
            tracing::error!(
                target: "bank",
                player_id = req.player_id,
                player_name = known_names::player_name(req.player_id),
                item_id = req.item_id,
                item_name = cimmeria_names::book().item(type_id),
                "org vault move: mission-item lookup failed, refusing: {e}"
            );
            return Err(OrgMoveRefusal::Shared(MoveRefusal::MissionItem));
        }
    }
    if !item_allows_container(ctx.pool, type_id, INV_BANK).await {
        return Err(OrgMoveRefusal::Shared(MoveRefusal::ItemNotAllowed {
            end: MoveEnd::Target,
        }));
    }
    Ok(())
}
