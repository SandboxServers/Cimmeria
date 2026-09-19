---
name: npc-range-gate-and-weapon-range-columns
description: Where NPC attack-range constants come from (resources.items melee/ranged range columns), the two places range is gated, and the layered-selector pattern that keeps existing tests unmodified
metadata:
  type: project
---

`resources.items` carries **four** range columns — `min_ranged_range`,
`max_ranged_range`, `min_melee_range`, `max_melee_range` — and they are the
data source for the server-wide NPC range constants in
`crates/services/src/cell/combat/threat/aggro.rs`.

**Why:** `resources.abilities.max_range` is the `0` "use the server default"
sentinel on every auto-attack in the seed, and `abilities.is_ranged` is read
only by `calculate_qr` to pick the accuracy/defence branch — it has never
gated distance. So both defaults have to come from the item table, not the
ability table.

**How to apply:** before inventing a range number for NPC combat, query
`resources.items`. Observed as of 2026-09-19 (packet H09):

| Query | Result |
|---|---|
| `max_ranged_range` over the 1,299 items binding an `EVENT_ITEM_MELEE` (`event_id = 6`) ability | `30` ×1,298, `35` ×1 |
| `max_melee_range` over the same items | `0` ×5, `2` ×1,116, `3` ×178 |
| staff family (204 items binding ability 710) | melee `3` ×177, `2` ×27 |
| ribbon family (151 items binding 711) | melee `2` ×150, `0` ×1 |

`NPC_ATTACK_RANGE = 30.0` and `NPC_MELEE_RANGE = 3.0` both fall out of that.
Client-side corroboration for the same pair of columns (`MeleeRanges` /
`RangeRanges` in the cooked item schema) is in
`docs/reverse-engineering/findings/combat-formulas-client-evidence.md` E4.

## Range is gated in TWO places, and only one knows about melee

1. `cell/service/npc_ai/ability_select.rs::effective_max_range` — melee-aware
   since H09. Used by `ability_ranges` and `choose_npc_ability_within_reach`.
2. `cell/abilities/use_ability/handle.rs` (~line 239) — `def.max_range > 0 ?
   def.max_range : 30.0`, **no `is_ranged` branch**. This is the gate the
   *player* melee path goes through, so a player can still land a melee
   auto-attack from 30 m. Fixing it is client-visible and needs its own
   packet.

Trap: 148 rows in `resources.abilities` have `is_ranged = false` **and** a
non-zero `max_range` between 100 and 2500 — not metres. Any melee-reach rule
that honours a non-zero `max_range` reintroduces melee-at-range in a worse
form. `is_ranged = false` must win over the ability's own `max_range`.

## Layered selector: adding a filter without touching existing tests

When a coordinator requires "existing tests pass unmodified" and the obvious
fix is a new parameter on an existing function, layer instead of widening the
signature:

- keep `choose_npc_ability(npc_id, space_mgr)` exactly as-is;
- add `choose_npc_ability_within_reach(..., target_dist, npc_attack_range)`
  that applies the filter and **delegates to the old function when the filter
  matches nothing**;
- point production at the new one.

The old function stays production-reachable as the fallback arm, so it is not
dead code and its tests still exercise the real path. The fallback also has a
behavioural job: returning `None` when nothing is in reach would read to
`npc_ai_fight` as "all cooling, hold fire" and freeze a melee-only NPC, where
returning the out-of-reach pick lets `ability_ranges` report its short
`max_range` and the existing out-of-range arm chase (mobile) or
`stationary_holds` (pinned).

**Revert-verify each arm separately.** The filter and the fallback fail
different guards; reverting the whole commit at once hides which guard tracks
which arm. See [[npc-ai-fight-test-fixtures]].

Related: [[stat-with-no-consumer-trap]], [[entity-template-seed-authoring]]
