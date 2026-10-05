//! Leaving a squad: CM 9 `organizationLeave` and a disconnect. The leader's
//! kick (base 0xD1 with a squad id) is `cimmeria-cell-interactions`'
//! `cell::organization::squad` (the base forwards it).

use cimmeria_cell_world::cell::squad::SquadResources;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{actor, fanout, feedback, reject};

/// CM 9 `organizationLeave` with a squad-range id (or one that routes
/// nowhere). The id must be the caller's own squad (CAT-M-04): routing on
/// the id range is not authorization.
#[tracing::instrument(
    name = "squad.leave",
    level = "info",
    target = "squad",
    skip_all,
    fields(entity_id = entity_id, squad_id = org_id)
)]
pub async fn leave(
    entity_id: u32,
    org_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut out = Outcome::new(
        Action::Leave,
        entity_id,
        tm::of_entity(space_mgr, entity_id),
    );
    out.squad_id = Some(org_id);
    let Some(me) = actor(space_mgr, entity_id) else {
        out.rejected(Reason::NotReady);
        return reject(tx, entity_id, org_id, feedback::NOT_READY).await;
    };
    let own = space_mgr.resources.squads().squad_of(me.player_id);
    if own != Some(org_id) {
        out.rejected(if own.is_some() {
            Reason::WrongSquad
        } else {
            Reason::NotInSquad
        });
        return reject(tx, entity_id, org_id, feedback::NOT_IN_THAT_SQUAD).await;
    }
    let d = space_mgr
        .resources
        .squads_mut()
        .leave(me.player_id)
        .expect("membership checked above");
    out.ok();
    fanout::announce_departure(tx, space_mgr, &d).await;
}

/// The `DisconnectEntity` arm: the player is gone (log off, crash, timeout
/// or a duplicate login). Remove them with `Logout`, promote or disband,
/// and drop every invite they sent or hold. Runs before the entity is torn
/// down, so the remaining members' [39] still names the departing entity.
///
/// A member in gate transit has no cell entity (the cell removed it and the
/// arrival has not re-created it), which is the state an aborted transfer
/// (`abandon_unspaced_session`) or a crash mid-transfer disconnects from.
/// They are found by their last entity id in the registry instead.
///
/// Not a player action, so no outcome row: the `member_left` transition
/// (`reason = logout`) records it.
pub async fn on_disconnect(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let live = space_mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .and_then(|e| e.player_id);
    let Some(player_id) = live.or_else(|| in_transit_member(space_mgr, entity_id)) else {
        return;
    };
    if let Some(d) = space_mgr.resources.squads_mut().remove_player(player_id) {
        fanout::announce_departure(tx, space_mgr, &d).await;
    }
}

/// The squad roster's name for `player_id`, for a log field. `None` when the
/// player is in no squad.
fn member_name(space_mgr: &SpaceManager, player_id: i32) -> Option<&str> {
    let squads = space_mgr.resources.squads();
    let squad = squads.squad(squads.squad_of(player_id)?)?;
    squad
        .members()
        .iter()
        .find(|m| m.player_id == player_id)
        .map(|m| m.name.as_str())
}

/// The squad member whose last entity was `entity_id`, when that entity is
/// gone (gate transit). `None`, logged, otherwise.
fn in_transit_member(space_mgr: &SpaceManager, entity_id: u32) -> Option<i32> {
    let Some(player_id) = space_mgr.resources.squads().member_by_entity(entity_id) else {
        // Not a squad member, or already removed: a full-exit logOff sends
        // DisconnectEntity and the socket close sends it again, so the
        // second one lands here for every player. Nothing to do.
        tracing::debug!(
            target: "squad",
            event = "squad.disconnect_no_member",
            entity_id, // nt:id-only the entity is already gone and in no squad, so nothing can name it
            "disconnect for an entity that is gone and in no squad"
        );
        return None;
    };
    if let Some(live) = space_mgr.player_entity_by_player_id(player_id) {
        // The member is live under another entity: the recorded id is
        // stale, and this disconnect is not theirs.
        tracing::warn!(
            target: "squad",
            event = "squad.disconnect_stale_entity",
            entity_id, // nt:id-only the recorded entity id is stale, so no live entity answers to it
            player_id,
            player_name = member_name(space_mgr, player_id),
            live_entity_id = live,
            live_entity_name = space_mgr.entity_label(live),
            reason = "stale_entity_id",
            "disconnect names a squad member's old entity id; member kept"
        );
        return None;
    }
    tracing::debug!(
        target: "squad",
        event = "squad.disconnect_in_transit",
        entity_id, // nt:id-only the entity is gone in gate transit, so nothing can name it
        player_id,
        player_name = member_name(space_mgr, player_id),
        "disconnect of a squad member in gate transit"
    );
    Some(player_id)
}
