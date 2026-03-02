# Organization System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 3 — Missing Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `OrganizationMember.def`, `SGWPlayer.def`, `alias.xml`, Ghidra decompilation of universal RPC dispatcher (`0x00c6fc40`)

---

## Overview

Organizations are persistent player groups — squads, guilds, and strike teams. The system handles creation, invitations, roster management, rank hierarchy, MOTD, notes, PvP settings, cash pooling, and loot distribution. Organization methods span all three categories: base methods for name-lookup operations, cell methods for gameplay, and client methods for UI updates.

**Interface**: `OrganizationMember` (implemented by `SGWPlayer`)

---

## Client → Server Messages

### Cell Methods (Exposed)

#### `organizationInviteResponse` — Accept/Decline Invite

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aRequestID` | `INT32` | 4B | Invite request ID |
| `aResponse` | `UINT8` | 1B | Non-zero = accept |

**Total wire size**: 1B header + 5B = **6 bytes**

#### `organizationLeave` — Leave Organization

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization to leave |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `BroadcastMinimapPing` — Ping Minimap

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aLocation` | `VECTOR3` | 12B | Map coordinates (x, y, z) |

**Total wire size**: 1B header + 16B = **17 bytes**

#### `strikeTeamResponse` — Respond to Strike Team PvP Change

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aResponse` | `UINT8` | 1B | Accept/decline |

**Total wire size**: 1B header + 5B = **6 bytes**

#### `pvpOrganizationLeaveResponse` — Respond to PvP-Forced Leave

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aResponse` | `UINT8` | 1B | Accept/decline |

**Total wire size**: 1B header + 5B = **6 bytes**

#### `organizationMOTD` — Set Message of the Day

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aMOTD` | `WSTRING` | 4B len + N×2B UTF-16LE | New MOTD text |

**Total wire size**: 1B header + 8B + string data

#### `organizationNote` — Set Player Note

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aNote` | `WSTRING` | 4B len + N×2B | Player's public note |

#### `organizationOfficerNote` — Set Officer Note

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aName` | `WSTRING` | 4B len + N×2B | Target player name |
| `aNote` | `WSTRING` | 4B len + N×2B | Officer note text |

#### `organizationSetRankPermissions` — Change Rank Permissions

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aRank` | `INT32` | 4B | Rank ID |
| `aPermissions` | `INT32` | 4B | Permission bitmask |

**Total wire size**: 1B header + 12B = **13 bytes**

#### `organizationSetRankName` — Rename a Rank

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aRank` | `INT32` | 4B | Rank ID |
| `aName` | `WSTRING` | 4B len + N×2B | New rank name |

#### `squadSetLootMode` — Set Squad Loot Mode

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aLootMode` | `INT32` | 4B | Loot distribution mode |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `organizationTransferCash` — Donate Cash to Org

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aCash` | `INT32` | 4B | Amount to transfer |

**Total wire size**: 1B header + 8B = **9 bytes**

### Base Methods (Exposed) — via OrganizationMember.def

#### `organizationInvite` — Invite Player by Name

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | Org to invite into |
| `aPlayerName` | `WSTRING` | 4B len + N×2B | Target player name |

#### `organizationInviteByType` — Invite to Org Type

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationType` | `UINT8` | 1B | Organization type enum |
| `aPlayerName` | `WSTRING` | 4B len + N×2B | Target player name |

#### `organizationKick` — Kick Player

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aPlayerName` | `WSTRING` | 4B len + N×2B | Player to kick |

#### `organizationRankChange` — Change Player Rank

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aPlayerName` | `WSTRING` | 4B len + N×2B | Target player |
| `aRank` | `UINT8` | 1B | New rank value |

### Additional Base Method (SGWPlayer.def)

#### `organizationCreation` — Create New Organization

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrgType` | `UINT8` | 1B | Organization type |
| `aName` | `WSTRING` | 4B len + N×2B | Organization name |

---

## Server → Client Messages

### `onOrganizationInvite` — Receive Organization Invite

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aInviterName` | `WSTRING` | 4B len + N×2B | Who invited you |
| `aOrganizationType` | `UINT8` | 1B | Org type enum |
| `aRequestID` | `INT32` | 4B | Unique invite ID (for response) |
| `aName` | `WSTRING` | 4B len + N×2B | Organization name |
| `aIsStrikeTeam` | `UINT8` | 1B | PvP strike team flag |

### `onOrganizationJoined` — Successfully Joined

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aOrganizationType` | `UINT8` | 1B | Org type |
| `aRank` | `UINT8` | 1B | Initial rank |
| `aNewMember` | `UINT8` | 1B | 1 = newly created member |

**Total wire size**: 1B header + 7B = **8 bytes**

### `onOrganizationLeft` — Left/Kicked from Org

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aReason` | `UINT8` | 1B | EReasons enum (left, kicked, disbanded) |
| `aOrganizationId` | `INT32` | 4B | Which org |

**Total wire size**: 1B header + 5B = **6 bytes**

### `onMemberJoinedOrganization` — Another Member Joined

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aMemberName` | `WSTRING` | 4B len + N×2B | New member's name |
| `aMember` | `INT32` | 4B | Member ID |
| `aOrganizationId` | `INT32` | 4B | Org ID |
| `aRank` | `UINT8` | 1B | Their rank |
| `aNewMember` | `UINT8` | 1B | Newly created flag |

### `onOrganizationRosterInfo` — Full Roster Data

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | Organization ID |
| `aRosterInfo` | `ARRAY<RosterInfo>` | 4B count + N × RosterInfo | Full member list |

**`RosterInfo` FIXED_DICT layout**:

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `name` | `WSTRING` | 4B len + N×2B |
| `level` | `UINT8` | 1B |
| `archetype` | `UINT8` | 1B |
| `rank` | `UINT8` | 1B |
| `note` | `WSTRING` | 4B len + N×2B |
| `officerNote` | `WSTRING` | 4B len + N×2B |

### `onMemberLeftOrganization` — Member Departure

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aMember` | `INT32` | 4B | Member ID |
| `aReason` | `UINT8` | 1B | EReasons enum |
| `aOrganizationId` | `INT32` | 4B | Org ID |
| `aMemberName` | `WSTRING` | 4B len + N×2B | Member name |

