//! Ending an engaged duel: the one clear every end path goes through.
//!
//! [`end_engaged`] removes the duel from the registry (so `can_harm` is false
//! from that instant), then, for each duelist still at the entity that was
//! engaged:
//!
//! 1. the PvP flag back to 0, to them and their witnesses;
//! 2. `onDuelEntitiesClear()` [153] to their own client;
//! 3. every active effect the other duelist's engaged entity applied to
//!    them (a bleed, a stun, a snare) removed, with its `on_remove` and the
//!    zero timer, so no partner harm lands after the end
//!    ([`effects::strip_from`](super::effects));
//! 4. the other duelist dropped as a combat source (`BSF_InCombat` clears
//!    unless a mob still holds them).
//!
//! Then the result (D-SS22): a decided duel ([`EndReason::Defeated`]) sends
//! "You won the duel" (879) to the winner and a feedback line to the loser;
//! any other end is an abort and sends "Duel aborted" (878) to both.
//!
//! The callers (every one goes through [`end_engaged`]):
//!
//! - forfeit, CM 103 ([`forfeit`](super::forfeit));
//! - the non-lethal clamp on partner damage, and death from anyone else
//!   ([`paths`](super::paths));
//! - disconnect (`SpaceManager::disconnect_entity`) and every teleport or
//!   gate travel ([`paths`](super::paths));
//! - [`sweep`], on the duel tick: range (D-SS19), a dead duelist the death
//!   path did not report, a duelist gone from the space, and the safety
//!   limit;
//! - the GM `.duel_end` (SS-U2), with [`EndReason::GmAborted`].

use super::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::cell::client_methods::duel::{
    EDUEL_DEFEAT_CONNECTION, EDUEL_DEFEAT_FORFEIT, EDUEL_DEFEAT_HEALTH, EDUEL_DEFEAT_RANGE,
    EDUEL_DEFEAT_TELEPORT, TEXT_DUEL_ABORTED, TEXT_DUEL_FORFEITED, TEXT_DUEL_LOST,
    TEXT_DUEL_LOST_RANGE, TEXT_DUEL_LOST_TELEPORT, TEXT_DUEL_OUT_OF_RANGE, TEXT_DUEL_WON,
};
use cimmeria_wire::state_field::BSF_DEAD;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::limits::{ARENA_RADIUS, RANGE_GRACE};
use super::outbound::{
    send_duel_entities_clear, send_line, send_pvp_flag, send_state_field, Recipient,
};
use super::registry::{Duel, DuelId, DuelState};
use super::{combat, connected_player, find_player};

/// Why the loser lost: the client's `EDuelDefeatReason` (`enumerations.xml`,
/// pinned in `cimmeria_wire`). `LeftSquad` and `InDuel` have no path while
/// squad duels are refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatReason {
    /// Partner damage clamped at 1 HP (D-SS20), or killed by a third party.
    Health,
    /// Disconnected.
    Connection,
    /// Outside the arena for [`RANGE_GRACE`] (D-SS19).
    Range,
    /// Teleported, gate-travelled or otherwise left the duel's space.
    Teleport,
    /// `duelForfeit` (CM 103).
    Forfeit,
}

impl DefeatReason {
    /// The `EDuelDefeatReason` value, logged as `defeat_reason`.
    pub fn value(self) -> u8 {
        match self {
            DefeatReason::Health => EDUEL_DEFEAT_HEALTH,
            DefeatReason::Connection => EDUEL_DEFEAT_CONNECTION,
            DefeatReason::Range => EDUEL_DEFEAT_RANGE,
            DefeatReason::Teleport => EDUEL_DEFEAT_TELEPORT,
            DefeatReason::Forfeit => EDUEL_DEFEAT_FORFEIT,
        }
    }

    /// Stable value for the `reason` log field.
    pub fn name(self) -> &'static str {
        match self {
            DefeatReason::Health => "health",
            DefeatReason::Connection => "connection",
            DefeatReason::Range => "range",
            DefeatReason::Teleport => "teleport",
            DefeatReason::Forfeit => "forfeit",
        }
    }

    /// The loser's feedback line. `None` for a disconnect: the loser's
    /// client is going away.
    fn loser_line(self) -> Option<&'static str> {
        match self {
            DefeatReason::Health => Some(TEXT_DUEL_LOST),
            DefeatReason::Connection => None,
            DefeatReason::Range => Some(TEXT_DUEL_LOST_RANGE),
            DefeatReason::Teleport => Some(TEXT_DUEL_LOST_TELEPORT),
            DefeatReason::Forfeit => Some(TEXT_DUEL_FORFEITED),
        }
    }
}

