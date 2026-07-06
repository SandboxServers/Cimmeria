# CAT-D — Inventory / Items — Findings

**Overall trust posture.** The MoveItem and RemoveItem core paths are surprisingly
well-defended: per-player and per-container `pg_advisory_xact_lock` plus
`FOR UPDATE` row locks plus a UNIQUE INDEX on `(character_id, container_id, slot_id)`
make the inventory-mutation flow concurrency-safe at the SQL boundary, and
ownership is uniformly keyed off `character_id = $player_id` (server-resolved).
The cell-method dispatcher in `connect_loop::cell_arms` correctly substitutes
the server-trusted `player_eid` for the wire-supplied `entity_id` prefix, so
cross-player actor spoofing on cell methods is blocked at the framing layer.

The serious gaps are around (1) bandolier ammo persistence using the
**type_id** as its TOCTOU guard instead of the unique inventory **item_id**
— the textbook same-type-swap ammo-duplication shape mentioned in the agent
brief; (2) `lootItem` having no range/state recheck (the only gate is the
in-memory `looting_entity` pin which is set by a range-checked `interact()`
but never re-validated for distance, line-of-sight, alive-state, or that the
corpse hasn't moved between window-open and item-take); (3) zero
loot-reservation / group-loot enforcement (any witness who can get the
`interact()` to succeed claims the whole drop); (4) zero protection against
dropping `bound` (no-drop) items; (5) `requestAmmoChange` accepts any
positive ammo type for "custom items the loader skipped" — a forged item_id
referencing an unmapped weapon bypasses the whitelist entirely.

GetItemInfo / requestItemData are emitted by the client (RTTI present) but
not handled server-side at all — info-disclosure attempts no-op. RepairItem
(singular, inventory dispatch index 40) is `UNIMPLEMENTED` and logs only;
the real repair flow is the vendor-domain `repairItems` (CAT-E). GMRemoveItem
(GM-only spawn/destroy variant) has no server handler.

---

### CAT-D-01 — Bandolier ammo persistence keyed by `type_id`, not `item_id` — same-type-swap dupe

**Severity**: High
**Class**: TOCTOU / Same-type swap ammo overwrite
**Wire surface**: `Event_NetOut_RequestAmmoChange`, `Event_NetOut_RequestActiveSlotChange`,
  cell-tick reload completion → `CellToBaseMsg::BandolierAmmoUpdate`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The cell sends `BandolierAmmoUpdate { player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type }`
expecting the SQL helper to refuse the write if the slot's instance has
been swapped. The field is named `expected_item_id` and the wire-comment in
`CellToBaseMsg::BandolierAmmoUpdate` says it "guards against TOCTOU" — but
the cell populates it from `BandolierItem.item_id`, which is documented in
`crates/entity/src/cell_entity/mod.rs:743-745` as "Item design ID" (i.e. the
`type_id`, not the unique `sgw_inventory.item_id` row id). The base-side
SQL guard at `crates/services/src/base/world_entry/methods/inventory/ammo.rs:25-29`
binds it against `type_id = $5`. When a player has two distinct bandolier
instances of the same weapon **type** (e.g., two pistols, different ammo
loads), any swap between them satisfies the `(slot_id, type_id)` predicate
even though it's the wrong row, and the in-flight UPDATE scribbles one
instance's ammo onto the other. This is the **bandolier-ammo-via-same-type-swap
TOCTOU** the agent brief calls out by name as the canonical SGW exploit shape.

**Evidence**
- Ghidra: `019b430c` `Event_NetOut_RequestAmmoChange` + `019be2bc`
  `Event_NetOut_RequestActiveSlotChange` — client wire surfaces that
  drive the swap. Reload-completion ticks also produce the same
  `BandolierAmmoUpdate` shape server-side.
- DB schema: `db/sgw/Inventory/Tables/sgw_inventory.sql:6-22` — `item_id`
  is the unique sequence-default PK ("the inventory instance"); `type_id`
  is the design id ("the item type"). The unique index
  `sgw_inventory_unique_slot` is on `(character_id, container_id, slot_id)`,
  NOT on `type_id`, so two same-type instances are perfectly representable
  (in different slots).
- Cross-ref to Rust (for fix author): `crates/services/src/base/world_entry/methods/inventory/ammo.rs:25-36`
  (the SQL) and `crates/services/src/cell/cell_methods/inventory/bandolier.rs:39-46`
  / `:645-653` (the cell senders).

**Attack scenario**
1. Player A has two pistols of `type_id = 3241` in bandolier slots 0 and 1.
   Slot 0's instance (item_id=A) has 15 rounds of ammo type X; slot 1's
   instance (item_id=B) has 0 rounds of ammo type Y.
2. Player fires from slot 0 down to 5 rounds. `bandolier_ammo_dirty` marks
   slot 0. The cell decides to flush at the next swap/logout boundary.
3. Player swaps to slot 1 (RequestActiveSlotChange). The cell snapshots
   slot 0's state at swap time, then sends `BandolierAmmoUpdate { slot_id=0,
   expected_item_id=3241, current_ammo=5, cur_ammo_type=X }`.
4. Before the base processes that message, the player executes a MoveItem
   that swaps slot 0's row (item_id=A, ammo=5) with the main-bag instance
   of a *third* pistol of same type 3241 (item_id=C, ammo=30).
5. Base processes the queued `BandolierAmmoUpdate`. WHERE
   `(character_id, container_id=3, slot_id=0, type_id=3241)` matches —
   but it now matches item_id=C, not A. The write overwrites C's
   `ammo=30, cur_ammo_type=...` with `ammo=5, cur_ammo_type=X`.
6. Player swaps back. Slot 0 now has the formerly-30-round pistol with
   ammo=5; the formerly-5-round one (A) was moved away with its old
   ammo=15 unchanged. Net: 15 + 30 → 15 + 5 isn't a dupe, but the
   inverse direction (snapshot the LOW state, race a swap that brings a
   HIGH-ammo instance into the slot, then race a reload completion that
   brings a 5→full FULL state forward) lets an attacker shape arbitrary
   ammo writes onto arbitrary same-type instances. Per the agent brief's
   own articulation, this is the "ammo duplication via same-type swap
   TOCTOU" shape.

**Suggested remediation (one line)**
Add a `sgw_inventory.item_id` field (instance id) to the `BandolierAmmoUpdate`
message and change the SQL `WHERE` to key on the unique row id rather than
`type_id`. Consult items-systems-advisor on whether the cell-side
`BandolierItem` struct should also carry the instance id explicitly (it
currently only carries the design id), and whether renaming the
mis-described `expected_item_id` field is worth the churn.

**Would benefit from x64dbg trace?**
Yes — verifying the live race window between a `requestAmmoChange` and an
adjacent MoveItem requires watching the Mercury bundle order under a real
client connection; pure unit/live-DB tests can only prove the SQL key is
wrong (they can't reproduce the racing client ordering).

---

### CAT-D-02 — `lootItem` does not recheck range, line-of-sight, or alive-state

**Status**: ✅ PARTIALLY RESOLVED (#446) — `handle_loot_item`
(`cell/interactions/loot.rs`) now re-validates the looter's live distance
to the corpse against `MAX_INTERACT_DISTANCE` on **every** `lootItem`
call, denying out-of-range takes (drop preserved, nothing granted,
`warn!` logged). This closes the position-spoof "vacuum loot" chain — the
highest-impact half of the finding. **Still open**: kill-credit / loot
ownership (a 0-damage player can still loot a corpse they walk up to) and
LOS/alive-state rechecks — the larger SGW lootability-window model, routed
through the combat-systems advisor as the #446 follow-up.

**Severity**: High
**Class**: Missing range/state recheck on stateful action
**Wire surface**: cell method 84 (`lootItem`) — index dispatched from
  `Event_NetOut_LootItem` / `register_NetOut_LootItem` at Ghidra `00d935a0`
**Demonstrable / Likely-theoretical**: Likely-theoretical (no client trace
  for the abuse, but the missing checks are obvious from the handler)

**Trust violation**
`handle_loot_item` in `crates/services/src/cell/interactions/loot.rs:87` gates
solely on `space_mgr.get_entity(entity_id).looting_entity` — set previously
by `handle_interact` only after a 5-unit distance check. After the pin is
set, the looter is free to walk arbitrarily far from the corpse, get
crowd-controlled, or even die, and still drain the loot list one item per
`lootItem(index)` packet. The corpse is identified by `looting_entity`, so
the looter has effectively a permanent pointer at it until the corpse
despawns. There is no per-`lootItem` distance recheck, no alive-check on
the looter, no LoS check, no faction check, and no recheck that the
corpse is still interactable (`INT_NormalLoot` still set).

**Evidence**
- Ghidra: `00d935a0` `register_NetOut_LootItem` data ref → typed emit info
  at `00d93680` — the wire surface is just `lootItem(index : INT32)`.
- Behavioral log: n/a (no observable client-side restraint in the wire
  shape; the client UI's "loot all" button is the only natural caller).
- Cross-ref to Rust (for fix author): `crates/services/src/cell/interactions/loot.rs:87-145`
  and `crates/services/src/cell/interactions/dispatch.rs:172-184` for the
  preceding `interact()` distance check.

**Attack scenario**
1. Player A kills a high-value mob; `handle_interact()` pins
   `looting_entity = corpse_eid` after the 5-unit range check.
2. Player A runs 100 units away, into a safe area / out of LoS.
3. Player A spams `lootItem(index)` for each remaining drop. Each one
   succeeds because the handler only checks `looting_entity.is_some()`.
4. Observable effect: a player can vacuum corpses without ever needing
   to stand near them — useful for kiting champions across hostile
   terrain, looting from inside a vehicle/transporter, looting after
   dying (looter has no alive-check), or looting an instance you've
   been kicked from but were briefly pinned to.

**Suggested remediation (one line)**
Recheck `(distance(player, looting_entity_pos) <= MAX_INTERACT_DISTANCE)` AND
`player.is_alive()` AND `corpse.interaction_type_flags & INT_NORMAL_LOOT != 0`
at the top of `handle_loot_item`, before touching the loot list.

**Would benefit from x64dbg trace?**
No — the missing checks are static; the test that fails on revert is a
live-DB integration test that drives `handle_interact` then walks the
player and asserts `handle_loot_item` rejects.

---

### CAT-D-03 — Zero loot-reservation / group-loot ownership on corpse drops

**Severity**: High
**Class**: Loot fairness / ownership
**Wire surface**: `Event_NetOut_LootItem` (cell method 84)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The corpse-side `LootItem` rows carry only `(design_id, quantity, index)`
— no `killer_player_id`, no `reserved_until`, no group-mode discriminator,
no per-item per-player claim. Any player who can pass the `interact()`
distance check is then free to drain the entire loot pile via `lootItem`
calls. This contradicts every standard MMO loot model (FFA, Round-Robin,
Master Looter, Need-Before-Greed, Group). The repo has `SquadSetLootMode`
in CAT-M, implying a loot-mode setting exists at the squad/social layer,
but nothing wires that mode into the corpse's loot dispatch.

**Evidence**
- `crates/entity/src/cell_entity/mod.rs:725-739` — `LootItem` struct has
  no ownership / reservation fields.
- `crates/services/src/cell/interactions/loot.rs:122-145` — handler
  removes by `index` only; no claim check.
- `Grep loot_reservation|loot_owner|reserved_for|killer|group_loot` over
  `crates/services/src/cell/interactions/`: zero matches.

**Attack scenario**
1. Player A is in a fireteam pulling a champion. Their squad's
   `SquadSetLootMode` is set to "Round Robin" or similar in the UI.
2. The champion dies. Player B (any nearby witness, not in the squad)
   walks into 5-unit range of the corpse.
3. B's `interact()` succeeds (no faction/squad/raid-membership gate),
   pinning `looting_entity` on B. B sends `lootItem(0..n)` for every drop.
4. Observable effect: the squad gets zero of the drops; B vacuums the
   corpse with no consent. With CAT-D-02 stacked, B can do this from
   100+ units away.

**Suggested remediation (one line)**
Stamp the killer's player_id (and squad_id when in a group) onto the
`LootItem` row at drop generation, and reject `lootItem` calls from
non-eligible looters with a wire-visible reason; consult
items-systems-advisor for the exact reservation matrix (FFA / RR / NBG /
ML) that matches the legacy design intent.

**Would benefit from x64dbg trace?**
No — the missing field/state is static.

---

### CAT-D-04 — `RemoveItem` accepts any positive quantity from the client with no bound-item gate

**Severity**: Medium
**Class**: Missing server-side authorization on destructive action
**Wire surface**: `Event_NetOut_RemoveItem` (cell method 36, `removeItem`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The cell-side `handle_remove_item` (item_ops.rs:27) reads
`(item_id: i32, quantity: i16-as-i32)` straight off the wire and forwards
to base. Base's `handle_remove_inventory_item` only checks
`quantity > 0`, `character_id = $player_id`, and `quantity ≤ stack_size`
— there is **no** check that the item is unbound (`bound = false`), no
mission-protection check (mission-pinned quest items can be silently
deleted), and no rate-limit on how often `removeItem` can be issued. Per
the legacy python `SGWPlayer.removeItem` (`deprecated/python/cell/SGWPlayer.py:2137`)
this matches a player-driven "drop item" UI affordance, but the python
also exposed it without a bound check; the wire surface is therefore
fully trusted to gate destruction client-side. A modified client can
delete the player's own quest tokens, breaking missions in
content-engine-dependent ways. (No cross-player effect — `character_id =
$1` clamp is intact.)

**Evidence**
- Ghidra: `019be770` `Event_NetOut_RemoveItem` → `register_NetOut_RemoveItem`
  at `00d94560`. Wire payload is `(item_id : INT32, quantity : INT16)`.
- Behavioral log: n/a.
- Cross-ref to Rust (for fix author): `crates/services/src/cell/cell_methods/inventory/item_ops.rs:27-60`
  and `crates/services/src/base/world_entry/methods/inventory/core/remove_instance.rs:26-100`.

**Attack scenario**
1. Player has a `bound = true` quest token in mission bag (container 2).
2. Modified client sends `Event_NetOut_RemoveItem(item_id, 1)`.
3. Server deletes the row, fires `onRemoveItem`. Mission is now
   unable to advance because the chain's `RemoveItem` action will
   no-op (no instance to consume) and `OnItemUse` events never fire.
4. Observable effect: durable self-griefing or, if mission rewards
   include re-grants, abuse of the re-grant loop (combined with vendor
   sells, etc).

**Suggested remediation (one line)**
Reject `removeItem` when the source row has `bound = true` (or when its
`type_id`'s row in `resources.items` flags it as mission-pinned /
no-drop); consult items-systems-advisor for the canonical bound /
no-drop predicate.

**Would benefit from x64dbg trace?**
No — bug is static.

---

### CAT-D-05 — `useItem` accepts an unvalidated `target_id` and forwards it through the outbox + cell

**Severity**: Medium
**Class**: Unvalidated trust on side-channel field
**Wire surface**: `Event_NetOut_UseItem` (cell method 39, `useItem`)
**Demonstrable / Likely-theoretical**: Demonstrable (the field is read but
  currently discarded by chain code — fix-it-now-or-it-bites-later shape)

**Trust violation**
The wire payload is `(item_id : INT32, target_id : INT32)`. The cell handler
(`item_ops.rs:144-153`) forwards `target_id` verbatim to base.
`handle_use_inventory_item` plumbs it into `CellOutboxPayload::ItemUsed`
and dispatches as `BaseToCellMsg::ItemUsed { ..., target_id }`. The cell's
`fire_item_use` (`content/event_dispatch/inventory.rs:28-73`) currently
sets `target_entity: None` in the trigger event, **so the field is
functionally dead today**. But (a) it's persisted in `cell_event_outbox`
across a server restart, and (b) any future content action that pulls
`target_id` out of `ItemUsed` will inherit the trust violation: the
client supplies an arbitrary entity id with zero ownership / range /
perception check. If a future chain uses `target_id` to e.g. apply a
buff or trigger an effect, the player can target arbitrary entities
(including other players, NPCs in a different cell, or invalid IDs).

**Evidence**
- Ghidra: `019be70c` `Event_NetOut_UseItem` → `register_NetOut_UseItem`
  at `00d94020`. Wire shape `(item_id, target_id)`.
- Cross-ref to Rust (for fix author): `crates/services/src/cell/cell_methods/inventory/item_ops.rs:137-174`
  and `crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs:80-263`,
  outbox enqueue at `:230-252`.

**Attack scenario**
1. A future content action `ApplyEffectToTarget` is added that reads
   `target_id` from `ItemUsed`.
2. Player crafts a `useItem(consumable_with_that_chain, target_id=other_player_eid)`.
3. The chain fires `ApplyEffectToTarget(other_player_eid, ...)` with no
   range / LoS / perception / faction check; the effect is applied
   server-side.
4. Observable: a "useItem" affordance becomes a cross-player effect
   delivery vector.

**Suggested remediation (one line)**
Either drop `target_id` at the cell decoder and pass `None` to base, or
validate against the player's perception list + range gate at the cell
handler before forwarding — match the validation shape `handle_interact`
already enforces.

**Would benefit from x64dbg trace?**
No — static.

---

### CAT-D-06 — `requestAmmoChange` falls through to "any positive ammo_type" when the weapon's `item_defs` cache row is missing

**Status**: ✅ RESOLVED (#448) — `handle_request_ammo_change`
(`cell/cell_methods/inventory/bandolier/ammo_change.rs`) now fails closed
on an `item_defs` cache miss: the request is rejected with a
`reason = "weapon_def_cache_miss"` warn instead of skipping the
whitelist. Every ammo-bearing weapon is in the cache (`load_item_defs`
selects `WHERE clip_size > 0`), so a miss means either a forged request
for a non-weapon or a broken cache load — never a legitimate swap.
Regression guard: `request_ammo_change_rejects_missing_weapon_def`
(fails when the fall-open is reinstated).

**Severity**: Medium
**Class**: Missing whitelist on edge case
**Wire surface**: `Event_NetOut_RequestAmmoChange` (cell method 42)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_request_ammo_change` (`bandolier.rs:518-712`) validates
`ammo_type` against the weapon definition's `allowed_ammo_types` set,
but only **if** `space_mgr.item_defs.get(&item_id).cloned()` returns
`Some`. The comment at lines 558-561 acknowledges that "`None` means the
cache had no entry (custom item the loader skipped), in which case we
accept any positive ammo_type." This is a wide-open hole: any
`type_id` that the loader doesn't populate into `item_defs` becomes a
whitelist-bypass vector. The hole is multiplied by the upstream lookup
being keyed on `item_id` (which `requestAmmoChange` carries) — so the
attacker controls which cache lookup happens. A forged or
loader-missing-by-design item type will silently pass through the
otherwise-correct whitelist.

