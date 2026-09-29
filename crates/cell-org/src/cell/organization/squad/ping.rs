//! CM 10 `BroadcastMinimapPing` for squads (ORG-04).
//!
//! The ping is validated and logged, never fanned out: ORG-E1 Q3 found no
//! client method that shows another member's ping (`receivedMinimapPing`
//! is server-internal), and the pinging client draws its own ping locally
//! (`createLocalMinimapPing`). So an accepted ping sends nothing.
//!
//! Refusals: a caller in no squad, or naming a squad they are not in, gets
//! the usual `onErrorCode` plus a feedback line. A ping over the one per
//! second limit gets nothing: the client already drew it, nobody else would
//! have seen it, and a line per click would flood the chat window of a
//! player holding the ping key.

use cimmeria_cell_world::cell::squad::SquadResources;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::PingReject;

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{feedback, reject};

/// CM 10 with a squad-range (or unroutable) organization id.
#[tracing::instrument(
    name = "squad.ping",
    level = "info",
    target = "squad",
    skip_all,
    fields(entity_id = entity_id, squad_id = org_id)
)]
pub async fn broadcast_minimap_ping(
    entity_id: u32,
    org_id: i32,
    location: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    ping_at(entity_id, org_id, location, tx, space_mgr, Instant::now()).await;
}

/// [`broadcast_minimap_ping`] on an explicit clock, so the rate limit can
/// be stepped exactly.
pub(in crate::cell::organization) async fn ping_at(
    entity_id: u32,
    org_id: i32,
    location: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    now: Instant,
) {
    let mut out = Outcome::new(Action::Ping, entity_id, tm::of_entity(space_mgr, entity_id));
    out.squad_id = Some(org_id);
    out.recipients = Some(0);
    let Some(player_id) = out.actor.player_id else {
        out.rejected(Reason::NotReady);
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    match space_mgr
        .resources
        .squads_mut()
        .check_ping(player_id, org_id, now)
    {
        Ok(()) => {
            tracing::debug!(
                target: "squad",
                event = "squad.ping_location",
                entity_id,
                account_id = out.actor.account_id,
                player_id,
                squad_id = org_id,
                x = location[0],
                y = location[1],
                z = location[2],
                "squad minimap ping accepted; no client receives pings"
            );
            out.ok();
        }
        Err(r) => {
            out.rejected(r.into());
            match r {
                PingReject::NotInSquad => {
                    reject(tx, entity_id, 0, feedback::PING_NOT_IN_SQUAD).await;
                }
                PingReject::WrongSquad => {
                    reject(tx, entity_id, org_id, feedback::NOT_IN_THAT_SQUAD).await;
                }
                PingReject::RateLimited => {}
            }
        }
    }
}
