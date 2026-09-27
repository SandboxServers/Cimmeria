//! The end paths that start outside the duel tick (SS-D3).
//!
//! - [`on_disconnect`], [`on_travel`] and [`on_death`]: the player is
//!   leaving. An engaged duel ends with them as the loser
//!   (`EDUEL_DEFEAT_Connection`, `Teleport`, `Health`); a challenge to or
//!   from them, or a duel still in its countdown, is withdrawn and the other
//!   side hears "Duel aborted" (878) at once.
//! - [`clamp_partner_lethal`] and [`finish_clamped`]: the non-lethal end
//!   (D-SS20). Partner damage that would take a duelist to 0 HP leaves them
//!   at 1 HP, and the duel ends with them as the loser. Nothing reaches the
//!   death path, so there is no corpse, loot, XP, Defeat Window or respawn.
//!
//! Every end goes through [`end_engaged`].

use tokio::sync::mpsc;

use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::end::{end_engaged, DefeatReason, EndReason};
use super::find_player;
use super::outbound::{send_line, Recipient};
use super::registry::{DuelId, DuelState};

/// The player at `entity_id` is disconnecting. Called from
/// `SpaceManager::disconnect_entity` before the entity is removed.
pub async fn on_disconnect(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    entity_id: u32,
) {
    leave(tx, mgr, entity_id, DefeatReason::Connection).await;
}

/// The player at `entity_id` is being teleported (in its space) or sent
/// through gate travel. Every cell path that sends `TeleportPlayer` or
/// `GateTravel` for a player calls this before the send
/// (`every_travel_site_ends_the_duel` scans for it).
pub async fn on_travel(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, entity_id: u32) {
    leave(tx, mgr, entity_id, DefeatReason::Teleport).await;
}

/// The player at `entity_id` died: killed by anyone but the duel partner,
/// whose damage never kills (D-SS20). Called from the death resolver.
pub async fn on_death(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, entity_id: u32) {
    leave(tx, mgr, entity_id, DefeatReason::Health).await;
}

async fn leave(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    entity_id: u32,
    reason: DefeatReason,
) {
    let Some(pid) = mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .and_then(|e| e.player_id)
    else {
        return;
    };
    if let Some(duel) = mgr.duels.duel_of(pid).copied() {
        if let (DuelState::Engaged { .. }, Some(entities)) = (duel.state, duel.engaged_entities) {
            // A different entity of the same player (a stale id) is not the
            // duelist; the sweep ends the duel if the engaged one is gone.
            if entities.contains(&entity_id) {
                end_engaged(
                    tx,
                    mgr,
                    duel.duel_id,
                    EndReason::Defeated { loser: pid, reason },
                )
                .await;
            }
            return;
        }
    }
    let withdrawn = mgr.duels.withdraw(pid);
    let pairs = withdrawn
        .challenges
        .iter()
        .map(|p| (p.duel_id, p.challenger, p.target, "challenge"))
        .chain(
            withdrawn
                .countdown
                .iter()
                .map(|d| (d.duel_id, d.challenger, d.target, "countdown")),
        );
    let id = mgr.player_identity(entity_id);
    for (duel_id, challenger, target, stage) in pairs.collect::<Vec<_>>() {
        let other = if challenger == pid {
            target
        } else {
            challenger
        };
        tracing::debug!(
            target: "duel",
            event = "duel.withdrawn",
            duel_id,
            account_id = id.account_id,
            player_id = pid,
            entity_id,
            target_player_id = other,
            stage,
            reason = reason.name(),
            "a duel challenge or countdown was withdrawn: its player is leaving"
        );
        // The player leaving hears it too, unless their client is going
        // away; the other side always does.
        let mut tell = vec![(other, pid)];
        if reason != DefeatReason::Connection {
            tell.push((pid, other));
        }
        for (to_pid, about) in tell {
            if let Some(p) = find_player(mgr, to_pid) {
                let to = Recipient::at(&p, to_pid, Some(about));
                send_line(tx, to, TEXT_DUEL_ABORTED, Some(duel_id)).await;
            }
        }
    }
}

/// A partner hit that [`clamp_partner_lethal`] held at 1 HP. The caller
/// passes it to [`finish_clamped`] once every step of the same damage
/// resolution has run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClampedHit {
    pub duel_id: DuelId,
    /// The duelist held at 1 HP: the loser.
    pub loser: i32,
}

/// The non-lethal clamp (D-SS20), called after damage from `attacker_eid`
/// has been written to `target_eid`'s stats and before anything reads them
/// for a death: the ability path (`damage_apply`, after the direct damage
/// and again after the effect scripts) and the effect pulse (`fire_pulse`).
///
/// When the two are the engaged pair's engaged entities and the target's
/// HEALTH is at or below 0, HEALTH becomes 1 (still dirty, so the same
/// stat flush tells the client 1, never 0) and the hit is returned. Damage
/// from anyone else is untouched: a third party can still kill a duelist.
///
/// The duel is **not** ended here. Ending it strips the partner's effects,
/// and the same resolution may still apply a script bleed or register a
/// DoT after this point; those must be clamped too (the duel is still
/// engaged) and then stripped by [`finish_clamped`].
pub fn clamp_partner_lethal(
    mgr: &mut SpaceManager,
    attacker_eid: u32,
    target_eid: u32,
    source: &'static str,
) -> Option<ClampedHit> {
    let tpid = mgr
        .get_entity(target_eid)
        .filter(|e| e.is_player)?
        .player_id?;
    let apid = mgr
        .get_entity(attacker_eid)
        .filter(|e| e.is_player)?
        .player_id?;
    let duel = mgr.duels.duel_of(tpid).copied()?;
    let (DuelState::Engaged { .. }, Some(entities)) = (duel.state, duel.engaged_entities) else {
        return None;
    };
    let pair_ok = duel.opponent_of(tpid) == Some(apid)
        && (entities == [attacker_eid, target_eid] || entities == [target_eid, attacker_eid]);
    if !pair_ok {
        return None;
    }
    let (before, target_account) = {
        let target = mgr.get_entity_mut(target_eid)?;
        let account = target.account_id;
        let stat = target.stats.get_mut(HEALTH)?;
        if stat.cur > 0 {
            return None;
        }
        let before = stat.cur;
        stat.update(stat.min, 1, stat.max);
        (before, account)
    };
    let attacker = mgr.player_identity(attacker_eid);
    tracing::debug!(
        target: "duel",
        event = "duel.lethal_clamped",
        duel_id = duel.duel_id,
        account_id = attacker.account_id,
        player_id = apid,
        entity_id = attacker_eid,
        target_player_id = tpid,
        target_account_id = target_account,
        target_entity_id = target_eid,
        source,
        health_before = before,
        health_after = 1,
        "partner damage would have killed a duelist: held at 1 HP, the duel ends"
    );
    Some(ClampedHit {
        duel_id: duel.duel_id,
        loser: tpid,
    })
}

/// End the duel of a clamped hit: `EDUEL_DEFEAT_Health`, the clamped
/// duelist losing. A no-op if the duel already ended.
pub async fn finish_clamped(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    hit: ClampedHit,
) {
    end_engaged(
        tx,
        mgr,
        hit.duel_id,
        EndReason::Defeated {
            loser: hit.loser,
            reason: DefeatReason::Health,
        },
    )
    .await;
}
