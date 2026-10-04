---
type: reference
audience: engineers implementing the native combat-debug surface (AB-N1) and GM ability-test commands (AB-N2)
last_updated: 2026-10-04
companion_docs:
  - ability-client-hook-anchors.md
  - client-wire-emit-suppression.md
  - chat-wire-formats.md
  - ../../protocol/cell-method-dispatch-table.md
  - ../../protocol/client-method-dispatch-table.md
  - ../../commands.md
  - ../../analysis/ability-mechanics/lab-uat-and-telemetry.md
---

# Native combat debug (AB-N0)

> **Diátaxis type**: reference
> **Packet**: AB-N0 of the ability-mechanics telemetry plan
> **Binary**: the QA `SGW.exe`; headless Ghidra 12.0.4, 2026-10-04, static only
> **Confidence legend**: **VERIFIED** = read in a decompile, disassembly or string search in this pass. **INFERRED** = follows from verified code plus naming. **UNRESOLVED** = not found; the next action is named.

## Summary

The plan for AB-N1 assumed the server could send `onSendCombatDebug(simple, verbose)` to the client and that the client would render it. **It cannot, and it does not.** Three of the assumptions in the AB-N0 question list are false:

1. `onSendCombatDebug` and `onSendEventDebug` are **not client methods**. In `entities/defs/SGWPlayer.def` they sit inside `<CellMethods>` (lines 564 to 1109 at the time of writing; the two methods are at lines 685 and 690), without `<Exposed/>`: server-internal cell methods that no one calls over the wire. The client's method-name table has no string `onSendCombatDebug` or `onSendEventDebug`, so the client has no handler, no event and no renderer for either. They have no client method index.
2. The cell methods `toggleCombatDebug` (2) and `toggleCombatVerboseDebug` (3) are exposed, but **no client event is bound to them**. The client has no string `toggleCombatDebug` or `toggleCombatVerboseDebug`, so the stock client can never send cell 2 or 3. The same is true of `toggleAbilityDebugging`, `setAbilityDebugTarget`, `clearAbilityDebug`, `toggleHealDebug` and `toggleBehaviorDebugging`.
3. What the client **can** send is four GM debug methods on `SGWGmPlayer`, through four `Event_NetOut_*` events, reached by four slash commands. A non-GM account cannot send them: the client drops the call before it reaches the wire.

The one client-visible text path that already exists is the chat line: `onPlayerCommunication` (client method 28) on the feedback channel. AB-N1 should use it.

## What the client can send

The client binds each of these in its outgoing-method bind sweep (`FUN_00db3390`, the function the annotation script named `register_NetOut_onStrikeTeamResponse`). Each bind pushes the method-name string and the class-name string `"SGWGmPlayer"` (`0x019c3780`, `0x019c379c`, ...), then calls a per-event registration thunk that constructs a `SGWNetworkManager::EventHandler<Event_NetOut_*>` and subscribes the shared callback `0x00d43dc0` ([ability-client-hook-anchors.md](ability-client-hook-anchors.md)).

| Slash event | NetOut event | Method string | Cell index | Arguments | Evidence |
|---|---|---|---|---|---|
| `Event_SlashCmd_AbilityDebug` | `Event_NetOut_AbilityDebug` | `gmDebugAbility` (`0x019c3770`) | 169 | `INT32 aAbilityId` | bind at `0x00dc555e`, thunk `0x00d71b30`, handler ctor `0x00d618f0` (vtable symbol names the event): VERIFIED |
| `Event_SlashCmd_CombatDebug` | `Event_NetOut_CombatDebug` | `gmDebugCombat` (`0x019c378c`) | 170 | none | bind `0x00dc55d8`, thunk `0x00d71c00`, ctor `0x00d61a30`: VERIFIED |
| `Event_SlashCmd_CombatDebugVerbose` | `Event_NetOut_CombatDebugVerbose` | `gmDebugCombatVerbose` (`0x019c37a8`) | 171 | none | bind `0x00dc5652`, thunk `0x00d71cd0`, ctor `0x00d61b70`: VERIFIED |
| (heal) | `Event_NetOut_HealDebug` | `gmDebugHeal` (`0x019c37cc`) | 172 | none | bind `0x00dc56cc`, thunk `0x00d71da0`, ctor `0x00d61cb0` (vtable symbol names `HealDebug`): VERIFIED |
| `Event_SlashCmd_DebugAbilityOnMob` | `Event_NetOut_DebugAbilityOnMob` | `gmDebugAbilityOnMob` (`0x019c384c`) | 176 | `INT32 AbilityID` | bind `0x00dc58b4`, thunk `0x00d720e0`, ctor `0x00d621b0`: VERIFIED |

