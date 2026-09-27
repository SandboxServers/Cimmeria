//! Organization enums: type, rank, leave reason and squad loot mode.
//!
//! Every wire value comes from `entities/defs/enumerations.xml`, and each is
//! pinned by a literal test in `organization/tests.rs`. Parsing from the wire
//! goes through `TryFrom`, which rejects any value the enum does not define.

use std::fmt;

/// A wire value outside the enum it was parsed as. Carries the raw value for
/// the rejection log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownValue(pub i64);

impl fmt::Display for UnknownValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown enum value {}", self.0)
    }
}

impl std::error::Error for UnknownValue {}

/// `EOrganizationType` (`enumerations.xml:1939`, UINT8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OrgType {
    /// `EORG_TYPE_Squad`: the ephemeral party of up to six. Cell-side, never
    /// persisted (D-ORG03).
    Squad = 0,
    /// `EORG_TYPE_Team`: a small persistent group.
    Team = 1,
    /// `EORG_TYPE_Command`: the persistent guild.
    Command = 2,
}

impl OrgType {
    /// Every type, in wire order.
    pub const ALL: [OrgType; 3] = [OrgType::Squad, OrgType::Team, OrgType::Command];

    /// `true` for Team and Command, which live in the database with the base
    /// as their authority (D-ORG04). Squads are cell state only.
    pub fn is_persistent(self) -> bool {
        !matches!(self, OrgType::Squad)
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Short lowercase name for logs and GM commands.
    pub fn name(self) -> &'static str {
        match self {
            OrgType::Squad => "squad",
            OrgType::Team => "team",
            OrgType::Command => "command",
        }
    }
}

impl TryFrom<u8> for OrgType {
    type Error = UnknownValue;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(OrgType::Squad),
            1 => Ok(OrgType::Team),
            2 => Ok(OrgType::Command),
            other => Err(UnknownValue(i64::from(other))),
        }
    }
}

/// `EOrganizationRank` (`enumerations.xml:1892`, UINT8): nine values, 0-8.
///
/// A newtype rather than an enum because the rank ladder is compared by
/// order (D-ORG09 (2): the actor's rank must be strictly above the
/// target's), and `Ord` on the raw value is exactly that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrgRank(u8);

impl OrgRank {
    /// `EORG_RANK_None`. Never a member's rank; no type uses it.
    pub const NONE: OrgRank = OrgRank(0);
    pub const INITIATE: OrgRank = OrgRank(1);
    pub const MEMBER: OrgRank = OrgRank(2);
    pub const SENIOR_MEMBER: OrgRank = OrgRank(3);
    pub const VETERAN: OrgRank = OrgRank(4);
    pub const SENIOR_VETERAN: OrgRank = OrgRank(5);
    pub const OFFICER: OrgRank = OrgRank(6);
    pub const SENIOR_OFFICER: OrgRank = OrgRank(7);
    /// `EORG_RANK_Leader`. The leader is the member holding this rank; there
    /// is no separate leader column (work-packets.md § Schema).
    pub const LEADER: OrgRank = OrgRank(8);

    pub fn as_u8(self) -> u8 {
        self.0
    }

    /// The ranks a type uses (D-ORG07), lowest first.
    ///
    /// - Squad: `Member` and `Leader`.
    /// - Team: `Member`, `SeniorMember` and `Leader`, the only three
    ///   `Team.lua` edits (`TeamMod.openTeamEditor`, audit A-12).
    /// - Command: `Initiate` to `Leader`; `Command.lua` edits 1-7 and shows 8.
    ///
    /// Any rank a client asks to assign must be in this slice (D-ORG09 (5)),
    /// so 0, and Team rank 5, are rejected.
    pub fn for_type(org_type: OrgType) -> &'static [OrgRank] {
        const SQUAD: [OrgRank; 2] = [OrgRank::MEMBER, OrgRank::LEADER];
        const TEAM: [OrgRank; 3] = [OrgRank::MEMBER, OrgRank::SENIOR_MEMBER, OrgRank::LEADER];
        const COMMAND: [OrgRank; 8] = [
            OrgRank::INITIATE,
            OrgRank::MEMBER,
            OrgRank::SENIOR_MEMBER,
            OrgRank::VETERAN,
            OrgRank::SENIOR_VETERAN,
            OrgRank::OFFICER,
            OrgRank::SENIOR_OFFICER,
            OrgRank::LEADER,
        ];
        match org_type {
            OrgType::Squad => &SQUAD,
            OrgType::Team => &TEAM,
            OrgType::Command => &COMMAND,
        }
    }

    /// The rank a new member joins at (D-ORG07): `Member` in a Squad or a
    /// Team, `Initiate` in a Command.
    pub fn entry_for(org_type: OrgType) -> OrgRank {
        OrgRank::for_type(org_type)[0]
    }

    /// `true` if `org_type` uses this rank.
    pub fn is_valid_for(self, org_type: OrgType) -> bool {
        OrgRank::for_type(org_type).contains(&self)
    }
}

