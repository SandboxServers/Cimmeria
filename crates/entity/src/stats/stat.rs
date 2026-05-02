//! Single-stat dynamic/base value pair with dirty tracking.

/// A single combat stat with dynamic and base value sets.
///
/// Mirrors `python/cell/SGWBeing.py:40 class Stat`.
#[derive(Debug, Clone)]
pub struct Stat {
    /// Dynamic minimum value (can change from effects).
    pub min: i32,
    /// Dynamic current value (clamped between min and max).
    pub cur: i32,
    /// Dynamic maximum value (can change from buffs/equipment).
    pub max: i32,
    /// Base minimum (changes only on level-up / archetype change).
    pub base_min: i32,
    /// Base current.
    pub base_cur: i32,
    /// Base maximum.
    pub base_max: i32,
    /// True if dynamic values changed since last sync.
    pub dirty: bool,
    /// True if base values changed since last sync.
    pub base_dirty: bool,
}

impl Stat {
    /// Create a new stat with explicit values for all 6 fields.
    pub fn new(min: i32, cur: i32, max: i32, base_min: i32, base_cur: i32, base_max: i32) -> Self {
        Self {
            min, cur, max,
            base_min, base_cur, base_max,
            dirty: false,
            base_dirty: false,
        }
    }

    /// Update dynamic min/current/max and mark dirty.
    pub fn update(&mut self, min: i32, current: i32, max: i32) {
        self.min = min;
        self.cur = current;
        self.max = max;
        self.dirty = true;
    }

    /// Set the dynamic current value, clamping to [min, max].
    /// Returns the new current value.
    pub fn set_current(&mut self, value: i32) -> i32 {
        self.cur = value.clamp(self.min, self.max);
        self.dirty = true;
        self.cur
    }

    /// Change the current value by a relative amount, clamping to [min, max].
    /// Returns the actual change applied.
    pub fn change(&mut self, delta: i32) -> i32 {
        let old = self.cur;
        self.cur = (self.cur + delta).clamp(self.min, self.max);
        self.dirty = true;
        self.cur - old
    }

    /// Change current by a percentage of current value.
    pub fn change_by_percent(&mut self, multiplier: f32) -> i32 {
        let delta = (self.cur as f32 * multiplier).round() as i32;
        self.change(delta)
    }

    /// Change current by a percentage of max value.
    pub fn change_by_max_percent(&mut self, multiplier: f32) -> i32 {
        let delta = (self.max as f32 * multiplier).round() as i32;
        self.change(delta)
    }

    /// Set dynamic maximum. Pulls `min` down with it if it would otherwise
    /// exceed the new max, then clamps `cur` into the resulting `[min, max]`.
    /// Preserves the `min ≤ cur ≤ max` invariant the wire format depends on.
    pub fn set_max(&mut self, max: i32) {
        self.max = max;
        if self.min > self.max {
            self.min = self.max;
        }
        self.cur = self.cur.clamp(self.min, self.max);
        self.dirty = true;
    }

    /// Set dynamic minimum. Pushes `max` up with it if it would otherwise
    /// fall below the new min, then clamps `cur` into the resulting
    /// `[min, max]`. Preserves the `min ≤ cur ≤ max` invariant.
    pub fn set_min(&mut self, min: i32) {
        self.min = min;
        if self.max < self.min {
            self.max = self.min;
        }
        self.cur = self.cur.clamp(self.min, self.max);
        self.dirty = true;
    }

    /// Update base values and mark base_dirty.
    pub fn set_base(&mut self, base_min: i32, base_cur: i32, base_max: i32) {
        self.base_min = base_min;
        self.base_cur = base_cur;
        self.base_max = base_max;
        self.base_dirty = true;
    }

    /// Clear dirty flags after syncing to clients.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        self.base_dirty = false;
    }
}