**Evidence**
- Cross-ref to Rust (for fix author): `crates/services/src/cell/cell_methods/inventory/bandolier.rs:555-619`.
  Specifically the comment block at 555-561 acknowledges the fall-through
  semantics; the resulting `if let Some(def) = weapon_def.as_ref()` block
  at 606-617 is the gate that's bypassed when the cache miss occurs.
- DB schema: `db/sgw/Inventory/Tables/sgw_inventory.sql:20` —
  `cur_ammo_type` only has `CHECK (cur_ammo_type >= 0)`, so a forged
  positive ammo_type WILL persist on commit if it gets that far. The
  cell-side rejection at line 545 (`ammo_type <= 0`) is the only
  remaining gate.

**Attack scenario**
1. Find a weapon `type_id` whose `resources.items` row exists but isn't
   loaded into the `item_defs` runtime cache at server startup (e.g., a
   data-load skip due to mis-named asset, a never-shipped weapon design
   that's in the seed but not in the loader's filter, or an item that
   becomes craftable after a hot-reload but isn't refreshed in cache).
2. Modified client sends `requestAmmoChange(item_id=that_weapon's_instance,
   ammo_type=arbitrary_positive_value)`.
3. Cell's `if let Some(def) = weapon_def.as_ref()` is skipped; only the
   `ammo_type > 0` check remains.
4. Server persists `cur_ammo_type = arbitrary_positive_value` via
   `BandolierAmmoUpdate`. The client UI may render incorrectly (per
   the existing TODO comment); downstream damage-type resolution may
   apply effects from a subtype the player shouldn't have been able to
   pick.

**Suggested remediation (one line)**
On `item_defs` cache miss, reject `requestAmmoChange` outright (warn-log
"unknown weapon type") rather than fall through to accept; consult
items-systems-advisor on whether the cache should be backfilled
lazily from `resources.items` or whether the cache-miss is a fatal
content-loading error.

**Would benefit from x64dbg trace?**
No — the fall-through is in the code comment block at 555-561.

---

### CAT-D-07 — `MoveItem` allows the client to move items into the buyback container (16)

**Severity**: Medium
**Class**: Container ACL — wire-controlled `target_container_id`
**Wire surface**: `Event_NetOut_MoveItem` (cell method 38)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`bag_max_slots(target_container_id)` returns nonzero for container 16
(`INV_BUYBACK`, 12 slots). The MoveItem handler accepts any
`target_container_id > 0` with `bag_max_slots(...) > target_slot_id` — it
does not maintain a whitelist of "containers a player is allowed to
move INTO via MoveItem." The only secondary gate is
`item_allows_container(pool, type_id, target_container_id)` which
consults the per-item `container_sets` array; if any seeded item has
`16` in its `container_sets`, the client can manually move that item
into the buyback container outside the normal vendor flow. Buyback rows
have special semantics (price, expiry) — landing a player-owned item
there outside a sell flow creates dangling state. INV_BANK (17),
INV_AUCTION (18), INV_TEAM_BANK (19), INV_COMMAND_BANK (20) are NOT
reachable because `bag_max_slots(17..=20)` returns 0 (default match arm),
so the slot-range check at move.rs:79 rejects.

**Evidence**
- `crates/services/src/base/resources/mod.rs:28-38` — `bag_max_slots(16) = 12`
  (the buyback container) but no flag distinguishes "system-managed" from
  "player-movable" containers.
- `crates/services/src/base/world_entry/methods/inventory/move_/mod.rs:64-91`
  — only checks `target_container_id > 0` and `slot_id` range; no
  player-movable whitelist.
- `crates/entity/src/inventory.rs:13-32` — defines `INV_BANK..INV_COMMAND_BANK`
  constants which are NOT in `bag_max_slots`'s match (return 0, so they're
  already blocked). Buyback is at 16 in both lists, but listed in
  `bag_max_slots` (so it IS reachable).

