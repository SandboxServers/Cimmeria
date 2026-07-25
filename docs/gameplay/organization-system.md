---
title: "Organization System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Organization System

> **Last updated**: 2026-07-25
> **Status**: Not implemented. Twelve inbound cell methods are decoded and dispatched, but every one logs `UNIMPLEMENTED` and drops. No persistence, no roster, no fanout.

## Overview

Organizations are persistent player groups: Commands (guilds), Squads (parties), and Teams (strike teams). Each organization has a roster, rank hierarchy with permissions, shared cash, experience, MOTD, and member/officer notes. The system supports inviting, kicking, rank changes, loot modes, minimap pings, PvP strike team status, and organization vault storage.

The `OrganizationMember` interface in `entities/defs/interfaces/OrganizationMember.def` is the largest gameplay interface by property + method count.

## Implementation Status

The Rust server wires the argument decoding for cell methods 8–19 in [`crates/services/src/cell/cell_methods/organization.rs`](../../crates/services/src/cell/cell_methods/organization.rs) — each arm parses its payload and emits a structured `UNIMPLEMENTED` log so the fields are visible in traces. Nothing beyond that exists: no base-side handler, no `sgw_organization*` table, and none of the eighteen `onOrganization*` client methods (indices 34–51) is ever sent.

| Feature | Status | Notes |
|---------|--------|-------|
| Organization types | DEFINED | Command, Squad, Team in entity defs |
| Invite response | STUB | `organizationInviteResponse` (CM 8) decodes, logs, drops |
| Leave | STUB | `organizationLeave` (CM 9) decodes, logs, drops |
| Minimap ping | STUB | `BroadcastMinimapPing` (CM 10) decodes, logs, drops |
| Strike team (PvP) | STUB | `strikeTeamResponse` (CM 11) decodes, logs, drops |
| PvP leave confirmation | STUB | `pvpOrganizationLeaveResponse` (CM 12) decodes, logs, drops |
| MOTD | STUB | `organizationMOTD` (CM 13) decodes, logs, drops |
| Member note | STUB | `organizationNote` (CM 14) decodes, logs, drops |
| Officer note | STUB | `organizationOfficerNote` (CM 15) decodes, logs, drops |
| Rank permissions | STUB | `organizationSetRankPermissions` (CM 16) decodes, logs, drops |
| Custom rank names | STUB | `organizationSetRankName` (CM 17) decodes, logs, drops |
| Loot mode | STUB | `squadSetLootMode` (CM 18) decodes, logs, drops |
| Cash management | STUB | `organizationTransferCash` (CM 19) decodes, logs, drops |
| Invite issue / kick / rank change | NOT IMPL | No inbound method dispatched; these are base-method chains in the defs |
| Roster info | NOT IMPL | `onOrganizationRosterInfo` (CM 38) never sent |
| Experience tracking | NOT IMPL | `onOrganizationExperienceUpdate` (CM 44) never sent |
| Persistence | NOT IMPL | No organization tables in `db/sgw/` |
| Organization vault | NOT IMPL | Only `onClearOrgVaultInventory` reference |

## Entity Definition (OrganizationMember.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `records` | PYTHON | CELL_PRIVATE | Organization membership records |
| `squad` | INT32 | CELL_PUBLIC | Current squad (party) group ID |
| `strikeTeamTimers` | PYTHON | CELL_PRIVATE | PvP change response timeouts |
| `pendingPvPTimers` | PYTHON | CELL_PRIVATE | PvP requests causing org leave |
| `pendingGroups` | PYTHON | CELL_PRIVATE | Groups pending creation (type -> invite list) |
| `pendingJoins` | PYTHON | CELL_PRIVATE | Pending join requests |
| `pendingInvitesByType` | PYTHON | CELL_PRIVATE | Pending invites by org type |

### Client Methods (Server -> Client) -- 16 methods

