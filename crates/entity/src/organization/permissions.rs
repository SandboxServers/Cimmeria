//! `EOrganizationPermission` bit flags and the default rank permissions.

use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

use super::types::{OrgRank, OrgType};

/// `EOrganizationPermission` names, for log lines (`from_mask_names`,
/// `to_mask_names`, NT-31).
pub const ORG_PERMISSIONS: cimmeria_common::flag_names::FlagSet =
    cimmeria_common::flag_names::FlagSet::new(&[
        (1, "EORG_PERM_DoNotUse"),
        (2, "EORG_PERM_Invite"),
        (4, "EORG_PERM_Promote"),
        (8, "EORG_PERM_Demote"),
        (16, "EORG_PERM_Eject"),
        (32, "EORG_PERM_RosterNotes"),
        (64, "EORG_PERM_OfficerNotes"),
        (128, "EORG_PERM_RankNames"),
        (256, "EORG_PERM_OfficerChat"),
        (512, "EORG_PERM_EmailLists"),
        (1024, "EORG_PERM_MOTD"),
        (2048, "EORG_PERM_HistoryLog"),
        (4096, "EORG_PERM_Calendar"),
        (8192, "EORG_PERM_RecruitDesc"),
        (16384, "EORG_PERM_Adjectives"),
        (32768, "EORG_PERM_Insignia"),
        (65536, "EORG_PERM_DepositBank"),
        (131072, "EORG_PERM_WithdrawBank"),
        (262144, "EORG_PERM_DepositCash"),
        (524288, "EORG_PERM_WithdrawCash"),
        (1048576, "EORG_PERM_ViewBankLogs"),
        (2097152, "EORG_PERM_LeaderChat"),
        (4194304, "EORG_PERM_AllianceChat"),
        (8388608, "EORG_PERM_AlterPerms"),
        (16777216, "EORG_PERM_TransferLeader"),
        (33554432, "EORG_PERM_AllianceCmds"),
    ]);

/// `EOrganizationPermission` (`enumerations.xml:1907`, UINT32): 26 flag bits.
///
/// Carried as an `INT32` mask by `organizationSetRankPermissions` (CM 16)
/// and `onOrganizationRankUpdate` [49]. [`OrgPermission::from_wire`] keeps
/// only the defined bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OrgPermission(u32);

impl OrgPermission {
    pub const NONE: OrgPermission = OrgPermission(0);
    /// `EORG_PERM_DoNotUse`. Defined but meaningless; only `Leader`'s
    /// all-bits row carries it.
    pub const DO_NOT_USE: OrgPermission = OrgPermission(1);
    pub const INVITE: OrgPermission = OrgPermission(2);
    pub const PROMOTE: OrgPermission = OrgPermission(4);
    pub const DEMOTE: OrgPermission = OrgPermission(8);
    pub const EJECT: OrgPermission = OrgPermission(16);
    pub const ROSTER_NOTES: OrgPermission = OrgPermission(32);
    pub const OFFICER_NOTES: OrgPermission = OrgPermission(64);
    pub const RANK_NAMES: OrgPermission = OrgPermission(128);
    pub const OFFICER_CHAT: OrgPermission = OrgPermission(256);
    pub const EMAIL_LISTS: OrgPermission = OrgPermission(512);
    pub const MOTD: OrgPermission = OrgPermission(1024);
    pub const HISTORY_LOG: OrgPermission = OrgPermission(2048);
    pub const CALENDAR: OrgPermission = OrgPermission(4096);
    pub const RECRUIT_DESC: OrgPermission = OrgPermission(8192);
    pub const ADJECTIVES: OrgPermission = OrgPermission(16384);
    pub const INSIGNIA: OrgPermission = OrgPermission(32768);
    pub const DEPOSIT_BANK: OrgPermission = OrgPermission(65536);
    pub const WITHDRAW_BANK: OrgPermission = OrgPermission(131072);
    pub const DEPOSIT_CASH: OrgPermission = OrgPermission(262144);
    pub const WITHDRAW_CASH: OrgPermission = OrgPermission(524288);
    pub const VIEW_BANK_LOGS: OrgPermission = OrgPermission(1048576);
    pub const LEADER_CHAT: OrgPermission = OrgPermission(2097152);
    pub const ALLIANCE_CHAT: OrgPermission = OrgPermission(4194304);
    pub const ALTER_PERMS: OrgPermission = OrgPermission(8388608);
    /// `EORG_PERM_TransferLeader`. No editor exposes it and no transfer
    /// feature exists, so it stays outside [`OrgPermission::editable_for`]
    /// (D-ORG08).
    pub const TRANSFER_LEADER: OrgPermission = OrgPermission(16777216);
    pub const ALLIANCE_CMDS: OrgPermission = OrgPermission(33554432);