**Attack scenario**
1. Find any item whose `resources.items.container_sets` includes 16
   (any seed row with that — verify against `db/resources/Items/Seed/`).
2. Modified client sends `moveItem(item_id, target_container_id=16,
   target_slot_id=N, quantity=1)`.
3. Server moves the player's item into the buyback container outside
   the vendor sell path. Subsequent vendor open / buyback-reload logic
   sees the row and treats it as a buyback entry without the
   accompanying `sgw_buyback`-style price row.
4. Observable: the item appears in the buyback UI without a sell having
   happened; depending on how the buyback price/expiry are stored,
   either the player can "buy back" the item for free (dupe) or the
   row becomes inaccessible (item-loss). Either is a content-state
   corruption.

**Suggested remediation (one line)**
Add a `target_container_id ∈ {1, 2, 3, 4..=14, 15}` whitelist to the move
handler (player-movable containers only); system-managed containers
(16 buyback, 17 bank, etc.) should be mutated only by their owning
service paths; consult items-systems-advisor on the canonical "what's
movable by the player" rule.

**Would benefit from x64dbg trace?**
No — static check against the seed `container_sets` data is sufficient
to confirm the existence of an item that triggers the bypass.

---

### CAT-D-08 — `MoveItem` stack-split copies durability/charges/bound without re-validating against the source

