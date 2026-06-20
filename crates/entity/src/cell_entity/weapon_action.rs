//! Pending weapon-action timer state reset.
//!
//! The cell entity carries a cluster of `pending_*_at` deadlines that drive
//! the deferred weapon flows (reload-while-holstered, fire-while-holstered,
//! slot-swap choreography, OOC holster). This module owns the bulk reset
//! that clears them in one shot when the entity transitions to a state
//! where running any of those flows would be wrong (e.g. player death).

use super::CellEntity;

impl CellEntity {
    /// Clear every pending weapon-action timer in one shot.
    ///
    /// Called when the entity transitions to a state where running any of
    /// the deferred weapon flows would be wrong. The current call site is
    /// player death: the cell-side entity survives same-world respawn
    /// (`ReanchorPlayer` keeps it intact), so stale `pending_*_at` fields
    /// would otherwise fire deferred ticks during the Defeat Window or
    /// post-respawn — surfacing as phantom reload animations, queued
    /// attacks against unrelated targets, or slot-swap choreography
    /// running on a corpse.
    ///
    /// Specifically resets:
    /// - `pending_reload_at` — reload-while-holstered Phase A draw window.
    /// - `reload_complete_at` + `reload_slot_id` — reload-while-drawn
    ///   Phase B warmup deadline + pinned slot.
    /// - `pending_attack_at` + `pending_attack_ability_id` +
    ///   `pending_attack_target_id` — fire-while-holstered queued attack.
    /// - `pending_slot_swap_at` + `pending_slot_swap_target` — bandolier
    ///   slot-swap choreography (`Item_Unequip` → wait → `Item_Equip`).
    /// - `holster_animation_complete_at` — OOC holster Phase 2 mesh-drop
    ///   deadline.
    /// - `combat_exit_at` — OOC holster Phase 1 deadline. Combat already
    ///   ended (death cleared `BSF_IN_COMBAT` via the threat fan-out), so
    ///   this is redundant with that path but keeps the surface clean.
    ///
    /// Does NOT clear:
    /// - `weapon_holstered` — the death broadcast path and the respawn
    ///   `ReanchorPlayer` handle the appearance side.
    /// - `threatened_mobs` — held separately and cleared by the
    ///   same-world respawn handler (see
    ///   `cell_methods/player/combat/respawn.rs`).
    /// - `ai_retry_at` / `pending_ai_retries` — NPC-AI fields; the call
    ///   site is gated on `is_player`, so these are structurally
    ///   unreachable here. If the gate is ever relaxed, audit NPC AI
    ///   timers separately rather than bolting them onto this helper.
    pub fn clear_weapon_action_state(&mut self) {
        self.pending_reload_at = None;
        self.reload_complete_at = None;
        self.reload_slot_id = None;
        self.pending_attack_at = None;
        self.pending_attack_ability_id = None;
        self.pending_attack_target_id = None;
        self.pending_slot_swap_at = None;
        self.pending_slot_swap_target = None;
        self.holster_animation_complete_at = None;
        self.combat_exit_at = None;
    }
}
