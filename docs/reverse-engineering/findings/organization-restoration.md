# Organization / Squad / Guild System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire format: .def + binary RTTI); MEDIUM (server logic: Python stubs only); LOW (persistence schema: no original DB recovered)
> **Sources**: `entities/defs/interfaces/{OrganizationMember,GroupAuthority}.def`,
>   `entities/defs/SGWPlayerGroupAuthority.def`, `python/Atrea/enums.py`,
>   `deprecated/python/{base,cell}/SGWPlayer.py`, `deprecated/python/{base,cell}/SGWPlayerGroupAuthority.py`,
>   `db/resources/Social/Types/*.sql`, `SGW.exe` Ghidra, `crates/services/src/cell/{cell_methods,client_methods}/organization.rs`,
>   `crates/game/src/social/guilds.rs`, `docs/reverse-engineering/findings/organization-wire-formats.md`
> **Tracking issue**: replaces #68

## Completeness assessment

Three org types: **Squad** (ephemeral/session-only), **Team** (persistent), **Command** (persistent
guild). All three are confirmed concrete C++ classes in the client (vtable/ctor chain). The original
Python server **never implemented org logic** — both `SGWPlayerGroupAuthority` Python files are empty
stubs and the `SGWPlayer` org handlers are all `pass`. So this is greenfield against the .def + wire spec.

| Dimension | Estimate |
|---|---|
| Wire format documentation | ~92% (existing doc accurate; additions below) |
| Client binary confirmation | ~88% (all named events confirmed) |
| Rust server implementation | ~8% (wire parsing stubs only) |
| DB schema | ~10% (type ENUMs only; zero runtime tables) |
| **Overall** | **~12%** |

## Entity model

**Client C++ hierarchy** (vtable-confirmed): `Squad → Organization ← Team ← Command`. Organization ctor
`0x00e4c570` registers 16 CME event subscribers. Squad ctor delegates via `0x00e5cc40`; Team `0x00eb4140`;
Command `0x00eb3000`.

**`EOrganizationType`**: `Squad=0, Team=1, Command=2`. `EPersistentOrganizationType` lists only
`POT_Team`, `POT_Command` — **Squad is not persisted**.

**Server entities**: `SGWPlayerGroupAuthority` (`<ServerOnly/>`, implements `GroupAuthority`) is a
singleton authority owning all groups in `authGroups: PYTHON`. Methods: `joinGroup`, `leaveGroup`,
`leaveGroupByName`, `callMethodOnGroup`. In Cimmeria this maps to a **base-side singleton service**, not
per-player state.

**`OrganizationMember` interface** (on SGWPlayer): `records: PYTHON` (CELL_PRIVATE, org→data),
`squad: INT32` (**CELL_PUBLIC** — replicated to AoI observers so they can see squad membership),
`strikeTeamTimers`, `pendingPvPTimers`, `pendingGroups`, `pendingJoins`, `pendingInvitesByType`. **All org
state lives in `records` and is pushed via explicit client methods — there are NO replicated org
properties.**

### Rank system (9 levels — DIVERGENCE ALERT)

`EORG_RANK_*`: `0 None, 1 Initiate, 2 Member, 3 SeniorMember, 4 Veteran, 5 SeniorVeteran, 6 Officer,
7 SeniorOfficer, 8 Leader`. **`crates/game/src/social/guilds.rs` defines only 3 ranks** (Member/Officer/
Leader) — the wire sends `UINT8` 0–8; the 3-rank model corrupts the field. Must be replaced with a 9-value enum.

### Permission system (26-bit bitmask)

`EORG_PERM_*` bits 0–25: DoNotUse(0x1), Invite(0x2), Promote(0x4), Demote(0x8), Eject(0x10),
RosterNotes(0x20), OfficerNotes(0x40), RankNames(0x80), OfficerChat(0x100), EmailLists(0x200), MOTD(0x400),
HistoryLog(0x800), Calendar(0x1000), RecruitDesc(0x2000), Adjectives(0x4000), Insignia(0x8000),
DepositBank(0x10000), WithdrawBank(0x20000), DepositCash(0x40000), WithdrawCash(0x80000), ViewBankLogs(0x100000),
LeaderChat(0x200000), AllianceChat(0x400000), AlterPerms(0x800000), TransferLeader(0x1000000), AllianceCmds(0x2000000).

## Wire message catalog

Existing `organization-wire-formats.md` is substantially accurate. **Additions discovered**:
`onOrganizationHeaderUpdate` (cell method — bulk header dump: orgId, name, UINT64 XP, MOTD, UINT64 cash, on
join/login, before the roster dump); `receivedMinimapPing` (org-wide fanout of CM 10);
`organizationInviteResults` (internal invite-handshake callback); `onOrganizationCreation` cell method.

