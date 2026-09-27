//! Leaving a squad: CM 9 `organizationLeave`, the leader's kick (base 0xD1
//! with a squad id), and a disconnect.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::{Departure, KickReject};

use cimmeria_wire::base::organization::ORGANIZATION_KICK;
use cimmeria_wire::cell::cell_methods::organization::LEAVE;

use super::{actor, fanout, feedback, forwarded_actor, reject};

fn log_departure(d: &Departure, event: &'static str) {
    tracing::info!(
        target: "squad",
        event,
        player_id = d.departed.player_id,
        squad_id = d.squad_id,
        reason = d.reason.as_u8(),
        remaining = d.remaining.len(),
        disbanded = d.disbanded,
        "player left a squad"
    );
}

/// CM 9 `organizationLeave` with a squad-range id (or one that routes
/// nowhere). The id must be the caller's own squad (CAT-M-04): routing on
/// the id range is not authorization.
pub async fn leave(
    entity_id: u32,
    org_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(me) = actor(space_mgr, entity_id) else {
        return reject(tx, entity_id, LEAVE, org_id, feedback::NOT_READY).await;
    };
    let own = space_mgr.squads.squad_of(me.player_id);
    if own != Some(org_id) {
        tracing::warn!(
            target: "squad",
            event = "squad.leave_rejected",
            player_id = me.player_id,
            entity_id,
            org_id,
            own_squad_id = own,
            reason = "foreign_squad_id",
            "squad leave names a squad the caller is not in"
        );
        return reject(tx, entity_id, LEAVE, org_id, feedback::NOT_IN_THAT_SQUAD).await;
    }
    let d = space_mgr
        .squads
        .leave(me.player_id)
        .expect("membership checked above");
    log_departure(&d, "squad.left");
    fanout::announce_departure(tx, space_mgr, &d).await;
}

/// `organizationKick(org_id, target_name)` with a squad-range id, forwarded
/// by the base with the actor's ids. Only the leader of that squad may
/// kick, the target is found by name among its members (so a member in
/// gate transit can be kicked too), and nobody kicks themselves.
pub async fn handle_kick(
    player_id: i32,
    entity_id: u32,
    org_id: i32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if forwarded_actor(space_mgr, player_id, entity_id).is_none() {
        return;
    }
    match space_mgr.squads.kick(player_id, org_id, target_name) {
        Ok(d) => {
            log_departure(&d, "squad.kicked");
            fanout::announce_departure(tx, space_mgr, &d).await;
        }
        Err(r) => {
            // A foreign id is a forged call; the rest are ordinary refusals.
            if r == KickReject::NotInThatSquad {
                tracing::warn!(
                    target: "squad",
                    event = "squad.kick_rejected",
                    player_id,
                    entity_id,
                    org_id,
                    reason = r.reason(),
                    "squad kick names a squad the caller is not in"
                );
            } else {
                tracing::debug!(
                    target: "squad",
                    event = "squad.kick_rejected",
                    player_id,
                    entity_id,
                    org_id,
                    reason = r.reason(),
                    "squad kick refused"
                );
            }
            let text = feedback::kick_rejected(r, target_name);
            reject(tx, entity_id, u16::from(ORGANIZATION_KICK), org_id, &text).await;
        }
    }
}

/// The `DisconnectEntity` arm: the player is gone (log off, crash, timeout
/// or a duplicate login). Remove them with `Logout`, promote or disband,
/// and drop every invite they sent or hold. Runs before the entity is torn
/// down, so the remaining members' [39] still names the departing entity.
pub async fn on_disconnect(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(player_id) = space_mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .and_then(|e| e.player_id)
    else {
        return;
    };
    if let Some(d) = space_mgr.squads.remove_player(player_id) {
        log_departure(&d, "squad.left");
        fanout::announce_departure(tx, space_mgr, &d).await;
    }
}
