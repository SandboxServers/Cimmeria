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
source 1 nor source 2 and can only display a **monologue** dialog.

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

Zero-button dialogs: chains 1014/1018/1020 in `castle_cellblock_chains.sql`
trigger `dialog_choice` on 5020/5021, which have **zero** buttons on every
screen. Shipped, but that proves authorship, not that the client emits
`dialogButtonChoice` for a button-less dialog. Treat as **unverified**.

## 4. `dialog_set_maps` with `dialog_id IS NULL` are dropped at load

`spawner/dialogs.rs:45`. `add_dialog_set <that id>` = `warn!` + total no-op
(`executor/dialog.rs:172-177`): no `available_interactions` entry, no
InteractionType push. Since most mission NPCs have `interaction_type = 0` at
rest, **the NPC is then never clickable and no `interact_tag` chain can ever
fire.** A NULL bind is not cosmetic; it kills the mission.

In the original data a NULL `dialog_id` means "bind the whole set, let the
set's own filters pick the dialog" — that is why `Castle.py` passes the NULL
row (3062) everywhere and never the specific sibling rows.

### The Castle flag ladder (dialog_set_id 649, all "Reinforce Copplemann")

| dsm_id | dialog | flags | const |
|---|---|---|---|
| 3059 | 2572 | 0x800000 | `INT_A_STORY_MISSION_AVAILABLE` (`?` offer) |
| 3060 | 2573 | 0 | not clickable |
| 3061 | 2574 | 0x1000000 | `INT_A_STORY_MISSION_ACTIVE` (`!` in progress) |
| 3062 | NULL | 0x1000000 | **dropped at load** |
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