**Client methods (S→C, on OrganizationMember)**: 34 onOrganizationInvite, 35 onOrganizationJoined,
36 onOrganizationLeft, 37 onMemberJoinedOrganization, 38 onOrganizationRosterInfo, 39 onMemberLeftOrganization,
40 onMemberRankChangedOrganization, 41 onStrikeTeamUpdate, 42 onPvPOrganizationLeaveRequest,
43 onOrganizationNameUpdate, 44 onOrganizationExperienceUpdate, 45 onOrganizationMOTDUpdate,
46 onOrganizationNoteUpdate, 47 onOrganizationOfficerNoteUpdate, 48 onOrganizationCashUpdate,
49 onOrganizationRankUpdate, 50 onOrganizationRankNameUpdate, 51 onSquadLootType. (SGWPlayer methods:
134 onOrganizationCreationResult, 135 launchOrganizationCreation.)

**Cell methods (C→S)**: 8 organizationInviteResponse, 9 organizationLeave, 10 BroadcastMinimapPing,
11 strikeTeamResponse, 12 pvpOrganizationLeaveResponse, 13 organizationMOTD, 14 organizationNote,
15 organizationOfficerNote, 16 organizationSetRankPermissions, 17 organizationSetRankName,
18 squadSetLootMode, 19 organizationTransferCash. **Bug**: the Rust stub for CM 17 decodes only 8 bytes
(orgId+rank) and drops the trailing WSTRING.

**Base methods (C→S)**: organizationInvite (orgId, WSTRING name), organizationInviteByType (UINT8 type,
WSTRING name), organizationKick, organizationRankChange (…, UINT8 rank), organizationCreation (UINT8 type, WSTRING name).

**Slash commands** confirmed in binary (RTTI strings `0x0184212c`–`0x0184244c`): Squad/Team/Command
Invite/Accept/Decline, SquadKick, SquadPromote, SquadLeave, ChooseOrgName, ReloadOrganizations (dev).
`TargetSquadMember1..6` at `0x0184094c`–`0x018409ec`.

## Flows (reconstructed)

- **Creation**: `/chooseOrgName` → `organizationCreation(type,name)` base → `GroupAuthority.joinGroup` →
  `onOrganizationJoined` cell → `onOrganizationCreationResult` [134]. The server sends
  `launchOrganizationCreation` [135] to OPEN the dialog (server-gated), before the client names the org.
- **Invite**: `organizationInvite`/`…ByType` base → resolve name → `organizationInvite` cell on target →
  `onOrganizationInvite` [34] → `organizationInviteResponse` [CM 8] → on accept: `joinGroup`,
  `onMemberJoinedOrganization` [37] to existing, `onOrganizationJoined` [35] + `onOrganizationRosterInfo`
  [38] to new member.
- **Leave/Kick**: `organizationLeave` [CM 9] / `organizationKick` base → `GroupAuthority.leaveGroup` →
  `onMemberLeftOrganization` [39] to members + `onOrganizationLeft` [36] to departer. (EReasons enum values
  not yet recovered — see open Q.)
- **Strike-team PvP**: `onStrikeTeamUpdate` [41] / `onPvPOrganizationLeaveRequest` [42] →
  `strikeTeamResponse` [11] / `pvpOrganizationLeaveResponse` [12]; timers auto-decline on expiry.

## Current Rust gaps

No `SGWPlayerGroupAuthority` handler; no org state on SGWPlayer; zero roster fanout; no persistence
(`Guild::save`/`load` are `todo!()`, no `sgw.organizations*` tables); wrong rank model; no base-method
handlers; chat channels CHAN_SQUAD/COMMAND/OFFICER defined but not org-routed; `squad` CELL_PUBLIC not synced.

## Open questions

1. EReasons enum values for `onOrganizationLeft` (left/kicked/disbanded). → x64dbg.
2. `RosterInfo` element layout — does it carry an `isOnline` bool not in the .def? → x64dbg.
3. `launchOrganizationCreation` trigger timing (first login? NPC interaction?). → x64dbg.
4. Cash field width — .def says UINT64; confirm not UINT32 in practice. → x64dbg.

## Dynamic-analysis needs (x64dbg)

- BP `0x00d8a360` (onOrganizationInvite register caller) — confirm WSTRING/byte order.
- BP `0x00d8ade0` (onOrganizationRosterInfo register) — capture `RosterInfo` ARRAY; check `isOnline`.
- BP `0x00d8c9f0` (onLaunchOrganizationCreation) — 1-byte payload; trace callers for trigger timing.
- BP `0x00e4c570` (Organization ctor) — map the 16 CME subscriptions to event names.
- BP `0x00d8c290` (onOrganizationCashUpdate) — confirm UINT64 LE.

## Ghidra annotations

None applied this session. Recommended renames: `0x00e4c570`→`Organization_ctor_body`,
`0x00e5cc40`→`Squad_ctor_body`; plate comments on Squad/Team/Command vfunc_0 ctors.
