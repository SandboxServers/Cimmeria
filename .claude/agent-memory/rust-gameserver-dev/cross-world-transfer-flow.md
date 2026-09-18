---
name: cross-world-transfer-flow
description: Cell/base split of the gate-travel flow — where teardown actually happens, why find_or_create_space can't join an instance, and which "default instance" mechanisms are fake.
metadata:
  type: project
---

Learned while implementing P45 (legacy-command-parity G05). Verify against
the code before acting on any of it — these are structural claims, not
line references.

## `handle_gate_travel` is the back half, not the transfer

`crates/services/src/base/world_entry/gate_travel/mod.rs::handle_gate_travel`
does **not** remove the entity from its old space. Its doc comment says so:
"The CellService has already removed the entity from its old space." The
destructive `space_mgr.destroy_entity(entity_id)` lives in the *cell*, in
each of the five callers that emit `CellToBaseMsg::GateTravel`:
`cell/gate_travel.rs` (stargate), `cell/cell_methods/gm/travel.rs`
(`gmGotoLocation`), `cell/content/executor/transport.rs` (chain teleport),
`cell/ring_transport/dispatch.rs`, `cell/cell_methods/player/combat/respawn.rs`.

**Why it matters:** any requirement of the form "validate before tearing
down the entity's space/AoI state" must be implemented cell-side. A
base-side wrapper is structurally too late.

**Correct ordering for a new cell-side caller:** validate → flush
bandolier ammo → `tx.send(GateTravel)` *checked* → for a **player** subject,
`cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx,
space_mgr)` → `destroy_entity`. Skipping the trade-cleanup step strands the
subject's trade partner with a dangling reference to a freed entity id —
`destroy_entity` alone does not clean trade state (see
`server-authority-enforcer/reference_cell_teardown_skips_session_state.md`).
Keep the failed-send early-return's behavior unchanged (a rejected send means
nothing happened, so there's nothing to clean up). `gmGotoLocation` is the
precedent that got the ordering right.

## `find_or_create_space` cannot join an existing instance

`SpaceManager::find_or_create_space` returns the startup space for a
non-instanced world, but for an **instanced** world it *always allocates a
brand new space* ("Instanced: always create a fresh space — do NOT cache in
world_spaces"). So resolving a destination by world name alone can never
land you in another player's instance. Use
`create_entity_in_space(entity_id, space_id, ...)` with an exact id, and
re-validate that id on arrival — an instanced space is destroyed the moment
its last player leaves (`destroy_entity` reaps it), which can happen while a
cell→base→cell round-trip is in flight.

## Two fake "default instance" mechanisms — don't reuse either

- `base/world_entry/space_registry.rs::resolve_space_id_fallback` is a
  **hardcoded three-entry match** (`Castle_CellBlock`/`SGC_W1`/`CombatSim`)
  whose catch-all warns and returns `DEFAULT_SPACE_ID`. It is a degraded
  last-resort path, not a resolver.
- `space_registry::register_space` writes a `world_name → space_id` map that
  **nothing reads**. The base has no live world→space index.

The authoritative world/space tables live in the cell's `SpaceManager`
(`worlds`, `world_spaces`, `spaces`). Resolve there.

## `BaseToCellMsg::CreateEntity.reply_tx` has no failure channel

It is `oneshot::Sender<u32>`. When the cell's create fails it drops the
sender, and the base's `reply_rx.await` `Err` arm falls back to
`resolve_space_id_fallback` — producing a world-entry packet for a space the
entity is not in (an "un-spaced" player). Widening it to
`Result<u32, String>` is ~4 call sites and is the real fix if you ever need
to make a create failure honest.

## Disconnect ordering you can rely on

`base/helpers/mod.rs::destroy_client_entities` removes the `entity_to_addr`
mapping **before** it queues `BaseToCellMsg::DisconnectEntity`. That is why
a `GateTravel` arriving after a disconnect fails at `handle_gate_travel`'s
first statement (the addr lookup) instead of resurrecting the entity. It is
load-bearing but incidental — don't reorder it, and don't assume a similar
handler is safe just because this one is.

`CreateEntity` and `DisconnectEntity` share one FIFO `BaseToCellMsg`
channel, so a disconnect racing an in-flight create round-trip can have its
`DisconnectEntity` processed *first* and leave a clientless ghost entity in
the destination space. Re-check the session after the round-trip.