impl TryFrom<u8> for OrgRank {
    type Error = UnknownValue;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        if v <= OrgRank::LEADER.0 {
            Ok(OrgRank(v))
        } else {
            Err(UnknownValue(i64::from(v)))
        }
    }
}

/// The rank arguments of cell methods 16 and 17 are `INT32` on the wire.
impl TryFrom<i32> for OrgRank {
    type Error = UnknownValue;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        u8::try_from(v)
            .ok()
            .and_then(|b| OrgRank::try_from(b).ok())
            .ok_or(UnknownValue(i64::from(v)))
    }
}

/// `EReasons` (`enumerations.xml:104`, UINT8): why a player left an
/// organization, carried by `onOrganizationLeft` [36] and
/// `onMemberLeftOrganization` [39].
///
/// Not disbanded 0 / left 1 / kicked 2, as an earlier draft had it (audit A-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OrgLeaveReason {
    /// `REAS_requested`: the player left.
    Requested = 0,
    /// `REAS_kicked`.
    Kicked = 1,
    /// `REAS_disbanded`: the organization is gone.
    Disbanded = 2,
    /// `REAS_logout`: a squad member disconnected.
    Logout = 3,
}

impl OrgLeaveReason {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for OrgLeaveReason {
    type Error = UnknownValue;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(OrgLeaveReason::Requested),
            1 => Ok(OrgLeaveReason::Kicked),
            2 => Ok(OrgLeaveReason::Disbanded),
            3 => Ok(OrgLeaveReason::Logout),
            other => Err(UnknownValue(i64::from(other))),
        }
    }
}

/// `EGroupLootType` (`enumerations.xml:674`): the squad loot mode, carried
/// as an `INT32` by `squadSetLootMode` (CM 18) and `onSquadLootType` [51].
///
/// Two values only (audit A-16); not the four an earlier draft guessed. D-ORG16
/// rejects anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(i32)]
pub enum SquadLootType {
    /// `GROUP_LOOT_RoundRobin`.
    #[default]
    RoundRobin = 0,
    /// `GROUP_LOOT_FreeForAll`.
    FreeForAll = 1,
}

impl SquadLootType {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl TryFrom<i32> for SquadLootType {
    type Error = UnknownValue;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(SquadLootType::RoundRobin),
            1 => Ok(SquadLootType::FreeForAll),
            other => Err(UnknownValue(i64::from(other))),
        }
    }
}

/// The direction and size of `organizationTransferCash` (CM 19).
///
/// The wire carries one signed `INT32 aCash`; the client sends a deposit as
/// a positive amount and a withdrawal as its negation
/// (`Command.lua` `CommandMod.onWithdrawClicked` calls
/// `commandTransferCash(-cashAmt)`, `onDepositClicked` passes `cashAmt`;
/// `Team.lua` is the same). The magnitude is taken with `unsigned_abs`, so
/// `i32::MIN` is a withdrawal of 2,147,483,648 rather than an overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CashDir {
    /// From the player's wallet into the organization.
    Deposit(u32),
    /// From the organization into the player's wallet.
    Withdraw(u32),
}

impl CashDir {
    /// Split the wire amount. Zero moves nothing and is `None`.
    pub fn from_wire(amount: i32) -> Option<CashDir> {
        match amount {
            0 => None,
            a if a > 0 => Some(CashDir::Deposit(a.unsigned_abs())),
            a => Some(CashDir::Withdraw(a.unsigned_abs())),
        }
    }
}
