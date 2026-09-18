---
name: region-trigger-world-scoping
description: Block on sight — client-hinted region ids are client-supplied and the server's region lookup is global, not world-scoped, so any enter_region chain is reachable from any world
metadata:
  type: project
---

**The `enter_region` content trigger has no world scoping anywhere in the chain, and the region id it keys off is a client-supplied integer.** Verified 2026-09-17 during the Harset H10 seed review.

The path: `triggerClientHintedGenericRegion` (`crates/services/src/cell/cell_methods/player/world/mod.rs:54-114`) reads `region_id` out of the client's arg blob, resolves it with `SpaceManager::get_region` (`space_manager/queries.rs:190-192`) — a **flat `HashMap<runtime_id, RegionData>` spanning every world**, not the player's space — and fires `fire_enter_region(tag)` with the resolved `point_sets.name`. The only world filter in the system is on the *outbound* hint burst: `player_init/mod.rs:365-370` sends the client only `regions_for_world(world) & REGION_FLAG_CLIENT_HINTED`. Inbound is unfiltered.

Downstream there is nothing to catch it either:

- `Trigger::OnRegionEnter::matches` (`content-engine/src/triggers/matching.rs:95-99`) compares `region_key` to the event's `region_key` string and nothing else.
- `fire_enter_region` **does** put `world_name` in the `ExecutionContext` (`cell/content/event_dispatch/region.rs:37-40`) — but no condition type can read it. The loader (`content-engine/src/loader/condition.rs`) has exactly six arms: `mission_status`, `step_status`, `archetype`, `objective_status`, `counter`, `stat_below_max`. `Condition::PropertyEquals` exists in the enum with **no loader arm**, so a seed row naming it is dropped with a `warn!`.
- `content_chains.scope_type` / `scope_id` is documentary; the resolver never reads it.

So "region keys are byte-distinct, therefore cross-firing is impossible" is true for *honest* clients and false for crafted packets. Impact is usually low (it opens a door the player could walk through anyway), but rate it per chain: an `enter_region` chain whose action grants, completes or gates something is remotely triggerable from any world.

**How to apply:** treat the missing `world_name` loader arm as the real fix and say so when reviewing any new `enter_region` chain. The integration request is one arm mapping `"world_name"` → `Condition::PropertyEquals { property: "world_name" }`; the context param is already populated for both enter and exit. Until it exists, don't let an `enter_region` chain be the only gate on anything valuable.

Related: `RegionEnter` and `RegionExit` are separate `TriggerType`s and `fire_exit_region` is a separate call, so an `enter_region` trigger genuinely cannot fire on exit — that half of the Python `args['entering']` check is already honored.

See [[harset-travel-ground-truth]] for the two Harset door chains this was found on, and [[cross-world-teleport-arrival-path]] for what happens at the far end.
