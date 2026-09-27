//! [`DuelRegistry`]: pending challenges, duels and the per-pair cooldowns.
//!
//! Keyed by `player_id`, never by entity id: entity ids are per-space slot
//! integers that gate travel changes and a later session can be handed. The
//! handlers resolve a player's current entity when they need to send.
//!
//! Pure state on an injected clock. Nothing here sends or logs; the
//! handlers in `challenge.rs`, `response.rs` and `tick.rs` do both.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_common::Vector3;

use super::limits::{CHALLENGE_TIMEOUT, COUNTDOWN, ENGAGED_LIMIT, PAIR_COOLDOWN};

/// Correlates every log row of one challenge and the duel it becomes.
/// Allocated per cell process, never reused while it runs.
pub type DuelId = u64;

/// A challenge waiting for the target's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingChallenge {
    pub duel_id: DuelId,
    pub challenger: i32,
    pub target: i32,
    pub expires_at: Instant,
}

/// Where an accepted duel is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelState {
    /// Accepted; the countdown runs until `engage_at` (D-SS18).
    StartPending { engage_at: Instant },
    /// Fighting. Only this state lets [`DuelRegistry::can_harm`] answer
    /// true. `until` is the safety end ([`ENGAGED_LIMIT`]): a duel that no
    /// end path has closed by then is aborted by the tick, so a missed end
    /// can never leave two players attackable for the life of the cell.
    Engaged { until: Instant },
}

impl DuelState {
    /// Stable value for a `state` log field and the GM status line.
    pub fn name(self) -> &'static str {
        match self {
            DuelState::StartPending { .. } => "countdown",
            DuelState::Engaged { .. } => "engaged",
        }
    }
}

/// One accepted duel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duel {
    pub duel_id: DuelId,
    pub challenger: i32,
    pub target: i32,
    pub space_id: u32,
    /// The midpoint of the two duelists at the accept: the arena centre
    /// (D-SS19) SS-D2's range check measures from.
    pub centre: Vector3,
    pub state: DuelState,
    /// `[challenger, target]` entity ids at the engage: the entities that got
    /// the PvP flag, `onDuelEntitiesSet` and each other as a combat source.
    /// The end clears exactly these. `None` before the engage.
    pub engaged_entities: Option<[u32; 2]>,
    /// `[challenger, target]`: since when each duelist has been outside the
    /// arena (D-SS19), or `None` while inside. SS-D3's range end fires when
    /// one of these is older than
    /// [`RANGE_GRACE`](super::limits::RANGE_GRACE).
    pub out_of_range_since: [Option<Instant>; 2],
}

impl Duel {
    /// The other duelist, if `player_id` is one of the two.
    pub fn opponent_of(&self, player_id: i32) -> Option<i32> {
        if player_id == self.challenger {
            Some(self.target)
        } else if player_id == self.target {
            Some(self.challenger)
        } else {
            None
        }
    }
}

/// Why a challenge was not opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeRefusal {
    SelfChallenge,
    /// The challenger already has a challenge out, has one to answer, or is
    /// in a duel.
    ChallengerBusy,
    /// Same, for the target.
    TargetBusy,
    /// The same challenger challenged the same target within
    /// [`PAIR_COOLDOWN`] of a decline or expiry (D-SS21).
    PairCooldown,
}

impl ChallengeRefusal {
    /// Stable value for the `reason` log field.
    pub fn reason(self) -> &'static str {
        match self {
            ChallengeRefusal::SelfChallenge => "self_challenge",
            ChallengeRefusal::ChallengerBusy => "challenger_busy",
            ChallengeRefusal::TargetBusy => "target_busy",
            ChallengeRefusal::PairCooldown => "pair_cooldown",
        }
    }
}

/// Why a response found nothing to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseRefusal {
    /// No challenge is addressed to the caller: never sent, already
    /// answered (a replay), or already expired and swept.
    NoPending,
    /// The challenge addressed to the caller outlived [`CHALLENGE_TIMEOUT`]
    /// and the sweep had not removed it yet. It is removed now and the
    /// cooldown starts, exactly as the sweep would have done.
    Expired(PendingChallenge),
}

