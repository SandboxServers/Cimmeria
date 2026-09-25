---
title: "Dialog UI Client Contract"
type: reference
audience: content authors wiring dialogs, buttons and chains
last_updated: 2026-09-21
---

# Dialog UI Client Contract

> **Type**: reference
> **Audience**: content authors — anyone adding a dialog, a button, or a chain keyed on one
> **Last updated**: 2026-09-21
> **Companion docs**: [reverse-engineering/findings/dialog-controller-wire-flow.md](../reverse-engineering/findings/dialog-controller-wire-flow.md) (the evidence), [content-engine.md](content-engine.md) (the runtime), [../architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md) (how an edit reaches the client), [../engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), [../analysis/dialog-ui-redesign/work-packets.md](../analysis/dialog-ui-redesign/work-packets.md) (the redesign ledger)

The dialog window is a 2009 client you cannot change. Almost everything an author assumes about it is negotiable except this: the server sends a number, and the client draws whatever its own cooked catalogue says that number means. This page is the list of what the client actually does, and the authoring rules that fall out of it.

Every rule below cites either a client source file you can read yourself, or the RE finding that traced the native path.

## The server sends an id, not a dialog

`onDialogDisplay` carries five integers — the speaker's entity id, the dialog id, a mission-flags word, a one-byte immediate flag and a mission id. No text, no button list, no screen count. The client looks the rest up in `CookedDataDialogs.pak` (`crates/services/src/cell/interactions/dialog.rs:51-56`).

Two consequences you will hit on your first change:

- **Editing `db/resources/Dialogs/Seed/` changes nothing in game.** The seed is the server's parallel copy. It is what the content engine and the linters read; it is not what the player sees. (`crates/services/src/base/dialog_overrides.rs:1-12`)
- **The only route to the player is a cooked-entry override**, pushed at handshake through the per-key invalidation path. See [mission-pak-overrides.md](../architecture/mission-pak-overrides.md) for the mechanism. When you change a dialog, you change the seed *and* the override, and they must agree.

## Window types

`ui_screen_type` on the `dialogs` row becomes the cooked `UIScreenType`, and the client maps it straight to a window with no remapping. The seed enum in `db/resources/Dialogs/Types/EDialogUIScreenType.sql` and the client's native constants at `0x01b16120` agree value for value.

| Value | Seed label | Window drawn | Notes |
|---|---|---|---|
| 0 | `DUIST_None` | Blurb | Registered to the Blurb window under a `TEMP HACK` comment (`Blurb.lua:38`). It is a modal single-screen popup, **not** a bark or a subtitle. 128 dialogs ship this way. |
| 1 | `DUIST_DefaultBlurb` | Blurb | Small modal popup. No Next, no Previous. |
| 2 | `DUIST_DefaultDialog` | Dialog | The full multi-screen window. 4,279 dialogs ship this way. |
| 3 | `DUIST_DefaultTutorial` | Tutorial | A paged window with its own model. Cooked buttons are never drawn on it. |
| 4 | `DUIST_DefaultRadio` | Dialog | Same window, same init function as type 2. Adds radio-lure eligibility. |
| 5 | `DUIST_DefaultRealization` | Dialog | Same window, same init function as type 2. Adds idea-lure eligibility. |

Types 2, 4 and 5 all register `DialogWin` with `DialogMod.initDialog` (`Dialog.lua:71-73`). **Relabelling a displayed dialog from 2 to 4 or 5 has no visual effect at all.** It buys lure eligibility and nothing else. No shipped dialog uses type 4 or 5 today.

A type the client has no registration for draws nothing and reports nothing (`DialogSetup.lua:54`). Stay inside 0-5.

## Buttons

Each screen can carry zero or more buttons, each with a `button_type` (what the client draws) and a `button_id` (what the server receives). The window hides every button it knows about, then shows only those whose type appears in its own map (`DialogSetup.lua:67-83`).