/// Why an engaged duel ended, for the `reason` log field and the result
/// texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// Engaged for [`ENGAGED_LIMIT`](super::limits::ENGAGED_LIMIT): the
    /// backstop for an end path that never fired. An abort.
    EngagedLimit,
    /// Both duelists are gone from the engaged entities (neither can be
    /// named the loser). An abort.
    DuelistGone,
    /// A GM ended it (`.duel_end`, SS-U2). The GM path must call
    /// [`end_engaged`] for an engaged duel instead of removing it from the
    /// registry itself, or both players keep their flag and combat state.
    /// An abort.
    GmAborted,
    /// Decided: `loser` (a `player_id`, one of the two) lost for `reason`;
    /// the other duelist won. `killer` is the entity whose damage decided a
    /// `Health` end (the partner for a clamped hit, anyone else for a
    /// death), and `clamped` says it was the partner's hit held at 1 HP; both
    /// are for the log row only.
    Defeated {
        loser: i32,
        reason: DefeatReason,
        killer: Option<u32>,
        clamped: bool,
    },
}

impl EndReason {
    /// A decided end with no killer to name (forfeit, disconnect, travel,
    /// range, a death the tick found).
    pub fn defeat(loser: i32, reason: DefeatReason) -> Self {
        EndReason::Defeated {
            loser,
            reason,
            killer: None,
            clamped: false,
        }
    }

    /// Stable value for the `reason` log field.
    pub fn reason(self) -> &'static str {
        match self {
            EndReason::EngagedLimit => "engaged_limit",
            EndReason::DuelistGone => "duelist_gone",
            EndReason::GmAborted => "gm_aborted",
            EndReason::Defeated { reason, .. } => reason.name(),
        }
    }
}

/// End the engaged duel `duel_id` and clear everything the engage set.
/// Returns the ended duel; `None` (and nothing done) when the duel is gone
/// or not engaged, so a second end of the same duel sends nothing. A
/// `Defeated` reason whose `loser` is not one of the duelists is treated as
/// an abort.
pub async fn end_engaged(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel_id: DuelId,
    reason: EndReason,
) -> Option<Duel> {
    let duel = mgr.resources.duels().duel(duel_id).copied()?;
    let (DuelState::Engaged { .. }, Some(entities)) = (duel.state, duel.engaged_entities) else {
        return None;
    };
    let (defeat, killer, clamped) = match reason {
        EndReason::Defeated {
            loser,
            reason,
            killer,
            clamped,
        } if duel.opponent_of(loser).is_some() => (Some((loser, reason)), killer, clamped),
        _ => (None, None, false),
    };
    // Identity before teardown: the log row must not depend on what is
    // still in the world after the clear. A duelist whose engaged entity is
    // already gone (a space change the sweep caught) is named from the
    // entity they play now, if any.
    let accounts =
        [(entities[0], duel.challenger), (entities[1], duel.target)].map(|(eid, pid)| {
            mgr.player_identity(eid)
                .account_id
                .or_else(|| find_player(mgr, pid).and_then(|p| p.account_id))
        });
    mgr.resources.duels_mut().end_duel(duel_id);

    let mut cleared = [false; 2];
    let mut effects_removed = [0usize; 2];
    for (i, (my_pid, other_pid, my_eid, other_eid)) in [
        (duel.challenger, duel.target, entities[0], entities[1]),
        (duel.target, duel.challenger, entities[1], entities[0]),
    ]
    .into_iter()
    .enumerate()
    {
        // Only the entity that was engaged carries the flag and the combat
        // source. A duelist who logged out has no entity left to clear; one
        // who is elsewhere now still hears the result on their new entity.
        if let Some(p) = connected_player(mgr, my_eid, my_pid) {
            let to = Recipient::at(&p, my_pid, Some(other_pid));
            send_pvp_flag(tx, mgr, to, false, duel_id).await;
            send_duel_entities_clear(tx, to, duel_id).await;
            effects_removed[i] = super::effects::strip_from(tx, mgr, my_eid, other_eid).await;
            if let Some(state) = combat::exit(mgr, my_eid, other_eid) {
                send_state_field(tx, mgr, to, state, duel_id).await;
            }
            cleared[i] = true;
        }
        let line = match defeat {
            None => Some(TEXT_DUEL_ABORTED),
            Some((loser, _)) if loser != my_pid => Some(TEXT_DUEL_WON),
            Some((_, why)) => why.loser_line(),
        };
        if let (Some(line), Some(p)) = (line, find_player(mgr, my_pid)) {
            let to = Recipient::at(&p, my_pid, Some(other_pid));
            send_line(tx, to, line, Some(duel_id)).await;
        }
    }
    let loser = defeat.map(|(l, _)| l);
    tracing::debug!(
        target: "duel",
        event = "duel.ended",
        duel_id,
        account_id = accounts[0],
        player_id = duel.challenger,
        entity_id = entities[0],
        target_player_id = duel.target,
        target_entity_id = entities[1],
        target_account_id = accounts[1],
        space_id = duel.space_id,
        state = "engaged",
        cleared = cleared[0],
        target_cleared = cleared[1],
        effects_removed = effects_removed[0],
        target_effects_removed = effects_removed[1],
        reason = reason.reason(),
        outcome = if defeat.is_some() { "decided" } else { "aborted" },
        loser_player_id = loser,
        winner_player_id = loser.and_then(|l| duel.opponent_of(l)),
        defeat_reason = defeat.map(|(_, r)| r.value()),
        killer_entity_id = killer,
        clamped,
        "duel ended: PvP flags, duel entities and the combat pair cleared"
    );
    Some(duel)
}

