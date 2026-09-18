---
name: dialog-chain-authoring-rules
description: Hard runtime constraints on display_dialog / dialog_choice / add_dialog_set chain authoring — speaker resolution, the open_dialog_id gate, the NULL-dialog_set trap, and the flag ladder. Verified 2026-09-17.
metadata:
  type: project
---

# Dialog chain authoring — the four rules that bite

Verified against `castle/m701-gerschon-copplemann` (base 3c1fed6c), 2026-09-17.

## 1. `display_dialog` can only resolve a speaker three ways — and two of them
## are unavailable to non-interact chains

`executor/dialog.rs:54-90` resolution order:

1. `params["target_entity_id"]` — stamped **only** by `fire_interact_tag` /
   `fire_interact_template` (`event_dispatch/interaction.rs:35-38`).
2. `player.last_interaction_target` — written in **exactly one place**,
   `cell/interactions/dispatch/interact.rs:90`, inside `handle_interact`.
3. Monologue fallback — only if **every** screen of the dialog has
   `speaker_id = 0` (`spawner/dialogs.rs:82-93`, NULL does NOT count).
4. Otherwise: `warn!` and **return without displaying**.

**The trap:** `handle_interact` is *skipped* whenever a tag/template chain
matches (`cell_methods/player/interaction/interact.rs:183-203` — `if !handled`).
So a chain that fires on `interact_tag` never populates
`last_interaction_target`. Any *follow-up* chain (triggered by
`dialog_choice`, `mission_accepted`, a minigame victory via
`fire_chain_by_id`, or a `delay_ms` deferral) therefore has **neither**
source 1, and source 2 is whatever `handle_interact` last pinned (absent on a fresh login, or a STALE NPC from an earlier non-chain interact): the follow-up either bails or binds the wrong portrait. Chain-handled interacts do not write the pin today (`cell_methods/player/interaction/interact.rs` skips `handle_interact`); the Castle mission-701 PR adds the pin write before the chain dispatch, after which follow-ups bind the interacted NPC.

Corollary: python `displayDialog(None, X)` ports cleanly **iff** X is an
all-speaker_id-0 dialog. If X has any NPC speaker, the port is silently dead.
Check before authoring:
`SELECT dialog_id FROM dialog_screens GROUP BY dialog_id HAVING count(*) = count(*) FILTER (WHERE speaker_id = 0)`.

`fire_chain_by_id` (`event_dispatch/mod.rs:53-87`) builds
`ResolvedActions { actions, action_delays, ..Default::default() }` — **params
is empty**, always. Minigame victory chains can never display an NPC dialog.

## 2. `dialog_choice` is server-gated on `open_dialog_id` (#479 / CAT-J-01)

`cell_methods/player/interaction/dialog.rs:36-49` rejects the choice unless
`entity.open_dialog_id == dialog_id`. The pin is set by `send_dialog_display`
(`cell/interactions/dialog.rs:48`) on **every** display path, and **cleared on
the first valid choice** (`dialog.rs:54`), before the chain fires.

Consequences:
- A `dialog_choice X` chain is unreachable unless some earlier chain (or
  `handle_interact`) actually displayed X. If rule 1 killed the display, the
  whole downstream tail is dead.
- One choice per display — no replay, no double-fire. This is a free one-shot
  guard, stronger than the dead `once` column.

## 3. `dialog_screen_buttons` is keyed by `screen_id`, the THIRD column

`(screen_button_id, button_id, screen_id, button_type, text)`. Grepping
`VALUES (<dialog_id>,` gives garbage (it matches screen_button_id). Correct
probe: get screen ids from `dialog_screens`, then
`VALUES \([0-9]+, [0-9]+, <screen_id>,`.

Button coverage is often **partial** — the last N screens frequently have no
button (2576: buttons on 96821-96823, none on 96824/96825; 5861: buttons on
96782-96786, none on 96787-96789). A player who reads to the end gets no
button and raises no choice.

**Button ids are NOT discriminable in a chain.** `content_triggers.event_key`
for `dialog_choice` is the *dialog id*; `button_id` reaches
`fire_dialog_choice` but there is no authorable condition for it (only six
conditions exist). Multi-button dialogs cannot branch — every button runs
every matching chain. Check button text before authoring: an
accept/decline pair would fire the accept chain on decline.

### Zero-button dialogs DO emit `dialogButtonChoice` — with ButtonId = -1 (RESOLVED 2026-09-18)

Previously marked "unverified". Ghidra + client Lua now prove it. Evidence chain:

1. `Content/UI/Core/Dialog/Dialog.lua:48-51` — `selectActiveDialogChoice(dialogId, this:getID())`
   is subscribed **only** to the buttons in `DialogMod.dialogButtonMap`
   (Accept / Generic1-3). A screen with no buttons has every one of them
   hidden (`DialogSetup.lua:67-82`, `getActiveDialogButtonCount == 0`), so
   the click path is unreachable.
