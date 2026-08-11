use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Nine-level rank hierarchy matching `EORG_RANK_*` from the wire spec.
///
/// The repr(u8) values are pinned to the wire values — the wire sends a UINT8 0–8.
/// Ord is derived so higher rank compares greater (Leader > Officer, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrgRank {
    None = 0,
    Initiate = 1,
    Member = 2,
    SeniorMember = 3,
    Veteran = 4,
    SeniorVeteran = 5,
    Officer = 6,
    SeniorOfficer = 7,
    Leader = 8,
}

impl OrgRank {
    /// Construct from a raw wire byte. Returns `None` if the value is out of range.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Initiate),
            2 => Some(Self::Member),
            3 => Some(Self::SeniorMember),
            4 => Some(Self::Veteran),
            5 => Some(Self::SeniorVeteran),
            6 => Some(Self::Officer),
            7 => Some(Self::SeniorOfficer),
            8 => Some(Self::Leader),
            _ => None,
        }
    }
}

/// Organization type matching `EOrganizationType` (Squad=0, Team=1, Command=2).
///
/// Squad is ephemeral and never persisted. Team and Command are the two
/// `EPersistentOrganizationType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrgType {
    Squad = 0,
    Team = 1,
    Command = 2,
}

impl OrgType {
    /// Returns `true` for the two org types that are persisted to the DB.
    /// Squad is session-only.
    pub fn is_persistent(self) -> bool {
        matches!(self, Self::Team | Self::Command)
    }

    /// Construct from a raw wire byte. Returns `None` if the value is out of range.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Squad),
            1 => Some(Self::Team),
            2 => Some(Self::Command),
            _ => None,
        }
    }
}

/// 26-bit permission bitmask matching `EORG_PERM_*`.
///
/// Stored as a `u32`; all 26 bits fit within the non-negative i32 range
/// (max combined = 0x3FFFFFF ≈ 67 M < 2^31) so the DB `integer` column
/// is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrgPermission(pub u32);

impl OrgPermission {
    pub const DO_NOT_USE: Self = Self(0x0000001);
    pub const INVITE: Self = Self(0x0000002);
    pub const PROMOTE: Self = Self(0x0000004);
    pub const DEMOTE: Self = Self(0x0000008);
    pub const EJECT: Self = Self(0x0000010);
    pub const ROSTER_NOTES: Self = Self(0x0000020);
    pub const OFFICER_NOTES: Self = Self(0x0000040);
    pub const RANK_NAMES: Self = Self(0x0000080);
    pub const OFFICER_CHAT: Self = Self(0x0000100);
    pub const EMAIL_LISTS: Self = Self(0x0000200);
    pub const MOTD: Self = Self(0x0000400);
    pub const HISTORY_LOG: Self = Self(0x0000800);
    pub const CALENDAR: Self = Self(0x0001000);
    pub const RECRUIT_DESC: Self = Self(0x0002000);
    pub const ADJECTIVES: Self = Self(0x0004000);
    pub const INSIGNIA: Self = Self(0x0008000);
    pub const DEPOSIT_BANK: Self = Self(0x0010000);
    pub const WITHDRAW_BANK: Self = Self(0x0020000);
    pub const DEPOSIT_CASH: Self = Self(0x0040000);
    pub const WITHDRAW_CASH: Self = Self(0x0080000);
    pub const VIEW_BANK_LOGS: Self = Self(0x0100000);
    pub const LEADER_CHAT: Self = Self(0x0200000);
    pub const ALLIANCE_CHAT: Self = Self(0x0400000);
    pub const ALTER_PERMS: Self = Self(0x0800000);
    pub const TRANSFER_LEADER: Self = Self(0x1000000);
    pub const ALLIANCE_CMDS: Self = Self(0x2000000);

    /// All 26 permission bits set — useful as the Leader default mask.
    pub const ALL: Self = Self(0x3FFFFFF);

