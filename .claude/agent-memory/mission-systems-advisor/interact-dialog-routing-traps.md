---
name: interact-dialog-routing-traps
description: interact_tag short-circuit skips bind auto-display (pin and range gate FIXED 2026-10-17 in zone-tollana); nested mission-event dispatch runs mid action list; has_dynamic_properties gates icon push.
metadata:
  type: project
---

# Interact / dialog routing traps (verified 2026-09-18; 1-2 re-verified 2026-10-17)

Companions: [[multi-chain-dispatch-semantics]],
[[dialog-choice-cannot-branch-on-button]],
[[add-dialog-set-no-dedupe-first-wins]],
[[advance-step-vs-complete-objective]].

## 1-2. FIXED (verified 2026-10-17, zone-tollana worktree): pin + range gate

`crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`
now runs `interact_target_in_range` FIRST, pins `last_interaction_target`
BEFORE `fire_interact_tag` / `fire_interact_template`, and calls
`handle_interact` only if no chain handled the click. So an `interact_tag`
chain followed by `dialog_choice -> display_dialog` now resolves its speaker,
and tag chains are distance-gated. The 2026-09-18 traps (pin never set, no
distance check) are STALE on branches that carry this file; re-check
`git log -S interact_target_in_range` before relying on it for an older
branch. Still true: `fire_dialog_choice` does not stamp `target_entity_id`,
and a `dialog_choice` display of a non-monologue dialog needs the pin.

Still-live consequence: a matching tag chain skips `handle_interact`, so a
bind's dialog never auto-opens; the click only falls to the FIRST bind that has
a dialog when NO tag chain matches. Binds shared by template across states
therefore need an exhaustive disjoint tag-chain set, or a stale bind
double-displays (see tollana-568-spec.md).

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
