//! Leaving a squad: CM 9 `organizationLeave`, the leader's kick (base 0xD1
//! with a squad id), and a disconnect.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{actor, fanout, feedback, forwarded_actor, reject};

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
    let own = space_mgr.squads.squad_of(me.player_id);
    if own != Some(org_id) {
        out.rejected(if own.is_some() {
            Reason::WrongSquad
        } else {
            Reason::NotInSquad
        });
        return reject(tx, entity_id, org_id, feedback::NOT_IN_THAT_SQUAD).await;
    }
    let d = space_mgr
        .squads
        .leave(me.player_id)
        .expect("membership checked above");
    out.ok();
    fanout::announce_departure(tx, space_mgr, &d).await;
}

/// `organizationKick(org_id, target_name)` with a squad-range id, forwarded
/// by the base with the actor's ids. Only the leader of that squad may
/// kick, the target is found by name among its members (so a member in
/// gate transit can be kicked too), and nobody kicks themselves.
#[tracing::instrument(
    name = "squad.kick",
    level = "info",
    target = "squad",
    skip_all,
    fields(player_id = player_id, entity_id = entity_id, squad_id = org_id)
)]
pub async fn handle_kick(
    player_id: i32,
    entity_id: u32,
    org_id: i32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if forwarded_actor(space_mgr, player_id, entity_id).is_none() {
        let mut out = Outcome::new(Action::Kick, entity_id, tm::claimed(player_id));
        out.squad_id = Some(org_id);
        return out.rejected(Reason::ActorMismatch);
    }
    let mut out = Outcome::new(Action::Kick, entity_id, tm::of_entity(space_mgr, entity_id));
    out.squad_id = Some(org_id);
    // The target, when the name is one of that squad's members.
    out.target = space_mgr
        .squads
        .squad(org_id)
        .and_then(|s| s.members().iter().find(|m| m.name == target_name))
        .map(|m| tm::of_player(space_mgr, m.player_id));
    let in_a_squad = space_mgr.squads.squad_of(player_id).is_some();
    match space_mgr.squads.kick(player_id, org_id, target_name) {
        Ok(d) => {
            out.ok();
            fanout::announce_departure(tx, space_mgr, &d).await;
        }
        Err(r) => {
            out.rejected(Reason::from_kick(r, in_a_squad));
            let text = feedback::kick_rejected(r, target_name);
            reject(tx, entity_id, org_id, &text).await;
        }
    }
}

/// The `DisconnectEntity` arm: the player is gone (log off, crash, timeout
/// or a duplicate login). Remove them with `Logout`, promote or disband,
/// and drop every invite they sent or hold. Runs before the entity is torn
/// down, so the remaining members' [39] still names the departing entity.
///
/// Not a player action, so no outcome row: the `member_left` transition
/// (`reason = logout`) records it.
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
        fanout::announce_departure(tx, space_mgr, &d).await;
    }
}
