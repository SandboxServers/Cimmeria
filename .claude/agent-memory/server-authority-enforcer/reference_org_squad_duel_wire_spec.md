---
name: reference-org-squad-duel-wire-spec
description: Authoritative wire shapes for SGW organization, squad, duel, PvP-leave client messages with .def + Ghidra anchors
metadata:
  type: reference
---

# CAT-M wire spec anchors

Wire shapes for the 18 client→server messages in the Organization /
Squad / Duel category. The .def files are the truth for arg ordering
because the BigWorld entity codegen consumes them on both client and
server. Ghidra `Event_NetOut_*` RTTI strings confirm the client emits
these exact classes.

## Org base methods (lifecycle / destructive)

`entities/defs/interfaces/OrganizationMember.def:421-447` (`<BaseMethods>`,
all `<Exposed/>`):

- `organizationInvite(INT32 aOrganizationId, WSTRING aPlayerName)` — RTTI `0x019be900`
- `organizationInviteByType(UINT8 aOrganizationType, WSTRING aPlayerName)` — RTTI `0x019be920`
- `organizationKick(INT32 aOrganizationId, WSTRING aPlayerName)` — RTTI `0x019be990`
- `organizationRankChange(INT32 aOrganizationId, WSTRING aPlayerName, UINT8 aRank)` — RTTI `0x019be9b0`

All four have **no base-dispatch arm in Rust** today; they hit the
catch-all warn at `crates/services/src/base/dispatch.rs:333-347`.

## Org / squad cell methods (rosters, text, bank, loot)

`entities/defs/interfaces/OrganizationMember.def`, `<CellMethods>` with
`<Exposed/>` (cell method index in `crates/services/src/cell/cell_methods/organization.rs:7-18`):

- `organizationInviteResponse(INT32 aRequestID, UINT8 aResponse)` — `INVITE_RESPONSE=8`, def:267-271
- `organizationLeave(INT32 aOrganizationId)` — `LEAVE=9`, def:286-289
- `BroadcastMinimapPing(INT32 aOrganizationId, VECTOR3 aLocation)` — `BROADCAST_MINIMAP_PING=10`, def:308-312
- `strikeTeamResponse(INT32 aOrganizationId, UINT8 aResponse)` — `STRIKE_TEAM_RESPONSE=11`, def:329-333
- `pvpOrganizationLeaveResponse(INT32 aOrganizationId, UINT8 aResponse)` — `PVP_LEAVE_RESPONSE=12`, def:336-340
- `organizationMOTD(INT32 aOrganizationId, WSTRING aMOTD)` — `MOTD=13`, def:371-375
- `organizationNote(INT32 aOrganizationId, WSTRING aNote)` — `NOTE=14`, def:377-381
- `organizationOfficerNote(INT32 aOrganizationId, WSTRING aName, WSTRING aNote)` — `OFFICER_NOTE=15`, def:383-388
- `organizationSetRankPermissions(INT32 aOrganizationId, INT32 aRank, INT32 aPermissions)` — `SET_RANK_PERMISSIONS=16`, def:390-395
- `organizationSetRankName(INT32 aOrganizationId, INT32 aRank, WSTRING aName)` — `SET_RANK_NAME=17`, def:397-402
- `squadSetLootMode(INT32 aLootMode)` — `SQUAD_SET_LOOT_MODE=18`, def:404-407
  — **note: no aOrganizationId on the wire**; server must resolve squad from session
- `organizationTransferCash(INT32 aOrganizationId, INT32 aCash)` — `TRANSFER_CASH=19`, def:409-413
  — `aCash` is **signed**, must reject `<= 0`

All twelve are stubs in `crates/services/src/cell/cell_methods/organization.rs:28-159`.

## SGWPlayer cell methods (creation / duel)

`entities/defs/SGWPlayer.def`, `<CellMethods>` with `<Exposed/>` (indices
in `crates/services/src/cell/cell_methods/player/constants.rs`):

- `onOrganizationCreation(WSTRING aOrganizationName)` — `ORG_CREATION=94`, def:877-880
- `sendDuelResponse(INT8 aResponse)` — `SEND_DUEL_RESPONSE=102`, def:975-978
  — **no challenger id**; server must hold pending-challenge state per session
