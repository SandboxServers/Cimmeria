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
/// `cimmeria_cell::cell::service::ticks::auto_cycle_tick` for the driver
/// loop.
///
/// Not persisted: every login starts with the loop off (owner decision
/// 2026-10-03). #412 had saved it to `sgw_player.state_field` as a player
/// preference, but the server clears it on every kill, so a saved "on" was
/// a loop the player had already watched switch off; and the legacy
/// `SGWBeing.def` declares `bStateField` `CELL_PUBLIC` with no
/// `<Persistent/>`. No `state_field` bit is persisted.
///
/// From python `Atrea.enums.BSF_AutoCycling = 1`.
pub const BSF_AUTO_CYCLING: u32 = 1 << 1;

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

/// Every `EStateField` token, as a mask, for log lines (`state_flags_names`,
/// NT-31). Bit 8 (`BSF_Holster`) is named though the client ignores it and
/// the server no longer sets it: a log that shows it points at whatever
/// stale path wrote it.
pub const STATE_FLAGS: cimmeria_common::flag_names::FlagSet =
    cimmeria_common::flag_names::FlagSet::new(&[
        (1 << 0, "BSF_Dead"),
        (1 << 1, "BSF_AutoCycling"),
        (1 << 2, "BSF_Crouching"),
        (1 << 3, "BSF_InCombat"),
        (1 << 4, "BSF_PlayingMinigame"),
        (1 << 5, "BSF_InStealth"),
        (1 << 6, "BSF_MovementLock"),
        (1 << 7, "BSF_Walking"),
        (1 << 8, "BSF_Holster"),
    ]);

#[cfg(test)]
mod tests {
    use super::*;

    /// `EStateField` tokens are bit indices; the table holds masks.
    #[test]
    fn state_flags_match_enumerations_xml() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../entities/defs/enumerations.xml"
        );
        let xml = std::fs::read_to_string(path).expect("read enumerations.xml");
        let start = xml.find("<EStateField>").expect("EStateField");
        let end = start + xml[start..].find("</EStateField>").expect("closing tag");
        let client: Vec<(u64, String)> = xml[start..end]
            .split("<Token>")
            .skip(1)
            .map(|t| {
                let field = |tag: &str| {
                    let a = t.find(&format!("<{tag}>")).unwrap() + tag.len() + 2;
                    let b = t.find(&format!("</{tag}>")).unwrap();
                    t[a..b].trim().to_owned()
                };
                let bit: u32 = field("Value").parse().expect("bit index");
                (1u64 << bit, field("Name"))
            })
            .collect();
        let ours: Vec<(u64, String)> = STATE_FLAGS
            .entries()
            .iter()
            .map(|&(m, n)| (m, n.to_owned()))
            .collect();
        assert_eq!(ours, client);
    }

    #[test]
    fn state_flags_render_the_masks_the_server_sets() {
        let word = BSF_DEAD | BSF_MOVEMENT_LOCK;
        assert_eq!(
            STATE_FLAGS.render(word).to_string(),
            "BSF_Dead|BSF_MovementLock"
        );
        assert_eq!(STATE_FLAGS.render(1u32 << 9).to_string(), "0x200");
    }
}
