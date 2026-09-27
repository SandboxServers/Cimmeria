# Organization / Squad / Guild System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire format: .def + binary RTTI); MEDIUM (server logic: Python stubs only); LOW (persistence schema: no original DB recovered)
> **Sources**: `entities/defs/interfaces/{OrganizationMember,GroupAuthority}.def`,
>   `entities/defs/SGWPlayerGroupAuthority.def`, `python/Atrea/enums.py`,
>   `deprecated/python/{base,cell}/SGWPlayer.py`, `deprecated/python/{base,cell}/SGWPlayerGroupAuthority.py`,
>   `db/resources/Social/Types/*.sql`, `SGW.exe` Ghidra, `crates/services/src/cell/{cell_methods,client_methods}/organization.rs`,
>   `crates/game/src/social/guilds.rs` (deleted in #614), `docs/reverse-engineering/findings/organization-wire-formats.md`
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
7 SeniorOfficer, 8 Leader`. The old `crates/game/src/social/guilds.rs` sketch defined only 3 ranks (Member/Officer/
Leader), which would corrupt the `UINT8` 0–8 wire field. It had no callers and was deleted in #614; any Rust
rank model must use the full 9-value enum.

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
WSTRING name), organizationKick, organizationRankChange (…, UINT8 rank). Creation is the exposed **cell** method 94
`onOrganizationCreation(WSTRING name)`; the type is implied by the dialog the server opened (corrected 2026-09-27).

**Slash commands** confirmed in binary (RTTI strings `0x0184212c`–`0x0184244c`): Squad/Team/Command
Invite/Accept/Decline, SquadKick, SquadPromote, SquadLeave, ChooseOrgName, ReloadOrganizations (dev).
`TargetSquadMember1..6` at `0x0184094c`–`0x018409ec`.

## Flows (reconstructed)

- **Creation**: registrar NPC → `launchOrganizationCreation(type)` [135] → `chooseOrgName` → `onOrganizationCreation(name)` cell 94 → `GroupAuthority.joinGroup` →
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

## ORG-E1 client evidence (2026-09-26, static Ghidra + client Lua/.int tree)

All six ORG-E1 packet questions plus Q7 (EDBErrorType text). No debugger was used; every claim below is
either a decompiled function body, a literal byte read from the binary, or a client Lua/`.int` grep. See
[analysis/organizations/worknotes/org-e1.md](../../analysis/organizations/worknotes/org-e1.md) for the
full evidence table and server-action recommendations.

### Q1 — Where the roster record's "member id" (offset 0) comes from — CONFIRMED

Fully traced the write side of the `piVar2`/`piVar4` roster-record table that `teamGetMemberInfo` reads
(A-11). Two, and only two, client methods ever touch a roster record's offset-0 (id) field:

- **`onOrganizationRosterInfo` [38]** (handler `0x00e4ea50`, `Organization::vfunc[0x20]`): for a name it has
  not seen before, it allocates a new record and **hardcodes the id argument to `0`**
  (`puStack_144 = (undefined4 *)0x0;` immediately before the `FUN_00e4b680` builder call). For a name it
  already has, it updates **only** `rank` (`puVar4[1] = local_146`) — it never touches the id field of an
  existing record. RosterInfo carries no id on the wire (confirmed by A-11/Q2-closed), so this is entirely a
  client-side default, not a value from any packet.
- **`onMemberJoinedOrganization` [37]** (handler `Mercury__unknown_00e4e4c0` @ `0x00e4e4c0`,
  `Organization::vfunc[0x1F]`): reads `aMember` (INT32) off the wire. For a **new** name (not found by the
  by-name lookup `ZipFileSystem__unknown_00e4b890`), the new record is built with `aMember` as its id
  (`FUN_00e4b680(puVar5,&puStack_98,rank,name,note)`, second arg = `aMember`) and, if `aMember != 0`, an
  entity-id → record reverse-lookup entry is registered (`ServerConnection__unknown_00575ff0`). For an
  **existing** name, the handler unconditionally compares the record's current id to the new `aMember` and,
  if different, unregisters the old reverse-lookup entry, registers the new one (if `aMember != 0`), and
  **overwrites the record's id field in place** (`*piVar4 = (int)puStack_98;`) — regardless of `aNewMember`.

  Server-facing conclusion: **`onOrganizationRosterInfo` always seeds every roster row as id 0 ("Offline").
  Only a subsequent `onMemberJoinedOrganization(aMember=<entity id>, aNewMember=0)` for that name can turn a
  row "Online"** by giving it a live, resolvable id. This settles the ORG-06 login sequence: send
  `onOrganizationRosterInfo` [38] for the full roster first, then, for every member whose entity happens to
  be visible/streamed to this client already, follow with `onMemberJoinedOrganization(aNewMember=0)` carrying
  their real entity id. No such follow-up is needed for members who are not currently streamed — `0` is
  already the roster-info default. `onMemberLeftOrganization` [39] (handler `0x00e4f400`,
  `Organization::vfunc[0x21]`) removes the record from the table entirely on `Logout`/`Kicked`/`Disbanded`
  (not just zeroing the id), consistent with a genuine departure rather than a mere visibility change.

### Q2 — `squadKick`/`squadPromote`/`squadLeave`/the `/squadinvite` family — CONFIRMED, no debugger needed

`Squad`, `Team` and `Command` each embed a distinct `Organization`-subclass sub-object inline in the local
player object (`GameEntityManager::instance()->localPlayer + 0x6c` = Squad, `+0x70` = Team, `+0x74` =
Command — three separate vtable-pointer slots, confirmed by comparing `squadLeave`/`teamLeave`/`commandLeave`
and `squadKick`/`teamKick`/`commandKick`'s native bodies side by side). Every one of these natives resolves
to the **same virtual slot** on whichever sub-object it targets:

| Native (Squad / Team / Command) | Sub-object offset | Shared virtual slot | Wire method |
|---|---|---|---|
| `squadLeave` / `teamLeave` / `commandLeave` | `+0x6c` / `+0x70` / `+0x74` | `vfunc[0x20]` (offset 0x20) | cell `organizationLeave` [CM9], `aOrganizationId` = the target sub-object's own org id |
| `squadKick` / `teamKick` / `commandKick` | `+0x6c` / `+0x70` / `+0x74` | `vfunc[4]` (offset 0x10), 1 WSTRING arg | base `organizationKick(orgId, name)` |
| `squadInvite` (native @ `0xac89a0`) | `+0x6c` | `vfunc[3]` (offset 0xc), 1 WSTRING arg | shared `Organization::Invite(name)` — same slot family as `teamInvite`/`commandInvite`; disambiguated by the sub-object, not the wire message |
| `squadInviteAccept` / `squadInviteDecline` (natives @ `0xaab0b0`/`0xaab100`) | `+0x6c` | `vfunc[0xF]` (0x3c) / `vfunc[0x10]` (0x40) | cell `organizationInviteResponse` [CM8] (accept/decline share the same shape, one bit apart) |

**`squadKick(name)` is not a separate, undocumented wire path** — A-21's open question is closed: it sends
the identical base method `organizationKick(orgId, name)` that `teamKick`/`commandKick` send, just invoked on
the Squad sub-object so `orgId` resolves to the caller's squad id. This retracts the "needs an x64dbg trace"
recommendation in `annotation-script-shift-bugs.md`-style prior notes — virtual-slot comparison across the
three sibling natives is sufficient and required no debugger.

**`squadPromote` has no Lua native at all** (confirmed by string search: only `Event_SlashCmd_SquadPromote`
exists, a `SGWTextCommandMgr`-owned slash-command event, same as `Event_SlashCmd_TeamPromote` and
`Event_SlashCmd_CommandPromote` — promotion was never wired to a UI button/menu for **any** org type in the
shipped client, only to `/squadpromote`, `/teampromote`, `/commandpromote`). Handler bodies at `0x005a7e50`
(Squad) / `0x005a8fd0` (Command) / `0x005aa8d0` (Team) were not traced to their wire call this session
(budget); by the established shared-vtable pattern above, expect a rank-change virtual slot shared with
`organizationRankChange`.

### Q3 — Which client method shows another member's minimap ping — CONFIRMED: none exists

Searched the binary for every RTTI/string form of a minimap-ping event. Only
**`Event_NetOut_BroadcastMinimapPing`** exists (the outbound event fired when the client calls cell method 10
`BroadcastMinimapPing`, via native `shareMinimapPingWithSquadmates`). **There is no `Event_NetIn_*` minimap
ping event anywhere in the binary** — no client method exists for a server to push another member's ping to
this client. The client's own ping marker (native `createLocalMinimapPing`) is purely local prediction on the
sender's own screen, not driven by any server round-trip. This corrects
`organization-wire-formats.md`'s existing "Implementation Notes" #7, which speculated a `receivedMinimapPing`
cell method — no such method exists. **Server action**: accept and validate `BroadcastMinimapPing` [CM10],
but there is nothing to fan out to; ORG-04 should implement it as a logged no-op, exactly as its packet's
contingency already anticipates.

### Q4 / Q7 — Does anything turn `onOrganizationCreationResult`/`onErrorCode` into readable org text — LIKELY, mechanism found but not content

No `ERRORCODE_*`-prefixed strings exist anywhere in the binary, and no `.int` string is keyed by any
`RetCode` value or by the `EDBErrorType` codes (-20071, -20072, -20090…-20092). However, `onErrorCode` [client
method 121] is `(UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID)` (`client-method-dispatch-table.md:268`)
— **not** a bare code — and the binary contains a substantial, genuinely data-driven
**`CookedData:ErrorTextType`** system (`CookedErrorTextType`, streamed through the same
`ServerSource`/`ZipStorage`/`CookedDataListener` cooked-data-cache pipeline used for other cooked categories,
confirmed by 33 distinct RTTI/mangled-name hits). This means: **error text is not a static client table —
it is a cooked-data resource the client expects the *server* to stream to it on demand**, keyed presumably by
`(SystemID, ErrorCodeID)`. Since the original Python server never implemented this category (no local cooked
package or `.int` file for it was found either), there is no evidence any org-specific error code ever had
authored text in the shipped game. The `EDBErrorType` values themselves (`EDB_ERROR_Player_in_org_type` etc.)
are 5-digit negative sentinels that cannot fit in the UINT16 `ErrorCodeID` field verbatim — they read as
pre-wire, server-internal DB-layer semantics, not values ever placed on the wire directly. **Server action**:
do not assume `onErrorCode` renders readable text for org rejections out of the box — Cimmeria would need to
author and serve its own `CookedData:ErrorTextType` category for that, which nothing in the codebase does
today. Prefer the ledger's other two feedback channels (a re-send of true state, and/or a chat-system
message) for org rejections until/unless that cooked category is implemented. Not chased further this
session: the exact `Event_NetIn_onErrorCode` handler body (a `Communicator`-owned `FreeCallback`) was not
located inside budget, so whether it even attempts the cooked-text lookup on receipt (vs. showing nothing)
is unconfirmed.

### Q5 — Does the client hardcode any `EChannel` id — CONFIRMED YES, and it settles D-ORG14

The client-side `UIChannel` Lua global is not a plain table — each field (`UIChannel.Officer`,
`UIChannel.Tell`, …) is a **native getter function** that pushes a hardcoded IEEE-754 double literal. Read
every literal directly out of the binary's data section:

| `UIChannel.*` | Getter | Literal (double) | Matches `enumerations.xml` `EChannel` |
|---|---|---|---|
| `Say` | `0xaa2160` (`FLDZ`) | 0 | `CHAN_say` = 0 ✓ |
| `Emote` | `0xaa2360` (`FLD1`) | 1 | `CHAN_emote` = 1 ✓ |
| `Yell` | `0xa9eab0` | 2 | `CHAN_yell` = 2 ✓ |
| `Team` | `0xa9ea90` | 3 | `CHAN_team` = 3 ✓ |
| `Squad` | `0xa9e610` | 4 | `CHAN_squad` = 4 ✓ |
| `Command` | `0xa9e630` | 5 | `CHAN_command` = 5 ✓ |
| `Officer` | `0xa9e650` | **6** | `CHAN_officer` = 6 ✓ |
| `Server` | `0xa9e690` | **8** | `CHAN_server` = 8 ✓ |
| `Feedback` | `0xa9e6b0` | **9** | `CHAN_feedback` = 9 ✓ |
| `Tell` | `0xa9e6d0` | **10** | `CHAN_tell` = 10 ✓ |
| `Splash` | `0xa9e6f0` | 11 | `CHAN_splash` = 11 ✓ |
| `Chat` | `0xa9e710` | 12 | `CHAN_chat` = 12 ✓ (threshold for user-created channels) |
| `GMTell` | `0xa9ed80` | -1 | not a wire value — client-local display category |
| `Mission` | `0xa9ed60` | -3 | not a wire value — client-local display category |
| `Combat` | `0xa9ed40` | -4 | not a wire value — client-local display category |

Every value matches `enumerations.xml`'s `EChannel` exactly, byte for byte — the client's compiled hardcoded
constants ARE the canonical enum this repo already documents. **D-ORG14 is CONFIRMED, not just proposed**:
`docs/gameplay/chat-system.md` and `crates/wire/src/cell/chat.rs`/`world_entry_chat.rs`'s use of server=7,
tell=9 is wrong and must change to server=8, tell=10 (feedback=9, officer=6 are already right by
coincidence). **`onChatJoined` [31] is not how the client learns these ids** — `Say`/`Yell`/`Team`/`Squad`/
`Command`/`Officer`/`Server`/`Feedback`/`Tell`/`Splash` are permanently baked into the client and require no
registration; `onChatJoined` only matters for dynamic, server-assigned ids ≥ `CHAN_chat` (12), i.e. real
player-created channels. **Officer (6) is accepted without registration** — the client recognizes the byte
value natively, exactly like team (3) or squad (4).

### Q6 — Which Lua path shows the squad member frames — CONFIRMED: entity presence, not roster query

`SquadMod.setupSquadMember(unitId, suffix)` (`Squad.lua:13`) binds each of the six squad frames directly to a
`unitId` via `UnitFramesMod.registerFrame(...)` — the same generic unit-frame/entity system used for target
frames, not the `teamGetMemberInfo`/roster-query mechanism Team and Command panels use. The frame is hidden
outright when the entity is absent: `if (not unitExists(unitId)) then win:hide() end`. `unitExists` resolves
against the client's local entity cache (the same AoI-bound cache `teamGetMemberInfo` reads for Team/Command
"Online" — see A-11). **A squad member in another space (not streamed to this client) has no local entity, so
`unitExists` returns false and that frame simply disappears** — there is no "Offline" state for squad frames
the way there is for the Team/Command roster panel; the slot goes blank instead. This is a real UX asymmetry
between Squad and Team/Command worth carrying into ORG-03/ORG-04's design: Cimmeria's squad member entities
(`Unit.Squad1..6`) need no special cross-space replication to make the frames work correctly — they are
expected to blank out when the member isn't in AoI, matching original client behavior, not a bug to route
around.

## Open questions

1. ~~EReasons enum values~~ — **closed**: requested 0, kicked 1, disbanded 2, logout 3 (`enumerations.xml:104`).
2. ~~`RosterInfo` `isOnline`~~ — **closed** (2026-09-27, static Ghidra): no online field. The client derives "Online" from a non-zero member id that resolves to a `GamePlayer` in its own entity table (`teamGetMemberInfo` `0x00ac8c70` → builder `0x00ae83d0`). See [analysis/organizations/audit.md](../../analysis/organizations/audit.md) A-11.
3. `launchOrganizationCreation` trigger timing — **likely** a registrar NPC interaction (`EInteractionType.OrganizationCreation = 9`; the `.int` strings send players to a team registrar on Harset and a command registrar at the Omega Site). The client only opens `CreateTeamWin` / `CreateCommandWin` on receipt.
4. Cash field width — .def says UINT64; confirm not UINT32 in practice. → x64dbg.
5. ~~Roster-record id provenance~~ — **closed** (2026-09-26, static Ghidra, ORG-E1 Q1): see "ORG-E1 client evidence" above.
6. ~~`squadKick` wire path~~ — **closed** (2026-09-26, static Ghidra, ORG-E1 Q2, closes A-21): shares `organizationKick` with `teamKick`/`commandKick`.
7. ~~Receiving-side minimap ping client method~~ — **closed** (2026-09-26, static Ghidra, ORG-E1 Q3): does not exist.
8. ~~`EChannel` hardcode vs. `onChatJoined`~~ — **closed** (2026-09-26, static Ghidra, ORG-E1 Q5, confirms D-ORG14): every well-known channel id is a hardcoded client literal, byte-identical to `enumerations.xml`.
9. ~~Squad frame roster-vs-entity path~~ — **closed** (2026-09-26, static Lua reading, ORG-E1 Q6): entity presence (`unitExists`), not roster query.
10. `onErrorCode`/`onOrganizationCreationResult` readable text (ORG-E1 Q4/Q7) — **mechanism found, content unconfirmed**: a data-driven `CookedData:ErrorTextType` cooked-cache system exists in the binary, but no authored text for any org error code was found; the exact `Event_NetIn_onErrorCode` handler body was not traced this session. → needs either a deeper static pass on the `Communicator`-owned `FreeCallback` handler, or x64dbg on a live rejection.

## Dynamic-analysis needs (x64dbg)

- BP `0x00d8a360` (onOrganizationInvite register caller) — confirm WSTRING/byte order.
- ~~BP `0x00d8ade0` (onOrganizationRosterInfo register) — capture `RosterInfo` ARRAY; check `isOnline`~~ — superseded by static evidence (ORG-E1 Q1 above); the handler body (`0x00e4ea50`) and its id-field behavior are fully decompiled.
- BP `0x00d8c9f0` (onLaunchOrganizationCreation) — 1-byte payload; trace callers for trigger timing.
- ~~BP `0x00e4c570` (Organization ctor) — map the 16 CME subscriptions to event names~~ — done statically this session (ORG-E1 pass): all 16 `Organization::vfunc` slots are now mapped to their `Event_NetIn_*` subscriptions (see the Q1/Q2 vtable-slot tables above for the ones that matter to this campaign).
- BP `0x00d8c290` (onOrganizationCashUpdate) — confirm UINT64 LE.
- NEW: BP the `Event_NetIn_onErrorCode` `Communicator` handler on a live org rejection, to settle Q4/Q7's remaining "does it even attempt a cooked-text lookup" question.

## Ghidra annotations

ORG-E1 session (2026-09-26): created (undefined-function-body) entries at `0x00e536e0`, `0x00e53690`,
`0x00e53720` (the three Organization vtable-thunk jumps for RosterInfo/MemberJoined/MemberLeft), and at
`0xac8a90`, `0xac9d40`, `0xac91e0`, `0xac89a0` (squadKick/commandKick/teamKick/squadInvite native bodies), and
at the thirteen `UIChannel.*` getter addresses listed in the Q5 table — all previously undefined code ranges
inside larger, un-split functions. No renames applied; addresses are cited by raw `FUN_00xxxxxx` name above.
Recommended renames for a future pass: `0x00e4c570`→`Organization_ctor_body`, `0x00e5cc40`→`Squad_ctor_body`;
plate comments on Squad/Team/Command vfunc_0 ctors; `0x00e4ea50`→`Organization_onOrganizationRosterInfo`,
`0x00e4e4c0`→`Organization_onMemberJoinedOrganization`, `0x00e4f400`→`Organization_onMemberLeftOrganization`.
