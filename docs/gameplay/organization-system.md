---
title: "Organization System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Organization System

> **Last updated**: 2026-09-27
> **Status**: Not implemented; the wire contract is in place. Every inbound organization call is decoded in full and answered or logged, and every outbound organization method has a serializer, but no gameplay behaviour exists yet: no persistence, no roster, no fanout. The organizations campaign ([docs/analysis/organizations/](../analysis/organizations/README.md)) builds it on this contract.

## Overview

Organizations are persistent player groups: Commands (guilds), Squads (parties), and Teams (strike teams). Each organization has a roster, rank hierarchy with permissions, shared cash, experience, MOTD, and member/officer notes. The system supports inviting, kicking, rank changes, loot modes, minimap pings, PvP strike team status, and organization vault storage.

The `OrganizationMember` interface in `entities/defs/interfaces/OrganizationMember.def` is the largest gameplay interface by property + method count.

## Implementation Status

What exists after the campaign's contract packet (ORG-01):

- **Models** in `cimmeria_entity::organization` ([`crates/entity/src/organization/`](../../crates/entity/src/organization/)): `OrgType`, `OrgRank` with the ranks each type uses, the 26 `OrgPermission` bits with the 12 (Team) and 14 (Command) bits the client's rank editors expose, `OrgLeaveReason`, `SquadLootType`, the id-space constants, the default rank permissions and `org_text`, the one implementation of the text rules (lengths, forbidden characters, the name normaliser and its uniqueness key). Every enum value is pinned against `entities/defs/enumerations.xml`.
- **Inbound decoders** in `cimmeria-wire`: cell methods 8–19 and SGWPlayer cell method 94 `onOrganizationCreation` ([`crates/wire/src/cell/cell_methods/organization/`](../../crates/wire/src/cell/cell_methods/organization/)), and base methods 0xCF–0xD2 ([`crates/wire/src/base/organization.rs`](../../crates/wire/src/base/organization.rs)). Each bounds a `WSTRING`'s declared length by the bytes left before allocating, and rejects truncation, trailing bytes and unpaired surrogates. CM 10 rejects a non-finite coordinate, and CM 19's signed amount decodes to a `CashDir` (positive deposits, negative withdraws, zero is rejected). CM 13, 14, 15, 17 and 94 used to drop their text; they now read it.
- **Dispatch.** The cell arms ([`crates/cell-methods/src/cell/cell_methods/organization.rs`](../../crates/cell-methods/src/cell/cell_methods/organization.rs), and CM 94 in `player/social.rs`) decode, log `UNIMPLEMENTED` at DEBUG on the `org` target (the text length, never the text), and answer every well-formed call with `onErrorCode` and the same feedback line as the base. The base arm for 0xCF–0xD2 ([`crates/base/src/base/dispatch/organization.rs`](../../crates/base/src/base/dispatch/organization.rs)) decodes and answers every well-formed call with `onErrorCode` and a feedback chat line, "Organizations are not available yet.", so the press is not silent.
- **Outbound serializers** for client methods 34–51, `onOrganizationCreationResult` (134) and `launchOrganizationCreation` (135) in [`crates/wire/src/cell/client_methods/organization/`](../../crates/wire/src/cell/client_methods/organization/) and `player.rs`, each byte-tested. Nothing sends them yet.
- **Cell↔base messages** `CellToBaseMsg::Org(OrgCellToBase)` and `BaseToCellMsg::Org(OrgBaseToCell)`, routed to logged no-ops on both sides.

There is no `sgw_organization*` table yet.

| Feature | Status | Notes |
|---------|--------|-------|
| Organization types | DEFINED | Command, Squad, Team in entity defs; typed models in `cimmeria_entity::organization` |
| Invite response | STUB | `organizationInviteResponse` (CM 8) decodes, logs, drops |
| Leave | STUB | `organizationLeave` (CM 9) decodes, logs, drops |
| Minimap ping | STUB | `BroadcastMinimapPing` (CM 10) decodes, logs, drops |
| Strike team (PvP) | STUB | `strikeTeamResponse` (CM 11) decodes, logs, drops |
| PvP leave confirmation | STUB | `pvpOrganizationLeaveResponse` (CM 12) decodes, logs, drops |
| MOTD | STUB | `organizationMOTD` (CM 13) decodes the org id and the MOTD, logs, drops |
| Member note | STUB | `organizationNote` (CM 14) decodes the org id and the note, logs, drops |
| Officer note | STUB | `organizationOfficerNote` (CM 15) decodes the org id, the member name and the note, logs, drops |
| Rank permissions | STUB | `organizationSetRankPermissions` (CM 16) decodes, logs, drops |
| Custom rank names | STUB | `organizationSetRankName` (CM 17) decodes the org id, rank and name, logs, drops |
| Loot mode | STUB | `squadSetLootMode` (CM 18) decodes, logs, drops |
| Cash management | STUB | `organizationTransferCash` (CM 19) decodes, logs, drops |
| Creation | STUB | `onOrganizationCreation` (SGWPlayer CM 94) decodes the name, logs, drops. `launchOrganizationCreation` (135) and `onOrganizationCreationResult` (134) have serializers, never sent |
| Invite issue / kick / rank change | STUB | Base methods 0xCF–0xD2 decode, log, and answer with `onErrorCode` and a feedback line |
| Roster info | NOT IMPL | `onOrganizationRosterInfo` (CM 38) has a serializer, never sent |
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

### Client Methods (Server -> Client) -- 18 methods

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
| Squad (0) | Temporary party of up to six, never persisted | Dungeon group |
| Team (1) | Small persistent group; ranks Member, SeniorMember, Leader | Regular group of friends |
| Command (2) | Persistent guild; ranks Initiate to Leader, vault and treasury | Player guild |

## Data References

- **Enumerations**: `EOrganizationType`, `EOrganizationRank`, `EReasons`
- **Custom types**: `RosterInfo` (roster data structure)
- **Related**: `GroupAuthority.def` manages group lifecycle

## RE Priorities

1. **Organization persistence** - Database schema for organization data
2. **Rank permissions** - Resolved: the 26 `EOrganizationPermission` bits, `OrgPermission` in `cimmeria_entity::organization`
3. **Loot modes** - Resolved: `EGroupLootType` has two values, RoundRobin (0) and FreeForAll (1)
4. **Group authority integration** - How `GroupAuthority` entity manages org lifecycle
5. **Organization vault** - Cross-entity vault storage protocol

## Related Docs

- [group-system.md](group-system.md) - GroupAuthority that manages organizations
- [chat-system.md](chat-system.md) - Organization channels (command, officer, squad)
- [inventory-system.md](inventory-system.md) - Organization vault
