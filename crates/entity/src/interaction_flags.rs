//! `EInteractionNotificationType` bit constants — the UINT64 bitfield that
//! controls every right-click cursor, vendor tab, quest indicator, and loot
//! glow on the client. Canonical source: [entities/defs/enumerations.xml].
//!
//! Stored on the cell entity as `interaction_type_flags: i64`. **Bit 63
//! (`INT_MissionLoot = 9223372036854775808`) is not expressible as i64** and
//! is intentionally omitted here — see issue #97 for the widening proposal.
//! The other high bits (`INT_NormalLoot` at 2^62 and below) are valid
//! positive i64 and are included.
//!
//! These constants exist so chain authors can write:
//! ```sql
//! INSERT INTO content_actions (chain_id, action_type, target_key, params, ...)
//! VALUES (1034, 'set_interaction_type', 'HackTheRings_Switch',
//!         '{"op": "|", "mask": "INT_MinigameLivewire"}', ...);
//! ```
//! instead of the magic-number form `'{"op": "|", "mask": 256}'`. The chain
//! loader resolves the symbolic name via [`mask_for_name`] at parse time.

/// Banker NPC interaction (2^1).
pub const INT_BANKER: i64 = 2;
/// Auction house (2^2).
pub const INT_AUCTION: i64 = 4;
/// PvP marker (2^3).
pub const INT_PVP: i64 = 8;
/// Stargate dial-home device (2^4).
pub const INT_DHD: i64 = 16;
/// Ring transporter network (2^5).
pub const INT_RING_NETWORK: i64 = 32;
/// Organization (guild) NPC (2^6).
pub const INT_ORGANIZATION: i64 = 64;
/// Ability trainer (2^7).
pub const INT_TRAINER: i64 = 128;

/// Livewire minigame interaction (2^8).
pub const INT_MINIGAME_LIVEWIRE: i64 = 256;
/// Activate minigame (2^9).
pub const INT_MINIGAME_ACTIVATE: i64 = 512;
/// Analyze minigame (2^10).
pub const INT_MINIGAME_ANALYZE: i64 = 1024;
/// Bypass minigame (2^11).
pub const INT_MINIGAME_BYPASS: i64 = 2048;
/// Converse minigame (2^12).
pub const INT_MINIGAME_CONVERSE: i64 = 4096;

/// Armor vendor (2^13).
pub const INT_VENDOR_ARMOR: i64 = 8192;
/// Weapons vendor (2^14).
pub const INT_VENDOR_WEAPONS: i64 = 16384;
/// Consumables vendor (2^15).
pub const INT_VENDOR_CONSUMABLES: i64 = 32768;
/// General-goods vendor (2^16).
pub const INT_VENDOR_GENERAL: i64 = 65536;
/// Mission-reward vendor (2^17).
pub const INT_VENDOR_MISSION: i64 = 131072;
/// Crafting (Bio) vendor (2^18).
pub const INT_VENDOR_CRAFT_BIO: i64 = 262144;
/// Crafting (Power) vendor (2^19).
pub const INT_VENDOR_CRAFT_POWER: i64 = 524288;
/// Crafting (Materials) vendor (2^20).
pub const INT_VENDOR_CRAFT_MATERIALS: i64 = 1048576;
/// Crafting (Electronics) vendor (2^21).
pub const INT_VENDOR_CRAFT_ELECTRONICS: i64 = 2097152;

/// Story mission pending (2^22).
pub const INT_A_STORY_MISSION_PENDING: i64 = 4194304;
/// Story mission available (2^23).
pub const INT_A_STORY_MISSION_AVAILABLE: i64 = 8388608;
/// Story mission active (2^24).
pub const INT_A_STORY_MISSION_ACTIVE: i64 = 16777216;
/// Story mission turn-in (2^25).
pub const INT_A_STORY_MISSION_TURN_IN: i64 = 33554432;
/// Side mission pending (2^26).
pub const INT_NON_A_STORY_MISSION_PENDING: i64 = 67108864;
/// Side mission available (2^27).
pub const INT_NON_A_STORY_MISSION_AVAILABLE: i64 = 134217728;
/// Side mission active (2^28).
pub const INT_NON_A_STORY_MISSION_ACTIVE: i64 = 268435456;
/// Side mission turn-in (2^29).
pub const INT_NON_A_STORY_MISSION_TURN_IN: i64 = 536870912;