/// The tick's ends for every engaged duel, in this order:
///
/// 1. the safety limit (abort);
/// 2. a duelist no longer the connected player at the engaged entity in the
///    duel's space: `Teleport` if they are in the world elsewhere,
///    `Connection` if not, an abort if both are gone. The disconnect and
///    travel hooks normally end it first; this catches a path that has none;
/// 3. a dead duelist (`Health`): a third-party death the death path did not
///    report, such as a DoT from an NPC (pulses kill no player);
/// 4. range (D-SS19): outside [`ARENA_RADIUS`] of the centre for
///    [`RANGE_GRACE`]. Leaving the arena starts the clock and warns the
///    duelist once; coming back clears it.
pub async fn sweep(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager, now: Instant) {
    for duel in mgr.resources.duels().engaged() {
        let (DuelState::Engaged { until }, Some(entities)) = (duel.state, duel.engaged_entities)
        else {
            continue;
        };
        let pids = [duel.challenger, duel.target];
        let at = [0, 1].map(|i| {
            connected_player(mgr, entities[i], pids[i]).filter(|p| p.space_id == duel.space_id)
        });
        let reason = if now >= until {
            EndReason::EngagedLimit
        } else if let Some(reason) = gone_reason(mgr, &pids, &at) {
            reason
        } else if let Some(loser) = dead_duelist(mgr, &pids, &entities) {
            EndReason::defeat(loser, DefeatReason::Health)
        } else if let Some(loser) = range_loser(tx, mgr, &duel, &at, now).await {
            EndReason::defeat(loser, DefeatReason::Range)
        } else {
            continue;
        };
        end_engaged(tx, mgr, duel.duel_id, reason).await;
    }
}

fn gone_reason(
    mgr: &SpaceManager,
    pids: &[i32; 2],
    at: &[Option<super::PlayerAt>; 2],
) -> Option<EndReason> {
    match (at[0].is_some(), at[1].is_some()) {
        (true, true) => None,
        (false, false) => Some(EndReason::DuelistGone),
        (c, _) => {
            let loser = if c { pids[1] } else { pids[0] };
            let reason = if find_player(mgr, loser).is_some() {
                DefeatReason::Teleport
            } else {
                DefeatReason::Connection
            };
            Some(EndReason::defeat(loser, reason))
        }
    }
}

/// The first duelist, challenger first, whose engaged entity is dead
/// (`BSF_Dead`) or at 0 HP. Two dead duelists (no single path produces
/// that: a partner hit clamps) end with the challenger as the loser.
fn dead_duelist(mgr: &SpaceManager, pids: &[i32; 2], entities: &[u32; 2]) -> Option<i32> {
    (0..2).find_map(|i| {
        let e = mgr.get_entity(entities[i])?;
        let dead = e.state_field & BSF_DEAD != 0 || e.stats.get(HEALTH).is_some_and(|s| s.cur <= 0);
        dead.then_some(pids[i])
    })
}

/// Advance each duelist's out-of-arena clock and return the one who has
/// been outside for [`RANGE_GRACE`], if any.
async fn range_loser(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    duel: &Duel,
    at: &[Option<super::PlayerAt>; 2],
    now: Instant,
) -> Option<i32> {
    let pids = [duel.challenger, duel.target];
    let mut loser = None;
    for side in 0..2 {
        let Some(p) = at[side] else { continue };
        let distance = p.position.distance_to(&duel.centre);
        let outside = distance > ARENA_RADIUS;
        let since = duel.out_of_range_since[side];
        let to = Recipient::at(&p, pids[side], Some(pids[1 - side]));
        match (outside, since) {
            (true, None) => {
                mgr.resources
                    .duels_mut()
                    .set_out_of_range(duel.duel_id, side, Some(now));
                tracing::debug!(
                    target: "duel",
                    event = "duel.out_of_range",
                    duel_id = duel.duel_id,
                    account_id = p.account_id,
                    player_id = pids[side],
                    entity_id = p.entity_id,
                    target_player_id = pids[1 - side],
                    distance,
                    arena_radius = ARENA_RADIUS,
                    grace_ms = RANGE_GRACE.as_millis() as u64,
                    "duelist left the arena: the range clock started"
                );
                send_line(tx, to, TEXT_DUEL_OUT_OF_RANGE, Some(duel.duel_id)).await;
            }
            (true, Some(since)) if now.duration_since(since) >= RANGE_GRACE => {
                loser = loser.or(Some(pids[side]));
            }
            (false, Some(since)) => {
                mgr.resources
                    .duels_mut()
                    .set_out_of_range(duel.duel_id, side, None);
                tracing::debug!(
                    target: "duel",
                    event = "duel.back_in_range",
                    duel_id = duel.duel_id,
                    account_id = p.account_id,
                    player_id = pids[side],
                    entity_id = p.entity_id,
                    target_player_id = pids[1 - side],
                    distance,
                    outside_ms = now.duration_since(since).as_millis() as u64,
                    "duelist came back into the arena: the range clock stopped"
                );
            }
            _ => {}
        }
    }
    loser
}
