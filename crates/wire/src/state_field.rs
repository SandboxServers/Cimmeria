//! The `stateField` bits (`BSF_*`) the server sends in `onStateFieldUpdate`
//! and persists in `sgw_player.state_field`.
//!
//! From `python/Atrea/enums.py:176-184`. `BSF_*` enum values are bit *indices*;
//! `setStateFlag(flag)` does `stateField |= 1 << flag`, so every constant here
//! is a mask unless its name ends in `_BIT`. The client dispatches on bits 0-7
//! only; see `docs/architecture/state-field-bits.md`.
//!
//! One definition for both services: the cell's combat state, crouch handler
//! and ring transporter, and the base's persisted-bits write, all read these.
//! `cimmeria-services` re-exports them at their old paths
//! (`cell::combat::state`, `cell::cell_methods::combatant`,
//! `cell::ring_transport`).

/// Bit position for dead flag in stateField (sent to client). Matches python
/// `Atrea.enums.BSF_Dead = 0`. The earlier value 13 was a wire-protocol bug
/// that left dead NPCs visible as "attackable" on the client.
pub const BSF_DEAD_BIT: u32 = 0;

/// `BSF_Dead` mask. Single-source today — death always comes from one
/// authoritative kill site, and respawn does a hard `clear_all_state_flags`.
/// Kept as a mask constant (not just the bit index) so callers route through
/// the ref-counted entity helpers consistently with the other BSF_* flags.
pub const BSF_DEAD: u32 = 1 << BSF_DEAD_BIT;

/// `BSF_AutoCycling` mask. The client emits `Event_UI_AutoCycle` on every
/// transition of this bit (verified via the XOR-delta dispatcher at
/// `ghidra://SGW.exe@0x00e01c90` — `TEST BL, 0x2` → `EmitAutoCycleStateChanged`
/// at `0x00e05fb0`). `USGWTargetIndicator` listens to the resulting CME event
/// to highlight the gun-icon button.
///
/// Server-side, the flag is set the moment the player presses the
/// auto-cycle button (`setAutoCycle(1)`) so the client gets immediate
/// visual feedback, independent of whether the loop has had its first
/// ability commit yet. Cleared on stop: `setAutoCycle(0)`, target death,
/// manual fire of a different ability, an
/// `AF_DEACTIVATE_AUTO_CYCLE`-flagged ability firing, target deselect,
/// or dead/despawned target during the loop. See
/// `cimmeria_services::cell::service::ticks::auto_cycle_tick` for the driver
/// loop.
///
/// From python `Atrea.enums.BSF_AutoCycling = 1`.
pub const BSF_AUTO_CYCLING: u32 = 1 << 1;

/// The subset of `state_field` bits that persist across logins.
///
/// `state_field` is mostly transient combat state — `BSF_Dead`,
/// `BSF_InCombat`, `BSF_MovementLock` all describe the current fight and
/// must reset on world entry (relog is a fresh combat slate; see the
/// cooldown-wipe rationale in PR #410). `BSF_AutoCycling` is the
/// exception: it's a player preference toggle the original game kept
/// across sessions, so the `setAutoCycle` handler persists it through
/// `CellToBaseMsg::StateFieldUpdate` and `InitPlayerState` restores it.
///
/// Both the cell-side send site and the base-side DB write mask with
/// this constant, so growing the persisted set is a one-line change
/// here — and a transient bit can never leak into `sgw_player.state_field`
/// even if a send site passes an unmasked value. (#412)
pub const PERSISTED_STATE_FIELD_MASK: u32 = BSF_AUTO_CYCLING;

/// `BSF_Crouching` mask, set and cleared by the `setCrouched` cell method.
/// From python `Atrea.enums.BSF_Crouching = 2`.
pub const BSF_CROUCHING: u32 = 1 << 2;

/// `BSF_InCombat` mask. The client uses this bit to route right-click on
/// selected entities to `useAbility` (auto-attack) instead of `interact`.
/// From python `Atrea.enums.BSF_InCombat = 3`.
///
/// Per-player threat-set management (#92) is the long-term setter; today the
/// flag is set on weapon-fire/reload and cleared on the kill that drops the
/// last (single-target) aggro source.
pub const BSF_IN_COMBAT: u32 = 1 << 3;

/// `BSF_MovementLock` mask. Multi-source flag: death applies it,
/// future stun/fear effects will too. Going through the ref-counted entity
/// helpers means clearing one source doesn't drop the others.
/// From python `Atrea.enums.BSF_MovementLock = 6`.
pub const BSF_MOVEMENT_LOCK: u32 = 1 << 6;