    /// Empty permission set.
    pub const NONE: Self = Self(0);

    /// Returns `true` if `other` is a subset of this mask.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Set (union) the given bits.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clear the given bits.
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Bitwise OR of two permission masks.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return the raw `u32` value for wire/DB storage.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Construct from a raw `u32`, masking to the 26 defined permission bits.
    /// Out-of-range / high bits (e.g. `0xFFFFFFFF` from a buggy or hostile wire
    /// path) are dropped before they can reach a permission check or be persisted.
    pub fn from_bits(v: u32) -> Self {
        Self(v & Self::ALL.0)
    }
}

/// Default permission ladder indexed by `OrgRank as u8` (0 = None, 8 = Leader).
///
/// This is the canonical default for new orgs. Both `Org::new()` and the
/// persistence `create_org` DB seed use this array so the in-memory model
/// and the DB row are always initialised to the same values.
///
/// Values are `u32` permission bitmasks (see `OrgPermission` constants).
pub const DEFAULT_RANK_PERMISSIONS: [u32; 9] = [
    0,                                                             // None (0)
    0,                                                             // Initiate (1)
    OrgPermission::ROSTER_NOTES.0,                                 // Member (2)
    OrgPermission::ROSTER_NOTES.0,                                 // SeniorMember (3)
    OrgPermission::ROSTER_NOTES.0 | OrgPermission::OFFICER_CHAT.0, // Veteran (4)
    OrgPermission::ROSTER_NOTES.0 | OrgPermission::OFFICER_CHAT.0, // SeniorVeteran (5)
    // Officer (6): invite/eject/roster_notes/officer_notes/officer_chat/motd
    OrgPermission::INVITE.0
        | OrgPermission::EJECT.0
        | OrgPermission::ROSTER_NOTES.0
        | OrgPermission::OFFICER_NOTES.0
        | OrgPermission::OFFICER_CHAT.0
        | OrgPermission::MOTD.0,
    // SeniorOfficer (7): adds PROMOTE/DEMOTE/RANK_NAMES
    OrgPermission::INVITE.0
        | OrgPermission::EJECT.0
        | OrgPermission::PROMOTE.0
        | OrgPermission::DEMOTE.0
        | OrgPermission::ROSTER_NOTES.0
        | OrgPermission::OFFICER_NOTES.0
        | OrgPermission::RANK_NAMES.0
        | OrgPermission::OFFICER_CHAT.0
        | OrgPermission::MOTD.0,
    OrgPermission::ALL.0, // Leader (8)
];

/// Per-rank configuration: optional custom display name and permission mask.
///
/// Stored in `sgw.organization_ranks` — one row per (org_id, rank_value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRankConfig {
    /// Custom rank name, if the guild leader has renamed this rank.
    pub custom_name: Option<String>,
    /// Permission bitmask for this rank.
    pub permissions: OrgPermission,
}

impl OrgRankConfig {
    /// Default config for a rank: no custom name, empty permissions.
    pub fn empty() -> Self {
        Self {
            custom_name: None,
            permissions: OrgPermission::NONE,
        }
    }
}

/// A member entry within a persistent organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    /// FK → `sgw_player.player_id`.
    pub player_id: i32,
    /// Wire-visible name for the member.
    pub character_name: String,
    pub rank: OrgRank,
    /// Player-editable public note.
    pub notes: Option<String>,
}

/// A persistent organization (Team or Command). Squad is ephemeral — not modelled here.
///
/// Corresponds to `sgw.organizations` + `sgw.organization_members` + `sgw.organization_ranks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Org {
    pub org_id: i32,
    pub org_type: OrgType,
    pub name: String,
    /// FK → `sgw_player.player_id` for the current leader.
    pub leader_player_id: i32,
    pub motd: Option<String>,
    /// Treasury in the wire's UINT64 denomination. Stored as i64 in Rust/Postgres
    /// (`bigint`) which covers the full positive range a player can accumulate.
    pub cash: i64,
    /// Members keyed by player_id.
    pub members: HashMap<i32, OrgMember>,
    /// Per-rank configuration keyed by `OrgRank as u8`.
    pub rank_configs: HashMap<u8, OrgRankConfig>,
}