The cell indices are the flat indices in [cell-method-dispatch-table.md](../../protocol/cell-method-dispatch-table.md) (rows 169 to 172 and 176). The argument key names are the `.def` `ArgName`s (`SGWGmPlayer.def`: `aAbilityId`, `AbilityID`), which is what the bag lookup uses ([ability-client-hook-anchors.md](ability-client-hook-anchors.md#the-event-bag)). Note the two different spellings.

**Not bound at all:** `Event_NetOut_ToggleCombatLOS` (`toggleCombatLOS`) is bound, but there is no NetOut for `toggleCombatDebug`, `toggleCombatVerboseDebug`, `toggleHealDebug`, `toggleAbilityDebugging`, `setAbilityDebugTarget` or `clearAbilityDebug`. There is no `Event_NetIn_*` for `onSendCombatDebug` or `onSendEventDebug` either.

## Can a non-GM account send them?

**No.** `RouteOutgoingEntityRpc` (`0x00c6fc40`) starts from the local player's entity type and climbs its class chain looking for the class named in the method description (`*(short*)(desc+0x1e)`), using `0x0158eca0` to step to each parent (`0x00c6fcf5` to `0x00c6fd41`). A normal player is an `SGWPlayer` (client index 2); its chain is `SGWPlayer`, `SGWBeing`, `SGWSpawnableEntity`, `SGWEntity` and never reaches `SGWGmPlayer` (client index 3). The walk ends at `0x00c6fd1d` / `0x00c6fd41` and the function returns **without sending and without a log line**. VERIFIED by control flow.

So the gate that matters on the wire is the avatar's class: a GM account's avatar must be an `SGWGmPlayer` (the class flip of issue #518 in the project notes); then the same four events pass. This is independent of the server's own `gm_gate.rs`, which still has to refuse a client that crafts the packet.

`toggleCombatDebug` (2) and `toggleCombatVerboseDebug` (3) are exposed on the server, so a crafted packet reaches them, but the stock client never produces one.

## The slash commands

The command names are the runtime keywords captured from the live client's `SGWTextCommandMgr` map on 2026-06-17 (266 entries, recorded in [commands.md](../../commands.md)):

| Keyword | Event class (name string, vtable) | Arguments |
|---|---|---|
| `/gmdebugability` | `Event_SlashCmd_AbilityDebug` (`0x01841f24`, vtable `0x018443d4`) | `<abilityId>` |
| `/gmdebugcombat` | `Event_SlashCmd_CombatDebug` (`0x01841ec8`, vtable `0x01844380`) | none |
| `/gmdebugcombatverbose` | `Event_SlashCmd_CombatDebugVerbose` (`0x01841ee4`, vtable `0x0184439c`) | none |
| `/gmdebugabilityonmob` | `Event_SlashCmd_DebugAbilityOnMob` (`0x01841c24`, vtable `0x01844118`) | `<abilityId>` |
| `/gmdebugheal` | not looked up (the NetOut side is verified) | none |

`SGWTextCommandMgr` is the subscriber of each slash event (the `MemberCallback<NoSubject, SGWTextCommandMgr, ...Event_SlashCmd_AbilityDebug...>` type descriptor is at `0x01e04840`, VERIFIED), and its handler builds the matching `Event_NetOut_*`. The keyword-to-class pairing is by name: the keyword is `/` plus the lowercased `.def` method name (`gmDebugCombat` becomes `/gmdebugcombat`), which is also how `/gmsetgodmode` follows from `gmSetGodMode`. That the keyword is derived this way is INFERRED. The keywords are **not** present in the image as ASCII or UTF-16 literals (searched for `gmdebugcombat`, `gmdebugability`, `debugcombat`, `abilitydebug`, `gmdebugheal`), so the map is built at runtime. **Next action** to prove the derivation: decompile `SGWTextCommandMgr`'s constructor (`0x00c8d0f0`) around the `Event_SlashCmd_CombatDebug` registration thunk, or read the map again on a live client.

`commands.md` marks these commands "Not yet" or "Partly". That column describes the server, not the client; the client accepts all five for a GM account.

## Where a debug line can be shown

There is no native debug window. The existing text paths into the client are:

| Path | Client method | Channel | Notes |
|---|---|---|---|
| `onPlayerCommunication` | 28: `WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel, WSTRING Text` | `CHAN_feedback` = 9 | the sky-blue Info-tab line used for GM feedback, refusals and the login welcome (`crates/wire/src/cell/chat.rs:42`); no popup |
| `onSystemCommunication` | 27 | by `StringId` | needs a string-table id; not suitable for free text |

`CHAN_server` (8) opens a modal "Server Message" popup and is wrong for routine debug text. There is **no combat channel on the wire**: `EChannel` (`entities/defs/enumerations.xml`) has no `CHAN_combat`; the Lua `UIChannel.Combat` is a client-local channel filled by `CHAT_onUnitCombat` (`ChatEvents.lua:204`), so a server line cannot be addressed to it.

## What AB-N1 should do

- **Deliver** the formatted debug text as `onPlayerCommunication` on `CHAN_FEEDBACK`, from the existing chat serializer, to the debug target. Do not add a client method: a new client method is a new opcode and needs a maintainer decision, and the client could not bind it without a patch anyway.
- **Toggle** on the four commands the client can really send (`gmDebugCombat` 170, `gmDebugCombatVerbose` 171, `gmDebugAbility` 169, `gmDebugAbilityOnMob` 176), not on cell 2 and 3. Keep cells 2 and 3 working for crafted callers, but do not describe them as reachable.
- **Say so in the docs.** The `onSendCombatDebug` and `onSendEventDebug` rows are server-internal (the dispatch tables now say so), and `docs/commands.md`'s debugging section should note that the five commands need a GM avatar.
- **Line length.** A feedback line is one chat line. Keep the simple form to one line per cast and put the verbose detail on `abilities.debug` (SigNoz); a multi-line verbose dump needs several `onPlayerCommunication` messages.

## Open questions

| Question | Evidence that resolves it | Next action |
|---|---|---|
| Is a GM account's avatar really an `SGWGmPlayer` on the client, so the walk at `0x00c6fcf5` succeeds? | a live `client.net.out` for `gmDebugCombat` from a GM account | first lab run with a GM character; the `client.net.out` hook already reports the method name |
| Is the slash keyword really derived from the method name? | the registration thunk or a live map read | decompile `0x00c8d0f0`'s thunk for `CombatDebug` |
| Which `Event_SlashCmd_*` class backs `/gmdebugheal`? | the `SGWTextCommandMgr` registration thunk | search `Event_SlashCmd_HealDebug` and read its subscriber |
