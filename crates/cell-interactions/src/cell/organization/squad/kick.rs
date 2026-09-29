//! The leader's squad kick: `organizationKick` (base 0xD1) with a squad id,
//! forwarded by the base.

use cimmeria_cell_world::cell::squad::SquadResources;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{fanout, feedback, forwarded_actor, reject};

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
        .resources
        .squads()
        .squad(org_id)
        .and_then(|s| s.members().iter().find(|m| m.name == target_name))
        .map(|m| tm::of_player(space_mgr, m.player_id));
    let in_a_squad = space_mgr.resources.squads().squad_of(player_id).is_some();
    match space_mgr
        .resources
        .squads_mut()
        .kick(player_id, org_id, target_name)
    {
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
