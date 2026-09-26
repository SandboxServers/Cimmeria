//! OrganizationMember interface ClientMethods (indices 34–51).

/// Invitation to join an organization.
pub const ON_ORGANIZATION_INVITE: u16 = 34;
/// Successfully joined an organization.
pub const ON_ORGANIZATION_JOINED: u16 = 35;
/// Left an organization.
pub const ON_ORGANIZATION_LEFT: u16 = 36;
/// Another member joined.
pub const ON_MEMBER_JOINED_ORGANIZATION: u16 = 37;
/// Full roster info dump.
pub const ON_ORGANIZATION_ROSTER_INFO: u16 = 38;
/// A member left.
pub const ON_MEMBER_LEFT_ORGANIZATION: u16 = 39;
/// A member's rank changed.
pub const ON_MEMBER_RANK_CHANGED_ORGANIZATION: u16 = 40;
/// Strike team PvP status update.
pub const ON_STRIKE_TEAM_UPDATE: u16 = 41;
/// PvP organization leave request.
pub const ON_PVP_ORGANIZATION_LEAVE_REQUEST: u16 = 42;
/// Organization name changed.
pub const ON_ORGANIZATION_NAME_UPDATE: u16 = 43;
/// Organization XP changed.
pub const ON_ORGANIZATION_EXPERIENCE_UPDATE: u16 = 44;
/// Message of the day changed.
pub const ON_ORGANIZATION_MOTD_UPDATE: u16 = 45;
/// Member note changed.
pub const ON_ORGANIZATION_NOTE_UPDATE: u16 = 46;
/// Officer note changed.
pub const ON_ORGANIZATION_OFFICER_NOTE_UPDATE: u16 = 47;
/// Organization cash changed.
pub const ON_ORGANIZATION_CASH_UPDATE: u16 = 48;
/// Rank permissions changed.
pub const ON_ORGANIZATION_RANK_UPDATE: u16 = 49;
/// Rank names changed.
pub const ON_ORGANIZATION_RANK_NAME_UPDATE: u16 = 50;
/// Squad loot type changed.
pub const ON_SQUAD_LOOT_TYPE: u16 = 51;
