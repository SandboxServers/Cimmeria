//! Interrupt effects, resolved (ability-mechanics AB-09c).
//!
//! An interrupt effect script ("Interrupts target", effect 723 of
//! Interrupting Shot; an EMP round's disruption) queues an
//! `InterruptRequest` on the `SpaceManager`, because a synchronous script
//! cannot reach the async cancel. This module resolves the queue:
//! [`resolve_interrupts_for`] right after a script runs (from
//! `flush_stat_buff_timers`, which every script caller awaits) and
//! [`resolve_all_interrupts`] on the stat-buff tick as a safety net.
//!
//! **What an interrupt breaks.** The target's warmup (`pending_cast`,
//! through AT-10's `interrupt_pending_cast`: the zeroed warmup and cooldown
//! timers to a player, `Ability_Interrupt` to the target and its witnesses,
//! the cooldown refunded as python's `interrupt()` did, and no lockout, as
//! AT-10 has none) and every channel the target is running
//! (`cancel_channels_from_attacker`). A target with neither is not rolled.
//!
//! **Resistance** (`alias.xml`): `interruptRes` "increases resistance to all
//! interrupts except movement", and `coordination` gives "+0.1% resistance
//! to interrupts per point". D-AB09 reads a resist stat at 10 points per
//! 1 %, which makes the two the same unit: the target resists with
//! probability `(interruptRes + coordination) / 1000`, clamped to 0..=1.
//! The effect's own `InterruptChance` (100 when unstated) scales what is
//! left: `P(interrupt) = chance x (1 - resist)`. DESIGN, not recovered: no
//! artefact gives the formula. Movement interrupts (AT-10) never roll.
//!
//! The roll is seeded from `(source, target, effect, the target's warmup
//! instance, the request's nonce)`. The nonce is a per-request counter on
//! the `SpaceManager`, so every attempt rolls afresh (a channel has no
//! warmup instance, and repeated EMP hits share one), while a test or a
//! replay with the same requests in the same order sees the same outcome.
//!
//! **A landed stun or knockdown** queues an `Incapacitated` request: never
//! rolled, it breaks the warmup (reason `incapacitated`) and the channels,
//! and logs nothing when the target was doing neither.
//!
//! Log target `abilities`: one `interrupt_effect` row per request, with
//! `decision_outcome` `interrupted`, `resisted` or `nothing_to_interrupt`.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tokio::sync::mpsc;

use cimmeria_entity::stats::{COORDINATION, INTERRUPT_RES};

use crate::cell::abilities::{interrupt_pending_cast, InterruptReason};
use crate::cell::effects::interrupt_request::{InterruptCause, InterruptRequest};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::cancel_channels_from_attacker;

/// Points of `interruptRes` (or `coordination`) per 1 % resistance
/// (D-AB09's 10:1 rule).
pub const RESIST_POINTS_PER_PERCENT: f64 = 10.0;

/// The probability `target` resists an interrupt, from its stats.
pub fn interrupt_resist_chance(interrupt_res: i32, coordination: i32) -> f64 {
    let points = interrupt_res.saturating_add(coordination).max(0) as f64;
    (points / (RESIST_POINTS_PER_PERCENT * 100.0)).min(1.0)
}

/// Whether an interrupt at `chance_pct` lands against `resist`, for a
/// uniform `roll` in `[0, 1)`.
pub fn interrupt_lands(chance_pct: i32, resist: f64, roll: f64) -> bool {
    let p = (f64::from(chance_pct.clamp(0, 100)) / 100.0) * (1.0 - resist.clamp(0.0, 1.0));
    roll < p
}

