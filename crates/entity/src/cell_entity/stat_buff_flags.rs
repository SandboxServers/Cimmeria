//! The timed effect ledger's state-flag payload (ability-mechanics AB-09):
//! an entry that holds `state_field` bits, a stun's or a knockdown's
//! `BSF_MovementLock`.
//!
//! **One reference per entry, for its whole life.** Each bit goes through
//! the counted helpers (`set_state_flag` / `unset_state_flag`, the same
//! counter death and ring transport use), once when the entry goes on and
//! once when it comes off. The ledger keys entries by `(effect, invoker)`,
//! so a script that runs again on a pulse or a same-source re-hit replaces
//! its own entry, releasing the old reference before taking the new one.
//! That is the fix for the stun leak: the old `Stun` script took a
//! reference on every `on_apply` and released one on `on_remove`, so a
//! pulsing or refreshed stun left the counter above zero and the bit set
//! for good. Two stuns from different casters hold two references, and the
//! bit clears when the last one comes off.
//!
//! **The client hears about the change once.** The synchronous ledger
//! cannot send, so it records the `state_field` from before its first
//! unsent change (`StatBuffLedger::state_field_before`); the cell's flush
//! sends `onStateFieldUpdate` when the value really changed. A refresh drops
//! the bit and takes it back in one call and sends nothing.
//!
//! **A hard reset forfeits the holds.** `clear_all_state_flags` (respawn,
//! revive) zeroes every counter; entries that outlive it stop holding their
//! bits, so their later removal cannot release a reference a newer holder
//! took.

use super::CellEntity;

/// The single-bit masks set in `flags`, low bit first.
fn bits(flags: u32) -> impl Iterator<Item = u32> {
    (0..u32::BITS)
        .map(|b| 1u32 << b)
        .filter(move |m| flags & m != 0)
}

impl CellEntity {
    /// Take one counted reference on each bit of `flags` for a new ledger
    /// entry.
    pub(super) fn hold_ledger_flags(&mut self, flags: u32) {
        for mask in bits(flags) {
            let before = self.state_field;
            if self.set_state_flag(mask) {
                self.note_ledger_state_change(before);
            }
        }
    }

    /// Release the references a removed entry held.
    pub(super) fn release_ledger_flags(&mut self, flags: u32) {
        for mask in bits(flags) {
            let before = self.state_field;
            if self.unset_state_flag(mask) {
                self.note_ledger_state_change(before);
            }
        }
    }

    fn note_ledger_state_change(&mut self, before: u32) {
        if self.stat_buffs.state_field_before.is_none() {
            self.stat_buffs.state_field_before = Some(before);
        }
    }

    /// The `state_field` to send when ledger entries changed it since the
    /// last call, or `None` when nothing changed (or a change was undone in
    /// the same burst). Clears the record either way.
    pub fn take_ledger_state_change(&mut self) -> Option<u32> {
        let before = self.stat_buffs.state_field_before.take()?;
        (before != self.state_field).then_some(self.state_field)
    }

    /// Whether a live ledger entry holds any bit of `mask` (a stun's or a
    /// knockdown's `BSF_MovementLock`, as opposed to death's or ring
    /// transport's).
    pub fn holds_ledger_flag(&self, mask: u32) -> bool {
        self.stat_buffs
            .entries
            .iter()
            .any(|e| e.state_flags & mask != 0)
    }

