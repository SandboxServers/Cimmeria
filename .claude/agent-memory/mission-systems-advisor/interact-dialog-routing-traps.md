---
name: interact-dialog-routing-traps
description: The interact_tag short-circuit silently destroys the last_interaction_target pin (breaks any dialog_choice → display_dialog follow-up), skips the distance check, and nests mission-event dispatch mid action list.
metadata:
  type: project
---

# Interact / dialog routing traps (verified 2026-09-18, branch harset/jaffa)

Companions: [[multi-chain-dispatch-semantics]],
[[dialog-choice-cannot-branch-on-button]],
[[add-dialog-set-no-dedupe-first-wins]],
[[advance-step-vs-complete-objective]].

## 1. `interact_tag` short-circuit kills the `last_interaction_target` pin

`last_interaction_target` has exactly ONE write site repo-wide:
`crates/services/src/cell/interactions/dispatch/interact.rs:90`, inside
`interactions::handle_interact`.

`crates/services/src/cell/cell_methods/player/interaction/interact.rs:161-198`
fires `fire_interact_tag` / `fire_interact_template` FIRST and only calls
`interactions::handle_interact` when `handled == false`. So:

> **If an `interact_tag` chain handles the click, the pin is never set.**

Consequence: a follow-up `dialog_choice` chain that does `display_dialog`
has neither `params.target_entity_id` (only `fire_interact_*` stamps it,
`event_dispatch/interaction.rs:35-38`; `fire_dialog_choice` does NOT,
`event_dispatch/dialog.rs:86-93`) nor the pin, so
`executor/dialog.rs:54-90` **bails with a warn** unless the dialog is an
all-`speaker_id=0` monologue. Worse: a stale pin from an earlier
right-click that *did* fall through binds the follow-up dialog's
portrait to the wrong actor.

**Working pattern (Castle, shipped):** do NOT author an `interact_tag`
chain for the dialog the dsm bind already opens. `add_dialog_set` +
native `handle_interact` → `available_interactions[tmpl].first()` →
`send_dialog_display` → returns `Some(dialog_id)` → `fire_dialog_open`.
That path pins `last_interaction_target` on the way through, so the
`dialog_choice` follow-up's `display_dialog` resolves. Castle 1012
(bind 5866) → 1014/1015 (`dialog_choice`) → 1020/1021
(`display_dialog`) is the canonical example.

No shipped chain does `interact_tag → display` **then**
`dialog_choice → display`; SGC_W1 3006/3008 stop at the first display.
Latent, not proven-broken in prod — the first Harset packet to try it
would be the first to hit it.

Engine fix if the `interact_tag` routing model must be kept: pin
`last_interaction_target` (with the distance check) before the
tag/template dispatch.

## 2. `interact_tag` chains have NO distance check

`MAX_INTERACT_DISTANCE` is enforced only inside
`interactions/dispatch/interact.rs:77-80`, which the tag path skips. A
client can send `interact(target)` from anywhere in the space and drive
any `interact_tag` chain (mission completes, item grants). Applies to
every shipped seed file, not just Harset — route to
`server-authority-enforcer`.

## 3. Nested `mission_accepted` / `mission_completed` dispatch runs MID-list

`executor/mission.rs::complete` awaits `fire_mission_completed` inside
the `complete_mission` action, so a `mission_completed` chain's actions
execute **between** the parent chain's action N and N+1. Put
`remove_dialog_set` BEFORE `complete_mission` / `accept_mission` so the
nested chain sees a clean `available_interactions` slot.

`fire_mission_completed` / `fire_mission_accepted` populate mission +
**archetype** (`event_dispatch/mission.rs:112-118`), so those chains can
carry `archetype` conditions — unlike `dialog_choice`, which has no
archetype. That makes
`mission_completed '<prev id>'` the right primitive for "paint the next
mission's icon without waiting for a zone re-entry", and the right
cross-packet handoff seam for a hub NPC's mission ladder.

## 4. `has_dynamic_properties` gates the deferred icon push

`add_dialog_set` only pushes an InteractionType update for NPCs already
in the player's witness set (`executor/dialog.rs:345-351`). The AoI-enter
path covers the not-yet-visible case, but only
`if other.has_dynamic_properties` (`space_manager/aoi.rs:157`), sourced
from `entity_templates.has_dynamic_properties`. Check that column is
`true` on any template a `player_loaded` bind targets, or the icon never
paints for a player who arrives before the NPC streams in.
