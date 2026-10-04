//! Crowd-control effect scripts (ability-mechanics AB-09a and AB-09c):
//! [`Stun`], [`Knockdown`] and [`Interrupt`]. Snares and slows are
//! `TimedStat` ledger entries on `movementSpeedMod` (AB-09b), and the ammo
//! Tranquilizer's `MovementSlow` is in `ammo_dart_cc.rs`.
//!
//! **Stun and knockdown are ledger entries.** Each lands as one entry on the
//! timed effect ledger per `(effect, caster)`, holding `BSF_MovementLock`
//! through the ledger's state-flag payload (`stat_buff_flags.rs` in
//! `cimmeria-entity`) for its duration. The entry takes one counted
//! reference when it goes on and releases it when it comes off, however
//! often the script re-runs (a pulse, a same-caster re-hit), so an
//! overlapping or refreshed stun can no longer leave the lock set. Expiry,
//! the death strip (`EF_ClearOnDeath`) and the stat-buff tick's
//! `onStateFieldUpdate` to the target and its witnesses come with the
//! ledger. The client's own movement input honours the bit; the NPC AI and
//! movement ticks honour it on the server.
//!
//! A knockdown is the same lock under its own effect, with its own
//! duration; it logs `kind = "knockdown"`. No knockdown animation is sent:
//! no sequence for one is known.
//!
//! Duration: the effect's [`CC_DURATION_NVP`] (seconds, the `cc` family of
//! `tools/ability_mechanics/`) when present, else its pulse span (one
//! `pulse_duration`, or `pulse_count x pulse_duration` for a pulsing row).
//! A channelled row (`pulse_count = 0`) holds the lock until its instance
//! is removed.
//!
//! **Resist rolls are not modelled** (D-AB13 waits on the owner): the
//! "Kinetic Resist Roll" effects that gate these in the 2009 data do
//! nothing, so every landed stun or knockdown applies. The
//! `crowd_control_applied` row says so (`resist_roll = "always_pass"`).
//!
//! A landed stun or knockdown also queues an unrolled interrupt
//! (`InterruptCause::Incapacitated`): a warmup the target had started, and
//! its channels, end with the lock. Combat also refuses a stunned player's
//! `useAbility` and consumable use (the "no ability/item use" half of
//! python's `PLAYER_STATE_Stun`).
//!
//! **Interrupt queues, combat resolves.** [`Interrupt`] cannot reach the
//! warmup table, so it queues an `InterruptRequest`; combat rolls the
//! target's `interruptRes` and breaks its warmup and channels
//! (`cimmeria-cell-combat`, `effects::interrupt`).
//!
//! Log target `abilities`.

use std::time::Instant;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::interrupt_request::{InterruptCause, InterruptRequest};
use super::stat_buff::StatBuffRemoval;
use super::{EffectContext, EffectScript};

/// `effect_nvps` name for a stun's or knockdown's length in seconds.
pub const CC_DURATION_NVP: &str = "CcDuration";

/// `effect_nvps` name for an interrupt's chance in percent (100 when the
/// row has none: "Interrupts target").
pub const INTERRUPT_CHANCE_NVP: &str = "InterruptChance";

/// How long a lock from `effect` lasts: `Some(secs)`, or `None` for a
/// channelled row held until removed, or `Some(0.0)` when the row states
/// no length at all.
pub fn lock_duration(effect: &EffectDef) -> Option<f32> {
    let stated = effect.param_f32(CC_DURATION_NVP);
    if stated > 0.0 {
        return Some(stated);
    }
    match effect.pulse_count {
        0 => None,
        1 => Some(effect.pulse_duration.max(0.0)),
        n => Some(n as f32 * effect.pulse_duration.max(0.0)),
    }
}

/// Put a `BSF_MovementLock` entry for `ctx`'s effect on its target.
fn apply_lock(ctx: &mut EffectContext, kind: &'static str) {
    let effect = ctx.effect;
    let who = ctx.space_mgr.player_identity(ctx.source_id);
    let target_who = ctx.space_mgr.player_identity(ctx.target_id);
    let duration = lock_duration(effect);
    if duration.is_some_and(|d| d <= 0.0) {
        tracing::warn!(
            target: "abilities",
            event = "crowd_control_skipped",
            reason = "no_duration",
            kind,
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = ctx.source_id,
            target_id = ctx.target_id,
            target_player_id = target_who.player_id,
            effect_id = effect.effect_id,
            ability_id = effect.ability_id,
            "{kind} effect has no CcDuration and no pulse_duration; nothing applied \
             (check the effect's effect_nvps and pulse_duration seed)"
        );
        return;
    }
    let was_locked = ctx
        .space_mgr
        .get_entity(ctx.target_id)
        .is_some_and(|t| t.has_state_flag(BSF_MOVEMENT_LOCK));
    let spec = TimedEffectSpec {
        effect_id: effect.effect_id,
        ability_id: effect.ability_id,
        invoker_id: ctx.source_id,
        effect_flags: effect.flags,
        moniker_ids: ctx.space_mgr.ability_moniker_ids(effect.ability_id),
        state_flags: BSF_MOVEMENT_LOCK,
        duration_secs: duration,
        stacking: TimedStacking::PerSource,
        ..Default::default()
    };
    if ctx
        .space_mgr
        .apply_timed_effect(ctx.target_id, spec, Instant::now())
        .is_none()
    {
        // The ledger logged why (target gone, or dead and clear-on-death).
        return;
    }
    tracing::info!(
        target: "abilities",
        event = "crowd_control_applied",
        kind,
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = ctx.source_id,
        target_id = ctx.target_id,
        target_player_id = target_who.player_id,
        effect_id = effect.effect_id,
        ability_id = effect.ability_id,
        duration_secs = duration.unwrap_or(0.0),
        held = duration.is_none(),
        was_locked,
        resist_roll = "always_pass",
        "{kind} applied: movement locked"
    );
    // An incapacitated entity cannot keep casting: break its warmup and
    // channels too, unrolled (python's `PLAYER_STATE_Stun` is "no ability
    // use"). Combat resolves it in the same flush as the stun.
    queue(ctx, 100, InterruptCause::Incapacitated);
}