    /// Drop every entry's flag holds without touching the counters: the
    /// counters were just reset by [`CellEntity::clear_all_state_flags`].
    pub(super) fn forfeit_ledger_flag_holds(&mut self) {
        for e in &mut self.stat_buffs.entries {
            e.state_flags = 0;
        }
        self.stat_buffs.state_field_before = None;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use cimmeria_common::{EntityId, SpaceId, Vector3};

    use super::super::{CellEntity, TimedEffectSpec, TimedStacking};
    use crate::stats::MOVEMENT_SPEED_MOD;

    /// `BSF_MovementLock` (`cimmeria_wire::state_field`, bit 6); the entity
    /// crate does not depend on the wire crate.
    const LOCK: u32 = 1 << 6;

    fn stun(invoker_id: u32, effect_id: i32) -> TimedEffectSpec {
        TimedEffectSpec {
            effect_id,
            ability_id: 1355,
            invoker_id,
            effect_flags: 64,
            state_flags: LOCK,
            duration_secs: Some(5.0),
            stacking: TimedStacking::PerSource,
            ..Default::default()
        }
    }

    fn entity() -> CellEntity {
        CellEntity::new(EntityId(9), SpaceId(1), Vector3::zero())
    }

    fn refs(e: &CellEntity) -> u32 {
        e.state_flag_counts.get(&LOCK).copied().unwrap_or(0)
    }

    /// **Regression guard (the stun leak).** The same caster's stun applied
    /// three times (the hit and two pulses, or three re-hits) holds one
    /// reference, and its removal clears the bit. Taking a reference per
    /// apply, as the old `Stun` script did, leaves the counter at 2 and the
    /// bit set after the removal (`refs after removal` fails).
    #[test]
    fn a_refreshed_stun_holds_one_reference_and_clears_on_removal() {
        let mut e = entity();
        let now = Instant::now();
        for _ in 0..3 {
            e.apply_timed_effect(stun(1, 1599), now).unwrap();
        }
        assert_eq!(e.stat_buffs.entries.len(), 1);
        assert_eq!(refs(&e), 1, "refs while stunned");
        assert!(e.has_state_flag(LOCK));

        e.remove_timed_effects_where(|t| t.effect_id == 1599);
        assert_eq!(refs(&e), 0, "refs after removal");
        assert!(!e.has_state_flag(LOCK), "the lock clears with the stun");
    }

    /// Two casters' stuns stack: the bit stays until the second comes off,
    /// even when the first caster refreshed theirs in between. A reference
    /// per application (the old script) leaves 3 and the bit set after both
    /// are gone (`lock after both` fails).
    #[test]
    fn two_casters_stuns_keep_the_lock_until_both_go() {
        let mut e = entity();
        let now = Instant::now();
        e.apply_timed_effect(stun(1, 1599), now).unwrap();
        e.apply_timed_effect(stun(2, 1599), now).unwrap();
        e.apply_timed_effect(stun(1, 1599), now).unwrap(); // caster 1 re-hits
        assert_eq!(refs(&e), 2, "one per caster");
        e.remove_timed_effects_where(|t| t.invoker_id == 1);
        assert!(e.has_state_flag(LOCK), "the other caster's stun holds it");
        e.remove_timed_effects_where(|t| t.invoker_id == 2);
        assert!(!e.has_state_flag(LOCK), "lock after both");
        assert_eq!(refs(&e), 0);
    }

    /// A knockdown and a stun are different effects on the same lock, each
    /// with its own duration; and a lock someone else holds (death) is not
    /// released by the ledger.
    #[test]
    fn the_ledger_never_releases_a_reference_it_did_not_take() {
        let mut e = entity();
        e.set_state_flag(LOCK); // death's reference
        let now = Instant::now();
        // Applied, refreshed twice, removed: the ledger's references net to
        // zero and death's one is left. A reference per application leaves 3
        // (`refs` fails); releasing with a raw bit clear drops death's.
        for _ in 0..3 {
            e.apply_timed_effect(stun(1, 2608), now).unwrap();
        }
        e.remove_timed_effects_where(|_| true);
        assert!(e.has_state_flag(LOCK), "death's reference survives");
        assert_eq!(refs(&e), 1);
    }

    /// The client is told once per real change: a refresh changes nothing.
    #[test]
    fn a_refresh_owes_no_state_field_update() {
        let mut e = entity();
        let now = Instant::now();
        e.apply_timed_effect(stun(1, 1599), now).unwrap();
        assert_eq!(e.take_ledger_state_change(), Some(LOCK));
        e.apply_timed_effect(stun(1, 1599), now + Duration::from_secs(1))
            .unwrap();
        assert_eq!(e.take_ledger_state_change(), None, "refresh: same bits");
        e.remove_timed_effects_where(|_| true);
        assert_eq!(e.take_ledger_state_change(), Some(0));
        assert_eq!(e.stat_buffs.state_field_before, None);
    }

    /// Respawn's hard reset forfeits the holds: an entry that outlives it
    /// does not release a reference a later stun took.
    #[test]
    fn a_hard_reset_forfeits_the_entries_holds() {
        let mut e = entity();
        let now = Instant::now();
        e.apply_timed_effect(stun(1, 1599), now).unwrap();
        e.clear_all_state_flags();
        e.apply_timed_effect(stun(2, 2736), now).unwrap();
        e.remove_timed_effects_where(|t| t.invoker_id == 1);
        assert!(e.has_state_flag(LOCK), "the newer stun still holds it");
    }

    /// `PerEffect`: a second invoker's entry of the same effect replaces the
    /// first instead of stacking (the Tranquilizer dart's rule).
    #[test]
    fn per_effect_stacking_keeps_one_entry_per_effect() {
        let mut e = entity();
        let now = Instant::now();
        let slow = |invoker_id| TimedEffectSpec {
            effect_id: 9142,
            invoker_id,
            stats: vec![(MOVEMENT_SPEED_MOD, -40)],
            duration_secs: Some(6.0),
            stacking: TimedStacking::PerEffect,
            ..Default::default()
        };
        e.apply_timed_effect(slow(1), now).unwrap();
        e.apply_timed_effect(slow(2), now).unwrap();
        assert_eq!(e.stat_buffs.entries.len(), 1);
        assert_eq!(e.stats.get(MOVEMENT_SPEED_MOD).unwrap().cur, 60);
        e.remove_timed_effects_where(|_| true);
        assert_eq!(e.stats.get(MOVEMENT_SPEED_MOD).unwrap().cur, 100);
    }
}