| `button_type` | Name | Drawn on Dialog? | Drawn on Blurb? | Drawn on Tutorial? |
|---|---|---|---|---|
| 1 | More Info | no | **yes** | no |
| 2 | Accept | **yes** | **yes** | no |
| 3 | Decline | no — chrome only | no — chrome only | no |
| 4 | Generic 1 | **yes** | no | no |
| 5 | Generic 2 | **yes** | no | no |
| 6 | Generic 3 | **yes** | no | no |

Sources: `Dialog.lua:4-8`, `Blurb.lua:5-6`, `TutorialScreen.lua:20-30`.

Rules that follow:

- **A button the window cannot draw is worse than no button.** It renders nothing and is unclickable, but it still counts toward the screen's button total — which suppresses the close sentinel described below. A Generic button on a Blurb is the classic way to silently kill a chain.
- **Only Generic 1-3 display your authored `text`.** They are text buttons (`Dialog.layout:111-123`). Accept, Decline and More Info are fixed-art image buttons (`Dialog.layout:128,135`, `Blurb.layout:71,79,86`), so the label you write on them is not what the player reads.
- **Button order inside a screen is wire-visible.** The client resolves a click to an array position, then sends the `button_id` stored at that position. Reorder a screen's buttons and you change what the server receives with nothing visible having moved. (`dialog-controller-wire-flow.md` § The two return paths)
- **Tutorial dialogs ignore buttons entirely.** `initTutorial` never builds a button map; it pages through text and shows its own Done. Do not author buttons on a type-3 dialog.

## Chrome you do not author

Next, Previous, Done, Decline and the window's X are all drawn by the client. You cannot add, remove or relabel them, and none of them is a choice.

| Control | When it appears | What it sends |
|---|---|---|
| Next / Previous | Next-and-Previous shows while more screens remain or Accept is visible; otherwise Previous-and-Done shows instead (`Dialog.lua:22-33`) | nothing — screen navigation is local |
| Done | On the last screen, when no Accept is visible (`Dialog.lua:22-33`, `Dialog.layout:90`) | the close path |
| Decline | Automatically, whenever Accept is visible — you never author it (`Dialog.lua:18`, `Blurb.lua:17`) | the close path, **not** a choice |
| X (title bar) | Always, on all three windows (`Dialog.layout:8-9`, `Blurb.layout:7-8`, `Tutorial.layout:9-10`) | the close path |

Decline deserves a second look: it is wired to the same handler as Done (`Dialog.lua:84`), so on a dialog that has buttons, clicking Decline sends **nothing**. If you need to distinguish "declined" from "ignored", you cannot do it with Decline.

## Close semantics, and the two hard rules

Closing a dialog runs the client's discard path, which walks every screen counting buttons and then sends `dialogButtonChoice(dialogId, -1)` **if and only if that total is zero**. A dialog with any button on any screen sends nothing when it is closed.

This is the mechanism most progression chains rely on. It is also the thing that breaks them.

> [!IMPORTANT]
> **Hard rule 1 — a dialog that keys a `dialog_choice` chain must have either zero buttons, or a button on its final screen.**
>
> A button that stops before the final screen soft-locks a player who reads to the end: the button total is non-zero so Done sends nothing, and there is no button on screen to click. Dialogs 3999, 5861 and 2576 ship in that broken shape today; packets DU-02a and DU-02b fix them.

The second rule is the same mechanism read from the other direction. A dialog that is *already* button-less is load-bearing, and a well-meaning button breaks it.

> [!IMPORTANT]
> **Hard rule 2 — never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004, 5008 or 5009.**
>
> Every one of them is button-less today and keys a chain through the `-1` close. Adding any button — even one the window cannot draw — stops the close emitting and silently soft-locks the step behind it.

Two ids sit outside the rules: **5019 must never be displayed as a dialog of any type**, and **4003 stays BlockedEvidence** until an energy-field actor is recovered.

Chains currently match on dialog id only; there is no authorable `button_id` condition yet, so Accept and More Info cannot be told apart (`crates/content-engine/src/triggers/matching.rs:141-142`). Packet DU-06 proposes one — do not design a flow that needs it before it lands.

## Immediate display versus lures