- `duelForfeit()` — `DUEL_FORFEIT=103`, def:1015-1017 — **no args**

All three are stubs in `crates/services/src/cell/cell_methods/player/social.rs:61-101`.

## SGWPlayer base method (duel challenge)

`entities/defs/SGWPlayer.def`, `<BaseMethods>` with `<Exposed/>`:

- `sendDuelChallenge(WSTRING aPlayerName, INT8 aSquadDuel)` — def:509-513

**No base-dispatch arm in Rust** — hits the catch-all warn.

## Permission and rank enums

`entities/defs/enumerations.xml`:

- `EOrganizationRank` (UINT8) lines 1892-1905: `None=0, Initiate=1, Member=2,
  SeniorMember=3, Veteran=4, SeniorVeteran=5, Officer=6, SeniorOfficer=7, Leader=8`
- `EOrganizationPermission` (UINT32 bitfield) lines 1907-1937:
  - `Invite=2`, `Promote=4`, `Demote=8`, `Eject=16`
  - `RosterNotes=32`, `OfficerNotes=64`, `RankNames=128`, `OfficerChat=256`
  - `MOTD=1024`, `HistoryLog=2048`
  - `DepositCash=262144`, `WithdrawCash=524288`
  - `AlterPerms=8388608`, `TransferLeader=16777216`, `AllianceCmds=33554432`
- `EOrganizationType` (UINT8) lines 1939-1946: `Squad=0, Team=1, Command=2`
- `EDBErrorType` (INT32) lines 1948-1959 contains the rank-related error codes
  the original Python reference returned (`Player_rank_too_low = -20072`,
  `Invalid_org_rank = -20092`, etc.) — useful for the new Rust handlers to
  mirror so client UI error display continues to work.

## Pending-state properties already declared

`entities/defs/interfaces/OrganizationMember.def:26-57` already declares
the right per-session pending maps (`strikeTeamTimers`, `pendingPvPTimers`,
`pendingGroups`, `pendingJoins`, `pendingInvitesByType`) as
`CELL_PRIVATE` PYTHON-typed properties. The implementer can wire the
state-machine-correlation checks (CAT-M-13, -16, -17, -18) against
these without needing to add new entity-level state.

## Duel marker

`entities/defs/SGWDuelMarker.def:7-18`:

- `duelDetectorID: CONTROLLER_ID`
- `duelEntities: ARRAY<of>MAILBOX` — array of participant base mailboxes

A `disconnect mid-duel` auto-forfeit path (CAT-M-15) must consult
`duelEntities` from any active marker the disconnecting player is in.

## Ghidra wire-class anchors (summary)

| Message | RTTI string addr |
|---|---|
| `Event_NetOut_OrganizationCreation` | `0x0195fb88` |
| `Event_NetOut_OrganizationInvite` | `0x019be900` |
| `Event_NetOut_OrganizationInviteByType` | `0x019be920` |
| `Event_NetOut_OrganizationInviteResponse` | `0x019be948` |
| `Event_NetOut_OrganizationLeave` | `0x019be970` |
| `Event_NetOut_OrganizationKick` | `0x019be990` |
| `Event_NetOut_OrganizationRankChange` | `0x019be9b0` |
| `Event_NetOut_OrganizationMOTD` | `0x019be9d4` |
| `Event_NetOut_OrganizationNote` | `0x019be9f4` |
| `Event_NetOut_OrganizationOfficerNote` | `0x019bea14` |
| `Event_NetOut_OrganizationSetRankPermissions` | `0x019bea3c` |
| `Event_NetOut_OrganizationSetRankName` | `0x019bea68` |
| `Event_NetOut_OrganizationTransferCash` | `0x019bea90` |
| `Event_NetOut_SquadSetLootMode` | `0x019beab8` |
| `Event_NetOut_PvPOrganizationLeaveResponse` | `0x019bfbec` (also `0x019cb5d8`) |
| `Event_NetOut_DuelChallenge` | `0x019b4448` |
| `Event_NetOut_DuelResponse` | `0x0195fb58` |
| `Event_NetOut_DuelForfeit` | `0x019b4478` |

Reference: [[project-org-squad-duel-unimplemented]] for the trust-
posture summary and the 18-finding checklist.
