//! Bandolier ammo read/write helpers.
//!
//! Per-slot ammo lives on `BandolierItem.current_ammo` and is mirrored to
//! the `Stat[AMMO_SLOT_1+slot]` map. These helpers are the read/write path
//! for fire, reload, slot swap, and ammo-change; the shadow scalars that
//! used to live on `CellEntity` were removed in Stage C.

use super::CellEntity;

impl CellEntity {
    /// Read the active slot's current ammo, or 0 if no item equipped.
    pub fn active_ammo(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.current_ammo)
    }

    /// Read the active slot's clip size, or 0 if no item equipped.
    pub fn active_clip_size(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.clip_size)
    }

    /// Read the active slot's selected ammo type, or 0 if no item equipped.
    pub fn active_ammo_type(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.cur_ammo_type)
    }

    /// Set ammo for a slot, mirroring to the AmmoSlot{N} stat. Returns the
    /// clamped value, or `None` if the slot is unequipped.
    ///
    /// Marks the slot dirty in `bandolier_ammo_dirty` for batched persistence.
    pub fn set_slot_ammo(&mut self, slot_id: i32, current: i32) -> Option<i32> {
        let item = self.bandolier_items.get_mut(&slot_id)?;
        item.current_ammo = current.clamp(0, item.clip_size);
        let clamped = item.current_ammo;
        let stat_id = crate::stats::AMMO_SLOT_1 + slot_id;
        if let Some(stat) = self.stats.get_mut(stat_id) {
            stat.set_current(clamped);
        }
        self.bandolier_ammo_dirty.insert(slot_id);
        Some(clamped)
    }

    /// Refill the active slot's magazine to its `clip_size`. Returns the new
    /// ammo value, or `None` if no slot is equipped.
    pub fn refill_active_slot(&mut self) -> Option<i32> {
        let slot = self.active_bandolier_slot;
        let max = self.bandolier_items.get(&slot).map(|i| i.clip_size)?;
        self.set_slot_ammo(slot, max)
    }
}