The one-byte `IsImmediate` field decides whether the window opens or the dialog waits behind an icon. The screen type has no say in that decision; it only decides whether a waiting dialog has anywhere to wait.

| Type | Sent immediate | Sent non-immediate |
|---|---|---|
| 0, 1 Blurb | opens the Blurb window | **vanishes** — no queue exists |
| 2 Dialog | opens the Dialog window | **vanishes** — no queue exists |
| 3 Tutorial | opens the Tutorial window | queues behind the tutorial button |
| 4 Radio | opens the Dialog window | queues behind the radio icon |
| 5 Realization | opens the Dialog window | queues behind the idea icon |

Queues are registered only for Radio and Realization (`PushMission.lua:3-4`) and Tutorial (`Tutorial.lua:104`); everything else falls out of `DialogMod.onDialogAvailable` at its first guard (`DialogSetup.lua:86-93`). There is no error, no log and no retry — the dialog is simply never seen.

**Opening a lure tells the server nothing.** `activateAvailableDialog` re-enters the local display path with no network send anywhere in it, so a lure the player ignores forever is indistinguishable server-side from one they read. Never gate progression on a player opening a lure.

Cimmeria sends `IsImmediate = 1` for every dialog today, as the original Python did. Packet DU-04 adds an opt-out.

## One dialog at a time

The client holds exactly **one** non-tutorial dialog and **one** tutorial dialog. A second non-tutorial display evicts the first through the discard path.

That means displaying B while zero-button A is open produces `(A, -1)` *after* the server has re-pinned to B — and today the server rejects it, so A's chain never fires. If your chain displays two dialogs, assume the first one's chain is lost until packet DU-08 lands. Re-displaying the *same* id is harmless; it is an idempotent refresh that evicts nothing.

Tutorials are independent: a tutorial and a normal dialog can be open together without either evicting the other.

## Barks are not dialogs

There is no bark, subtitle or floating-text path anywhere in the dialog module. Type 0 is a modal popup, not a bark — if you use it for a one-liner you stop the player, freeze them in front of a box and make them close it.

Non-modal text goes through `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`, the same route chat and GM feedback already use (`entities/defs/interfaces/Communicator.def:48-53`, `crates/services/src/cell/chat.rs:49,167`). Packet DU-03 wraps it in an authorable action; until that lands, there is no supported way to author one.

## What is not built yet

Do not write a chain against any of this. Each is tracked in the [dialog UI redesign ledger](../analysis/dialog-ui-redesign/work-packets.md), which is the only place their shape is decided.

| Packet | What it will add | Status |
|---|---|---|
| DU-01 | Patch-mode dialog overrides that can emit buttons | in progress |
| DU-03 | `npc_bark` — non-modal NPC lines through the chat channel | in progress |
| DU-04 | A non-immediate option on `display_dialog`, for lures | planned |
| DU-06 | An authorable `button_id` condition | blocked on a decision |
| DU-08 | An offered-dialog set replacing the single open-dialog pin | in progress |
| DU-L | A seed linter enforcing both hard rules | in progress |

## Evidence index

| Claim | Where to check it |
|---|---|
| Wire fields, the display-versus-queue split, slot eviction, button-id resolution | [dialog-controller-wire-flow.md](../reverse-engineering/findings/dialog-controller-wire-flow.md) |
| Portrait and speaker-name lookup | [dialog-portrait-lookup.md](../reverse-engineering/findings/dialog-portrait-lookup.md) (Track 2 is disputed — read its correction note) |
| Window, button and chrome behaviour | `…\SGWGame\Content\UI\Core\Dialog\` in your own client install — `DialogSetup.lua`, `Dialog.lua`, `Blurb.lua`, `TutorialScreen.lua`, `Tutorial.lua`, `PushMission.lua`, and the matching `.layout` files |
| How an override reaches the client | [mission-pak-overrides.md](../architecture/mission-pak-overrides.md), [cooked-data-pak-format.md](../engine/cooked-data-pak-format.md) |
| Which dialogs are in which shape today | [../analysis/dialog-ui-redesign/work-packets.md](../analysis/dialog-ui-redesign/work-packets.md), Target Matrix |