/// Take `ctx`'s caster's entry of its effect off the target.
fn remove_lock(ctx: &mut EffectContext) {
    let key = (ctx.effect.effect_id, ctx.source_id);
    let _ = ctx
        .space_mgr
        .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |e| e.key() == key);
}

/// Locks the target's movement and actions for the effect's duration
/// (`BSF_MovementLock`, held by a ledger entry). See the module docs.
pub struct Stun;

impl EffectScript for Stun {
    fn on_apply(&self, ctx: &mut EffectContext) {
        apply_lock(ctx, "stun");
    }

    /// A pulsing or channelled stun's instance ended (expiry, channel
    /// cancel, duel strip): its entry comes off with it.
    fn on_remove(&self, ctx: &mut EffectContext) {
        remove_lock(ctx);
    }
}

/// A knockdown: the stun's lock under its own effect and duration.
pub struct Knockdown;

impl EffectScript for Knockdown {
    fn on_apply(&self, ctx: &mut EffectContext) {
        apply_lock(ctx, "knockdown");
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        remove_lock(ctx);
    }
}

/// Queue an interrupt of `ctx`'s target by `ctx`'s effect at `chance_pct`.
pub fn queue_interrupt(ctx: &mut EffectContext, chance_pct: i32) {
    queue(ctx, chance_pct, InterruptCause::Effect);
}

fn queue(ctx: &mut EffectContext, chance_pct: i32, cause: InterruptCause) {
    let request = InterruptRequest {
        source_id: ctx.source_id,
        target_id: ctx.target_id,
        effect_id: ctx.effect.effect_id,
        ability_id: ctx.effect.ability_id,
        chance_pct,
        cause,
        // `request_interrupt` assigns it.
        nonce: 0,
    };
    tracing::debug!(
        target: "abilities",
        event = "interrupt_requested",
        entity_id = ctx.source_id,
        target_id = ctx.target_id,
        effect_id = request.effect_id,
        ability_id = request.ability_id,
        chance_pct,
        cause = ?cause,
        "interrupt queued for combat to resolve"
    );
    ctx.space_mgr.request_interrupt(request);
}

/// The interrupt chance `effect` states, if it carries
/// [`INTERRUPT_CHANCE_NVP`].
pub fn stated_interrupt_chance(effect: &EffectDef) -> Option<i32> {
    effect
        .params
        .contains_key(INTERRUPT_CHANCE_NVP)
        .then(|| effect.param_i32(INTERRUPT_CHANCE_NVP))
}

/// Breaks the target's warmup and channels ("Interrupts target", effect 723
/// of Interrupting Shot), unless its interrupt resistance holds.
pub struct Interrupt;

impl EffectScript for Interrupt {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let chance = stated_interrupt_chance(ctx.effect).unwrap_or(100);
        queue_interrupt(ctx, chance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::test_fixtures::make_mgr_with_target;
    use crate::cell::effects::{dispatch_by_name, dispatch_on_remove};
    use crate::cell::space_manager::SpaceManager;
    use std::collections::HashMap;

    fn lock(script: &str, effect_id: i32, secs: f32) -> EffectDef {
        EffectDef {
            effect_id,
            ability_id: 1355,
            script_name: Some(script.to_string()),
            pulse_count: 1,
            pulse_duration: secs,
            flags: 68,
            ..Default::default()
        }
    }

    fn run(mgr: &mut SpaceManager, effect: &EffectDef, source: u32, remove: bool) {
        let name = effect.script_name.clone().unwrap();
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        if remove {
            dispatch_on_remove(&name, &mut ctx);
        } else {
            assert!(dispatch_by_name(&name, &mut ctx), "{name} registered");
        }
    }

    fn locked(mgr: &SpaceManager) -> bool {
        mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK)
    }

