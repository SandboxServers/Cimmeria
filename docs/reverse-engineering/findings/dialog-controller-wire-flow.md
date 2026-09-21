---
title: "DialogController Wire Flow — display, queue, eviction, and the button-choice return path"
type: reference
audience: engineers doing RE or wiring dialog content
last_updated: 2026-09-21
---

# DialogController Wire Flow

> **Last updated**: 2026-09-21
> **Source**: SGW.exe read-only Ghidra pass (image base `0x00400000`, no renames or comments written), plus the uncooked client UI module at `…\SGWGame\Content\UI\Core\Dialog\` and colo SigNoz `dialogButtonChoice` telemetry for the seven days to 2026-09-21
> **Confidence**: HIGH for the display split, the constants, the slot model and the button-id resolution; MEDIUM and LOW items are fenced off in [Unverified](#unverified--needs-a-live-trace)
> **Companion docs**: [dialog-portrait-lookup.md](dialog-portrait-lookup.md) (the portrait and name half of the same window), [docs/content/dialog-ui-client-contract.md](../../content/dialog-ui-client-contract.md) (the author-facing rules this finding produces), [docs/protocol/client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md), [docs/engine/cooked-data-pak-format.md](../../engine/cooked-data-pak-format.md)

---

## Summary

When the server wants a dialog on screen it sends one message carrying five integers, and none of them is text. Everything the player reads — the window type, the screen bodies, the speakers, the buttons and their labels — comes out of the client's own `CookedDataDialogs.pak`. This finding traces what the native `DialogController` does with those five integers, and what comes back when the player clicks.

Three things you probably assumed are wrong:

- **`IsImmediate` alone** decides whether the window opens or the dialog waits in a lure queue. The screen type never influences that choice.
- **Clicking a lure tells the server nothing.** `activateAvailableDialog` re-enters the same local display core; there is no send anywhere in the chain.
- **The wire carries the cooked `ButtonID`,** not the index of the button the player clicked. Button order inside a screen is therefore load-bearing.

## Wire signature

The server-to-client half is `onDialogDisplay`, client method 105, declared in [`entities/defs/SGWPlayer.def:1150-1156`](../../../entities/defs/SGWPlayer.def). The handler reads all five fields by name through Mercury accessors, so the struct offsets in the client's own `AvailableDialog` object do not match wire order — read the order off the `.def`, not off the decompiler.

| Offset | Size | Type | Field | What the native handler does with it |
|---|---|---|---|---|
| 0 | 4 | `INT32` | `EntityId` | Pinned into GameEntityManager slot `0x11` (`DialogSpeaker`) when `IsImmediate != 0`; drives the portrait |
| 4 | 4 | `INT32` | `DialogID` | Key into `AllAvailableDialogs`, and the key the cooked entry is looked up by |
| 8 | 4 | `INT32` | `MissionFlags` | Stored, never read on the display path |
| 12 | 1 | `UINT8` | `IsImmediate` | Display-versus-queue switch, read with the byte accessor `Mercury__unknown_00e57390` |
| 13 | 4 | `INT32` | `aMissionId` | Stored, never read on the display path |

Total payload 17 bytes. Cimmeria builds exactly this in `crates/services/src/cell/interactions/dialog.rs:51-56`, and hardcodes `IsImmediate = 1` at line 55.

The client-to-server half is `dialogButtonChoice`, cell method 75, declared `<Exposed/>` at [`entities/defs/SGWPlayer.def:621-625`](../../../entities/defs/SGWPlayer.def): `INT32 DialogId, INT32 ButtonId`. It is the only thing a dialog ever sends back, and it fires on exactly two occasions — see [The two return paths](#the-two-return-paths).

## The IsImmediate split

The true `Event_NetIn_DialogDisplay` handler is `FUN_00d25900`, registered through the `MemberCallback` constructor `FUN_00d26e60` from `DialogController`'s constructor `FUN_00d26850`.

Whatever `IsImmediate` says, the handler does two things unconditionally. It builds a 0x20-byte `AvailableDialog` and inserts it into the container at `this+0x30` (`AllAvailableDialogs`, keyed by `DialogID` via `FUN_00d288c0`), and it asks the `CacheLibrary` singleton (`Detail__unknown_004786f0`, `DAT_01ea56d8`) to load the dialog's cooked `KismetEventSetID` data. That load is asynchronous and later raises `Event_Cache_ElementReady<long,CookedKismetEventSetData>`.

Only when `IsImmediate != 0` does it also stash the object in the single handoff slot at `this+0x44` and pin GameEntityManager slot `0x11` to the dialog's `EntityId` straight away (`FUN_00c67bd0`).

The decision itself happens later, when the cache load completes. `FUN_00d25310` — registered for `Event_Cache_ElementReady`, **not** for `Event_NetIn_DialogDisplay` — walks the pending list and calls `FUN_00d25200`:

```c
// FUN_00d25200
if (*(this+0x44) == param_1) {            // this object was the IsImmediate handoff
    FUN_00d24f10(this, param_1);           // DISPLAY path
} else if (*(char*)(param_1 + 7) == 0) {   // IsImmediate byte == 0
    FUN_00d27a80(...);                     // emits Event_UI_DialogAvailable(dialogId)
} else {
    FUN_00e68340((void*)((int)this+0x30), param_1+4);   // not the handoff object, but IsImmediate != 0
}
```

RTTI pins both emitters. `FUN_00d26aa0`, called from `FUN_00d27a80`, constructs `TypedEmitInfo<Event_UI_DialogAvailable>`. `FUN_00d26b70`, called from `FUN_00d27b80` at the end of the display core `FUN_00d24f10`, constructs `TypedEmitInfo<Event_UI_DialogDisplay>`.

So: `IsImmediate = 1` opens the window once the cooked entry is cached, and `IsImmediate = 0` raises an availability event. The screen type is read inside `FUN_00d24f10` (`*(char*)(param_1[2]+0x28) == 3`, the Tutorial test) only to pick which display slot to occupy and whether to pin the portrait, never to decide display versus queue.

**Where the screen type does bite is one layer up, in Lua.** `DialogMod.onDialogAvailable` looks the type up in `DialogMod.DialogQueues` and returns immediately if there is no queue for it (`DialogSetup.lua:86-93`). Queues exist only for Radio and Realization (`PushMission.lua:3-4`) and Tutorial (`Tutorial.lua:104`). Send a Blurb or a plain Dialog with `IsImmediate = 0` and the native side is perfectly happy while the dialog silently never surfaces. `DialogMod.onDialogDisplay` has the mirror-image hole: if no window is registered for the type it draws nothing at all (`DialogSetup.lua:54`).

**Confidence: HIGH.** Both emitters are RTTI-confirmed and the Lua routing is read from recovered source.

## activateAvailableDialog is local only

The Lua-argument shim at `0x00aa5870` reaches `FUN_00ad8670`, which calls `FUN_00d25160(DialogController*, dialogId)`. That looks the dialog up in `this+0x30` by id and calls `FUN_00d24f10` — the same display core the `IsImmediate = 1` path uses, which emits `Event_UI_DialogDisplay`.

There is no network send anywhere in that chain. Clicking a lure is a purely local promotion of a dialog the server already delivered. The server hears nothing until a button click or a qualifying close, which means a lure the player never opens is indistinguishable, server-side, from one they opened and read.

**Confidence: HIGH.**

## The Dialog constants table

The `Dialog` table is not defined in any `.lua` file. It is registered natively as read-only properties inside the Lua-binding function spanning `0x00acbb10-0x00ad6b65` (`RegisterProperty` at `0x00403f20`, setter `NULL`). Each getter reads a static `int32` out of a table at `0x01b16120`.

| Getter | Lua name | Slot | Value |
|---|---|---|---|
| `0x00aa5eb0` | `Dialog.BlurbType` | `0x01b16120` | 1 |
| `0x00aa5ee0` | `Dialog.DialogType` | `0x01b16124` | 2 |
| `0x00aa5f10` | `Dialog.TutorialType` | `0x01b16128` | 3 |
| `0x00aa5f40` | `Dialog.RadioType` | `0x01b1612c` | 4 |
| `0x00aa5f70` | `Dialog.RealizationType` | `0x01b16130` | 5 |
| `0x00aa5fa0` | `Dialog.ButtonMoreInfoType` | `0x01b16134` | 1 |
| `0x00aa5fd0` | `Dialog.ButtonAcceptType` | `0x01b16138` | 2 |
| `0x00aa6000` | `Dialog.ButtonDeclineType` | `0x01b1613c` | 3 |
| `0x00aa6030` | `Dialog.ButtonGeneric1Type` | `0x01b16140` | 4 |
| `0x00aa6060` | `Dialog.ButtonGeneric2Type` | `0x01b16144` | 5 |
| `0x00aa6090` | `Dialog.ButtonGeneric3Type` | `0x01b16148` | 6 |

Raw bytes at `0x01b16120`, 48 of them:

```text
01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
05 00 00 00 01 00 00 00 02 00 00 00 03 00 00 00
04 00 00 00 05 00 00 00 06 00 00 00
```

`getAvailableDialogType` (the `FUN_00adcba0` family, Lua name string at `0x01953d28`) returns the raw cooked `UIScreenType` with no remapping. There is no constant for type 0; it is reachable only because `Blurb.lua:38` registers it to the Blurb window under a `TEMP HACK` comment.

Where each type lands, read from the recovered Lua:

| Type | Constant | Window | Init function | Lure queue |
|---|---|---|---|---|
| 0 | none | `BlurbWin` | `DialogMod.initBlurb` | none |
| 1 | `BlurbType` | `BlurbWin` | `DialogMod.initBlurb` | none |
| 2 | `DialogType` | `DialogWin` | `DialogMod.initDialog` | none |
| 3 | `TutorialType` | `TutorialWin` | `DialogMod.initTutorial` | tutorial button |
| 4 | `RadioType` | `DialogWin` | `DialogMod.initDialog` | radio icon |
| 5 | `RealizationType` | `DialogWin` | `DialogMod.initDialog` | idea icon |

Registrations: `Blurb.lua:35,38`, `Dialog.lua:71-73`, `TutorialScreen.lua:81`, `PushMission.lua:3-4`, `Tutorial.lua:104`. Types 2, 4 and 5 share one window and one init function, so promoting a dialog from 2 to 4 or 5 changes nothing a player sees on an immediately displayed dialog — it only makes the dialog eligible for a lure.

**Confidence: HIGH.**

## Two active slots, and eviction through discard

`DialogController`'s constructor `FUN_00d26850` builds a two-element array at `this+0x3c` and `this+0x40`. The display core `FUN_00d24f10` picks between them on the screen-type byte, `(*(char*)(param_1[2] + 0x28) == 3)`: `this+0x3c` is the single non-tutorial slot, `this+0x40` the single tutorial slot.

If the chosen slot already holds a **different** dialog id, `FUN_00d24f10` calls the discard function `FUN_00d249c0` on the old id before installing the new one. The same id is an idempotent re-display and evicts nothing.

Discard closes the old window and — only if the old dialog has zero cooked buttons across all its screens — sends `dialogButtonChoice` with `ButtonId = 0xFFFFFFFF`. A dialog that has buttons sends nothing on eviction, so it simply disappears with no trace on the wire.

The container at `this+0x30` is a different thing: it holds every delivered dialog, queued or displayed, and is unbounded. Nothing evicts from it except an explicit discard.

**This is a live server-side defect, not a theoretical one.** Display a zero-button dialog A, then display B. The client evicts A and sends `(A, -1)` *after* the server has already re-pinned `open_dialog_id` to B, so `crates/services/src/cell/cell_methods/player/interaction/dialog.rs:36-45` rejects the choice and A's content chain never fires. The original Python server kept a dictionary of displayed dialogs rather than a single pin. Packet DU-08 in the [dialog UI redesign ledger](../../analysis/dialog-ui-redesign/work-packets.md) tracks the fix.

**Confidence: HIGH.**

## The two return paths

A dialog reaches the server through `dialogButtonChoice` in exactly two ways, and they resolve `ButtonId` through completely separate code.

### Button click resolves the cooked ButtonID

1. Lua calls `selectActiveDialogChoice(dialogId, this:getID())`, where `getID()` is the 1-based button index that `DialogMod.enableDialogButtons` assigned (`DialogSetup.lua:78`, called from `Dialog.lua:49` and `Blurb.lua:24`).
2. The Lua-argument shim `0x00aa5d70` reaches `FUN_00ad8690`, which calls `FUN_00d24e70(DialogController*, dialogId, windowId - 1)` — 1-based becomes 0-based.
3. `FUN_00d24e70` finds the active dialog in the slot pair and calls `FUN_00d24860(screenDescriptor, index0)`.
4. `FUN_00d24860` is the send site:

```c
piVar2 = FUN_00adff40(descriptor+0x30, *(byte*)(descriptor+0x29));   // current screen record
iVar1  = *piVar2;
if (0 <= (int)param_1 && (int)param_1 < screenButtonCount(iVar1)) {
    // build Event_NetOut_DialogButtonChoice, field "DialogId" = descriptor+0xc
    puVar5  = FUN_00adff40(iVar1 + 0x44, param_1);   // index into the screen's cooked Buttons array
    param_1 = *(uint*)*puVar5;                       // first field of the button record = cooked ButtonID
    // field "ButtonId" = param_1
    FUN_00caed50(this_00, 0, event, 1);              // network send
}
```

The click index is used purely as an **array position** into the screen's cooked `Buttons` array; the value that goes on the wire is the `ButtonID` attribute stored at that position. Reordering buttons within a screen therefore changes what the server receives even though nothing visible moved.

Corroboration from colo SigNoz `dialogButtonChoice` logs, seven days to 2026-09-21, as `(dialogId, buttonId)` pairs:

| Pair | Count | Pair | Count | Pair | Count |
|---|---:|---|---:|---|---:|
| (2299, 8) | 19 | (2305, 8) | 11 | (2573, 8) | 3 |
| (2298, 8) | 18 | (2518, 8) | 5 | (2298, 9) | 2 |
| (4001, 8) | 15 | (5859, 8) | 5 | (5861, 8) | 1 |
| (4000, 8) | 14 | (2309, 8) | 4 | (5023, 70) | 1 |
| (3999, 70) | 14 | (2576, 71) | 4 | (5022, 8) | 1 |

Every observed value is a cooked attribute — 8 Accept, 9 More Info, 70 Receive Item, 71 Take Missions — and never a small click index.

### Close sends the sentinel, but only when there are no buttons

`FUN_00d249c0`, reached from `discardAvailableDialog` (`0x00ad86c0`), walks the dialog's screens accumulating a button count and sends `ButtonId = 0xFFFFFFFF` **iff** that total is zero. This is a separate hardcoded path that never touches the cooked array. Cimmeria reads the field as a plain `i32`, so it arrives as `-1`; local `logs/server.log` shows `-1` for closes of 2982, 3995 and 3996.

Every close control routes here, and none of them is a choice: Done (`Dialog.lua:64-67,83`), Decline (`Dialog.lua:84`, `Blurb.lua:46`), the window's X (`Dialog.lua:85`, `Blurb.lua:45`, `TutorialScreen.lua:88`) and the tutorial's Done (`TutorialScreen.lua:89`). Clicking Decline on a dialog that *has* buttons sends nothing at all — it is the close path, not a third choice value.

**Confidence: HIGH**, and the two paths agree with independent telemetry.

## What the window actually draws

`DialogMod.enableDialogButtons` (`DialogSetup.lua:67-83`) hides every button in the window's map, then walks the current screen's cooked buttons and shows only those whose `ButtonType` has an entry in that map. A cooked button whose type is absent from the map renders nothing and is unclickable — it still counts toward the button total that suppresses the close sentinel.

| Window | Map | Types it can draw | Source |
|---|---|---|---|
| `DialogWin` | `DialogMod.dialogButtonMap` | 2 Accept, 4/5/6 Generic1-3 | `Dialog.lua:4-8` |
| `BlurbWin` | `DialogMod.blurbButtonMap` | 1 More Info, 2 Accept | `Blurb.lua:5-6` |
| `TutorialWin` | none | none — `initTutorial` never calls `enableDialogButtons` | `TutorialScreen.lua:20-30` |

Type 3 Decline is commented out of both maps (`Dialog.lua:5`, `Blurb.lua:7`) and exists only as chrome, shown automatically whenever Accept is visible (`Dialog.lua:18`, `Blurb.lua:17`).

Two widget details matter to anyone authoring button text. Generic1-3 are `TextButton_2` (`Dialog.layout:111-123`) and render the cooked `Text` as their label. Accept, Decline and More Info are `ImageButton_2` with fixed `NormalImage` art (`Dialog.layout:128,135`, `Blurb.layout:71,79,86`), so although `setText` is still called on them (`DialogSetup.lua:79`) the player reads the art, not the authored string.

All three windows carry a working X: `TitlebarEnabled` false with `CloseButtonEnabled` true in `Dialog.layout:8-9`, `Blurb.layout:7-8` and `Tutorial.layout:9-10`. The X is not a Blurb-only affordance.

**Confidence: HIGH** (recovered Lua and layout source), except the claim that image buttons ignore the cooked label, which is **MEDIUM** — inferred from the widget type, not observed in a running client.

## Address inventory

| Address | Role |
|---|---|
| `0x00d25900` | True `Event_NetIn_DialogDisplay` handler |
| `0x00d26e60` | `MemberCallback` ctor registering `FUN_00d25900` |
| `0x00d26850` | `DialogController` constructor; builds the two-slot array at `+0x3c` / `+0x40` |
| `0x00d25310` | `Event_Cache_ElementReady<long,CookedKismetEventSetData>` handler |
| `0x00d25200` | Available-versus-Display dispatcher |
| `0x00d24f10` | Display core: slot pick, eviction via discard, portrait pin, emits `Event_UI_DialogDisplay` |
| `0x00d27a80` / `0x00d26aa0` | Emits `Event_UI_DialogAvailable` |
| `0x00d27b80` / `0x00d26b70` | Emits `Event_UI_DialogDisplay` |
| `0x00d249c0` | Discard: button-less `ButtonId = -1` send; the `0x1c20 + type` and `0x1b59` calls |
| `0x00ad86c0` | `discardAvailableDialog` implementation |
| `0x00d22c90` | Generic entity-listener pin/tag primitive (shared with the portrait pins) |
| `0x00d288c0` | `AllAvailableDialogs` insert, keyed by `DialogID` |
| `0x00c67bd0` | GameEntityManager slot `0x11` (`DialogSpeaker`) pin |
| `0x00aa5870` → `0x00ad8670` → `0x00d25160` | `activateAvailableDialog` chain, local only |
| `0x00aa5d70` → `0x00ad8690` → `0x00d24e70` → `0x00d24860` | `selectActiveDialogChoice` chain, resolves the cooked `ButtonID` |
| `0x00aa5eb0` .. `0x00aa6090` | `Dialog.*Type` and `Dialog.Button*Type` getters |
| `0x01b16120` .. `0x01b16148` | Static ints behind the `Dialog` constants |
| `0x00aa5970` → `0x00adcba0` | `getActiveDialogMissionFlags` getter |
| `0x00e3cba0` / `0x00e57390` | Mercury int32 / byte field accessors used by `FUN_00d25900` |

## Unverified — needs a live trace

Each of these was reached for and not closed. Treat them as open, not as absent.

| Question | What is known | Confidence | What would close it |
|---|---|---|---|
| Does the cooked parser accept `UIScreenType` 4 and 5? | Attribute names (`KismetEventSetID`, `DialogFlags`, `ScreenID`, `SpeakerID`, `Buttons`, `ButtonID`, `ButtonType`) sit as individual C strings at `0x01b23bdc-0x01b23cfc`, each appearing twice, consistent with a name-keyed property-tree reader. No validation was found, but the parser `FUN_015e4d10` was not decompiled in this pass. | LOW-MEDIUM | Packet DU-00: flip two Cimmeria-authored overrides to types 4 and 5 in a client session |
| What is the `0x1c20 + type` call in the discard path? | `FUN_00d249c0` calls `FUN_00d22c90(desc, *(byte*)(desc+0x29) + 0x1c20)` then `FUN_00d22c90(desc, 0x1b59)`. The display core calls the same primitive with fixed keys `0x1b58` and `0x1bbc` (the portrait pins). `FUN_00d22c90` reaches a generic entity-listener pin/tag primitive through `FUN_00dd0de0`, `FUN_00d06ca0`, `FUN_00d01c50`/`bd0`/`ac0`. **No audio call was found — treat "per-type close sound" as unsupported.** | MEDIUM | x64dbg trace of the discard path with a non-freezing breakpoint |
| What raises `Event_UI_SplashMessageReceived`? | The class exists — RTTI `0x01e0d5e8`, `TypedEmitInfo` `0x01e1d948`, `SGWScriptedWindow` handler `0x01e1c668` / `0x01e18178`. Which native wire handler raises it was not traced, and the `CHAN_splash` (11) linkage is unverified. | LOW | Trace back from the RTTI descriptor's constructors |
| Byte layout of the cooked button record | `FUN_00d24860` proves the **first** field is the `ButtonID`. Nothing beyond offset 0 was identified. | HIGH for offset 0, unknown after | Decompile `FUN_015e4d10`, or dump a record in the debugger |
| Who consumes `MissionFlags` and `aMissionId`? | Neither is read in `FUN_00d25900`, `FUN_00d25200`, `FUN_00d24f10` or `FUN_00d249c0`. `MissionFlags` is exposed to Lua as `getActiveDialogMissionFlags(dialogId)` via `FUN_00aa5970` → `FUN_00adcba0`; no caller exists in the Dialog module. | HIGH that the display path ignores them | Grep the wider Lua tree for the getter |
| Does the Blurb Decline button work at all? | `Blurb.lua:46` reads `BlurbWin:subscribe(Blurb_DeclineButton.EventClicked, …)` — it subscribes the *window*, where the sibling at `Dialog.lua:84` subscribes the *button*. This looks like an original defect that leaves Blurb's Decline inert. | LOW | Click Decline on a Blurb that shows Accept and watch for the close |

## Correction to dialog-portrait-lookup.md

That document's Track 1 call-chain diagram labels `FUN_00d25310` as the `Event_NetIn_DialogDisplay` handler. RTTI shows `0x00d25310` is registered for `Event_Cache_ElementReady<long,CookedKismetEventSetData>`; the actual wire handler is `FUN_00d25900`, and the cache-ready handler only runs after the asynchronous cooked-data load resolves. The correction is recorded in [dialog-portrait-lookup.md](dialog-portrait-lookup.md); the rest of its portrait analysis stands.

A second, larger discrepancy in that document is **not** resolved here. Its Track 2 infers the speaker name from a CookedData `speakers` lookup, noting in its open questions that the dialog Lua was not recovered. It has since been recovered, and `Dialog.lua:42` and `Blurb.lua:19` both set the name label from `unitName(Unit.Dialog)` — the GameEntityManager `DialogSpeaker` slot — with no CookedData lookup in sight. If that reading holds, the blank portrait and the wrong speaker name are the same bug, both downstream of a slot-`0x11` pin that never landed. Confirming it needs a reader who can check what `unitName` does with an unpinned slot; until then Track 2 should be treated as disputed rather than wrong.

## Implementation impact

- **Never send `IsImmediate = 0` for a Blurb or a plain Dialog.** There is no queue for those types and the dialog vanishes with no error anywhere.
- **A dialog that keys a content chain must be reachable.** Either give it zero buttons, so the close path emits `-1`, or put a button on its final screen. A button that stops short of the final screen soft-locks a player who reads to the end and presses Done.
- **Button order inside a screen is wire-visible.** Reordering a screen's `<Buttons>` children changes which cooked `ButtonID` a given click sends.
- **The single `open_dialog_id` pin is too narrow** for a client that holds two active dialogs plus lures, and it currently loses the chain of any zero-button dialog that gets evicted.
- **Changing a dialog's `ui_screen_type` between 2, 4 and 5 is documentary** for an immediately displayed dialog. It buys you lure eligibility, nothing visual.

The author-facing version of these rules, with the dialog ids they apply to, is [docs/content/dialog-ui-client-contract.md](../../content/dialog-ui-client-contract.md).