**Severity**: Low
**Class**: Bound-flag drift on split
**Wire surface**: `Event_NetOut_MoveItem` split path (`quantity < stack_size`)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The split branch (move/mod.rs:289-369) INSERTs a new row carrying
`source.bound`, `source.durability`, `source.charges`. For *consumable
stacks* this is fine — splitting a stack of slappacks doesn't change
their bound-ness — but for a stack that legitimately had a bind-on-pickup
applied via a chain action between the FOR-UPDATE read and the split's
INSERT, the new row would carry the **pre-bind** bound flag if a race
fits in the gap. The advisory lock makes this hard to hit in practice
(per-player serialization), so this is filed as a Low; flagged primarily
so future refactors that loosen the lock don't accidentally regress
this.

**Evidence**
- `crates/services/src/base/world_entry/methods/inventory/move_/mod.rs:326-341`
  — the split insert binds `source.bound, source.durability,
  source.charges` from the row read at line 162.

**Suggested remediation (one line)**
None required today (per-player advisory lock closes the race). If the
lock is ever scoped down or removed, refactor the split to RETURNING-style
re-read or copy the durability/charges/bound straight from a
`SELECT ... FOR UPDATE` of the row that's about to be inserted-from.

**Would benefit from x64dbg trace?**
No.

---

