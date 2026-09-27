//! CM 18 `squadSetLootMode` (D-ORG16, CAT-M-11).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::LootReject;

use super::telemetry::{self as tm, Action, Outcome, Reason};
use super::{actor, fanout, feedback, reject};

/// Set the caller's squad loot mode. The leader only, and only
/// `RoundRobin` (0) or `FreeForAll` (1); then every member gets
/// `onSquadLootType` [51].
///
/// The client offers the loot menu to every member (audit A-16), so a
/// non-leader's change is an ordinary press: it gets `onErrorCode`, a
/// feedback line, and a re-send of the current [51] so the menu snaps back.
/// An out-of-range value cannot come from the client's menu; it gets the
/// same answer.
#[tracing::instrument(
    name = "squad.loot_mode",
    level = "info",
    target = "squad",
    skip_all,
    fields(entity_id = entity_id, loot_mode = raw)
)]
pub async fn set_loot_mode(
    entity_id: u32,
    raw: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let actor_id = tm::of_entity(space_mgr, entity_id);
    let mut out = Outcome::new(Action::LootMode, entity_id, actor_id);
    let Some(me) = actor(space_mgr, entity_id) else {
        out.rejected(Reason::NotReady);
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    let player_id = me.player_id;
    out.squad_id = space_mgr.squads.squad_of(player_id);
    let before = space_mgr.squads.squad_for(player_id).map(|s| s.loot());
    match space_mgr.squads.set_loot(player_id, raw) {
        Ok(squad_id) => {
            let squad = space_mgr
                .squads
                .squad(squad_id)
                .cloned()
                .expect("set_loot returned it");
            if let Some(from) = before {
                tm::loot_mode_changed(squad_id, actor_id, from, squad.loot());
            }
            out.ok();
            for m in squad.members() {
                if let Some(eid) = space_mgr.player_entity_by_player_id(m.player_id) {
                    fanout::send_loot_type(tx, eid, &squad).await;
                }
            }
        }
        Err(r) => {
            out.rejected(r.into());
            let squad_id = match r {
                LootReject::OutOfRange { squad_id, .. }
                | LootReject::NotLeader { squad_id, .. } => Some(squad_id),
                LootReject::NotInSquad => None,
            };
            reject(
                tx,
                entity_id,
                squad_id.unwrap_or(0),
                feedback::loot_rejected(r),
            )
            .await;
            if let Some(squad) = squad_id.and_then(|id| space_mgr.squads.squad(id)) {
                fanout::send_loot_type(tx, entity_id, squad).await;
            }
        }
    }
}