    /// All 26 defined bits: `Leader`'s pinned row (D-ORG08).
    pub const ALL: OrgPermission = OrgPermission(0x3FF_FFFF);

    /// The bits a Team's rank editor exposes (`Team.lua`
    /// `TeamMod.teamPermissions[1..12]`, audit A-12).
    const TEAM_EDITABLE: OrgPermission = OrgPermission(
        Self::INVITE.0
            | Self::PROMOTE.0
            | Self::DEMOTE.0
            | Self::EJECT.0
            | Self::OFFICER_NOTES.0
            | Self::RANK_NAMES.0
            | Self::MOTD.0
            | Self::DEPOSIT_BANK.0
            | Self::WITHDRAW_BANK.0
            | Self::DEPOSIT_CASH.0
            | Self::WITHDRAW_CASH.0
            | Self::ALTER_PERMS.0,
    );

    /// Team's twelve plus `OfficerChat` and `EmailLists` (`Command.lua`
    /// `CommandMod.commandPermissions[1..14]`).
    const COMMAND_EDITABLE: OrgPermission =
        OrgPermission(Self::TEAM_EDITABLE.0 | Self::OFFICER_CHAT.0 | Self::EMAIL_LISTS.0);

    /// Build from raw bits, keeping only the 26 defined ones.
    pub const fn from_bits_truncate(bits: u32) -> OrgPermission {
        OrgPermission(bits & Self::ALL.0)
    }

    /// Build from the `INT32` wire mask. Undefined bits (including the sign
    /// bit) are dropped.
    pub const fn from_wire(mask: i32) -> OrgPermission {
        Self::from_bits_truncate(mask as u32)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The mask as the `INT32` the wire carries. Never negative: the top
    /// defined bit is 25.
    pub const fn to_wire(self) -> i32 {
        self.0 as i32
    }

    /// `true` if every bit of `other` is set in `self`.
    pub const fn contains(self, other: OrgPermission) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The bits the client's rank editor for `org_type` can change: 14 for
    /// Command, 12 for Team, none for Squad (squads have no rank editor).
    /// A new mask from the client is clamped to this set (D-ORG09 (6)).
    pub const fn editable_for(org_type: OrgType) -> OrgPermission {
        match org_type {
            OrgType::Squad => Self::NONE,
            OrgType::Team => Self::TEAM_EDITABLE,
            OrgType::Command => Self::COMMAND_EDITABLE,
        }
    }

    /// Apply a rank-permission edit from the client (CM 16) to a rank's
    /// stored mask `old` (D-ORG09 (6), semantics D-ORG22).
    ///
    /// Only the bits the type's editor exposes can change:
    /// `stored = (old & !editable) | (wire & editable)`, so a bit the editor
    /// does not show (`RosterNotes`, `ViewBankLogs`, ...) keeps its stored
    /// value whatever the client sends. The edit is rejected when any bit it
    /// actually changes, granted or revoked, is one `actor` does not hold;
    /// an unheld bit left as it was is fine.
    ///
    /// The `Leader` row is never editable, and this function does not know
    /// the rank: the caller refuses an edit of the `Leader` row before
    /// calling it (D-ORG08).
    pub fn apply_edit(
        old: OrgPermission,
        wire: OrgPermission,
        org_type: OrgType,
        actor: OrgPermission,
    ) -> Result<OrgPermission, PermEditReject> {
        let editable = Self::editable_for(org_type);
        let stored = OrgPermission((old.0 & !editable.0) | (wire.0 & editable.0));
        let unheld = (stored.0 ^ old.0) & !actor.0;
        if unheld != 0 {
            return Err(PermEditReject::ChangesUnheldBits(OrgPermission(unheld)));
        }
        Ok(stored)
    }
}

/// Why [`OrgPermission::apply_edit`] refused an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermEditReject {
    /// The edit changes these bits, which the editing actor does not hold.
    ChangesUnheldBits(OrgPermission),
}