### CAT-D-09 — `RemoveItem` quantity decoded as `i16` then sign-extended to `i32` — no upper-bound clamp on cell side

**Severity**: Low
**Class**: Wire-decoding integer-width mismatch (defense in depth)
**Wire surface**: `Event_NetOut_RemoveItem`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The cell-side decoder at `item_ops.rs:34-35` reads `quantity` as `i16`
then sign-extends to `i32`. The base-side validates `quantity > 0` and
`quantity ≤ source.stack_size`. The `i16` lift is fine in practice because
stack sizes never exceed `i16::MAX`, but the wire shape per the legacy
python (`SGWPlayer.py:2137` `removeItem(itemId, quantity)`) is an i32
quantity, not i16. If a future content path raises `stack_size > i16::MAX`,
the wire decode silently truncates / wraps and the player removes a
different quantity than they asked for. The base-side `quantity <= source.stack_size`
catches *invalid* values but not *truncated* ones — a player sending
quantity=70000 (legitimate for a large craft stack) would arrive at the
base as quantity = 4464 (70000 - 65536). Self-grief mostly, but the
wire shape is wrong relative to the legacy convention.

**Evidence**
- `crates/services/src/cell/cell_methods/inventory/item_ops.rs:34-35`.
- `deprecated/python/cell/SGWPlayer.py:2137` `removeItem(itemId, quantity)`
  uses an unannotated int; the def-file shape is i32 to match other
  inventory methods (`useItem` carries i32 itemID + i32 target_id).

