//! CM 18 `squadSetLootMode` (D-ORG16, CAT-M-11).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::LootReject;

use cimmeria_wire::cell::cell_methods::organization::SQUAD_SET_LOOT_MODE;

use super::{actor, fanout, feedback, reject};

/// Set the caller's squad loot mode. The leader only, and only
/// `RoundRobin` (0) or `FreeForAll` (1); then every member gets
/// `onSquadLootType` [51].
///
/// The client offers the loot menu to every member (audit A-16), so a
/// non-leader's change is an ordinary press: it gets `onErrorCode`, a
/// feedback line, and a re-send of the current [51] so the menu snaps back.
/// An out-of-range value cannot come from the client's menu and is logged
/// as a forged call, with the same answer.
pub async fn set_loot_mode(
    entity_id: u32,
    raw: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(me) = actor(space_mgr, entity_id) else {
        return reject(tx, entity_id, SQUAD_SET_LOOT_MODE, 0, feedback::NOT_READY).await;
    };
    let player_id = me.player_id;
    match space_mgr.squads.set_loot(player_id, raw) {
        Ok(squad_id) => {
            let squad = space_mgr
                .squads
                .squad(squad_id)
                .cloned()
                .expect("set_loot returned it");
            tracing::info!(
                target: "squad",
                event = "squad.loot_changed",
                player_id,
                squad_id,
                loot_mode = raw,
                "squad loot mode changed"
            );
            for m in squad.members() {
                if let Some(eid) = space_mgr.player_entity_by_player_id(m.player_id) {
                    fanout::send_loot_type(tx, eid, &squad).await;
                }
            }
        }
        Err(r) => {
            let squad_id = match r {
                LootReject::OutOfRange { squad_id, .. }
                | LootReject::NotLeader { squad_id, .. } => Some(squad_id),
                LootReject::NotInSquad => None,
            };
            if matches!(r, LootReject::OutOfRange { .. }) {
                tracing::warn!(
                    target: "squad",
                    event = "squad.loot_rejected",
                    player_id,
                    entity_id,
                    loot_mode = raw,
                    reason = r.reason(),
                    "squad loot mode outside RoundRobin/FreeForAll"
                );
            } else {
                tracing::debug!(
                    target: "squad",
                    event = "squad.loot_rejected",
                    player_id,
                    entity_id,
                    loot_mode = raw,
                    reason = r.reason(),
                    "squad loot mode change refused"
                );
            }
            let instance = squad_id.unwrap_or(0);
            reject(
                tx,
                entity_id,
                SQUAD_SET_LOOT_MODE,
                instance,
                feedback::loot_rejected(r),
            )
            .await;
            if let Some(squad) = squad_id.and_then(|id| space_mgr.squads.squad(id)) {
                fanout::send_loot_type(tx, entity_id, squad).await;
            }
        }
    }
}