| Method | Args | Purpose |
|--------|------|---------|
| `onOrganizationInvite` | InviterName, OrgType, RequestID, Name, IsStrikeTeam | Invitation prompt |
| `onOrganizationJoined` | OrgId, OrgType, Rank, NewMember | Joined notification |
| `onOrganizationLeft` | Reason, OrgId | Left notification |
| `onMemberJoinedOrganization` | MemberName, MemberId, OrgId, Rank, NewMember | Roster update: join |
| `onOrganizationRosterInfo` | OrgId, ARRAY\<RosterInfo\> | Full roster sync |
| `onMemberLeftOrganization` | MemberId, Reason, OrgId, MemberName | Roster update: leave |
| `onMemberRankChangedOrganization` | MemberId, Rank, OrgId, MemberName | Rank change |
| `onStrikeTeamUpdate` | OrgId, PvPValue | PvP status change |
| `onPvPOrganizationLeaveRequest` | OrgId, PvPValue | Confirm PvP flag change |
| `onOrganizationNameUpdate` | OrgId, Name | Name change |
| `onOrganizationExperienceUpdate` | OrgId, Experience (UINT64) | XP update |
| `onOrganizationMOTDUpdate` | OrgId, MOTD | MOTD change |
| `onOrganizationNoteUpdate` | OrgId, Name, Note | Member note |
| `onOrganizationOfficerNoteUpdate` | OrgId, Name, Note | Officer note |
| `onOrganizationCashUpdate` | OrgId, Cash (UINT64) | Cash update |
| `onOrganizationRankUpdate` | OrgId, RankIds, RankFlags | Rank permissions |
| `onOrganizationRankNameUpdate` | OrgId, RankIds, RankNames | Custom rank names |
| `onSquadLootType` | OrgId, LootType | Loot mode |

### Cell Methods -- 30+ methods

Key exposed (client-invoked) methods:

| Method | Args | Purpose |
|--------|------|---------|
| `organizationInviteResponse` | RequestID, Response | Accept/decline invite |
| `organizationLeave` | OrgId | Leave organization |
| `organizationMOTD` | OrgId, MOTD | Set MOTD |
| `organizationNote` | OrgId, Note | Set member note |
| `organizationOfficerNote` | OrgId, Name, Note | Set officer note |
| `organizationSetRankPermissions` | OrgId, Rank, Permissions | Set rank permissions |
| `organizationSetRankName` | OrgId, Rank, Name | Set rank name |
| `squadSetLootMode` | LootMode | Change squad loot mode |
| `organizationTransferCash` | OrgId, Cash | Deposit/withdraw cash |
| `BroadcastMinimapPing` | OrgId, Location | Ping minimap for group |
| `strikeTeamResponse` | OrgId, Response | Respond to PvP change |
| `pvpOrganizationLeaveResponse` | OrgId, Response | Confirm PvP leave |

### Base Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `organizationInvite` | YES | OrgId, PlayerName | Invite by org ID |
| `organizationInviteByType` | YES | OrgType, PlayerName | Invite by org type (auto-create) |
| `organizationKick` | YES | OrgId, PlayerName | Kick member |
| `organizationRankChange` | YES | OrgId, PlayerName, Rank | Change member rank |

## Organization Types

| Type | Purpose | Example |
|------|---------|---------|
| Command | Persistent guild | Player guild |
| Squad | Temporary party | Dungeon group |
| Team | Strike team (PvP) | PvP group |

## Data References

- **Enumerations**: `EOrganizationType`, `EOrganizationRank`, `EReasons`
- **Custom types**: `RosterInfo` (roster data structure)
- **Related**: `GroupAuthority.def` manages group lifecycle

## RE Priorities

1. **Organization persistence** - Database schema for organization data
2. **Rank permissions** - Bitmask format for `aRankFlags` / `aPermissions`
3. **Loot modes** - Loot distribution types (need/greed/round-robin/etc.)
4. **Group authority integration** - How `GroupAuthority` entity manages org lifecycle
5. **Organization vault** - Cross-entity vault storage protocol

## Related Docs

- [group-system.md](group-system.md) - GroupAuthority that manages organizations
- [chat-system.md](chat-system.md) - Organization channels (command, officer, squad)
- [inventory-system.md](inventory-system.md) - Organization vault