**Suggested remediation (one line)**
Decode `quantity` as `i32` (read 4 bytes) instead of `i16`; verify the
wire-shape against `entities/defs/SGWInventoryManager.def` before
landing — the .def is the actual schema, items-systems-advisor can
confirm.

**Would benefit from x64dbg trace?**
Yes — a single intercepted `removeItem` packet shows the field width
unambiguously. The decision tree is "what does the client actually send?"
not "what should it send?".

---

## Not Filed (considered but did not meet the bar)

- **Player can swap bandolier slots while stunned / mid-cast** — combat
  state has no defense in `handle_request_active_slot_change`. Reason
  not filed: the legacy convention doesn't appear to enforce a
  slot-swap stun gate either, and combat-systems-advisor owns the
  stun-vs-action-eligibility matrix. Worth a heads-up to that advisor
  but not a server-authority finding per se.

- **`useItem` doesn't decrement charges/uses on the cell side** — the
  consume path is the chain's `Action::RemoveItem`. Reason not filed:
  this is the intended decoupling (per the module comment block at
  use_instance.rs:1-22) and the agent brief's "no double-consume"
  invariant is upheld — `remove_item` + `UseInventoryItem` don't both
  run on the same instance; the chain decides.

- **`requestAmmoChange` carries no slot id, ambiguates when same item_id
  in multiple slots** — already handled by the
  `multiple slots hold this item_id → reject` branch at
  `bandolier.rs:580-598`. Reason not filed: defense exists. The
  underlying protocol shape is awkward but the cell does the right
  thing.

