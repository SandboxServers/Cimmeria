---
name: ability-client-hook-anchors-ab-c0
description: AB-C0/AB-N0 recovery (2026-10-04) - the client's single outgoing-RPC seam, central event dispatch, ability press drop branches, and the corrected "onSendCombatDebug is not a client method" premise; plus headless-Ghidra tips learned
metadata:
  type: project
---

Findings docs: `docs/reverse-engineering/findings/ability-client-hook-anchors.md` and `native-combat-debug.md` (read these first; this note is only the non-obvious lessons).

- **Event bag.** CME events carry args as a name-keyed `std::map<std::string, PropertyBase*>` at `event+4`; keys are the `.def` `ArgName`s. Read them with the game's own `0x00e3cba0` (int), `0x00e3cc20` (float), `0x00d434d0` (byte): `bool thiscall(event, std::string* name, T* out)`. Arrays are `BasicPropertyList`, layout unread: decode arrays from wire bytes instead.
- **One outgoing seam.** `0x00c6fc40` has one caller, the shared NetOut member callback `0x00d43dc0` (`this+4` desc, `this+8` method). `Channel::send` is too early for the Mercury seq: the seq is assigned on the network thread by `0x0158bb40` inside `Nub::send` `0x01582160`.
- **One dispatch seam.** `0x00a372f0` (`thiscall(sys, subject, event, baseTD, classTD)`, `ret 0x10`) delivers every event, `Event_UI_*` included; UI events pass a plain struct, not a bag.
- **Press path has no cooldown/range/dead/target gate.** The silent drop is `FUN_00d2afc0`'s not-found branch (`0x00d2afcf`): ability not in the client's `AbilitySet`. `0x00d2b020` is `addAbilityIfAbsent`, NOT the "in-flight queue gate" earlier docs claimed; `0x00403280` is `tolua_isnoobj` (third arg must be absent).
- **Premise errors found.** `onSendCombatDebug`/`onSendEventDebug` are non-exposed *cell* methods (SGWPlayer.def:685/690), no client handler; `toggleCombatDebug`/`Verbose` (cell 2/3) have no client NetOut so the stock client cannot send them; the four `gmDebug*` NetOut events only pass `RouteOutgoingEntityRpc` for an `SGWGmPlayer` avatar (class-chain walk at `0x00c6fcf5`..`0x00c6fd41`). The cell table's `confirmationResponse` arg row was wrong (`INT32 aEffectId, UINT8 aAccepted`).
- **Naming traps.** `resetMyAbilities` is `Event_NetOut_RespecAbility`; `onTimerUpdate` is `Event_NetIn_TimerUpdate`; `onKnownAbilitiesUpdate` is `Event_NetIn_KnownAbilitiesUpdate`; the annotation script names the bind sweep `0x00db3390` `register_NetOut_onStrikeTeamResponse` (it binds many methods, both directions).
- **Headless Ghidra tips.** `Probe.java` gained `IF:` (whole function disassembly), `RE:` (instruction-text regex), `BYTES:` (hex dump for undefined regions) and `U16:`; use `+` not `,` in `I:`/`VT:` (cmd splits at commas). `cmd` also eats `^ & | < >`. Regions can be undefined in the project (`0x00cf33e0`, `0x00ac67e0`): `BYTES:` to find a prologue, then `F:`. The worktree sandbox refuses shell scripts with computed paths; run `analyzeHeadless.bat` from PowerShell with literal paths.
- **Still open** (listed in the finding): live check of `GameBeing+0xfc` as the current target, bag arrays, `Event_UI_*` field layouts, `0x00e0a810` removal body, `onErrorCode` tail at `0x00cf33e0`, slash keyword derivation.
