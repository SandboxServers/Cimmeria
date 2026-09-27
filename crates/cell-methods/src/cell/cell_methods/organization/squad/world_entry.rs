//! Re-sending the squad on world entry.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_left, ON_ORGANIZATION_LEFT,
};

use super::fanout;

/// Called from `InitPlayerState`, which the base sends on every world
/// entry, the first login and every gate arrival alike.
///
/// Gate travel keeps the membership (it is keyed by `player_id`), but the
/// arrival resets the client's entities and re-creates the player, and the
/// cell entity is re-created with no `squad_id`. So:
///
/// 1. an `onOrganizationLeft` queued while the player was in transit (a
///    kick or a disband that could not reach them) is delivered now;
/// 2. a player still in a squad gets the whole squad again with
///    `aNewMember = 0`: [35], the roster [38], a [37] per other member with
///    their live entity id, and the loot mode [51]. The other members are
///    told nothing: their frames follow entity presence (ORG-E1 Q6);
/// 3. `CellEntity::squad_id` is re-stamped from the registry.
///
/// On a first login the player is in no squad (a disconnect removes them)
/// and nothing is sent.
pub async fn on_world_entry(
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some((squad_id, reason)) = space_mgr.squads.take_owed_left(player_id) {
        fanout::send(
            tx,
            entity_id,
            ON_ORGANIZATION_LEFT,
            build_on_organization_left(reason, squad_id),
        )
        .await;
    }
    let squad = space_mgr.squads.squad_for(player_id).cloned();
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.squad_id = squad.as_ref().map(|s| s.id());
    }
    let Some(squad) = squad else {
        return;
    };
    space_mgr.squads.note_entity(player_id, entity_id);
    tracing::debug!(
        target: "squad",
        event = "squad.world_entry_replay",
        player_id,
        entity_id,
        squad_id = squad.id(),
        "squad re-sent to a member entering a world"
    );
    fanout::send_whole_squad(tx, space_mgr, &squad, entity_id, player_id, false, &[]).await;
}