- **`GetItemInfo` / `requestItemData` info disclosure** — the Ghidra RTTI
  shows the class exists (`019b3090` / `019bccb0`) but no
  `register_NetOut_*` symbol, no handler index, and the Rust
  dispatcher has no arm. The client builds the typeinfo but doesn't
  emit. Reason not filed: no observable wire surface today; if a
  future patch exposes `getItemInfo(targetPlayerId)` then re-audit.

- **GMRemoveItem** — listed in CAT-N; in this category we observe that
  no server-side handler exists, so the GM destruction path is
  currently unwired. Reason not filed: CAT-N owns GM-gating; nothing
  to authorize here yet.

- **`RepairItem` (cell method 40, singular)** — handler logs
  "UNIMPLEMENTED" and returns. Reason not filed: there's no trust
  violation in code that does nothing; flag for review at
  implementation time. Worth a comment in the handler that it must be
  vendor-gated when implemented.

- **`ListItems` (cell method 37)** — passes server-resolved `player_id`
  through to a self-only `SELECT ... WHERE character_id = $player_id`.
  Reason not filed: validation is uniformly applied; the wire payload
  carries no targetable field, so info-disclosure is structurally
  impossible.

- **`LootItem` index out-of-range** — handler rejects unknown index
  cleanly (loot.rs:139-143). Reason not filed: defense exists.

- **`MoveItem` source-item ownership** — gated by `character_id = $1`
  on the FOR-UPDATE source query at move.rs:162-169. Reason not filed:
  defense exists.

- **Bandolier `requestActiveSlotChange` slot-range** — explicitly
  range-checked against `bag_max_slots(3)` at bandolier.rs:119-130 with
  a saturating_sub guard against `i32::MIN`. Reason not filed: defense
  exists.