fn roll_seed(r: &InterruptRequest, instance: i32) -> u64 {
    let mut h = u64::from(r.source_id).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(r.target_id)
            .wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            .rotate_left(17)
        ^ u64::from(r.effect_id as u32)
            .wrapping_mul(0x1656_67B1_9E37_79F9)
            .rotate_left(31)
        ^ u64::from(instance as u32).rotate_left(47)
        ^ r.nonce.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// Resolve the interrupts queued against `target_id`.
pub async fn resolve_interrupts_for(
    target_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for request in space_mgr.take_interrupt_requests_for(target_id) {
        resolve_one(request, tx, space_mgr).await;
    }
}

/// Resolve every queued interrupt (the stat-buff tick's safety net).
pub async fn resolve_all_interrupts(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for request in space_mgr.take_all_interrupt_requests() {
        resolve_one(request, tx, space_mgr).await;
    }
}

/// Whether `entity_id` runs a channel (an instance it invoked whose effect
/// has `pulse_count == 0`) on any entity.
fn is_channelling(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr.all_entity_ids().into_iter().any(|eid| {
        space_mgr.get_entity(eid).is_some_and(|e| {
            e.active_effects.iter().any(|i| {
                i.invoker_id == entity_id
                    && space_mgr
                        .effect_defs
                        .get(&i.effect_id)
                        .is_some_and(|d| d.pulse_count == 0)
            })
        })
    })
}

async fn resolve_one(
    r: InterruptRequest,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let who = space_mgr.player_identity(r.source_id);
    let target_who = space_mgr.player_identity(r.target_id);
    let Some(target) = space_mgr.get_entity(r.target_id) else {
        return;
    };
    let stat = |id| target.stats.get(id).map_or(0, |s| s.cur);
    let (interrupt_res, coordination) = (stat(INTERRUPT_RES), stat(COORDINATION));
    let warming = target
        .pending_cast
        .as_ref()
        .map(|pc| (pc.ability_id, pc.effect_seq));
    let channelling = is_channelling(space_mgr, r.target_id);

    let incapacitated = r.cause == InterruptCause::Incapacitated;
    if warming.is_none() && !channelling {
        if incapacitated {
            // Every stun queues one; most land on an entity doing nothing.
            return;
        }
        tracing::debug!(
            target: "abilities",
            event = "interrupt_effect",
            decision_outcome = "nothing_to_interrupt",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = r.source_id,
            target_id = r.target_id,
            target_player_id = target_who.player_id,
            effect_id = r.effect_id,
            ability_id = r.ability_id,
            "interrupt landed on a target with no warmup and no channel"
        );
        return;
    }

    // A stun is not resisted here: its own resist roll is D-AB13's.
    let resist = if incapacitated {
        0.0
    } else {
        interrupt_resist_chance(interrupt_res, coordination)
    };
    let roll: f64 = ChaCha8Rng::seed_from_u64(roll_seed(&r, warming.map_or(0, |w| w.1))).random();
    let lands = incapacitated || interrupt_lands(r.chance_pct, resist, roll);
    let reason = if incapacitated {
        InterruptReason::Incapacitated
    } else {
        InterruptReason::Interrupted
    };
    let (warmup_interrupted, channels_cancelled) = if lands {
        (
            interrupt_pending_cast(r.target_id, reason, tx, space_mgr).await,
            cancel_channels_from_attacker(r.target_id, None, tx, space_mgr).await,
        )
    } else {
        (false, 0)
    };
    tracing::info!(
        target: "abilities",
        event = "interrupt_effect",
        decision_outcome = if lands { "interrupted" } else { "resisted" },
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = r.source_id,
        target_id = r.target_id,
        target_player_id = target_who.player_id,
        effect_id = r.effect_id,
        ability_id = r.ability_id,
        cause = if incapacitated { "incapacitated" } else { "effect" },
        nonce = r.nonce,
        interrupted_ability_id = warming.map(|w| w.0),
        chance_pct = r.chance_pct,
        interrupt_res,
        coordination,
        resist_chance = resist,
        roll,
        warmup_interrupted,
        channels_cancelled,
        "interrupt effect resolved"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_is_ten_points_per_percent_of_both_stats() {
        assert_eq!(interrupt_resist_chance(0, 0), 0.0);
        // "+10% Interrupt Resistance" = 100 points = 10 %.
        assert!((interrupt_resist_chance(100, 0) - 0.10).abs() < 1e-12);
        // alias.xml: coordination is +0.1 % per point.
        assert!((interrupt_resist_chance(0, 12) - 0.012).abs() < 1e-12);
        assert_eq!(interrupt_resist_chance(2_000, 0), 1.0, "capped at certain");
        assert_eq!(interrupt_resist_chance(-500, 0), 0.0, "never negative");
    }

    #[test]
    fn the_chance_scales_what_resistance_leaves() {
        assert!(interrupt_lands(100, 0.0, 0.999), "certain with no resist");
        assert!(!interrupt_lands(100, 1.0, 0.0), "immune at full resist");
        // 25 % x (1 - 0.2) = 20 %.
        assert!(interrupt_lands(25, 0.2, 0.199));
        assert!(!interrupt_lands(25, 0.2, 0.2));
        assert!(!interrupt_lands(0, 0.0, 0.0));
    }
}