/// World object that's part of a mission (2^30).
pub const INT_MISSION_WORLD_OBJECT: i64 = 1073741824;
/// Mission waypoint marker (2^31).
pub const INT_MISSION_WAYPOINT: i64 = 2147483648;
/// Dross pile (2^32).
pub const INT_DROSS_PILE: i64 = 4294967296;

/// Poor-cover attackable (2^53).
pub const INT_ATTACKABLE_IN_POOR_COVER: i64 = 9007199254740992;
/// Normal-cover attackable (2^54).
pub const INT_ATTACKABLE_IN_NORMAL_COVER: i64 = 18014398509481984;
/// Good-cover attackable (2^55).
pub const INT_ATTACKABLE_IN_GOOD_COVER: i64 = 36028797018963968;

/// Reverse-engineering machine (2^56).
pub const INT_MACHINE_REVERSE_ENG: i64 = 72057594037927936;
/// Biomedical machine (2^57).
pub const INT_MACHINE_BIOMEDICAL: i64 = 144115188075855872;
/// Power machine (2^58).
pub const INT_MACHINE_POWER: i64 = 288230376151711744;
/// Materials machine (2^59).
pub const INT_MACHINE_MATERIALS: i64 = 576460752303423488;
/// Electronics machine (2^60).
pub const INT_MACHINE_ELECTRONICS: i64 = 1152921504606846976;

/// Generic attackable target (2^61).
pub const INT_ATTACKABLE: i64 = 2305843009213693952;
/// Normal loot drop on a corpse (2^62).
pub const INT_NORMAL_LOOT: i64 = 4611686018427387904;

// Bit 63 (INT_MissionLoot = 9223372036854775808) is NOT expressible as i64.
// See #97 for the widening discussion.