### `onMemberRankChangedOrganization` — Rank Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aMember` | `INT32` | 4B | Member ID |
| `aRank` | `UINT8` | 1B | New rank |
| `aOrganizationId` | `INT32` | 4B | Org ID |
| `aMemberName` | `WSTRING` | 4B len + N×2B | Member name |

### `onStrikeTeamUpdate` — PvP Status Change

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aPvPValue` | `UINT8` | 1B | New PvP flag |

**Total wire size**: 1B header + 5B = **6 bytes**

### `onPvPOrganizationLeaveRequest` — PvP Change Warning

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | Org that would be left |
| `aPvPValue` | `UINT8` | 1B | Requested PvP value |

**Total wire size**: 1B header + 5B = **6 bytes**

### `onOrganizationNameUpdate` — Org Name Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aName` | `WSTRING` | 4B len + N×2B | New name |

### `onOrganizationExperienceUpdate` — Org XP Update

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aExperience` | `UINT64` | 8B | Organization XP total |

**Total wire size**: 1B header + 12B = **13 bytes**

### `onOrganizationMOTDUpdate` — MOTD Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aMOTD` | `WSTRING` | 4B len + N×2B | New MOTD |

### `onOrganizationNoteUpdate` — Player Note Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aName` | `WSTRING` | 4B len + N×2B | Player name |
| `aNote` | `WSTRING` | 4B len + N×2B | New note |

### `onOrganizationOfficerNoteUpdate` — Officer Note Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aName` | `WSTRING` | 4B len + N×2B | Player name |
| `aNote` | `WSTRING` | 4B len + N×2B | New officer note |

### `onOrganizationCashUpdate` — Org Treasury Update

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aCash` | `UINT64` | 8B | Total treasury |

**Total wire size**: 1B header + 12B = **13 bytes**

### `onOrganizationRankUpdate` — Rank Permissions Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aRankIds` | `ARRAY<INT32>` | 4B count + N×4B | Rank IDs |
| `aRankFlags` | `ARRAY<INT32>` | 4B count + N×4B | Permission bitmasks |

### `onOrganizationRankNameUpdate` — Rank Names Changed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aRankIds` | `ARRAY<INT32>` | 4B count + N×4B | Rank IDs |
| `aRankNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) | Rank display names |

### `onSquadLootType` — Loot Mode Changed

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrganizationId` | `INT32` | 4B | |
| `aLootType` | `INT32` | 4B | Loot distribution mode |

**Total wire size**: 1B header + 8B = **9 bytes**

### `onOrganizationCreationResult` — Creation Response (SGWPlayer.def)

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `Result` | `UINT8` | 1B | Success/failure |
| `RetCode` | `UINT8` | 1B | Detailed return code |

**Total wire size**: 1B header + 2B = **3 bytes**

### `launchOrganizationCreation` — Open Creation UI (SGWPlayer.def)

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aOrgType` | `UINT8` | 1B | Organization type to create |

**Total wire size**: 1B header + 1B = **2 bytes**

---

## Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `records` | `PYTHON` | `CELL_PRIVATE` | Organization DB records |
| `squad` | `INT32` | `CELL_PUBLIC` | Squad group ID (0 = none) |
| `strikeTeamTimers` | `PYTHON` | `CELL_PRIVATE` | PvP change response timers |
| `pendingPvPTimers` | `PYTHON` | `CELL_PRIVATE` | PvP flag change pending ops |
| `pendingGroups` | `PYTHON` | `CELL_PRIVATE` | Pending group creates |
| `pendingJoins` | `PYTHON` | `CELL_PRIVATE` | Pending join operations |
| `pendingInvitesByType` | `PYTHON` | `CELL_PRIVATE` | Pending type-based invites |

**Note**: Only `squad` is `CELL_PUBLIC`. All others are `CELL_PRIVATE`. None have `ALL_CLIENTS` or `OWN_CLIENT` flags, so no organization properties are sent to the client via property sync — all UI state is communicated through client method calls.

---

## Implementation Notes

1. **Three org types**: Squad (party/group), Guild, Strike Team (PvP). Differentiated by `aOrganizationType` UINT8.
2. **Invite flow**: Client sends `organizationInvite` (base method, name-based lookup) → server finds target → target receives `onOrganizationInvite` → target responds via `organizationInviteResponse` (cell method).
3. **Base vs Cell methods**: Name-based operations (invite, kick, rank change) go through base methods to resolve player names to entities. Direct operations (leave, respond, MOTD) use cell methods.
4. **Cash uses UINT64**: Organization treasury and XP use 8-byte unsigned integers, supporting large values.
5. **Rank permissions**: Stored as INT32 bitmasks. Rank IDs and names are sent as parallel arrays.
6. **PvP flag changes** that would force org leaves trigger a confirmation flow: `onPvPOrganizationLeaveRequest` → `pvpOrganizationLeaveResponse`.
7. **Minimap pings**: Squad members can ping map locations via `BroadcastMinimapPing`, received by other members via the `receivedMinimapPing` cell method (not exposed — server-distributed).
