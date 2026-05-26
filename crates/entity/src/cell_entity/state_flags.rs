//! Ref-counted state-field bit manipulation helpers.
//!
//! Mirror python's per-flag counter pattern (`SGWBeing.py:697-734` for the
//! generic `combatantStates` map; `:770-787` for the dedicated movement-lock
//! counter). Two stuns from different abilities both bump the BSF_MovementLock
//! counter to 2; clearing one drops it to 1 and the bit STAYS set until the
//! second clear drains the counter.
//!
//! **When to use these helpers vs raw bitmask ops:**
//!
//!   - **Use the helpers** for flags with multiple potential set/unset
//!     sources that don't coordinate, where a single source clearing
//!     would silently drop the others' refs. Today: `BSF_DEAD` (death/
//!     respawn pair), `BSF_MOVEMENT_LOCK` (death + future stun/cast/fear).
//!
//!   - **Raw `|=` / `&=` is fine** for flags that are either (a) driven
//!     by an idempotent player input (BSF_CROUCHING from `setCrouched` —
//!     clicking twice should set, not bump), or (b) externally managed
//!     via a separate dedup mechanism (BSF_IN_COMBAT gated on
//!     `threatened_mobs` non-empty in `combat::threat`).
//!
//! Mixing the two patterns on the same flag will desync the counter from
//! the bit: a raw `|=` doesn't bump the counter, so the next `unset_*`
//! helper sees count==0, takes the no-op branch, and **does not clear the
//! bit** — a real production hazard. If you migrate a flag to the helpers,
//! migrate ALL its writers in the same change, and force-reset via
//! `clear_all_state_flags` on any hard-reset path (respawn, world entry).

use super::CellEntity;

impl CellEntity {
    /// Increment the per-flag counter and set the bit on a 0->1 transition.
    /// Returns `true` when the bit transitioned (caller should send
    /// `onStateFieldUpdate`); `false` when the flag was already set by a
    /// prior source.
    pub fn set_state_flag(&mut self, mask: u32) -> bool {
        debug_assert!(
            mask.count_ones() == 1,
            "state_flag helpers require single-bit masks (got {mask:#x}) — multi-bit masks would conflate counts across independent flags"
        );
        let count = self.state_flag_counts.entry(mask).or_insert(0);
        *count += 1;
        if self.state_field & mask == 0 {
            self.state_field |= mask;
            true
        } else {
            false
        }
    }

    /// Decrement the per-flag counter and clear the bit on a 1->0 transition.
    /// Returns `true` when the bit transitioned (caller should send
    /// `onStateFieldUpdate`); `false` when other sources are still holding
    /// the flag set, when no source has set it, or when the bit is clear.
    ///
    /// **A best-effort clear (no prior `set_state_flag`) is a silent no-op.**
    /// The earlier version warned + inserted a 0-entry into the counter map
    /// on every stray clear, which (a) leaked map entries on hot paths like
    /// `npc_ai_leash` that defensively unset flags they may not own, and
    /// (b) buried real desync warnings under the noise. If a caller mixes
    /// raw `|=` with `unset_state_flag`, the bit stays stuck and the
    /// debug_assert on misuse below isn't enough to catch it — that's a
    /// project-policy issue documented in the helper-block doc above.
    pub fn unset_state_flag(&mut self, mask: u32) -> bool {
        debug_assert!(
            mask.count_ones() == 1,
            "state_flag helpers require single-bit masks (got {mask:#x})"
        );
        // Use `get_mut` so missing keys aren't materialized — best-effort
        // clears on flags this entity has never owned should be a no-op,
        // not a map-growth event.
        let Some(count) = self.state_flag_counts.get_mut(&mask) else {
            return false;
        };
        if *count == 0 {
            return false;
        }
        *count -= 1;
        if *count == 0 {
            // Drained the last ref — drop the entry rather than leaving a
            // 0-count straggler so the map stays bounded by the set of
            // flags currently held, not the set ever touched.
            self.state_flag_counts.remove(&mask);
            if self.state_field & mask != 0 {
                self.state_field &= !mask;
                return true;
            }
        }
        false
    }

    /// Force-clear all state flags and counters. Used by respawn paths
    /// where the entity returns to a known-clean state regardless of how
    /// many sources had previously set things. Bypasses ref-counting on
    /// purpose — respawn is a hard reset, not a per-source unwind.
    pub fn clear_all_state_flags(&mut self) {
        self.state_field = 0;
        self.state_flag_counts.clear();
    }

    /// Convenience read: is the given flag bit set?
    pub fn has_state_flag(&self, mask: u32) -> bool {
        self.state_field & mask != 0
    }
}