/// Resolve a symbolic name to its bit value, or `None` for unknown
/// names. The content engine loader (`crates/content-engine/src/
/// loader.rs::convert_action`) treats `None` as a soft authoring error:
/// it logs a `tracing::warn!` and falls back to a mask of `0`, so the
/// resulting `Action::SetInteractionType` runs but has no effect on the
/// flag bits. Future hardening (validating mask names at chain-load
/// time, failing the test suite on unknown names) is tracked in #97.
pub fn mask_for_name(name: &str) -> Option<i64> {
    match name {
        "INT_Banker" => Some(INT_BANKER),
        "INT_Auction" => Some(INT_AUCTION),
        "INT_Pvp" => Some(INT_PVP),
        "INT_Dhd" => Some(INT_DHD),
        "INT_RingNetwork" => Some(INT_RING_NETWORK),
        "INT_Organization" => Some(INT_ORGANIZATION),
        "INT_Trainer" => Some(INT_TRAINER),
        "INT_MinigameLivewire" => Some(INT_MINIGAME_LIVEWIRE),
        "INT_MinigameActivate" => Some(INT_MINIGAME_ACTIVATE),
        "INT_MinigameAnalyze" => Some(INT_MINIGAME_ANALYZE),
        "INT_MinigameBypass" => Some(INT_MINIGAME_BYPASS),
        "INT_MinigameConverse" => Some(INT_MINIGAME_CONVERSE),
        "INT_VendorArmor" => Some(INT_VENDOR_ARMOR),
        "INT_VendorWeapons" => Some(INT_VENDOR_WEAPONS),
        "INT_VendorConsumables" => Some(INT_VENDOR_CONSUMABLES),
        "INT_VendorGeneral" => Some(INT_VENDOR_GENERAL),
        "INT_VendorMission" => Some(INT_VENDOR_MISSION),
        "INT_VendorCraftBio" => Some(INT_VENDOR_CRAFT_BIO),
        "INT_VendorCraftPower" => Some(INT_VENDOR_CRAFT_POWER),
        "INT_VendorCraftMaterials" => Some(INT_VENDOR_CRAFT_MATERIALS),
        "INT_VendorCraftElectronics" => Some(INT_VENDOR_CRAFT_ELECTRONICS),
        "INT_AStoryMissionPending" => Some(INT_A_STORY_MISSION_PENDING),
        "INT_AStoryMissionAvaliable" | "INT_AStoryMissionAvailable" => Some(INT_A_STORY_MISSION_AVAILABLE),
        "INT_AStoryMissionActive" => Some(INT_A_STORY_MISSION_ACTIVE),
        "INT_AStoryMissionTurnIn" => Some(INT_A_STORY_MISSION_TURN_IN),
        "INT_NonAStoryMissionPending" => Some(INT_NON_A_STORY_MISSION_PENDING),
        "INT_NonAStoryMissionAvaliable" | "INT_NonAStoryMissionAvailable" => Some(INT_NON_A_STORY_MISSION_AVAILABLE),
        "INT_NonAStoryMissionActive" => Some(INT_NON_A_STORY_MISSION_ACTIVE),
        "INT_NonAStoryMissionTurnIn" => Some(INT_NON_A_STORY_MISSION_TURN_IN),
        "INT_MissionWorldObject" => Some(INT_MISSION_WORLD_OBJECT),
        "INT_MissionWaypoint" => Some(INT_MISSION_WAYPOINT),
        "INT_DrossPile" => Some(INT_DROSS_PILE),
        "INT_AttackableInPoorCover" | "INT_Attackable_In_Poor_Cover" => Some(INT_ATTACKABLE_IN_POOR_COVER),
        "INT_AttackableInNormalCover" | "INT_Attackable_In_Normal_Cover" => Some(INT_ATTACKABLE_IN_NORMAL_COVER),
        "INT_AttackableInGoodCover" | "INT_Attackable_In_Good_Cover" => Some(INT_ATTACKABLE_IN_GOOD_COVER),
        "INT_Machine_ReverseEng" => Some(INT_MACHINE_REVERSE_ENG),
        "INT_Machine_Biomedical" => Some(INT_MACHINE_BIOMEDICAL),
        "INT_Machine_Power" => Some(INT_MACHINE_POWER),
        "INT_Machine_Materials" => Some(INT_MACHINE_MATERIALS),
        "INT_Machine_Electronics" => Some(INT_MACHINE_ELECTRONICS),
        "INT_Attackable" => Some(INT_ATTACKABLE),
        "INT_NormalLoot" => Some(INT_NORMAL_LOOT),
        // INT_MissionLoot deliberately omitted — see file-level docs.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_constants_resolve_by_name() {
        assert_eq!(mask_for_name("INT_MinigameLivewire"), Some(256));
        assert_eq!(mask_for_name("INT_NormalLoot"), Some(1i64 << 62));
        assert_eq!(mask_for_name("INT_AStoryMissionAvaliable"), Some(8388608));
        // Misspelling tolerance: enumerations.xml uses the historical
        // typo "Avaliable"; we accept both forms.
        assert_eq!(
            mask_for_name("INT_AStoryMissionAvaliable"),
            mask_for_name("INT_AStoryMissionAvailable"),
        );
    }

    #[test]
    fn unknown_name_returns_none() {
        assert_eq!(mask_for_name("INT_Bogus"), None);
        assert_eq!(mask_for_name(""), None);
    }

    #[test]
    fn bit_63_omitted() {
        // Bit 63 (INT_MissionLoot = 9223372036854775808) cannot be
        // represented as i64. Make sure we didn't accidentally try.
        assert_eq!(mask_for_name("INT_MissionLoot"), None);
    }

    #[test]
    fn high_bits_match_python_enum_values() {
        // Spot-check values against entities/defs/enumerations.xml.
        assert_eq!(INT_NORMAL_LOOT, 4_611_686_018_427_387_904);
        assert_eq!(INT_ATTACKABLE, 2_305_843_009_213_693_952);
        assert_eq!(INT_MACHINE_REVERSE_ENG, 72_057_594_037_927_936);
    }
}