/// What [`DuelRegistry::gm_abort`] removed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GmAborted {
    Duel(Duel),
    Challenge(PendingChallenge),
}

impl GmAborted {
    pub fn duel_id(&self) -> DuelId {
        match self {
            GmAborted::Duel(d) => d.duel_id,
            GmAborted::Challenge(p) => p.duel_id,
        }
    }

    /// The two players, challenger first.
    pub fn players(&self) -> (i32, i32) {
        match self {
            GmAborted::Duel(d) => (d.challenger, d.target),
            GmAborted::Challenge(p) => (p.challenger, p.target),
        }
    }

    /// Stable value for the `stage` log field.
    pub fn stage(&self) -> &'static str {
        match self {
            GmAborted::Duel(d) => d.state.name(),
            GmAborted::Challenge(_) => "challenge",
        }
    }
}

/// What [`DuelRegistry::withdraw`] removed.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Withdrawn {
    /// Challenges to or from the player, now gone.
    pub challenges: Vec<PendingChallenge>,
    /// A duel that was still in its countdown, now gone.
    pub countdown: Option<Duel>,
}

/// Every pending challenge and duel on this cell.
#[derive(Debug, Default)]
pub struct DuelRegistry {
    next_id: DuelId,
    /// Target `player_id` -> the one challenge addressed to them.
    pending: HashMap<i32, PendingChallenge>,
    /// Challenger `player_id` -> the target of their one open challenge
    /// (the reverse index).
    pending_from: HashMap<i32, i32>,
    duels: HashMap<DuelId, Duel>,
    /// Either duelist's `player_id` -> their duel.
    in_duel: HashMap<i32, DuelId>,
    /// `(challenger, target)` -> the instant the pair may challenge again.
    cooldowns: HashMap<(i32, i32), Instant>,
}

