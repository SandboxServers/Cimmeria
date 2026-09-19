---
name: items-event-sets-dual-purpose
description: items_event_sets is NOT uniformly legacy — event_id 6/7 (melee/ranged weapon auto-attack binding) is live and load-bearing; only event_id 5 (item-use-ability) is unused. Corrects the blanket claim in item_use_trigger_mechanism.md.
metadata:
  type: project
---

Confirmed 2026-09-18 during a read-only audit of the SGW handoff pack v1.2's weapon/ammo data against our own seed + runtime (`docs/analysis/sgw-handoff-pack-v1.2/pack/`).

**`resources.items_event_sets (item_event_id, item_id, ability_id, event_id)` has (at least) three distinct `event_id` consumers, with very different liveness:**

- `event_id = 5` (`EVENT_ITEM_USE_ABILITY`) — **not read anywhere in `crates/services` or `crates/content-engine`** for the UseInventoryItem flow. That flow uses `content_triggers(item_use, <item_id>)` instead — see [[item_use_trigger_mechanism]]. This is the part that's genuinely legacy/dead.
- `event_id = 6` (`EVENT_ITEM_MELEE`) and `event_id = 7` (`EVENT_ITEM_RANGED`) — **fully live.** Loaded at cell startup into `space_mgr.item_event_set_abilities: HashMap<(item_id,event_id), ability_id>` by `load_item_event_set_abilities` (`crates/services/src/cell/spawner/abilities.rs:317`). Consulted by:
  - `crates/services/src/cell/abilities/use_ability/weapon_redirect.rs` — rewrites the universal archetype-default ranged starter (ability 592 "Pistol Shot") to the active weapon's real ranged binding at fire time (fixes "P90 fires using the pistol shot ability").
  - `crates/services/src/cell/abilities/resolve.rs` (`ability_for_item`, `ability_for_active_weapon`) — the right-click-hostile-NPC auto-attack path, falls back to 592/594 on a lookup miss.
  - `crates/services/src/cell/cell_methods/inventory/bandolier/active_slot.rs:577+` — probes RANGED/MELEE/USE_ABILITY on slot-swap.
- Constants live in `crates/services/src/cell/spawner/abilities.rs:16-28`: `EVENT_ITEM_EQUIP=4000`, `EVENT_ITEM_UNEQUIP=4001`, `EVENT_ITEM_RELOAD=4002`, `EVENT_ITEM_USE=4003` (these four are a *different* numbering scheme, for `getItemSequence`/kismet animation lookups, not `items_event_sets` rows) vs. `EVENT_ITEM_USE_ABILITY=5`, `EVENT_ITEM_MELEE=6`, `EVENT_ITEM_RANGED=7` (these three ARE the `items_event_sets.event_id` values). Don't conflate the two numbering schemes — the 4000-series and the 5/6/7 values look like they could be related but are unrelated lookup tables (`event_sets_sequences`/`sequences` vs `items_event_sets`).

**Verification method:** cross-checked all 1,278 weapon/related-equipment records in the handoff pack's `weapons_full_recovered_variants.json` (which independently derived "Legacy Event6/Event7 Ability IDs" from raw client monikers) against a full parse of our own `db/resources/Items/Seed/items_event_sets.sql` — 1,274/1,274 non-empty bindings matched exactly (item 21 SGHC 6 SMG: event 6→595, event 7→559, etc.). Also 100% match on `items.clip_size` and `items.ammo_types[]` (1,278/1,278). This confirms the table's data is intact and its melee/ranged rows are exactly what production code reads.

**Rule for future item-system reviews:** never say "items_event_sets is legacy" without naming the `event_id`. Ask which event_id a change touches before judging liveness — 5 is dead, 6/7 are load-bearing production data for every weapon's auto-attack in the game.

**Known pre-existing seed noise (not a bug to fix blindly):** 58/1,278 weapon records show the moniker-derived "expected" ability disagreeing with the actual `items_event_sets` row (e.g. item 3584 "CO2 Pistol Dartgun": moniker suggests melee ability 1087, but the seed row binds melee to 708, the generic Pistol Melee AA). Worst-hit families: Dart Gun (0/25 match), Ribbon Device (151/178), Grenade Launcher (25/27). Likely the same "orphaned generic placeholder" pattern documented in [[item_use_trigger_mechanism]] (ability 597 bound to unrelated Harset items) — original content authoring left some weapons on a generic fallback binding instead of a weapon-specific one. Don't silently "fix" these by trusting the moniker guess over the seed; flag for a content-authoring decision.

Related: [[project_pr520_bandolier_ammo_fix]] (the runtime consumer of `BandolierItem`, which is populated using this same event-binding data at equip time), [[vendor_trainer_seed_gap]] (a parallel finding on other item-adjacent tables being real-but-unseeded rather than legacy).
