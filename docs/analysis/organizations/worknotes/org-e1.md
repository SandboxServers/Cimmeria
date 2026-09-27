# ORG-E1 Worknotes — Client Evidence

> Type: reference. Audience: organizations-campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md),
> [organization-restoration.md](../../../reverse-engineering/findings/organization-restoration.md),
> [organization-wire-formats.md](../../../reverse-engineering/findings/organization-wire-formats.md).

## Contract

- **Packet:** ORG-E1 — Client evidence (Wave 0). Writer: `game-archaeology-specialist`.
- **Method:** static Ghidra (decompile, disassemble, memory reads, xrefs) against `SGW.exe`, and the client
  Lua/`.int` tree at `..\SGW\Stargate Worlds-QA\Working\SGWGame\`. **No debugger was used** — every claim
  below is a decompiled function body, a disassembled instruction sequence, a literal byte/double read out
  of the binary's data section, or a client-side Lua/`.int` grep.
- **Base:** `origin/main` @ `95366c59`, rebased onto `origin/docs/org-campaign-plan` @ `6f8809bc` per the
  worker rules (stacks cleanly on plan PR #855).
- **Owned paths (this packet):**
  - `docs/reverse-engineering/findings/organization-restoration.md` (new "ORG-E1 client evidence" section,
    open-questions closures, dynamic-analysis-needs updates, Ghidra-annotations note)
  - `docs/reverse-engineering/findings/organization-wire-formats.md` (Implementation Notes #7/#8 — minimap
    ping correction and squad-native wire clarification)
  - `docs/analysis/organizations/worknotes/org-e1.md` (this file)
- **Read set:** `docs/analysis/organizations/work-packets.md` (ORG-E1 section), `README.md` (D-ORG11,
  D-ORG14), `audit.md` (A-11, A-20, A-21, A-40, A-43); prior RE scratchpad from this session
  (`scratchpad/org-re.md`, Q2/Q3/Q5/Q7/Q8/Q9/Q10 from an earlier pass — reused, not redone); the two
  findings docs above; `entities/defs/enumerations.xml` (`EReasons`, `EChannel`, `EDBErrorType`); the client
  Lua tree under `Content/UI/Core/{Squad,Team,Command,Organization,SelfWindow,ChatWindow,MiniMap}/`.

## Answers

| # | Question | Verdict | Key evidence (addresses) | Server action |
|---|---|---|---|---|
| 1 | How does the per-member roster record get its id (offset 0)? Does `onOrganizationRosterInfo` rebuild with id 0? Does `onMemberJoinedOrganization` update an existing name's id in place? | **CONFIRMED** | `onOrganizationRosterInfo` handler `0x00e4ea50` (`Organization::vfunc[0x20]`): new records get id hardcoded to `0` (`puStack_144 = 0` before the `FUN_00e4b680` builder call); existing records get only `rank` updated. `onMemberJoinedOrganization` handler `0x00e4e4c0` (`Organization::vfunc[0x1F]`): new records get id = wire `aMember`; existing records get `*piVar4 = (int)puStack_98` (id **overwritten in place**) whenever the new `aMember` differs from the stored one, regardless of `aNewMember`, plus a reverse entity-id→record lookup swap via `ServerConnection__unknown_00575ff0`/`...00e221a0`. | ORG-06 login sequence: send `onOrganizationRosterInfo` [38] for the full roster first (every row lands "Offline", id 0), then send `onMemberJoinedOrganization(aMember=<entity id>, aNewMember=0)` [37] only for members whose entity is already streamed/visible to this client. No follow-up is needed for members not currently streamed. |
| 2 | Which method(s) do `squadKick`, `squadPromote`, `squadLeave`, and the `/squadinvite` family send? | **CONFIRMED** (closes audit A-21) | `Squad`/`Team`/`Command` embed distinct `Organization` sub-objects at local-player `+0x6c`/`+0x70`/`+0x74`. `squadKick` (native `0xac8a90`) calls the same virtual slot (`vfunc[4]`, offset `0x10`) as `teamKick` (`0xac9d40`→`0xada0a0`, `+0x74`) and `commandKick` (`0xac91e0`→`0xad9dc0`, `+0x70`). `squadLeave` (`0xaab350`→`0xad9bf0`, `+0x6c`, `vfunc[8]`/`0x20`) matches `teamLeave`/`commandLeave` (`0xad9e90`/`0xada170`) on the same slot. `squadInviteAccept`/`squadInviteDecline` (`0xaab0b0`/`0xaab100`→`0xad9ac0`/`0xad9ae0`, `+0x6c`, `vfunc[0xF]`/`vfunc[0x10]`) share the accept/decline shape of `organizationInviteResponse` [CM8]. `squadInvite` (`0xac89a0`→`0xad9aa0`, `+0x6c`, `vfunc[3]`/`0xc`). `squadPromote` has **no Lua native** — only `Event_SlashCmd_SquadPromote` (`0x005a7e50`), mirrored by `Event_SlashCmd_TeamPromote`/`CommandPromote` — rank promotion was never wired to any UI button/menu for any org type in the shipped client. | Implement `squadKick`/`squadLeave`/`squadInvite*` as the *same* CM8/CM9/base-`organizationKick`/base-`organizationInvite` handlers Team/Command use, keyed off the caller's squad id — no separate squad-specific wire decoder needed. Rank-promotion has no discoverable UI path in the shipped client for any org type; treat `/squadpromote` (and the console GM commands) as the only entry points, not a missing button. |
| 3 | Which client method shows another member's minimap ping? | **CONFIRMED: none exists** | Only `Event_NetOut_BroadcastMinimapPing` (`0xad7640`/`0xaea220`/`0xaea230`/`0xaea350`/`0xd69470`) exists in the binary. No `Event_NetIn_*` minimap-ping event of any name was found. `createLocalMinimapPing` is local-only client prediction. | `BroadcastMinimapPing` [CM10] should be accepted and validated server-side but has nothing to fan out to — implement as a logged no-op (ORG-04's stated contingency), not a broadcast. Corrected `organization-wire-formats.md`'s prior (unconfirmed) claim of a `receivedMinimapPing` cell method. |
| 4 | Does anything turn `onOrganizationCreationResult`'s `Result`/`RetCode` into text? Which `onErrorCode` ids render readable org text? | **LIKELY / mechanism found, content unconfirmed** | No `ERRORCODE_*` strings and no `.int` string keyed by any `RetCode` or `EDBErrorType` value exists. `onErrorCode` [121] is `(UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID)`, and the binary has a real, data-driven `CookedData:ErrorTextType` cooked-cache system (33 RTTI hits: `CookedErrorTextType`, `ServerSource`, `ZipStorage`, `CookedDataListener`) that streams error text from the server on demand. The `Communicator`-owned `Event_NetIn_onErrorCode` `FreeCallback` handler body itself was not located/traced this session. | Do not assume `onErrorCode` shows readable text for org rejections without Cimmeria authoring and serving its own `CookedData:ErrorTextType` category — nothing in the codebase does this today. Prefer a re-send-of-true-state and/or a chat-system message for org rejection feedback until that's built. Flagged as a follow-up: trace the `Communicator` handler statically or capture a live rejection with x64dbg. |
| 5 | Does the client hardcode any `EChannel` id, or take every id from `onChatJoined`? Is officer (6) accepted without registration? | **CONFIRMED — settles D-ORG14** | Every `UIChannel.*` Lua field is a native getter returning a hardcoded IEEE-754 double, read directly out of the data section: Say=0, Emote=1, Yell=2, Team=3, Squad=4, Command=5, **Officer=6**, **Server=8**, **Feedback=9**, **Tell=10**, Splash=11, Chat=12 (GMTell=-1, Mission=-3, Combat=-4 are client-local, non-wire display categories). Every value matches `entities/defs/enumerations.xml`'s `EChannel` exactly. | **D-ORG14 is CONFIRMED, not merely proposed.** Fix `crates/wire/src/cell/chat.rs` and `world_entry_chat.rs` (currently server=7, tell=9) to server=8, tell=10 (feedback=9 and officer=6 already happen to be right). `onChatJoined` [31] is not how the client learns say/yell/team/squad/command/officer/server/feedback/tell/splash — those are permanent client constants; `onChatJoined` only matters for dynamic ids ≥ 12 (real player-created channels). Officer (6) is accepted without any registration, exactly like team/squad. |
| 6 | Which Lua path shows the squad member frames — squad roster or entity presence? Does a squad member in another space show at all? | **CONFIRMED: entity presence** | `SquadMod.setupSquadMember(unitId, suffix)` (`Squad.lua:13`) binds each frame to a `unitId` via `UnitFramesMod.registerFrame`, the generic unit-frame system — not `teamGetMemberInfo`'s roster query. `if (not unitExists(unitId)) then win:hide() end` hides the frame outright when the entity isn't in the client's local (AoI-bound) entity cache. | A squad member outside this client's AoI has no local entity, so their frame goes blank — there is no "Offline" state for squad frames the way there is for Team/Command rosters. `Unit.Squad1..6` need no special cross-space replication; blanking on out-of-AoI is expected original-client behavior, not a bug. |
| 7 | Do any client strings or Lua map `EDBErrorType` org codes (-20071, -20072, -20090..-20092) or `onErrorCode` ids to readable org text? | **UNRESOLVED (same finding as Q4)** | See Q4 — no static text mapping was found for these specific codes; the generic `CookedData:ErrorTextType` mechanism exists but its content isn't shipped/authored anywhere found. | Same as Q4: this is not currently the best feedback channel for org rejections until Cimmeria implements and serves cooked error text. Use re-sent true state and/or chat-system messages instead. |

## Design decisions

- Edited the two existing findings docs in new, clearly-dated sections rather than rewriting their existing
  prose, per the worker rules' preference to rebase onto `docs/org-campaign-plan` and add alongside plan
  PR #855's edits rather than collide with them.
- Created (but did not rename) several previously-undefined function bodies in Ghidra to get decompiles
  (listed in `organization-restoration.md`'s "Ghidra annotations" section) — these are internal vtable-thunk
  jumps and native bodies inside larger, un-split functions that the auto-analyzer had not split out. No
  renames were applied to avoid contested naming with concurrent RE sessions; addresses are cited directly.
- Did not chase the `Event_NetIn_onErrorCode` handler body to completion (Q4/Q7) — the `CookedData:ErrorTextType`
  discovery is a strong, decisive finding on its own (the mechanism exists and is server-fed, which changes
  the server-side recommendation regardless of exactly how the client renders the lookup miss), and further
  static tracing had a low expected payoff against the remaining session budget.

## Regression proof

Not applicable — this is a docs-only, read-only research packet. No code was changed. The commands run were
Ghidra MCP calls (`decompile_function`, `disassemble_bytes`, `read_memory`, `get_xrefs_to`,
`search_strings`/`search_functions_enhanced`, `create_function`) and shell `grep`/`file` calls against the
client Lua tree and the repo's own `entities/defs/enumerations.xml` and `docs/protocol/` tables. No `cargo`
commands apply to this packet.

## Known gaps / follow-ups

1. **Q2**: `squadPromote`/`teamPromote`/`commandPromote` slash-command handler bodies (`0x005a7e50`,
   `0x005a8fd0`, `0x005aa8d0`) were not traced to their final wire call — expected to share a rank-change
   virtual slot with `organizationRankChange`, by the same pattern established for Leave/Kick/Invite, but
   this is inference, not confirmed decompile.
2. **Q4/Q7**: the `Communicator`-owned `Event_NetIn_onErrorCode` `FreeCallback` handler body itself was not
   located. Confirming whether it even attempts a `CookedData:ErrorTextType` lookup on receipt (vs. silently
   dropping unresolvable codes) needs either a deeper static pass or an x64dbg capture of a live rejection.
3. This packet did not check whether `onChatJoined`'s `ChannelID` field is ever sent by the server for the
   twelve well-known channels in any circumstance (e.g., an idempotent re-announce) — only that the client
   does not *need* it for those ids.

## Contradicts / confirms the ledger

- **D-ORG11** (online status stays client-derived, server sends each member's entity id or 0) — **confirmed
  and sharpened** by Q1: the mechanism is specifically "`onOrganizationRosterInfo` seeds 0, a follow-up
  `onMemberJoinedOrganization` with a live id turns it Online," not merely "send some id." Recommend ORG-06
  copy that exact sequencing into its login-push implementation notes.
- **D-ORG14** (chat channel ids follow `enumerations.xml`) — **CONFIRMED**, upgraded from "proposed pending
  ORG-E1" to settled fact. No remaining reason to keep the current server=7/tell=9 values.
- **ORG-03's kick path** (work-packets.md: "per ORG-E1 Q2 it arrives either as a squad-range id on base method
  0xD1 or through its own path") — **the first branch is correct**: `squadKick` is base method `0xD1`
  (`organizationKick`) with a squad-range `orgId`, not a separate path. ORG-01's base arm forwarding a
  squad-range `orgId` to the cell (as ORG-03's packet already plans) is the right design; no change needed
  there, just confirmation.
- No finding here contradicts D-ORG09, D-ORG10, D-ORG12, D-ORG13, D-ORG15, D-ORG16 or D-ORG18.