    /// A stun lands as a 5 s ledger entry holding the lock.
    #[test]
    fn a_stun_is_a_timed_ledger_entry_holding_the_lock() {
        let mut mgr = make_mgr_with_target();
        run(&mut mgr, &lock("Stun", 1599, 5.0), 7, false);
        let e = mgr.get_entity(1).unwrap();
        assert!(locked(&mgr));
        assert_eq!(e.stat_buffs.entries.len(), 1);
        let entry = &e.stat_buffs.entries[0];
        assert_eq!((entry.effect_id, entry.invoker_id), (1599, 7));
        assert_eq!(entry.duration_secs, 5.0);
        assert_eq!(entry.state_flags, BSF_MOVEMENT_LOCK);
    }

    /// **Regression guard (the stun leak).** The same caster's stun re-run
    /// three times (a pulse or a re-hit each) then removed once leaves the
    /// lock clear. The old script took a reference per run: two stay and
    /// the lock never clears (`lock after removal` fails).
    #[test]
    fn a_rerun_stun_releases_the_lock_on_its_one_removal() {
        let mut mgr = make_mgr_with_target();
        let stun = lock("Stun", 1599, 5.0);
        for _ in 0..3 {
            run(&mut mgr, &stun, 7, false);
        }
        run(&mut mgr, &stun, 7, true);
        assert!(!locked(&mgr), "lock after removal");
        assert!(mgr.get_entity(1).unwrap().state_flag_counts.is_empty());
    }

    /// Two casters, and a knockdown beside a stun: the lock holds until the
    /// last entry goes.
    #[test]
    fn stuns_and_knockdowns_from_two_sources_hold_until_the_last_goes() {
        let mut mgr = make_mgr_with_target();
        let stun = lock("Stun", 1599, 5.0);
        let knockdown = lock("Knockdown", 2608, 3.0);
        run(&mut mgr, &stun, 7, false);
        run(&mut mgr, &knockdown, 8, false);
        run(&mut mgr, &stun, 7, true);
        assert!(locked(&mgr), "the knockdown still holds it");
        run(&mut mgr, &knockdown, 8, true);
        assert!(!locked(&mgr));
    }

    #[test]
    fn duration_comes_from_the_nvp_then_the_pulse_span() {
        let mut e = lock("Stun", 1, 5.0);
        assert_eq!(lock_duration(&e), Some(5.0));
        e.params = HashMap::from([(CC_DURATION_NVP.to_string(), "8".to_string())]);
        assert_eq!(lock_duration(&e), Some(8.0));
        e.params.clear();
        e.pulse_count = 4;
        e.pulse_duration = 2.0;
        assert_eq!(lock_duration(&e), Some(8.0));
        e.pulse_count = 0;
        assert_eq!(lock_duration(&e), None, "a channel holds it");
    }

    /// No length anywhere: nothing lands, with a WARN naming the effect.
    #[test]
    fn a_stun_with_no_duration_lands_nothing_and_warns() {
        let logs = crate::test_support::LogCapture::install();
        let mut mgr = make_mgr_with_target();
        run(&mut mgr, &lock("Stun", 3200, 0.0), 7, false);
        assert!(!locked(&mgr));
        assert!(mgr.get_entity(1).unwrap().stat_buffs.entries.is_empty());
        let rows: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "crowd_control_skipped"))
            .collect();
        assert_eq!(rows.len(), 1, "one WARN; got {:#?}", logs.all());
        assert_eq!(rows[0].level, tracing::Level::WARN);
        assert_eq!(rows[0].target, "abilities");
        assert!(rows[0].has_field("reason", "no_duration"), "{:#?}", rows[0]);
        assert!(rows[0].has_field("effect_id", "3200"), "{:#?}", rows[0]);
    }

    /// `Interrupt` queues one request at the stated chance (100 by default).
    #[test]
    fn interrupt_queues_a_request_for_combat() {
        let mut mgr = make_mgr_with_target();
        let mut effect = lock("Interrupt", 723, 0.0);
        effect.ability_id = 657;
        run(&mut mgr, &effect, 7, false);
        effect
            .params
            .insert(INTERRUPT_CHANCE_NVP.to_string(), "40".to_string());
        run(&mut mgr, &effect, 7, false);
        let queued = mgr.take_all_interrupt_requests();
        assert_eq!(
            queued
                .iter()
                .map(|r| (r.source_id, r.target_id, r.effect_id, r.chance_pct))
                .collect::<Vec<_>>(),
            vec![(7, 1, 723, 100), (7, 1, 723, 40)]
        );
        assert!(queued.iter().all(|r| r.cause == InterruptCause::Effect));
    }

    /// A landed stun queues an unrolled interrupt of the target's cast.
    #[test]
    fn a_landed_stun_queues_an_incapacitating_interrupt() {
        let mut mgr = make_mgr_with_target();
        run(&mut mgr, &lock("Knockdown", 2608, 5.0), 7, false);
        let queued = mgr.take_all_interrupt_requests();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].cause, InterruptCause::Incapacitated);
        assert_eq!((queued[0].source_id, queued[0].target_id), (7, 1));
        // Nothing lands, nothing is queued.
        run(&mut mgr, &lock("Stun", 3200, 0.0), 7, false);
        assert!(mgr.take_all_interrupt_requests().is_empty());
    }
}