impl Org {
    /// Create a new organization with the founding player as Leader.
    pub fn new(
        org_id: i32,
        org_type: OrgType,
        name: String,
        leader_player_id: i32,
        leader_name: String,
    ) -> Self {
        // `Org` is the persistent model (Team/Command); Squad is ephemeral and
        // never built here. Fail fast on misuse during development/testing.
        debug_assert!(
            org_type.is_persistent(),
            "Org::new is persistent-only (Team/Command); Squad must not be persisted"
        );
        let mut members = HashMap::new();
        members.insert(
            leader_player_id,
            OrgMember {
                player_id: leader_player_id,
                character_name: leader_name,
                rank: OrgRank::Leader,
                notes: None,
            },
        );

        // Default rank configs: apply the canonical permission ladder so the
        // in-memory model is consistent with what the DB row will contain after
        // `create_org` seeds the DEFAULT_PERMISSION_LADDER. Each rank starts
        // with no custom name.
        let mut rank_configs = HashMap::new();
        for (rank_value, &perm_bits) in DEFAULT_RANK_PERMISSIONS.iter().enumerate() {
            rank_configs.insert(
                rank_value as u8,
                OrgRankConfig {
                    custom_name: None,
                    permissions: OrgPermission(perm_bits),
                },
            );
        }

        Self {
            org_id,
            org_type,
            name,
            leader_player_id,
            motd: None,
            cash: 0,
            members,
            rank_configs,
        }
    }

    /// Add a new member at Initiate rank.
    pub fn add_member(&mut self, player_id: i32, character_name: String) {
        self.members.insert(
            player_id,
            OrgMember {
                player_id,
                character_name,
                rank: OrgRank::Initiate,
                notes: None,
            },
        );
    }

    /// Remove a member. Returns `false` if the member is the leader or not found.
    pub fn remove_member(&mut self, player_id: i32) -> bool {
        if player_id == self.leader_player_id {
            return false;
        }
        self.members.remove(&player_id).is_some()
    }

    /// Change a member's rank. Returns `false` if the member is not found.
    pub fn set_rank(&mut self, player_id: i32, rank: OrgRank) -> bool {
        if let Some(member) = self.members.get_mut(&player_id) {
            member.rank = rank;
            true
        } else {
            false
        }
    }