2. Done / Decline / window-X all route to `onDialogDoneClicked` →
   `discardAvailableDialog(dialogId)` (`Dialog.lua:64-67, 83-85`).
3. `discardAvailableDialog` native = `FUN_00ad86c0` → `FUN_00d249c0`. That
   function sums the button count across **all** the dialog's screens and,
   **iff the total is 0**, constructs `Event_NetOut_DialogButtonChoice`
   with `DialogId = <dialog>` and `ButtonId = 0xFFFFFFFF` (-1) and sends it.
   When the total is non-zero it checks a per-dialog flag and returns
   without sending (so *declining* a button-bearing dialog sends nothing).

Practical rules:
- A **zero-button** dialog fires `dialog_choice` **on close**, `button_id = -1`.
  This is how `Castle.py`'s `dialog.choice::2574` / `::2575` worked in 2009.
- A **button-bearing** dialog fires `dialog_choice` only on an actual
  Accept/Generic click; closing it fires nothing.
- **Never mix**: adding a button to a dialog whose chain relies on the
  close-path choice silently kills that chain.
- Cimmeria handles `button_id = -1` fine (`interaction/dialog.rs:21-22` reads
  a plain i32; the #479 gate is on `open_dialog_id` only).
- The `button_id` on the click path is the **1-based index within the current
  screen's button list** (`DialogSetup.lua:77` `setID(i)`), not the DB
  `dialog_screen_buttons.button_id`.

Castle 702-708 audit (2026-09-18): dialogs 2584, 2586, 5003, 5004, 5008,
5009, 5010, 5011, 2574, 2575, 2577, 2580, 2581, 4866 all have **zero**
buttons on every screen → all are close-path (`button_id = -1`) choices.
Only 2573 (Accept) and 2576 (Take Missions, screens 96821-96823 only) carry
buttons.

## 4. `dialog_set_maps` with `dialog_id IS NULL` are interaction-only binds (was: dropped at load — FIXED by #661)

**Superseded 2026-09-18.** This section used to read "dropped at load": the
loader kept only rows carrying a dialog, so `add_dialog_set <a NULL row>` was a
`warn!` plus total no-op, the NPC stayed unclickable, and a NULL bind killed
the mission.

PR #661 (CA02, defect B3) widened `DialogSetMapEntry.dialog_id` to
`Option<i32>` and keeps all 626 NULL rows in the seed
(`cell/spawner/dialogs.rs`). Such a row is now an **interaction-only** bind: it
contributes its `interaction_flags` bit to the per-player indicator over the
NPC's head and nothing else. Clicking the NPC displays no dialog — pair a NULL
bind with an `interact_tag` chain if the click should say something.

Two details that follow from the fix: `handle_interact` scans with `find_map`,
so a NULL row can never shadow a sibling row that does carry a dialog (either
bind order); and `initialResponse` bails on a NULL row rather than substituting
dialog 0, which would open an empty window.

In the original data a NULL `dialog_id` means "bind the whole set, let the
set's own filters pick the dialog" — that is why `Castle.py` passes the NULL
row (3062) everywhere and never the specific sibling rows. We do not implement
set filters, so for us the NULL row is purely a flag carrier and the dialog
comes from a chain.

### The Castle flag ladder (dialog_set_id 649, all "Reinforce Copplemann")

| dsm_id | dialog | flags | const |
|---|---|---|---|
| 3059 | 2572 | 0x800000 | `INT_A_STORY_MISSION_AVAILABLE` (`?` offer) |
| 3060 | 2573 | 0 | not clickable |
| 3061 | 2574 | 0x1000000 | `INT_A_STORY_MISSION_ACTIVE` (`!` in progress) |
| 3062 | NULL | 0x1000000 | `INT_A_STORY_MISSION_ACTIVE`, **interaction-only** (flag carrier) |
| 3063 | 2576 | 0x2000000 | `INT_A_STORY_MISSION_TURN_IN` (`?` turn-in) |
| 4961 | 2575 | 0 | not clickable |

Constants: `crates/entity/src/interaction_flags.rs:66-85`. Any non-zero bit
makes the entity clickable; the bit chosen picks the indicator glyph.

Binding a sibling row purely for its flag is safe when the chain fires on
`interact_tag`, because the tag match short-circuits `handle_interact` and the
bound dialog never auto-opens — the chain's own `display_dialog` wins.

## 5. Deferred (`delay_ms`) actions ARE scrubbed on logout

`space_manager/deferred_content_actions.rs` — tests at `:197-212`
(`disconnect_entity`) and `:228-242` (`destroy_entity`) pin the scrub. A
`player_loaded` restore chain that re-arms a deferred action is therefore safe;
no stale entry can survive to double-fire.

But conditions are evaluated at **resolve** time, not at fire time
(`docs/content/content-engine.md` §4), so a deferred action carries **no gate**
and runs unconditionally when it elapses. Never defer an action that would be
wrong if the player advanced past that state during the delay.