impl DuelRegistry {
    /// `true` when there is nothing for the tick to do. A running pair
    /// cooldown counts: the tick is what prunes it ([`Self::expire_pending`]),
    /// so an idle short-circuit that ignored cooldowns would keep every
    /// cooldown since the last challenge for the life of the cell process.
    pub fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.duels.is_empty() && self.cooldowns.is_empty()
    }

    /// How many directed pair cooldowns are stored, expired or not.
    pub fn cooldown_count(&self) -> usize {
        self.cooldowns.len()
    }

    /// `player_id` has a challenge out, one to answer, or a duel.
    pub fn is_busy(&self, player_id: i32) -> bool {
        self.pending.contains_key(&player_id)
            || self.pending_from.contains_key(&player_id)
            || self.in_duel.contains_key(&player_id)
    }

    /// The challenge addressed to `target`, if any.
    pub fn pending_for(&self, target: i32) -> Option<&PendingChallenge> {
        self.pending.get(&target)
    }

    /// The duel `player_id` is in, if any.
    pub fn duel_of(&self, player_id: i32) -> Option<&Duel> {
        self.in_duel
            .get(&player_id)
            .and_then(|id| self.duels.get(id))
    }

    /// True only for the two players of one engaged duel, in either order.
    /// The contract predicate SS-D2's harm gate calls.
    pub fn can_harm(&self, attacker: i32, target: i32) -> bool {
        self.engaged_opponent(attacker) == Some(target)
    }

    /// The other duelist, when `player_id` is in an engaged duel. What the
    /// PvP-flag replay and the AoE/cone candidate scan ask.
    pub fn engaged_opponent(&self, player_id: i32) -> Option<i32> {
        self.duel_of(player_id)
            .filter(|d| matches!(d.state, DuelState::Engaged { .. }))
            .and_then(|d| d.opponent_of(player_id))
    }

    /// Move a duel whose countdown ran out to `Engaged`. `None` when the
    /// duel is gone or not in its countdown.
    pub fn engage(&mut self, duel_id: DuelId, entities: [u32; 2], now: Instant) -> Option<Duel> {
        let duel = self.duels.get_mut(&duel_id)?;
        if !matches!(duel.state, DuelState::StartPending { .. }) {
            return None;
        }
        duel.state = DuelState::Engaged {
            until: now + ENGAGED_LIMIT,
        };
        duel.engaged_entities = Some(entities);
        Some(*duel)
    }

    /// The duel with this id, in any state.
    pub fn duel(&self, duel_id: DuelId) -> Option<&Duel> {
        self.duels.get(&duel_id)
    }

    /// Every engaged duel, oldest first.
    pub fn engaged(&self) -> Vec<Duel> {
        let mut out: Vec<Duel> = self
            .duels
            .values()
            .filter(|d| matches!(d.state, DuelState::Engaged { .. }))
            .copied()
            .collect();
        out.sort_by_key(|d| d.duel_id);
        out
    }

    /// Open a challenge, or say why not. Checked in the ledger's order:
    /// self, busy (challenger, then target), then the per-pair cooldown.
    pub fn open_challenge(
        &mut self,
        challenger: i32,
        target: i32,
        now: Instant,
    ) -> Result<PendingChallenge, ChallengeRefusal> {
        if challenger == target {
            return Err(ChallengeRefusal::SelfChallenge);
        }
        if self.is_busy(challenger) {
            return Err(ChallengeRefusal::ChallengerBusy);
        }
        if self.is_busy(target) {
            return Err(ChallengeRefusal::TargetBusy);
        }
        if self
            .cooldowns
            .get(&(challenger, target))
            .is_some_and(|&until| now < until)
        {
            return Err(ChallengeRefusal::PairCooldown);
        }
        self.next_id += 1;
        let pending = PendingChallenge {
            duel_id: self.next_id,
            challenger,
            target,
            expires_at: now + CHALLENGE_TIMEOUT,
        };
        self.pending.insert(target, pending);
        self.pending_from.insert(challenger, target);
        Ok(pending)
    }

    /// Consume the challenge addressed to `responder`. It is removed on
    /// every path, so a second response finds nothing (CAT-M-13).
    pub fn take_pending_for(
        &mut self,
        responder: i32,
        now: Instant,
    ) -> Result<PendingChallenge, ResponseRefusal> {
        let pending = self
            .remove_pending(responder)
            .ok_or(ResponseRefusal::NoPending)?;
        if now >= pending.expires_at {
            self.start_cooldown(&pending, now);
            return Err(ResponseRefusal::Expired(pending));
        }
        Ok(pending)
    }

    /// Withdraw the challenge addressed to `target` without a cooldown: the
    /// prompt never reached the target, so the pair did nothing to cool off.
    pub fn cancel_pending(&mut self, target: i32) -> Option<PendingChallenge> {
        self.remove_pending(target)
    }

    /// The challenge `challenger` has out, if any.
    pub fn challenge_from(&self, challenger: i32) -> Option<&PendingChallenge> {
        self.pending_from
            .get(&challenger)
            .and_then(|target| self.pending.get(target))
    }

    /// GM abort (`.duel_end`, SS-U2): remove whatever `player_id` is part
    /// of, a duel or a challenge in either direction, without a cooldown.
    /// A GM ending it is not the pair declining. `None` when the player is
    /// in nothing. The busy check makes the three cases exclusive.
    ///
    /// Registry only: an engaged duel removed here keeps both PvP flags and
    /// the combat pair. Callers end one through `gm::gm_end`, which routes
    /// it to `end::end_engaged`.
    pub fn gm_abort(&mut self, player_id: i32) -> Option<GmAborted> {
        if let Some(&duel_id) = self.in_duel.get(&player_id) {
            return self.end_duel(duel_id).map(GmAborted::Duel);
        }
        if let Some(p) = self.remove_pending(player_id) {
            return Some(GmAborted::Challenge(p));
        }
        let target = *self.pending_from.get(&player_id)?;
        self.remove_pending(target).map(GmAborted::Challenge)
    }

    /// Record a decline: the pair cooldown starts (D-SS21).
    pub fn decline(&mut self, pending: &PendingChallenge, now: Instant) {
        self.start_cooldown(pending, now);
    }

    /// Turn an accepted challenge into a duel in the countdown.
    pub fn start_duel(
        &mut self,
        pending: &PendingChallenge,
        space_id: u32,
        centre: Vector3,
        now: Instant,
    ) -> Duel {
        let duel = Duel {
            duel_id: pending.duel_id,
            challenger: pending.challenger,
            target: pending.target,
            space_id,
            centre,
            state: DuelState::StartPending {
                engage_at: now + COUNTDOWN,
            },
            engaged_entities: None,
            out_of_range_since: [None, None],
        };
        self.duels.insert(duel.duel_id, duel);
        self.in_duel.insert(duel.challenger, duel.duel_id);
        self.in_duel.insert(duel.target, duel.duel_id);
        duel
    }

    /// Remove every challenge that has expired by `now`, start its pair
    /// cooldown, and drop cooldowns that have run out (so the map stays
    /// bounded by recent activity).
    pub fn expire_pending(&mut self, now: Instant) -> Vec<PendingChallenge> {
        self.cooldowns.retain(|_, &mut until| now < until);
        let expired: Vec<i32> = self
            .pending
            .values()
            .filter(|p| now >= p.expires_at)
            .map(|p| p.target)
            .collect();
        let mut out = Vec::with_capacity(expired.len());
        for target in expired {
            if let Some(p) = self.remove_pending(target) {
                self.start_cooldown(&p, now);
                out.push(p);
            }
        }
        out.sort_by_key(|p| p.duel_id);
        out
    }

    /// Duels whose countdown has run out by `now`, oldest first.
    pub fn countdowns_due(&self, now: Instant) -> Vec<DuelId> {
        let mut due: Vec<DuelId> = self
            .duels
            .values()
            .filter(
                |d| matches!(d.state, DuelState::StartPending { engage_at } if now >= engage_at),
            )
            .map(|d| d.duel_id)
            .collect();
        due.sort_unstable();
        due
    }

    /// Record whether duelist `side` (0 = challenger, 1 = target) is outside
    /// the arena: `Some(since)` starts or keeps the clock, `None` clears it.
    /// Returns the previous value, so the caller can log the transition.
    pub fn set_out_of_range(
        &mut self,
        duel_id: DuelId,
        side: usize,
        since: Option<Instant>,
    ) -> Option<Instant> {
        let duel = self.duels.get_mut(&duel_id)?;
        std::mem::replace(&mut duel.out_of_range_since[side], since)
    }

    /// Withdraw everything `player_id` has open short of an engaged duel:
    /// the challenge addressed to them, the challenge they sent, and a duel
    /// still in its countdown. No cooldown starts: nobody declined. Used
    /// when the player disconnects, travels or dies (SS-D3), so the other
    /// side is told at once instead of at the expiry or the engage.
    pub fn withdraw(&mut self, player_id: i32) -> Withdrawn {
        let mut challenges = Vec::new();
        if let Some(p) = self.remove_pending(player_id) {
            challenges.push(p);
        }
        if let Some(&target) = self.pending_from.get(&player_id) {
            if let Some(p) = self.remove_pending(target) {
                challenges.push(p);
            }
        }
        let countdown = self
            .duel_of(player_id)
            .filter(|d| matches!(d.state, DuelState::StartPending { .. }))
            .map(|d| d.duel_id)
            .and_then(|id| self.end_duel(id));
        Withdrawn {
            challenges,
            countdown,
        }
    }

    /// Remove a duel and both players' index entries.
    pub fn end_duel(&mut self, duel_id: DuelId) -> Option<Duel> {
        let duel = self.duels.remove(&duel_id)?;
        self.in_duel.remove(&duel.challenger);
        self.in_duel.remove(&duel.target);
        Some(duel)
    }

    fn remove_pending(&mut self, target: i32) -> Option<PendingChallenge> {
        let p = self.pending.remove(&target)?;
        self.pending_from.remove(&p.challenger);
        Some(p)
    }

    fn start_cooldown(&mut self, pending: &PendingChallenge, now: Instant) {
        self.cooldowns
            .insert((pending.challenger, pending.target), now + PAIR_COOLDOWN);
    }
}