    /// Check whether `player_id` holds `perm` via their current rank's config.
    pub fn has_permission(&self, player_id: i32, perm: OrgPermission) -> bool {
        let Some(member) = self.members.get(&player_id) else {
            return false;
        };
        let Some(config) = self.rank_configs.get(&(member.rank as u8)) else {
            return false;
        };
        config.permissions.contains(perm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OrgRank wire-value pins ──────────────────────────────────────────────
    // These must fail if the repr is ever accidentally shuffled.

    #[test]
    fn org_rank_wire_values() {
        assert_eq!(OrgRank::None as u8, 0);
        assert_eq!(OrgRank::Initiate as u8, 1);
        assert_eq!(OrgRank::Member as u8, 2);
        assert_eq!(OrgRank::SeniorMember as u8, 3);
        assert_eq!(OrgRank::Veteran as u8, 4);
        assert_eq!(OrgRank::SeniorVeteran as u8, 5);
        assert_eq!(OrgRank::Officer as u8, 6);
        assert_eq!(OrgRank::SeniorOfficer as u8, 7);
        assert_eq!(OrgRank::Leader as u8, 8);
    }

    #[test]
    fn org_rank_from_u8_round_trip() {
        for v in 0u8..=8 {
            assert!(OrgRank::from_u8(v).is_some(), "rank {v} should be valid");
        }
        assert!(OrgRank::from_u8(9).is_none());
        assert!(OrgRank::from_u8(255).is_none());
    }

    #[test]
    fn org_rank_ordering() {
        assert!(OrgRank::Initiate < OrgRank::Leader);
        assert!(OrgRank::Member < OrgRank::Officer);
        assert!(OrgRank::Officer < OrgRank::SeniorOfficer);
        assert!(OrgRank::SeniorOfficer < OrgRank::Leader);
        assert!(OrgRank::None < OrgRank::Initiate);
        // same rank is not less
        assert!(OrgRank::Leader >= OrgRank::Leader);
    }

    // ── OrgType wire-value pins ──────────────────────────────────────────────

    #[test]
    fn org_type_wire_values() {
        assert_eq!(OrgType::Squad as u8, 0);
        assert_eq!(OrgType::Team as u8, 1);
        assert_eq!(OrgType::Command as u8, 2);
    }

    #[test]
    fn org_type_persistence_flag() {
        assert!(!OrgType::Squad.is_persistent());
        assert!(OrgType::Team.is_persistent());
        assert!(OrgType::Command.is_persistent());
    }

    // ── OrgPermission literal pins ───────────────────────────────────────────
    // Every flag must equal its documented hex value from EORG_PERM_*.
    // These tests will catch any future accidental reordering.

    #[test]
    fn org_permission_flag_values() {
        assert_eq!(OrgPermission::DO_NOT_USE.bits(), 0x0000001);
        assert_eq!(OrgPermission::INVITE.bits(), 0x0000002);
        assert_eq!(OrgPermission::PROMOTE.bits(), 0x0000004);
        assert_eq!(OrgPermission::DEMOTE.bits(), 0x0000008);
        assert_eq!(OrgPermission::EJECT.bits(), 0x0000010);
        assert_eq!(OrgPermission::ROSTER_NOTES.bits(), 0x0000020);
        assert_eq!(OrgPermission::OFFICER_NOTES.bits(), 0x0000040);
        assert_eq!(OrgPermission::RANK_NAMES.bits(), 0x0000080);
        assert_eq!(OrgPermission::OFFICER_CHAT.bits(), 0x0000100);
        assert_eq!(OrgPermission::EMAIL_LISTS.bits(), 0x0000200);
        assert_eq!(OrgPermission::MOTD.bits(), 0x0000400);
        assert_eq!(OrgPermission::HISTORY_LOG.bits(), 0x0000800);
        assert_eq!(OrgPermission::CALENDAR.bits(), 0x0001000);
        assert_eq!(OrgPermission::RECRUIT_DESC.bits(), 0x0002000);
        assert_eq!(OrgPermission::ADJECTIVES.bits(), 0x0004000);
        assert_eq!(OrgPermission::INSIGNIA.bits(), 0x0008000);
        assert_eq!(OrgPermission::DEPOSIT_BANK.bits(), 0x0010000);
        assert_eq!(OrgPermission::WITHDRAW_BANK.bits(), 0x0020000);
        assert_eq!(OrgPermission::DEPOSIT_CASH.bits(), 0x0040000);
        assert_eq!(OrgPermission::WITHDRAW_CASH.bits(), 0x0080000);
        assert_eq!(OrgPermission::VIEW_BANK_LOGS.bits(), 0x0100000);
        assert_eq!(OrgPermission::LEADER_CHAT.bits(), 0x0200000);
        assert_eq!(OrgPermission::ALLIANCE_CHAT.bits(), 0x0400000);
        assert_eq!(OrgPermission::ALTER_PERMS.bits(), 0x0800000);
        assert_eq!(OrgPermission::TRANSFER_LEADER.bits(), 0x1000000);
        assert_eq!(OrgPermission::ALLIANCE_CMDS.bits(), 0x2000000);
    }

    #[test]
    fn org_permission_all_mask() {
        assert_eq!(OrgPermission::ALL.bits(), 0x3FFFFFF);
        // ALL should contain every named flag
        assert!(OrgPermission::ALL.contains(OrgPermission::INVITE));
        assert!(OrgPermission::ALL.contains(OrgPermission::TRANSFER_LEADER));
        assert!(OrgPermission::ALL.contains(OrgPermission::ALLIANCE_CMDS));
    }

    #[test]
    fn org_permission_insert_contains_union() {
        let mut mask = OrgPermission::NONE;
        assert!(!mask.contains(OrgPermission::INVITE));
        mask.insert(OrgPermission::INVITE);
        assert!(mask.contains(OrgPermission::INVITE));
        assert!(!mask.contains(OrgPermission::EJECT));

        let combined = mask.union(OrgPermission::EJECT);
        assert!(combined.contains(OrgPermission::INVITE));
        assert!(combined.contains(OrgPermission::EJECT));
    }

    #[test]
    fn org_permission_remove() {
        let mut mask = OrgPermission::ALL;
        mask.remove(OrgPermission::TRANSFER_LEADER);
        assert!(!mask.contains(OrgPermission::TRANSFER_LEADER));
        assert!(mask.contains(OrgPermission::LEADER_CHAT));
    }

    #[test]
    fn org_permission_round_trip_bits() {
        let raw: u32 = OrgPermission::INVITE.bits()
            | OrgPermission::MOTD.bits()
            | OrgPermission::TRANSFER_LEADER.bits();
        let mask = OrgPermission::from_bits(raw);
        assert!(mask.contains(OrgPermission::INVITE));
        assert!(mask.contains(OrgPermission::MOTD));
        assert!(mask.contains(OrgPermission::TRANSFER_LEADER));
        assert!(!mask.contains(OrgPermission::EJECT));
    }

    #[test]
    fn leader_all_perms_mask_round_trip() {
        let leader = OrgPermission::ALL;
        let raw = leader.bits();
        let reconstructed = OrgPermission::from_bits(raw);
        assert_eq!(leader, reconstructed);
        // Confirm it fits in a positive i32 (DB integer column)
        assert!(raw <= i32::MAX as u32);
    }

    // ── Org model behaviour ──────────────────────────────────────────────────

    #[test]
    fn new_org_has_leader() {
        let o = Org::new(1, OrgType::Command, "SG-1".into(), 10, "O'Neill".into());
        assert_eq!(o.members.len(), 1);
        assert_eq!(o.members[&10].rank, OrgRank::Leader);
        assert_eq!(o.leader_player_id, 10);
    }

    #[test]
    fn cannot_remove_leader() {
        let mut o = Org::new(1, OrgType::Command, "SG-1".into(), 10, "O'Neill".into());
        assert!(!o.remove_member(10));
    }

    #[test]
    fn add_and_promote_member() {
        let mut o = Org::new(1, OrgType::Team, "SG-1".into(), 10, "O'Neill".into());
        o.add_member(11, "Carter".into());
        assert_eq!(o.members[&11].rank, OrgRank::Initiate);
        assert!(o.set_rank(11, OrgRank::Officer));
        assert_eq!(o.members[&11].rank, OrgRank::Officer);
    }

    #[test]
    fn leader_has_all_permissions() {
        let o = Org::new(1, OrgType::Command, "SG-1".into(), 10, "O'Neill".into());
        assert!(o.has_permission(10, OrgPermission::INVITE));
        assert!(o.has_permission(10, OrgPermission::TRANSFER_LEADER));
        assert!(o.has_permission(10, OrgPermission::ALLIANCE_CMDS));
    }

    #[test]
    fn initiate_has_no_permissions_by_default() {
        let mut o = Org::new(1, OrgType::Command, "SG-1".into(), 10, "O'Neill".into());
        o.add_member(11, "Carter".into());
        assert!(!o.has_permission(11, OrgPermission::INVITE));
    }
}