impl PermEditReject {
    /// Stable value for the `reason` log field.
    pub fn reason(&self) -> &'static str {
        match self {
            PermEditReject::ChangesUnheldBits(_) => "changes_unheld_bits",
        }
    }
}

impl BitOr for OrgPermission {
    type Output = OrgPermission;

    fn bitor(self, rhs: OrgPermission) -> OrgPermission {
        OrgPermission(self.0 | rhs.0)
    }
}

impl BitOrAssign for OrgPermission {
    fn bitor_assign(&mut self, rhs: OrgPermission) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for OrgPermission {
    type Output = OrgPermission;

    fn bitand(self, rhs: OrgPermission) -> OrgPermission {
        OrgPermission(self.0 & rhs.0)
    }
}

/// Complement within the 26 defined bits.
impl Not for OrgPermission {
    type Output = OrgPermission;

    fn not(self) -> OrgPermission {
        OrgPermission(!self.0 & Self::ALL.0)
    }
}

/// The vault bits every default rank below `Leader` holds (D-ORG21):
/// deposit items and cash, and read the bank log. Withdrawing is opt-in, so
/// no default row below `Leader` has `WithdrawBank` or `WithdrawCash`.
const BANK_DEFAULT: OrgPermission = OrgPermission(
    OrgPermission::DEPOSIT_BANK.0 | OrgPermission::DEPOSIT_CASH.0 | OrgPermission::VIEW_BANK_LOGS.0,
);

/// `Officer`'s default bits (D-ORG08, vault bits per D-ORG21), also Team
/// `SeniorMember`'s.
const OFFICER_DEFAULT: OrgPermission = OrgPermission(
    BANK_DEFAULT.0
        | OrgPermission::INVITE.0
        | OrgPermission::EJECT.0
        | OrgPermission::ROSTER_NOTES.0
        | OrgPermission::OFFICER_NOTES.0
        | OrgPermission::OFFICER_CHAT.0
        | OrgPermission::MOTD.0,
);

/// `SeniorOfficer`: `Officer` plus Promote, Demote, RankNames and AlterPerms.
const SENIOR_OFFICER_DEFAULT: OrgPermission = OrgPermission(
    OFFICER_DEFAULT.0
        | OrgPermission::PROMOTE.0
        | OrgPermission::DEMOTE.0
        | OrgPermission::RANK_NAMES.0
        | OrgPermission::ALTER_PERMS.0,
);

/// `SeniorVeteran` down to `Member`: roster notes and the vault bits.
const MEMBER_DEFAULT: OrgPermission = OrgPermission(BANK_DEFAULT.0 | OrgPermission::ROSTER_NOTES.0);

/// The permissions each rank starts with when a Team or Command is created
/// (D-ORG08 as amended by D-ORG21), one row per rank in
/// [`OrgRank::for_type`], lowest first.
///
/// Project policy, not recovered data: the client ships no defaults. Both
/// the Rust creation path and any seed read this one table. `Leader` always
/// holds [`OrgPermission::ALL`], and no editor may change that row. Every
/// other rank, `Initiate` included, may deposit and read the bank log;
/// none may withdraw until an editor grants it (D-ORG21).
///
/// Squads return no rows: they are never persisted, have no rank editor,
/// and are authorized by leadership alone (D-ORG16).
pub fn default_rank_permissions(org_type: OrgType) -> Vec<(OrgRank, OrgPermission)> {
    let for_rank = |rank: OrgRank| -> OrgPermission {
        match (org_type, rank) {
            (_, OrgRank::LEADER) => OrgPermission::ALL,
            (OrgType::Command, OrgRank::SENIOR_OFFICER) => SENIOR_OFFICER_DEFAULT,
            (OrgType::Command, OrgRank::OFFICER) => OFFICER_DEFAULT,
            (OrgType::Team, OrgRank::SENIOR_MEMBER) => OFFICER_DEFAULT,
            (_, OrgRank::INITIATE) => BANK_DEFAULT,
            _ => MEMBER_DEFAULT,
        }
    };
    if !org_type.is_persistent() {
        return Vec::new();
    }
    OrgRank::for_type(org_type)
        .iter()
        .map(|&rank| (rank, for_rank(rank)))
        .collect()
}
